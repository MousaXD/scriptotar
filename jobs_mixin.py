#!/usr/bin/env python3
from __future__ import annotations

from scriptotar_common import *


class JobsMixin:
    def _choose_output(self):
        path = filedialog.askdirectory(title="Choose output folder", initialdir=self.output_var.get() or str(Path.home()))
        if path:
            self.output_var.set(path)
            self._save_settings()

    def _new_job(self, input_type: str, source: str) -> dict:
        self._save_settings()
        return {
            "id": uuid.uuid4().hex[:12],
            "created_at": datetime.now().strftime("%Y-%m-%d %H:%M:%S"),
            "input_type": input_type,
            "source": source,
            "output_root": self.output_var.get().strip(),
            "model": self.model_var.get(),
            "device": self.device_var.get(),
            "language": self.language_var.get(),
            "quality": self.quality_var.get(),
            "cookies": self.cookies_var.get(),
            "max_duration_seconds": max_duration_seconds(self.max_duration_var.get()),
            "copy_source": bool(self.copy_source_var.get()),
            "translate": bool(self.translate_var.get()),
            "batched": bool(self.batched_var.get()),
            "batch_size": 8,
            "keep_failed": bool(self.keep_failed_var.get()),
            "project": self.project_var.get() or "Inbox",
            "status": "Queued",
            "progress": 0,
            "language_result": "",
        }

    def _enqueue(self, job: dict):
        self.jobs[job["id"]] = job
        self.queue_order.append(job["id"])
        self.queue_tree.insert(
            "", "end", iid=job["id"],
            values=("URL" if job["input_type"] == "url" else "File", job["source"], "Queued", "0%", ""),
        )
        self._record_history(job, "Queued")

    def _add_url(self):
        value = self.url_var.get().strip()
        if not value:
            return
        urls = re.findall(r"https?://[^\s<>]+", value)
        if not urls:
            messagebox.showerror(APP, "No URL found.")
            return
        for url in urls:
            self._enqueue(self._new_job("url", url.rstrip(".,);]}>") ))
        self.url_var.set("")

    def _add_clipboard_urls(self):
        try:
            text = self.clipboard_get()
        except tk.TclError:
            return
        urls = re.findall(r"https?://[^\s<>]+", text)
        if not urls:
            messagebox.showinfo(APP, "No URLs found in the clipboard.")
            return
        for url in urls:
            self._enqueue(self._new_job("url", url.rstrip(".,);]}>") ))

    def _add_files(self):
        paths = filedialog.askopenfilenames(
            title="Choose video files",
            filetypes=[("Video files", "*.mp4 *.mkv *.mov *.webm *.m4v *.avi"), ("All files", "*.*")],
        )
        for path in paths:
            self._enqueue(self._new_job("file", path))

    def _remove_selected(self):
        for item in self.queue_tree.selection():
            if item == self.current_job_id:
                continue
            self.queue_tree.delete(item)
            self.jobs.pop(item, None)
            if item in self.queue_order:
                self.queue_order.remove(item)

    def _clear_completed(self):
        for job_id in list(self.queue_order):
            job = self.jobs.get(job_id)
            if job and job["status"] in {"Done", "Failed", "Canceled"}:
                if self.queue_tree.exists(job_id):
                    self.queue_tree.delete(job_id)
                self.queue_order.remove(job_id)
                self.jobs.pop(job_id, None)

    def _update_tree(self, job_id):
        job = self.jobs.get(job_id)
        if not job or not self.queue_tree.exists(job_id):
            return
        self.queue_tree.item(
            job_id,
            values=(
                "URL" if job["input_type"] == "url" else "File",
                job["source"], job["status"], f"{int(job['progress'])}%", job.get("language_result", ""),
            ),
        )

    def _install_engine(self, after=False):
        if self.installing:
            return
        self.installing = True
        self.status_label.configure(text="Installing / repairing engine...")
        self.progress.configure(mode="indeterminate")
        self.progress.start(12)

        def task():
            try:
                DATA.mkdir(parents=True, exist_ok=True)
                if not VPY.exists():
                    subprocess.run([sys.executable, "-m", "venv", str(VENV)], check=True)
                commands = [
                    [str(VPY), "-m", "pip", "install", "--upgrade", "pip", "wheel"],
                    [str(VPY), "-m", "pip", "install", "--upgrade", "-r", str(REQS)],
                    [str(VPY), "-m", "pip", "check"],
                    [str(VPY), "-c", "import faster_whisper, yt_dlp; print(faster_whisper.__version__); print(yt_dlp.version.__version__)"],
                ]
                for command in commands:
                    self.events.put(("install_log", "$ " + " ".join(command)))
                    proc = subprocess.Popen(command, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
                    for line in proc.stdout:
                        self.events.put(("install_log", line.rstrip()))
                    if proc.wait() != 0:
                        raise RuntimeError(f"Engine setup command failed with exit code {proc.returncode}.")
                MARKER.write_text(ENGINE_VERSION + "\n", encoding="utf-8")
                self.events.put(("installed", after))
            except Exception as exc:
                self.events.put(("install_error", str(exc)))
            finally:
                self.events.put(("install_done", None))

        threading.Thread(target=task, daemon=True).start()

    def _start_worker(self):
        if self.worker and self.worker.poll() is None:
            return True
        if not self._engine_ready():
            self._install_engine(after=True)
            return False
        self.worker_ready = False
        self.worker = subprocess.Popen(
            [str(VPY), str(WORKER)],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
            bufsize=1,
            start_new_session=True,
        )

        def reader(proc):
            for line in proc.stdout:
                line = line.strip()
                if not line:
                    continue
                try:
                    self.events.put(("worker_event", json.loads(line)))
                except json.JSONDecodeError:
                    self.events.put(("worker_log", line))
            rc = proc.wait()
            self.events.put(("worker_exit", rc))

        self.worker_reader = threading.Thread(target=reader, args=(self.worker,), daemon=True)
        self.worker_reader.start()
        return True

    def _send_worker(self, payload):
        if not self.worker or self.worker.poll() is not None or not self.worker.stdin:
            raise RuntimeError("Worker is not running.")
        self.worker.stdin.write(json.dumps(payload, ensure_ascii=False) + "\n")
        self.worker.stdin.flush()

    def _start_queue(self):
        if self.current_job_id:
            return
        pending = any(self.jobs[jid]["status"] == "Queued" for jid in self.queue_order if jid in self.jobs)
        if not pending:
            self.status_label.configure(text="Nothing queued.")
            return
        if not self._start_worker():
            return
        if self.worker_ready:
            self._start_next_job()
        else:
            self.status_label.configure(text="Starting transcription worker...")

    def _start_next_job(self):
        if self.current_job_id:
            return
        next_id = next((jid for jid in self.queue_order if jid in self.jobs and self.jobs[jid]["status"] == "Queued"), None)
        if not next_id:
            self.status_label.configure(text="Queue finished.")
            self.cancel_btn.configure(state="disabled")
            return
        job = self.jobs[next_id]
        self.current_job_id = next_id
        job["status"] = "Starting"
        job["progress"] = 0
        self._update_tree(next_id)
        self.cancel_btn.configure(state="normal")
        self.progress.configure(mode="determinate", value=0)
        self.status_label.configure(text="Starting job...")
        self._record_history(job, "Running")
        self._send_worker({"command": "job", "job": {k: v for k, v in job.items() if k not in {"status", "progress", "language_result"}}})

    def _kill_worker_group(self):
        proc = self.worker
        if not proc or proc.poll() is not None:
            return
        try:
            os.killpg(os.getpgid(proc.pid), signal.SIGTERM)
        except ProcessLookupError:
            return
        except Exception:
            proc.terminate()
        deadline = time.time() + 2.0
        while proc.poll() is None and time.time() < deadline:
            time.sleep(0.05)
        if proc.poll() is None:
            try:
                os.killpg(os.getpgid(proc.pid), signal.SIGKILL)
            except Exception:
                proc.kill()

    def _cancel_current(self):
        if not self.current_job_id:
            return
        job = self.jobs.get(self.current_job_id)
        if job:
            job["status"] = "Canceled"
            self._record_history(job, "Canceled", error="Canceled by user")
            self._update_tree(job["id"])
        self.status_label.configure(text="Cancelling current job...")
        self._kill_worker_group()
        self.current_job_id = None
        self.worker = None
        self.worker_ready = False
        self.cancel_btn.configure(state="disabled")
        self.after(300, self._start_queue)

    def _handle_worker_event(self, event):
        kind = event.get("type")
        job_id = event.get("job_id")
        if kind == "ready":
            self.worker_ready = True
            self.status_label.configure(text="Worker ready.")
            self._start_next_job()
            return
        if kind == "log":
            self.status_label.configure(text=event.get("message", "Working..."))
            return
        if kind == "status":
            self.status_label.configure(text=event.get("message", "Working..."))
            return
        if kind == "job_started" and job_id in self.jobs:
            self.jobs[job_id]["status"] = "Running"
            self._update_tree(job_id)
            return
        if kind == "progress" and job_id in self.jobs:
            value = max(0.0, min(100.0, float(event.get("value", 0))))
            self.jobs[job_id]["progress"] = value
            self.jobs[job_id]["status"] = "Running" if value < 100 else "Finishing"
            self._update_tree(job_id)
            if job_id == self.current_job_id:
                self.progress.configure(value=value)
                self.status_label.configure(text=event.get("message", "Working..."))
            return
        if kind == "result" and job_id in self.jobs:
            job = self.jobs[job_id]
            job["status"] = "Done"
            job["progress"] = 100
            job["language_result"] = event.get("language") or ""
            job["output_dir"] = event.get("output_dir")
            job["transcript"] = event.get("transcript")
            self._update_tree(job_id)
            self._record_history(
                job, "Done", title=event.get("title"), language=event.get("language"),
                output_dir=event.get("output_dir"), transcript=event.get("transcript"),
            )
            self.last_output = event.get("output_dir")
            self.last_transcript = event.get("transcript")
            self.last_language = event.get("language")
            self._load_transcript(self.last_transcript, self.last_language, event.get("title"))
            self._refresh_library()
            self.current_job_id = None
            self.cancel_btn.configure(state="disabled")
            self.after(100, self._start_next_job)
            return
        if kind == "error":
            target = job_id or self.current_job_id
            if target in self.jobs:
                job = self.jobs[target]
                job["status"] = "Failed"
                job["progress"] = 0
                self._update_tree(target)
                self._record_history(job, "Failed", error=event.get("message"))
            self.status_label.configure(text=event.get("message", "Job failed"))
            if target == self.current_job_id:
                self.current_job_id = None
                self.cancel_btn.configure(state="disabled")
                self.after(100, self._start_next_job)

    def _drain_events(self):
        try:
            while True:
                kind, payload = self.events.get_nowait()
                if kind == "worker_event":
                    self._handle_worker_event(payload)
                elif kind == "worker_log":
                    self.status_label.configure(text=payload[-180:])
                elif kind == "worker_exit":
                    if self.current_job_id:
                        job = self.jobs.get(self.current_job_id)
                        if job and job["status"] not in {"Canceled", "Done", "Failed"}:
                            job["status"] = "Failed"
                            self._update_tree(job["id"])
                            self._record_history(job, "Failed", error=f"Worker exited with code {payload}")
                        self.current_job_id = None
                    self.worker = None
                    self.worker_ready = False
                    self.cancel_btn.configure(state="disabled")
                elif kind == "install_log":
                    self.status_label.configure(text=payload[-180:])
                elif kind == "installed":
                    self._refresh_engine_badge()
                    if payload:
                        self.after(200, self._start_queue)
                elif kind == "install_error":
                    messagebox.showerror(APP, "Engine setup failed:\n\n" + payload)
                    self.status_label.configure(text="Engine setup failed.")
                elif kind == "install_done":
                    self.installing = False
                    self.progress.stop()
                    self.progress.configure(mode="determinate", value=0)
                    self._refresh_engine_badge()
                elif kind == "research_item":
                    self._accept_research_item(payload)
                elif kind == "research_done":
                    self._finish_research(payload)
                elif kind == "research_error":
                    self.research_busy = False
                    self.research_status.configure(text=payload)
                    messagebox.showerror(APP, payload)
                elif kind == "ai_done":
                    self._finish_ai(payload)
                elif kind == "ai_error":
                    self.ai_busy = False
                    self.ai_status.configure(text=payload)
                    messagebox.showerror(APP, payload)
        except queue.Empty:
            pass
        self.after(100, self._drain_events)

    def _load_transcript(self, path, language=None, title=None):
        if not path or not Path(path).is_file():
            return
        text = Path(path).read_text(encoding="utf-8")
        self.transcript_text.delete("1.0", "end")
        self.transcript_text.insert("1.0", text)
        if hasattr(self, "ai_source_text"):
            self.ai_source_text.delete("1.0", "end")
            self.ai_source_text.insert("1.0", text)
            self._update_script_timer()
        tag = "rtl" if language == "ar" else "ltr"
        self.transcript_text.tag_add(tag, "1.0", "end")
        self.last_transcript = path
        self.last_output = str(Path(path).parent)
        self.last_language = language
        self.transcript_info.configure(text=f"{title or Path(path).parent.name}  •  {language or 'unknown language'}")

    def _save_transcript_edits(self):
        if not self.last_transcript:
            return
        text = self.transcript_text.get("1.0", "end-1c")
        Path(self.last_transcript).write_text(text + ("\n" if text else ""), encoding="utf-8")
        self.status_label.configure(text="Transcript edits saved.")

    def _copy_transcript(self):
        text = self.transcript_text.get("1.0", "end-1c")
        if text:
            self.clipboard_clear()
            self.clipboard_append(text)
            self.status_label.configure(text="Transcript copied to clipboard.")

    def _open_last_output(self):
        if self.last_output and Path(self.last_output).exists():
            subprocess.Popen(["xdg-open", self.last_output])

    def _open_selected_result(self):
        selection = self.queue_tree.selection()
        if not selection:
            return
        job = self.jobs.get(selection[0])
        if job and job.get("output_dir") and Path(job["output_dir"]).exists():
            subprocess.Popen(["xdg-open", job["output_dir"]])

    def _open_history_selected(self):
        selection = self.history_tree.selection()
        if not selection:
            return
        row = self._history_row(selection[0])
        if row and row[0] and Path(row[0]).exists():
            subprocess.Popen(["xdg-open", row[0]])

    def _load_history_transcript(self):
        selection = self.history_tree.selection()
        if not selection:
            return
        row = self._history_row(selection[0])
        if row and row[1] and Path(row[1]).is_file():
            title = self.history_tree.item(selection[0], "values")[1]
            self._load_transcript(row[1], row[2], title)
            self.notebook.select(self.transcript_tab)
