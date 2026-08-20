#!/usr/bin/env python3
from __future__ import annotations

from scriptotar_common import *
from ui_mixin import UIMixin
from persistence_mixin import PersistenceMixin
from jobs_mixin import JobsMixin
from research_mixin import ResearchMixin
from ai_mixin import AIMixin
from library_mixin import LibraryMixin


class App(UIMixin, PersistenceMixin, JobsMixin, ResearchMixin, AIMixin, LibraryMixin, tk.Tk):
    def __init__(self):
        super().__init__()
        self.title(f"{APP} {VERSION}")
        self.geometry("1240x900")
        self.minsize(1040, 760)
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
        self.project_var = tk.StringVar(value=self.settings.get("project", "Inbox"))
        self.research_url_var = tk.StringVar()
        self.research_limit_var = tk.StringVar(value="25")
        self.research_search_var = tk.StringVar()
        self.watch_var = tk.StringVar()
        self.auto_watch_var = tk.BooleanVar(value=bool(self.settings.get("auto_watch", False)))
        self.watch_interval_var = tk.StringVar(value=self.settings.get("watch_interval", "60 min"))
        self.ai_mode_var = tk.StringVar(value=self.settings.get("ai_mode", "Copy prompt only"))
        self.ai_task_var = tk.StringVar(value="Viral breakdown")
        self.ai_provider_var = tk.StringVar(value=self.settings.get("ai_provider", "OpenAI"))
        self.ai_model_var = tk.StringVar(value=self.settings.get("ai_model", "gpt-5.2"))
        self.ai_base_url_var = tk.StringVar(value=self.settings.get("ai_base_url", ""))
        self.ai_key_var = tk.StringVar()
        self.ai_topic_var = tk.StringVar()
        self.ai_audience_var = tk.StringVar()
        self.ai_duration_var = tk.StringVar(value="30-45 seconds")
        self.ai_cta_var = tk.StringVar()
        self.ai_voice_var = tk.StringVar()
        self.ai_busy = False
        self.research_busy = False
        self.research_proc = None
        self.research_rows = {}
        self.library_rows = {}
        self.active_watch_profile = None

        self._init_db()
        self._style()
        self._build_ui()
        self._refresh_history()
        self._refresh_projects()
        self._refresh_watch_combo()
        self._refresh_library()
        self._load_saved_ai_key()
        self.after(100, self._drain_events)
        self.after(250, self._refresh_engine_badge)
        self.after(600, self._recover_interrupted_jobs)
        self.after(60_000, self._watch_tick)



if __name__ == "__main__":
    App().mainloop()
