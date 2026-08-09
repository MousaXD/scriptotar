#!/usr/bin/env python3
from __future__ import annotations

from scriptotar_common import *


class AIMixin:
    def _build_ai_tab(self):
        tab = self.ai_tab
        config = self._card(tab)
        config.pack(fill="x", pady=(14, 10))
        grid = tk.Frame(config, bg=PANEL)
        grid.pack(fill="x", padx=14, pady=12)
        for c in range(4): grid.grid_columnconfigure(c, weight=1)
        fields = [
            ("Task", self.ai_task_var, list(TASKS), 0, 0),
            ("AI mode", self.ai_mode_var, ["Copy prompt only", "Use API key"], 0, 1),
            ("Provider", self.ai_provider_var, list(PROVIDERS), 0, 2),
        ]
        for label, var, values, row, col in fields:
            frame = tk.Frame(grid, bg=PANEL); frame.grid(row=row, column=col, sticky="ew", padx=(0, 10))
            tk.Label(frame, text=label, bg=PANEL, fg=MUTED, font=("Sans", 9)).pack(anchor="w")
            combo = ttk.Combobox(frame, textvariable=var, values=values, state="readonly")
            combo.pack(fill="x", pady=(4,0))
            if label == "Provider": combo.bind("<<ComboboxSelected>>", lambda _e: self._provider_changed())
        frame = tk.Frame(grid, bg=PANEL); frame.grid(row=0, column=3, sticky="ew")
        tk.Label(frame, text="Model", bg=PANEL, fg=MUTED, font=("Sans", 9)).pack(anchor="w")
        self._entry(frame, self.ai_model_var).pack(fill="x", pady=(4,0))

        context = tk.Frame(config, bg=PANEL); context.pack(fill="x", padx=14, pady=(0, 12))
        for c in range(4): context.grid_columnconfigure(c, weight=1)
        for label, var, col in [("Topic / goal", self.ai_topic_var,0),("Audience",self.ai_audience_var,1),("Target duration",self.ai_duration_var,2),("CTA",self.ai_cta_var,3)]:
            f=tk.Frame(context,bg=PANEL); f.grid(row=0,column=col,sticky="ew",padx=(0 if col==0 else 10,0)); tk.Label(f,text=label,bg=PANEL,fg=MUTED,font=("Sans",9)).pack(anchor="w"); self._entry(f,var).pack(fill="x",pady=(4,0))
        f=tk.Frame(context,bg=PANEL); f.grid(row=1,column=0,columnspan=2,sticky="ew",pady=(10,0)); tk.Label(f,text="Voice / style instructions",bg=PANEL,fg=MUTED,font=("Sans",9)).pack(anchor="w"); self._entry(f,self.ai_voice_var).pack(fill="x",pady=(4,0))
        f=tk.Frame(context,bg=PANEL); f.grid(row=1,column=2,columnspan=2,sticky="ew",padx=(10,0),pady=(10,0)); tk.Label(f,text="OpenAI-compatible base URL (custom provider only)",bg=PANEL,fg=MUTED,font=("Sans",9)).pack(anchor="w"); self._entry(f,self.ai_base_url_var).pack(fill="x",pady=(4,0))

        panes = tk.PanedWindow(tab, orient="horizontal", bg=BG, sashwidth=6, bd=0, relief="flat")
        panes.pack(fill="both", expand=True, pady=(0, 10))
        left=self._card(panes); right=self._card(panes); panes.add(left, stretch="always"); panes.add(right, stretch="always")
        tk.Label(left,text="Source transcript / research",bg=PANEL,fg=TEXT,font=("Sans",10,"bold")).pack(anchor="w",padx=12,pady=(10,4))
        self.ai_source_text=tk.Text(left,bg=ENTRY,fg=TEXT,insertbackground=TEXT,wrap="word",relief="flat",font=("Sans",10),height=9,padx=10,pady=8)
        self.ai_source_text.pack(fill="both",expand=True,padx=10,pady=(0,6)); self.ai_source_text.bind("<KeyRelease>",lambda _e:self._update_script_timer())
        self.ai_timer_label=tk.Label(left,text="Speaking time: 0:00",bg=PANEL,fg=MUTED,font=("Sans",9)); self.ai_timer_label.pack(anchor="w",padx=12,pady=(0,8))
        tk.Label(right,text="Generated prompt",bg=PANEL,fg=TEXT,font=("Sans",10,"bold")).pack(anchor="w",padx=12,pady=(10,4))
        self.ai_prompt_text=tk.Text(right,bg=ENTRY,fg=TEXT,insertbackground=TEXT,wrap="word",relief="flat",font=("Sans",10),height=9,padx=10,pady=8)
        self.ai_prompt_text.pack(fill="both",expand=True,padx=10,pady=(0,8))

        keys=self._card(tab); keys.pack(fill="x",pady=(0,10)); k=tk.Frame(keys,bg=PANEL); k.pack(fill="x",padx=12,pady=10)
        tk.Label(k,text="API key",bg=PANEL,fg=MUTED,font=("Sans",9)).pack(side="left")
        self.ai_key_entry=tk.Entry(k,textvariable=self.ai_key_var,show="•",bg=ENTRY,fg=TEXT,insertbackground=TEXT,relief="flat",highlightbackground=BORDER,highlightthickness=1)
        self.ai_key_entry.pack(side="left",fill="x",expand=True,padx=(8,8),ipady=5)
        self._button(k,"Remember in keyring",self._remember_ai_key,secondary=True,compact=True).pack(side="left")
        self._button(k,"Forget",self._forget_ai_key,secondary=True,compact=True).pack(side="left",padx=(8,0))
        self.keyring_label=tk.Label(k,text="Secret Service" if secret_tool_available() else "Session only",bg=PANEL,fg=GOOD if secret_tool_available() else WARN,font=("Sans",8,"bold")); self.keyring_label.pack(side="left",padx=(10,0))

        actions=tk.Frame(tab,bg=BG); actions.pack(fill="x",pady=(0,8))
        self._button(actions,"Build prompt",self._build_ai_prompt).pack(side="left")
        self._button(actions,"Copy prompt",self._copy_ai_prompt,secondary=True).pack(side="left",padx=(8,0))
        self.ai_run_btn=self._button(actions,"Run with API",self._run_ai,secondary=True); self.ai_run_btn.pack(side="left",padx=(8,0))
        self._button(actions,"Copy result",self._copy_ai_result,secondary=True).pack(side="right")
        self.ai_status=tk.Label(actions,text="Prompt-only mode works without any API key.",bg=BG,fg=MUTED,font=("Sans",9)); self.ai_status.pack(side="left",padx=(14,0))

        result=self._card(tab); result.pack(fill="both",expand=True)
        self.ai_result_text=tk.Text(result,bg=ENTRY,fg=TEXT,insertbackground=TEXT,wrap="word",relief="flat",font=("Sans",10),height=7,padx=10,pady=8)
        self.ai_result_text.pack(fill="both",expand=True,padx=10,pady=10)

    def _update_script_timer(self):
        if not hasattr(self,"ai_source_text"): return
        seconds=estimate_speaking_seconds(self.ai_source_text.get("1.0","end-1c"),2.5)
        self.ai_timer_label.configure(text=f"Speaking time: ~{format_duration(seconds)} at 2.5 words/sec")

    def _provider_changed(self):
        provider=self.ai_provider_var.get()
        self.ai_model_var.set(DEFAULT_MODELS.get(provider,""))
        self._load_saved_ai_key()
        self._save_settings()

    def _load_saved_ai_key(self):
        if not hasattr(self,"ai_key_var"): return
        key=lookup_secret(self.ai_provider_var.get())
        self.ai_key_var.set(key)

    def _remember_ai_key(self):
        key=self.ai_key_var.get().strip()
        if not key: return
        if store_secret(self.ai_provider_var.get(),key):
            self.ai_status.configure(text="API key stored in the Linux Secret Service keyring.")
        else:
            self.ai_status.configure(text="No Secret Service available. Key remains session-only.")

    def _forget_ai_key(self):
        clear_secret(self.ai_provider_var.get()); self.ai_key_var.set(""); self.ai_status.configure(text="Stored key removed for this provider.")

    def _prompt_value(self):
        return build_prompt(self.ai_task_var.get(),self.ai_source_text.get("1.0","end-1c"),topic=self.ai_topic_var.get(),audience=self.ai_audience_var.get(),duration=self.ai_duration_var.get(),cta=self.ai_cta_var.get(),voice=self.ai_voice_var.get())

    def _build_ai_prompt(self):
        prompt=self._prompt_value(); self.ai_prompt_text.delete("1.0","end"); self.ai_prompt_text.insert("1.0",prompt); self.ai_status.configure(text="Prompt built locally. Nothing was sent anywhere."); return prompt

    def _copy_ai_prompt(self):
        prompt=self.ai_prompt_text.get("1.0","end-1c").strip() or self._build_ai_prompt()
        self.clipboard_clear(); self.clipboard_append(prompt)
        with sqlite3.connect(DB_FILE) as db:
            db.execute(
                "INSERT INTO ai_runs(id,created_at,project,task,mode,provider,model,source_title,prompt,result) VALUES(?,?,?,?,?,?,?,?,?,?)",
                (uuid.uuid4().hex[:16], datetime.now().strftime("%Y-%m-%d %H:%M:%S"), self.project_var.get() or "Inbox",
                 self.ai_task_var.get(), "prompt", None, None, self.ai_topic_var.get()[:200], prompt, None),
            )
        self._refresh_library()
        self.ai_status.configure(text="Prompt copied and saved locally. Paste it into any AI you want.")

    def _run_ai(self):
        if self.ai_busy: return
        if self.ai_mode_var.get() != "Use API key":
            self._copy_ai_prompt(); return
        prompt=self.ai_prompt_text.get("1.0","end-1c").strip() or self._build_ai_prompt()
        provider=self.ai_provider_var.get(); model=self.ai_model_var.get(); key=self.ai_key_var.get(); base=self.ai_base_url_var.get()
        if not key.strip():
            messagebox.showinfo(APP,"Enter your API key, or choose Copy prompt only."); return
        self.ai_busy=True; self.ai_run_btn.configure(state="disabled"); self.ai_status.configure(text=f"Calling {provider} ...")
        task=self.ai_task_var.get(); project=self.project_var.get() or "Inbox"
        def run():
            try:
                result=request_ai(provider,model,key,prompt,base_url=base)
                self.events.put(("ai_done",{"result":result,"prompt":prompt,"provider":provider,"model":model,"task":task,"project":project}))
            except Exception as exc:
                self.events.put(("ai_error",f"AI request failed: {exc}"))
        threading.Thread(target=run,daemon=True).start()

    def _finish_ai(self,payload):
        self.ai_busy=False; self.ai_run_btn.configure(state="normal"); result=payload["result"]
        self.ai_result_text.delete("1.0","end"); self.ai_result_text.insert("1.0",result)
        run_id=uuid.uuid4().hex[:16]
        with sqlite3.connect(DB_FILE) as db:
            db.execute("INSERT INTO ai_runs(id,created_at,project,task,mode,provider,model,source_title,prompt,result) VALUES(?,?,?,?,?,?,?,?,?,?)",(run_id,datetime.now().strftime("%Y-%m-%d %H:%M:%S"),payload["project"],payload["task"],"api",payload["provider"],payload["model"],self.ai_topic_var.get()[:200],payload["prompt"],result))
        self.ai_status.configure(text=f"Finished with {payload['provider']} / {payload['model']}. Saved to local library."); self._refresh_library()

    def _copy_ai_result(self):
        text=self.ai_result_text.get("1.0","end-1c").strip()
        if text: self.clipboard_clear(); self.clipboard_append(text); self.ai_status.configure(text="AI result copied.")
