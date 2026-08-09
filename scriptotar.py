#!/usr/bin/env python3
from __future__ import annotations

import json
import os
import queue
import re
import shutil
import signal
import sqlite3
import subprocess
import sys
import threading
import time
import tkinter as tk
import uuid
from datetime import datetime
from pathlib import Path
from tkinter import filedialog, messagebox, ttk

APP = "Scriptotar"
VERSION = "1.1.0"
INSTALL = Path("/opt/scriptotar")
DATA = Path.home() / ".local" / "share" / "scriptotar"
LEGACY_DATA = Path.home() / ".local" / "share" / "wesamboss"
DEFAULT_OUTPUT = Path.home() / "Videos" / "Scriptotar"
LEGACY_DEFAULT_OUTPUT = Path.home() / "Videos" / "WesamBoss"
VENV = DATA / "venv"
VPY = VENV / "bin" / "python"
WORKER = INSTALL / "worker.py"
REQS = INSTALL / "requirements-engine.txt"
MARKER = VENV / ".scriptotar-engine-version"
SETTINGS_FILE = DATA / "settings.json"
DB_FILE = DATA / "history.sqlite3"
ENGINE_VERSION = "1.1.0-engine1"

BG = "#101217"
PANEL = "#191c22"
PANEL2 = "#22262e"
ENTRY = "#0c0e12"
TEXT = "#f3f5f7"
MUTED = "#a7afb9"
BORDER = "#343a44"
ACCENT = "#eceef1"
GOOD = "#8ad49f"
WARN = "#f2c97d"
BAD = "#ff9d9d"

DEFAULTS = {
    "output": str(DEFAULT_OUTPUT),
    "model": "medium",
    "device": "auto",
    "language": "auto",
    "quality": "720p",
    "cookies": "none",
    "max_duration": "60 min",
    "copy_source": True,
    "translate": False,
    "batched": False,
    "keep_failed": False,
}


def migrate_legacy_branding() -> None:
    """Migrate WesamBoss 1.1 user data into Scriptotar on first launch."""
    try:
        DATA.parent.mkdir(parents=True, exist_ok=True)
        if not DATA.exists() and LEGACY_DATA.is_dir():
            try:
                LEGACY_DATA.rename(DATA)
            except OSError:
                shutil.copytree(LEGACY_DATA, DATA)

        legacy_marker = VENV / ".wesamboss-engine-version"
        if legacy_marker.is_file() and not MARKER.exists():
            MARKER.write_text(legacy_marker.read_text(encoding="utf-8"), encoding="utf-8")

        if LEGACY_DEFAULT_OUTPUT.is_dir() and not DEFAULT_OUTPUT.exists():
            try:
                LEGACY_DEFAULT_OUTPUT.rename(DEFAULT_OUTPUT)
            except OSError:
                # Moving across filesystems can fail. Leave the existing output in place
                # rather than copying potentially many gigabytes during app startup.
                pass
    except Exception:
        # Branding migration must never prevent the application from starting.
        pass


def load_settings() -> dict:
    try:
        data = json.loads(SETTINGS_FILE.read_text(encoding="utf-8"))
        legacy_output = str(LEGACY_DEFAULT_OUTPUT)
        if data.get("output") == legacy_output and DEFAULT_OUTPUT.exists():
            data["output"] = str(DEFAULT_OUTPUT)
        return {**DEFAULTS, **data}
    except Exception:
        return dict(DEFAULTS)


def max_duration_seconds(label: str) -> int:
    return {
        "30 min": 1800,
        "60 min": 3600,
        "2 hours": 7200,
        "Unlimited": 0,
    }.get(label, 3600)


class App(tk.Tk):
    def __init__(self):
        super().__init__()
        self.title(f"{APP} {VERSION}")
        self.geometry("1080x780")
        self.minsize(920, 680)
        self.configure(bg=BG)
        self.protocol("WM_DELETE_WINDOW", self._on_close)

        migrate_legacy_branding()
        DATA.mkdir(parents=True, exist_ok=True)
        self.settings = load_settings()
        self.events = queue.Queue()
        self.worker = None
        self.worker_reader = None
        self.worker_ready = False
        self.installing = False
        self.current_job_id = None
        self.jobs: dict[str, dict] = {}
        self.queue_order: list[str] = []
        self.last_output = None
        self.last_transcript = None
        self.last_language = None

        self.url_var = tk.StringVar()
        self.output_var = tk.StringVar(value=self.settings["output"])
        self.model_var = tk.StringVar(value=self.settings["model"])
        self.device_var = tk.StringVar(value=self.settings["device"])
        self.language_var = tk.StringVar(value=self.settings["language"])
        self.quality_var = tk.StringVar(value=self.settings["quality"])
        self.cookies_var = tk.StringVar(value=self.settings["cookies"])
        self.max_duration_var = tk.StringVar(value=self.settings["max_duration"])
        self.copy_source_var = tk.BooleanVar(value=bool(self.settings["copy_source"]))
        self.translate_var = tk.BooleanVar(value=bool(self.settings["translate"]))
        self.batched_var = tk.BooleanVar(value=bool(self.settings["batched"]))
        self.keep_failed_var = tk.BooleanVar(value=bool(self.settings["keep_failed"]))

        self._init_db()
        self._style()
        self._build_ui()
        self._refresh_history()
        self.after(100, self._drain_events)
        self.after(250, self._refresh_engine_badge)
        self.after(600, self._recover_interrupted_jobs)

    def _style(self):
        style = ttk.Style(self)
        try:
            style.theme_use("clam")
        except tk.TclError:
            pass
        style.configure("TNotebook", background=BG, borderwidth=0)
        style.configure("TNotebook.Tab", background=PANEL2, foreground=MUTED, padding=(18, 9))
        style.map("TNotebook.Tab", background=[("selected", PANEL)], foreground=[("selected", TEXT)])
        style.configure("TProgressbar", troughcolor=PANEL2, background="#d7dae0", bordercolor=PANEL2)
        style.configure("Treeview", background=ENTRY, fieldbackground=ENTRY, foreground=TEXT, rowheight=28, borderwidth=0)
        style.configure("Treeview.Heading", background=PANEL2, foreground=TEXT, relief="flat")
        style.map("Treeview", background=[("selected", "#303642")], foreground=[("selected", TEXT)])
        style.configure("TCombobox", fieldbackground=ENTRY, background=PANEL2, foreground=TEXT, arrowcolor=TEXT)
        style.map("TCombobox", fieldbackground=[("readonly", ENTRY)], foreground=[("readonly", TEXT)])

    def _button(self, parent, text, command, secondary=False, compact=False):
        return tk.Button(
            parent, text=text, command=command,
            bg=PANEL2 if secondary else ACCENT,
            fg=TEXT if secondary else BG,
            activebackground="#343a44" if secondary else "#ffffff",
            activeforeground=TEXT if secondary else BG,
            relief="flat", bd=0, cursor="hand2",
            padx=11 if compact else 18, pady=6 if compact else 9,
            font=("Sans", 9 if compact else 10, "bold"),
        )

    def _entry(self, parent, variable):
        return tk.Entry(
            parent, textvariable=variable, bg=ENTRY, fg=TEXT,
            insertbackground=TEXT, relief="flat", bd=0,
            highlightbackground=BORDER, highlightcolor="#676f7d",
            highlightthickness=1, font=("Sans", 10),
        )

    def _card(self, parent):
        return tk.Frame(parent, bg=PANEL, highlightbackground=BORDER, highlightthickness=1)

    def _build_ui(self):
        root = tk.Frame(self, bg=BG)
        root.pack(fill="both", expand=True, padx=24, pady=18)

        header = tk.Frame(root, bg=BG)
        header.pack(fill="x", pady=(0, 12))
        tk.Label(header, text="Scriptotar", bg=BG, fg=TEXT, font=("Sans", 25, "bold")).pack(side="left")
        tk.Label(header, text="Local Reel & video transcription", bg=BG, fg=MUTED, font=("Sans", 10)).pack(side="left", padx=(13, 0), pady=(8, 0))
        self.engine_badge = tk.Label(header, text="Checking engine...", bg=PANEL2, fg=MUTED, padx=10, pady=5, font=("Sans", 9, "bold"))
        self.engine_badge.pack(side="right")

        self.notebook = ttk.Notebook(root)
        self.notebook.pack(fill="both", expand=True)
        self.queue_tab = tk.Frame(self.notebook, bg=BG)
        self.transcript_tab = tk.Frame(self.notebook, bg=BG)
        self.history_tab = tk.Frame(self.notebook, bg=BG)
        self.settings_tab = tk.Frame(self.notebook, bg=BG)
        self.notebook.add(self.queue_tab, text="Queue")
        self.notebook.add(self.transcript_tab, text="Transcript")
        self.notebook.add(self.history_tab, text="History")
        self.notebook.add(self.settings_tab, text="Settings")

        self._build_queue_tab()
        self._build_transcript_tab()
        self._build_history_tab()
        self._build_settings_tab()

    def _build_queue_tab(self):
        tab = self.queue_tab
        card = self._card(tab)
        card.pack(fill="x", pady=(14, 12))
        inner = tk.Frame(card, bg=PANEL)
        inner.pack(fill="x", padx=16, pady=14)
        tk.Label(inner, text="Paste one or several links", bg=PANEL, fg=TEXT, font=("Sans", 10, "bold")).pack(anchor="w")
        row = tk.Frame(inner, bg=PANEL)
        row.pack(fill="x", pady=(6, 0))
        self._entry(row, self.url_var).pack(side="left", fill="x", expand=True)
        self._button(row, "Add URL", self._add_url, compact=True).pack(side="left", padx=(8, 0))
        self._button(row, "Clipboard URLs", self._add_clipboard_urls, secondary=True, compact=True).pack(side="left", padx=(8, 0))
        self._button(row, "Add video files", self._add_files, secondary=True, compact=True).pack(side="left", padx=(8, 0))

        out = tk.Frame(card, bg=PANEL)
        out.pack(fill="x", padx=16, pady=(0, 14))
        tk.Label(out, text="Output", bg=PANEL, fg=MUTED, font=("Sans", 9)).pack(side="left")
        self._entry(out, self.output_var).pack(side="left", fill="x", expand=True, padx=(10, 8))
        self._button(out, "Choose", self._choose_output, secondary=True, compact=True).pack(side="left")

        queue_card = self._card(tab)
        queue_card.pack(fill="both", expand=True, pady=(0, 12))
        self.queue_tree = ttk.Treeview(queue_card, columns=("kind", "source", "status", "progress", "lang"), show="headings")
        widths = {"kind": 70, "source": 470, "status": 120, "progress": 90, "lang": 90}
        headings = {"kind": "Type", "source": "Source", "status": "Status", "progress": "Progress", "lang": "Language"}
        for col in widths:
            self.queue_tree.heading(col, text=headings[col])
            self.queue_tree.column(col, width=widths[col], minwidth=60, stretch=col == "source")
        self.queue_tree.pack(fill="both", expand=True, padx=12, pady=12)
        self.queue_tree.bind("<Double-1>", lambda _e: self._open_selected_result())

        actions = tk.Frame(tab, bg=BG)
        actions.pack(fill="x")
        self.start_queue_btn = self._button(actions, "Start queue", self._start_queue)
        self.start_queue_btn.pack(side="left")
        self.cancel_btn = self._button(actions, "Cancel current", self._cancel_current, secondary=True)
        self.cancel_btn.pack(side="left", padx=(8, 0))
        self.cancel_btn.configure(state="disabled")
        self._button(actions, "Remove selected", self._remove_selected, secondary=True).pack(side="left", padx=(8, 0))
        self._button(actions, "Clear completed", self._clear_completed, secondary=True).pack(side="left", padx=(8, 0))
        self._button(actions, "Install / Repair Engine", self._install_engine, secondary=True).pack(side="right")

        self.progress = ttk.Progressbar(tab, mode="determinate", maximum=100)
        self.progress.pack(fill="x", pady=(12, 6))
        self.status_label = tk.Label(tab, text="Ready", bg=BG, fg=MUTED, anchor="w", font=("Sans", 9))
        self.status_label.pack(fill="x")

    def _build_transcript_tab(self):
        tab = self.transcript_tab
        toolbar = tk.Frame(tab, bg=BG)
        toolbar.pack(fill="x", pady=(14, 8))
        self.transcript_info = tk.Label(toolbar, text="No transcript selected", bg=BG, fg=MUTED, font=("Sans", 10))
        self.transcript_info.pack(side="left")
        self._button(toolbar, "Save edits", self._save_transcript_edits, compact=True).pack(side="right")
        self._button(toolbar, "Copy", self._copy_transcript, secondary=True, compact=True).pack(side="right", padx=(0, 8))
        self._button(toolbar, "Open folder", self._open_last_output, secondary=True, compact=True).pack(side="right", padx=(0, 8))

        card = self._card(tab)
        card.pack(fill="both", expand=True)
        self.transcript_text = tk.Text(
            card, bg=ENTRY, fg=TEXT, insertbackground=TEXT,
            relief="flat", bd=0, highlightthickness=0,
            font=("Sans", 12), wrap="word", undo=True,
            padx=16, pady=14,
        )
        self.transcript_text.pack(fill="both", expand=True, padx=12, pady=12)
        self.transcript_text.tag_configure("rtl", justify="right")
        self.transcript_text.tag_configure("ltr", justify="left")

    def _build_history_tab(self):
        tab = self.history_tab
        bar = tk.Frame(tab, bg=BG)
        bar.pack(fill="x", pady=(14, 8))
        tk.Label(bar, text="Completed and failed jobs", bg=BG, fg=MUTED, font=("Sans", 10)).pack(side="left")
        self._button(bar, "Refresh", self._refresh_history, secondary=True, compact=True).pack(side="right")
        self._button(bar, "Open result", self._open_history_selected, secondary=True, compact=True).pack(side="right", padx=(0, 8))
        self._button(bar, "Load transcript", self._load_history_transcript, compact=True).pack(side="right", padx=(0, 8))

        card = self._card(tab)
        card.pack(fill="both", expand=True)
        self.history_tree = ttk.Treeview(card, columns=("time", "title", "status", "lang", "source"), show="headings")
        for col, title, width in [
            ("time", "Time", 145), ("title", "Title", 260), ("status", "Status", 90),
            ("lang", "Language", 80), ("source", "Source", 420),
        ]:
            self.history_tree.heading(col, text=title)
            self.history_tree.column(col, width=width, stretch=col in {"title", "source"})
        self.history_tree.pack(fill="both", expand=True, padx=12, pady=12)
        self.history_tree.bind("<Double-1>", lambda _e: self._load_history_transcript())

    def _build_settings_tab(self):
        tab = self.settings_tab
        card = self._card(tab)
        card.pack(fill="x", pady=(14, 12))
        grid = tk.Frame(card, bg=PANEL)
        grid.pack(fill="x", padx=18, pady=18)
        for c in range(3):
            grid.grid_columnconfigure(c, weight=1)

        self._combo_setting(grid, "Whisper model", self.model_var, ["small", "medium", "turbo", "large-v3"], 0, 0)
        self._combo_setting(grid, "Device", self.device_var, ["auto", "cpu", "cuda"], 0, 1)
        self._combo_setting(grid, "Language", self.language_var, ["auto", "Arabic", "English"], 0, 2)
        self._combo_setting(grid, "Download quality", self.quality_var, ["720p", "1080p", "Best", "Audio only"], 1, 0)
        self._combo_setting(grid, "Browser cookies", self.cookies_var, ["none", "firefox", "chrome", "chromium", "brave"], 1, 1)
        self._combo_setting(grid, "Duration safety limit", self.max_duration_var, ["30 min", "60 min", "2 hours", "Unlimited"], 1, 2)

        checks = tk.Frame(card, bg=PANEL)
        checks.pack(fill="x", padx=18, pady=(0, 18))
        self._check(checks, "Copy local videos into result folders", self.copy_source_var).pack(anchor="w")
        self._check(checks, "Translate speech to English", self.translate_var).pack(anchor="w", pady=(5, 0))
        self._check(checks, "Batched inference (faster, uses more RAM/VRAM)", self.batched_var).pack(anchor="w", pady=(5, 0))
        self._check(checks, "Keep failed partial folders for debugging", self.keep_failed_var).pack(anchor="w", pady=(5, 0))

        notes = self._card(tab)
        notes.pack(fill="x")
        tk.Label(
            notes,
            text=(
                "Balanced default: medium + auto device + 720p. The worker stays alive between jobs, so repeated jobs reuse the loaded model. "
                "The first use of a model downloads it. Instagram may require browser cookies."
            ),
            bg=PANEL, fg=MUTED, justify="left", wraplength=900, padx=18, pady=16,
        ).pack(fill="x")

    def _combo_setting(self, parent, label, var, values, row, col):
        frame = tk.Frame(parent, bg=PANEL)
        frame.grid(row=row, column=col, sticky="ew", padx=(0 if col == 0 else 10, 0), pady=(0 if row == 0 else 14, 0))
        tk.Label(frame, text=label, bg=PANEL, fg=MUTED, font=("Sans", 9)).pack(anchor="w")
        ttk.Combobox(frame, textvariable=var, values=values, state="readonly").pack(fill="x", pady=(5, 0))

    def _check(self, parent, text, var):
        return tk.Checkbutton(
            parent, text=text, variable=var, bg=PANEL, fg=MUTED,
            activebackground=PANEL, activeforeground=TEXT, selectcolor=ENTRY,
            highlightthickness=0, font=("Sans", 9),
        )

    def _init_db(self):
        with sqlite3.connect(DB_FILE) as db:
            db.execute(
                """
                CREATE TABLE IF NOT EXISTS jobs (
                    id TEXT PRIMARY KEY,
                    created_at TEXT NOT NULL,
                    source TEXT NOT NULL,
                    input_type TEXT NOT NULL,
                    title TEXT,
                    status TEXT NOT NULL,
                    language TEXT,
                    output_dir TEXT,
                    transcript TEXT,
                    error TEXT
                )
                """
            )

    def _record_history(self, job: dict, status: str, **extra):
        with sqlite3.connect(DB_FILE) as db:
            db.execute(
                """
                INSERT INTO jobs(id,created_at,source,input_type,title,status,language,output_dir,transcript,error)
                VALUES(?,?,?,?,?,?,?,?,?,?)
                ON CONFLICT(id) DO UPDATE SET
                  title=excluded.title,status=excluded.status,language=excluded.language,
                  output_dir=excluded.output_dir,transcript=excluded.transcript,error=excluded.error
                """,
                (
                    job["id"], job["created_at"], job["source"], job["input_type"],
                    extra.get("title"), status, extra.get("language"), extra.get("output_dir"),
                    extra.get("transcript"), extra.get("error"),
                ),
            )
        self._refresh_history()

    def _refresh_history(self):
        if not hasattr(self, "history_tree"):
            return
        for item in self.history_tree.get_children():
            self.history_tree.delete(item)
        with sqlite3.connect(DB_FILE) as db:
            rows = db.execute(
                "SELECT id,created_at,COALESCE(title,''),status,COALESCE(language,''),source FROM jobs ORDER BY created_at DESC LIMIT 300"
            ).fetchall()
        for row in rows:
            self.history_tree.insert("", "end", iid=row[0], values=(row[1], row[2], row[3], row[4], row[5]))

    def _history_row(self, job_id):
        with sqlite3.connect(DB_FILE) as db:
            return db.execute("SELECT output_dir,transcript,language FROM jobs WHERE id=?", (job_id,)).fetchone()

    def _engine_ready(self):
        try:
            return VPY.exists() and MARKER.read_text(encoding="utf-8").strip() == ENGINE_VERSION
        except Exception:
            return False

    def _refresh_engine_badge(self):
        ready = self._engine_ready()
        self.engine_badge.configure(text="Engine ready" if ready else "Engine update needed", fg=GOOD if ready else WARN)

    def _save_settings(self):
        data = {
            "output": self.output_var.get().strip(),
            "model": self.model_var.get(),
            "device": self.device_var.get(),
            "language": self.language_var.get(),
            "quality": self.quality_var.get(),
            "cookies": self.cookies_var.get(),
            "max_duration": self.max_duration_var.get(),
            "copy_source": bool(self.copy_source_var.get()),
            "translate": bool(self.translate_var.get()),
            "batched": bool(self.batched_var.get()),
            "keep_failed": bool(self.keep_failed_var.get()),
        }
        SETTINGS_FILE.write_text(json.dumps(data, indent=2), encoding="utf-8")

    def _recover_interrupted_jobs(self):
        root = Path(self.output_var.get()).expanduser()
        if not root.is_dir():
            return
        recovered = []
        partials = list(root.glob(".scriptotar-*.partial")) + list(root.glob(".wesamboss-*.partial"))
        for partial in sorted(set(partials)):
            job_file = partial / "job.json"
            if not job_file.is_file():
                continue
            try:
                job = json.loads(job_file.read_text(encoding="utf-8"))
                if job.get("id") and job.get("source") and job.get("input_type") in {"url", "file"}:
                    recovered.append(job)
            except Exception:
                continue
        if not recovered:
            return
        if not messagebox.askyesno(
            APP,
            f"Found {len(recovered)} interrupted job{'s' if len(recovered) != 1 else ''}. Requeue them?",
        ):
            return
        for raw in recovered:
            job = {**raw, "status": "Queued", "progress": 0, "language_result": ""}
            # Avoid duplicate rows if the same interrupted job was already restored.
            if job["id"] in self.jobs:
                continue
            self.jobs[job["id"]] = job
            self.queue_order.append(job["id"])
            self.queue_tree.insert(
                "", "end", iid=job["id"],
                values=("URL" if job["input_type"] == "url" else "File", job["source"], "Queued", "0%", ""),
            )
            self._record_history(job, "Queued")
        self.status_label.configure(text=f"Recovered {len(recovered)} interrupted job(s).")

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
            self._enqueue(self._new_job("url", url.rstrip(".,);]}>")))
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
            self._enqueue(self._new_job("url", url.rstrip(".,);]}>")))

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
        except queue.Empty:
            pass
        self.after(100, self._drain_events)

    def _load_transcript(self, path, language=None, title=None):
        if not path or not Path(path).is_file():
            return
        text = Path(path).read_text(encoding="utf-8")
        self.transcript_text.delete("1.0", "end")
        self.transcript_text.insert("1.0", text)
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

    def _on_close(self):
        if self.current_job_id:
            if not messagebox.askyesno(APP, "A transcription is still running. Cancel it and quit?"):
                return
        self._save_settings()
        self._kill_worker_group()
        self.destroy()


if __name__ == "__main__":
    App().mainloop()
