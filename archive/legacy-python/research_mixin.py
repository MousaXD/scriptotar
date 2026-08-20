#!/usr/bin/env python3
from __future__ import annotations

from scriptotar_common import *


class ResearchMixin:
    def _project_names(self):
        with sqlite3.connect(DB_FILE) as db:
            rows = db.execute("SELECT name FROM projects ORDER BY CASE WHEN name='Inbox' THEN 0 ELSE 1 END, name COLLATE NOCASE").fetchall()
        return [row[0] for row in rows]

    def _refresh_projects(self):
        if not hasattr(self, "project_combo"):
            return
        names = self._project_names()
        self.project_combo.configure(values=names)
        if self.project_var.get() not in names:
            self.project_var.set("Inbox")

    def _new_project(self):
        name = simpledialog.askstring(APP, "Project name:", parent=self)
        if not name:
            return
        name = re.sub(r"\s+", " ", name).strip()[:80]
        if not name:
            return
        with sqlite3.connect(DB_FILE) as db:
            db.execute("INSERT OR IGNORE INTO projects(name,created_at) VALUES(?,?)", (name, datetime.now().strftime("%Y-%m-%d %H:%M:%S")))
        self.project_var.set(name)
        self._refresh_projects()
        self._project_changed()

    def _project_changed(self):
        self._save_settings()
        self._refresh_watch_combo()
        self._refresh_library()

    def _build_research_tab(self):
        tab = self.research_tab
        top = self._card(tab)
        top.pack(fill="x", pady=(14, 10))
        inner = tk.Frame(top, bg=PANEL)
        inner.pack(fill="x", padx=16, pady=14)
        tk.Label(inner, text="Public creator / profile research", bg=PANEL, fg=TEXT, font=("Sans", 10, "bold")).pack(anchor="w")
        row = tk.Frame(inner, bg=PANEL)
        row.pack(fill="x", pady=(7, 0))
        self._entry(row, self.research_url_var).pack(side="left", fill="x", expand=True)
        ttk.Combobox(row, textvariable=self.research_limit_var, values=["10", "25", "50", "100", "200"], state="readonly", width=6).pack(side="left", padx=(8, 0))
        self.research_scan_btn = self._button(row, "Scan profile", self._research_scan, compact=True)
        self.research_scan_btn.pack(side="left", padx=(8, 0))
        self._button(row, "Save watchlist", self._save_watchlist, secondary=True, compact=True).pack(side="left", padx=(8, 0))

        watch = tk.Frame(top, bg=PANEL)
        watch.pack(fill="x", padx=16, pady=(0, 14))
        tk.Label(watch, text="Saved watchlist", bg=PANEL, fg=MUTED, font=("Sans", 9)).pack(side="left")
        self.watch_combo = ttk.Combobox(watch, textvariable=self.watch_var, state="readonly", width=45)
        self.watch_combo.pack(side="left", fill="x", expand=True, padx=(8, 8))
        self._button(watch, "Refresh", self._refresh_selected_watchlist, secondary=True, compact=True).pack(side="left")
        self._button(watch, "Remove", self._remove_watchlist, secondary=True, compact=True).pack(side="left", padx=(8, 0))

        results = self._card(tab)
        results.pack(fill="both", expand=True, pady=(0, 10))
        self.research_tree = ttk.Treeview(results, columns=("platform", "title", "views", "likes", "comments", "eng", "date"), show="headings")
        specs = [
            ("platform", "Platform", 85), ("title", "Title", 360), ("views", "Views", 95),
            ("likes", "Likes", 85), ("comments", "Comments", 85), ("eng", "Eng %", 75), ("date", "Date", 95),
        ]
        for col, title, width in specs:
            self.research_tree.heading(col, text=title, command=lambda c=col: self._sort_research(c))
            self.research_tree.column(col, width=width, stretch=col == "title")
        self.research_tree.pack(fill="both", expand=True, padx=12, pady=12)
        self.research_tree.bind("<Double-1>", lambda _e: self._queue_research_selected())

        bar = tk.Frame(tab, bg=BG)
        bar.pack(fill="x")
        self._button(bar, "Queue selected", self._queue_research_selected).pack(side="left")
        self._button(bar, "Use in AI", self._research_use_in_ai, secondary=True).pack(side="left", padx=(8, 0))
        self._button(bar, "Copy URLs", self._copy_research_urls, secondary=True).pack(side="left", padx=(8, 0))
        self._button(bar, "Export CSV", self._export_research_csv, secondary=True).pack(side="left", padx=(8, 0))
        self._button(bar, "Load local library", self._load_research_library, secondary=True).pack(side="right")
        self.research_status = tk.Label(tab, text="Scan a public Instagram, TikTok, or YouTube creator URL. Availability depends on the source and yt-dlp extractor.", bg=BG, fg=MUTED, anchor="w", font=("Sans", 9))
        self.research_status.pack(fill="x", pady=(8, 0))

    def _clear_research_tree(self):
        for iid in self.research_tree.get_children():
            self.research_tree.delete(iid)
        self.research_rows.clear()

    def _research_scan(self, profile_url=None, watch=False):
        if self.research_busy:
            return
        url = (profile_url or self.research_url_var.get()).strip()
        if not url:
            messagebox.showinfo(APP, "Paste a public creator/profile URL first.")
            return
        if not self._engine_ready():
            messagebox.showinfo(APP, "Install / Repair Engine first. Creator scanning uses the same yt-dlp engine.")
            return
        try:
            limit = int(self.research_limit_var.get() or 25)
        except ValueError:
            limit = 25
        if not watch:
            self._clear_research_tree()
        project = self.project_var.get() or "Inbox"
        self.research_busy = True
        self.active_watch_profile = url if watch else None
        self.research_status.configure(text=f"Scanning {url} in {project} ...")
        self.research_scan_btn.configure(state="disabled")
        command = research_scan_command(str(VPY), url, limit, self.cookies_var.get())

        def run():
            count = 0
            try:
                proc = subprocess.Popen(command, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, bufsize=1, start_new_session=True)
                self.research_proc = proc
                assert proc.stdout is not None
                for line in proc.stdout:
                    line = line.strip()
                    if not line:
                        continue
                    try:
                        raw = json.loads(line)
                        item = normalize_research_item(raw, url)
                    except Exception:
                        continue
                    count += 1
                    self.events.put(("research_item", {"item": item, "watch": watch, "project": project}))
                stderr = proc.stderr.read().strip() if proc.stderr else ""
                code = proc.wait()
                if code != 0 and count == 0:
                    raise RuntimeError(stderr[-1200:] or f"yt-dlp exited with code {code}")
                self.events.put(("research_done", {"count": count, "profile_url": url, "watch": watch, "project": project}))
            except Exception as exc:
                self.events.put(("research_error", f"Creator scan failed: {exc}"))
            finally:
                self.research_proc = None
        threading.Thread(target=run, daemon=True).start()

    def _research_storage_id(self, item, project):
        identity = item.get("id") or item.get("source_url") or uuid.uuid4().hex
        return uuid.uuid5(uuid.NAMESPACE_URL, f"scriptotar:{project}:{item.get('platform','Web')}:{identity}").hex

    def _accept_research_item(self, payload):
        item = payload["item"]
        project = payload.get("project") or "Inbox"
        storage_id = self._research_storage_id(item, project)
        item["storage_id"] = storage_id
        with sqlite3.connect(DB_FILE) as db:
            db.execute(
                """INSERT INTO research_items(storage_id,project,creator_url,source_url,platform,title,view_count,like_count,comment_count,engagement_rate,published_at,duration,thumbnail,raw_json,scanned_at)
                   VALUES(?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)
                   ON CONFLICT(storage_id) DO UPDATE SET project=excluded.project,creator_url=excluded.creator_url,source_url=excluded.source_url,
                   platform=excluded.platform,title=excluded.title,view_count=excluded.view_count,like_count=excluded.like_count,
                   comment_count=excluded.comment_count,engagement_rate=excluded.engagement_rate,published_at=excluded.published_at,
                   duration=excluded.duration,thumbnail=excluded.thumbnail,raw_json=excluded.raw_json,scanned_at=excluded.scanned_at""",
                (storage_id, project, item.get("creator_url", ""), item.get("source_url", ""), item.get("platform", ""), item.get("title", ""),
                 item.get("view_count"), item.get("like_count"), item.get("comment_count"), item.get("engagement_rate"), item.get("published_at", ""),
                 item.get("duration"), item.get("thumbnail", ""), item.get("raw_json", "{}"), datetime.now().strftime("%Y-%m-%d %H:%M:%S")),
            )
        self.research_rows[storage_id] = item
        if self.research_tree.exists(storage_id):
            self.research_tree.delete(storage_id)
        self.research_tree.insert("", "end", iid=storage_id, values=(
            item.get("platform", ""), item.get("title", "")[:120], self._metric(item.get("view_count")), self._metric(item.get("like_count")),
            self._metric(item.get("comment_count")), f"{item.get('engagement_rate'):.2f}" if item.get("engagement_rate") is not None else "—",
            item.get("published_at", "") or "—",
        ))

    def _finish_research(self, payload):
        self.research_busy = False
        self.research_scan_btn.configure(state="normal")
        count = int(payload.get("count", 0))
        url = payload.get("profile_url", "")
        project = payload.get("project") or "Inbox"
        if payload.get("watch"):
            with sqlite3.connect(DB_FILE) as db:
                db.execute("UPDATE watchlists SET last_scan_at=? WHERE project=? AND profile_url=?", (datetime.now().strftime("%Y-%m-%d %H:%M:%S"), project, url))
        self.research_status.configure(text=f"Scan complete: {count} item(s) saved to {project}.")
        self._refresh_watch_combo()
        self._refresh_library()
        self.active_watch_profile = None

    def _csv_safe(self, value):
        text = "" if value is None else str(value)
        if text.startswith(("=", "+", "-", "@", "\t", "\r")):
            return "'" + text
        return text

    def _metric(self, value):
        if value is None:
            return "—"
        try:
            n = int(value)
        except Exception:
            return str(value)
        if n >= 1_000_000:
            return f"{n/1_000_000:.1f}M"
        if n >= 1_000:
            return f"{n/1_000:.1f}K"
        return str(n)

    def _sort_research(self, col):
        numeric = {"views", "likes", "comments", "eng"}
        rows = []
        for iid in self.research_tree.get_children():
            vals = self.research_tree.item(iid, "values")
            idx = {"platform":0,"title":1,"views":2,"likes":3,"comments":4,"eng":5,"date":6}[col]
            raw = vals[idx]
            if col in numeric:
                text = str(raw).replace("—", "0")
                mult = 1
                if text.endswith("K"): mult, text = 1_000, text[:-1]
                elif text.endswith("M"): mult, text = 1_000_000, text[:-1]
                try: key = float(text) * mult
                except Exception: key = 0
            else:
                key = str(raw).casefold()
            rows.append((key, iid))
        current = getattr(self, "_research_sort", (None, False))
        reverse = not current[1] if current[0] == col else True
        self._research_sort = (col, reverse)
        for pos, (_, iid) in enumerate(sorted(rows, key=lambda x: x[0], reverse=reverse)):
            self.research_tree.move(iid, "", pos)

    def _selected_research_items(self):
        return [self.research_rows[iid] for iid in self.research_tree.selection() if iid in self.research_rows]

    def _queue_research_selected(self):
        items = self._selected_research_items()
        if not items:
            return
        added = 0
        for item in items:
            url = item.get("source_url", "")
            if url.startswith(("http://", "https://")):
                self._enqueue(self._new_job("url", url))
                added += 1
        if added:
            self.notebook.select(self.queue_tab)
            self.status_label.configure(text=f"Queued {added} research item(s) for download/transcription.")

    def _research_use_in_ai(self):
        items = self._selected_research_items()
        if not items:
            return
        item = items[0]
        source = item.get("title", "")
        try:
            raw = json.loads(item.get("raw_json") or "{}")
            description = str(raw.get("description") or "").strip()
            if description and description != source:
                source += "\n\n" + description
        except Exception:
            pass
        self.ai_source_text.delete("1.0", "end")
        self.ai_source_text.insert("1.0", source)
        self.ai_topic_var.set(item.get("title", "")[:180])
        self.notebook.select(self.ai_tab)
        self._update_script_timer()

    def _copy_research_urls(self):
        urls = [item.get("source_url", "") for item in self._selected_research_items() if item.get("source_url")]
        if not urls:
            return
        self.clipboard_clear(); self.clipboard_append("\n".join(urls))
        self.research_status.configure(text=f"Copied {len(urls)} URL(s).")

    def _export_research_csv(self):
        items = self._selected_research_items() or list(self.research_rows.values())
        if not items:
            return
        path = filedialog.asksaveasfilename(title="Export research CSV", defaultextension=".csv", filetypes=[("CSV", "*.csv")], initialfile="scriptotar-research.csv")
        if not path:
            return
        fields = ["platform", "title", "source_url", "creator_url", "view_count", "like_count", "comment_count", "engagement_rate", "published_at", "duration"]
        with open(path, "w", newline="", encoding="utf-8-sig") as fh:
            writer = csv.DictWriter(fh, fieldnames=fields)
            writer.writeheader()
            for item in items:
                writer.writerow({k: self._csv_safe(item.get(k)) for k in fields})
        self.research_status.configure(text=f"Exported {len(items)} item(s) to {path}")

    def _load_research_library(self):
        self._clear_research_tree()
        project = self.project_var.get() or "Inbox"
        with sqlite3.connect(DB_FILE) as db:
            rows = db.execute("SELECT storage_id,creator_url,source_url,platform,title,view_count,like_count,comment_count,engagement_rate,published_at,duration,thumbnail,raw_json FROM research_items WHERE project=? ORDER BY COALESCE(view_count,0) DESC, scanned_at DESC LIMIT 500", (project,)).fetchall()
        for row in rows:
            item = dict(zip(["storage_id","creator_url","source_url","platform","title","view_count","like_count","comment_count","engagement_rate","published_at","duration","thumbnail","raw_json"], row))
            item["id"] = row[0]
            self.research_rows[row[0]] = item
            self.research_tree.insert("", "end", iid=row[0], values=(row[3], row[4][:120], self._metric(row[5]), self._metric(row[6]), self._metric(row[7]), f"{row[8]:.2f}" if row[8] is not None else "—", row[9] or "—"))
        self.research_status.configure(text=f"Loaded {len(rows)} saved research item(s) for {project}.")

    def _save_watchlist(self):
        url = self.research_url_var.get().strip()
        if not url:
            return
        label = simpledialog.askstring(APP, "Watchlist label:", initialvalue=url.rstrip('/').split('/')[-1] or "Creator", parent=self)
        if not label:
            return
        try: limit = int(self.research_limit_var.get() or 25)
        except ValueError: limit = 25
        with sqlite3.connect(DB_FILE) as db:
            db.execute("INSERT INTO watchlists(project,label,profile_url,limit_count) VALUES(?,?,?,?) ON CONFLICT(project,profile_url) DO UPDATE SET label=excluded.label,limit_count=excluded.limit_count", (self.project_var.get() or "Inbox", label[:80], url, limit))
        self._refresh_watch_combo()

    def _watch_rows(self):
        with sqlite3.connect(DB_FILE) as db:
            return db.execute("SELECT id,label,profile_url,limit_count,last_scan_at FROM watchlists WHERE project=? ORDER BY label COLLATE NOCASE", (self.project_var.get() or "Inbox",)).fetchall()

    def _refresh_watch_combo(self):
        if not hasattr(self, "watch_combo"):
            return
        self._watch_map = {f"{row[1]}  •  {row[2]}": row for row in self._watch_rows()}
        values = list(self._watch_map)
        self.watch_combo.configure(values=values)
        if self.watch_var.get() not in self._watch_map:
            self.watch_var.set(values[0] if values else "")

    def _refresh_selected_watchlist(self):
        row = getattr(self, "_watch_map", {}).get(self.watch_var.get())
        if not row:
            return
        self.research_limit_var.set(str(row[3]))
        self.research_url_var.set(row[2])
        self._research_scan(row[2], watch=True)

    def _remove_watchlist(self):
        row = getattr(self, "_watch_map", {}).get(self.watch_var.get())
        if not row:
            return
        with sqlite3.connect(DB_FILE) as db:
            db.execute("DELETE FROM watchlists WHERE id=?", (row[0],))
        self._refresh_watch_combo()

    def _watch_interval_seconds(self):
        return {"30 min":1800, "60 min":3600, "2 hours":7200, "6 hours":21600}.get(self.watch_interval_var.get(), 3600)

    def _watch_tick(self):
        try:
            if self.auto_watch_var.get() and not self.research_busy and self._engine_ready():
                now = time.time()
                interval = self._watch_interval_seconds()
                for row in self._watch_rows():
                    last = row[4]
                    due = True
                    if last:
                        try:
                            due = now - datetime.strptime(last, "%Y-%m-%d %H:%M:%S").timestamp() >= interval
                        except Exception:
                            pass
                    if due:
                        self.research_limit_var.set(str(row[3]))
                        self._research_scan(row[2], watch=True)
                        break
        finally:
            self.after(60_000, self._watch_tick)
