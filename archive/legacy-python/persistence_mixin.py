#!/usr/bin/env python3
from __future__ import annotations

from scriptotar_common import *


class PersistenceMixin:
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
                    error TEXT,
                    project TEXT NOT NULL DEFAULT 'Inbox'
                )
                """
            )
            columns = {row[1] for row in db.execute("PRAGMA table_info(jobs)")}
            if "project" not in columns:
                db.execute("ALTER TABLE jobs ADD COLUMN project TEXT NOT NULL DEFAULT 'Inbox'")
            db.execute(
                "CREATE TABLE IF NOT EXISTS projects(name TEXT PRIMARY KEY, created_at TEXT NOT NULL)"
            )
            db.execute(
                "INSERT OR IGNORE INTO projects(name,created_at) VALUES('Inbox',?)",
                (datetime.now().strftime("%Y-%m-%d %H:%M:%S"),),
            )
            db.execute(
                """
                CREATE TABLE IF NOT EXISTS research_items (
                    storage_id TEXT PRIMARY KEY,
                    project TEXT NOT NULL,
                    creator_url TEXT NOT NULL,
                    source_url TEXT,
                    platform TEXT,
                    title TEXT,
                    view_count INTEGER,
                    like_count INTEGER,
                    comment_count INTEGER,
                    engagement_rate REAL,
                    published_at TEXT,
                    duration REAL,
                    thumbnail TEXT,
                    raw_json TEXT,
                    scanned_at TEXT NOT NULL
                )
                """
            )
            db.execute(
                """
                CREATE TABLE IF NOT EXISTS watchlists (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    project TEXT NOT NULL,
                    label TEXT NOT NULL,
                    profile_url TEXT NOT NULL,
                    limit_count INTEGER NOT NULL DEFAULT 25,
                    last_scan_at TEXT,
                    UNIQUE(project, profile_url)
                )
                """
            )
            db.execute(
                """
                CREATE TABLE IF NOT EXISTS ai_runs (
                    id TEXT PRIMARY KEY,
                    created_at TEXT NOT NULL,
                    project TEXT NOT NULL,
                    task TEXT NOT NULL,
                    mode TEXT NOT NULL,
                    provider TEXT,
                    model TEXT,
                    source_title TEXT,
                    prompt TEXT NOT NULL,
                    result TEXT
                )
                """
            )

    def _record_history(self, job: dict, status: str, **extra):
        with sqlite3.connect(DB_FILE) as db:
            db.execute(
                """
                INSERT INTO jobs(id,created_at,source,input_type,title,status,language,output_dir,transcript,error,project)
                VALUES(?,?,?,?,?,?,?,?,?,?,?)
                ON CONFLICT(id) DO UPDATE SET
                  title=excluded.title,status=excluded.status,language=excluded.language,
                  output_dir=excluded.output_dir,transcript=excluded.transcript,error=excluded.error,project=excluded.project
                """,
                (
                    job["id"], job["created_at"], job["source"], job["input_type"],
                    extra.get("title"), status, extra.get("language"), extra.get("output_dir"),
                    extra.get("transcript"), extra.get("error"), job.get("project", self.project_var.get() or "Inbox"),
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
            "project": self.project_var.get() or "Inbox",
            "ai_mode": self.ai_mode_var.get(),
            "ai_provider": self.ai_provider_var.get(),
            "ai_model": self.ai_model_var.get(),
            "ai_base_url": self.ai_base_url_var.get().strip(),
            "auto_watch": bool(self.auto_watch_var.get()),
            "watch_interval": self.watch_interval_var.get(),
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
