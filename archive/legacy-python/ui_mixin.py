#!/usr/bin/env python3
from __future__ import annotations

from scriptotar_common import *


class UIMixin:
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
        tk.Label(header, text="Local short-form research & transcription", bg=BG, fg=MUTED, font=("Sans", 10)).pack(side="left", padx=(13, 0), pady=(8, 0))
        self.engine_badge = tk.Label(header, text="Checking engine...", bg=PANEL2, fg=MUTED, padx=10, pady=5, font=("Sans", 9, "bold"))
        self.engine_badge.pack(side="right")
        self._button(header, "+ Project", self._new_project, secondary=True, compact=True).pack(side="right", padx=(0, 8))
        self.project_combo = ttk.Combobox(header, textvariable=self.project_var, state="readonly", width=16)
        self.project_combo.pack(side="right", padx=(0, 8), pady=(4, 0))
        self.project_combo.bind("<<ComboboxSelected>>", lambda _e: self._project_changed())
        tk.Label(header, text="Project", bg=BG, fg=MUTED, font=("Sans", 9)).pack(side="right", padx=(0, 6), pady=(7, 0))

        self.notebook = ttk.Notebook(root)
        self.notebook.pack(fill="both", expand=True)
        self.queue_tab = tk.Frame(self.notebook, bg=BG)
        self.transcript_tab = tk.Frame(self.notebook, bg=BG)
        self.research_tab = tk.Frame(self.notebook, bg=BG)
        self.ai_tab = tk.Frame(self.notebook, bg=BG)
        self.library_tab = tk.Frame(self.notebook, bg=BG)
        self.history_tab = tk.Frame(self.notebook, bg=BG)
        self.settings_tab = tk.Frame(self.notebook, bg=BG)
        self.notebook.add(self.queue_tab, text="Queue")
        self.notebook.add(self.transcript_tab, text="Transcript")
        self.notebook.add(self.research_tab, text="Research")
        self.notebook.add(self.ai_tab, text="AI Studio")
        self.notebook.add(self.library_tab, text="Library")
        self.notebook.add(self.history_tab, text="History")
        self.notebook.add(self.settings_tab, text="Settings")

        self._build_queue_tab()
        self._build_transcript_tab()
        self._build_research_tab()
        self._build_ai_tab()
        self._build_library_tab()
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

        creator = self._card(tab)
        creator.pack(fill="x", pady=(0, 12))
        creator_inner = tk.Frame(creator, bg=PANEL)
        creator_inner.pack(fill="x", padx=18, pady=16)
        self._check(creator_inner, "Auto-refresh saved creator watchlists while Scriptotar is open", self.auto_watch_var).pack(side="left")
        tk.Label(creator_inner, text="Interval", bg=PANEL, fg=MUTED).pack(side="left", padx=(22, 6))
        ttk.Combobox(creator_inner, textvariable=self.watch_interval_var, values=["30 min", "60 min", "2 hours", "6 hours"], state="readonly", width=10).pack(side="left")

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
