#!/usr/bin/env python3
from __future__ import annotations

from scriptotar_common import *


class LibraryMixin:
    def _build_library_tab(self):
        tab=self.library_tab
        bar=tk.Frame(tab,bg=BG); bar.pack(fill="x",pady=(14,8))
        tk.Label(bar,text="Search your own transcripts, competitor research, and AI outputs",bg=BG,fg=MUTED,font=("Sans",10)).pack(side="left")
        self.library_search_var=tk.StringVar(); ent=self._entry(bar,self.library_search_var); ent.pack(side="left",fill="x",expand=True,padx=(14,8)); ent.bind("<Return>",lambda _e:self._refresh_library())
        self._button(bar,"Search",self._refresh_library,secondary=True,compact=True).pack(side="left")
        card=self._card(tab); card.pack(fill="both",expand=True)
        self.library_tree=ttk.Treeview(card,columns=("kind","title","project","platform","metric","date"),show="headings")
        for col,title,width in [("kind","Kind",90),("title","Title",420),("project","Project",120),("platform","Platform",90),("metric","Views / Task",130),("date","Date",140)]:
            self.library_tree.heading(col,text=title); self.library_tree.column(col,width=width,stretch=col=="title")
        self.library_tree.pack(fill="both",expand=True,padx=12,pady=12); self.library_tree.bind("<Double-1>",lambda _e:self._library_use_in_ai())
        actions=tk.Frame(tab,bg=BG); actions.pack(fill="x",pady=(10,0)); self._button(actions,"Use in AI",self._library_use_in_ai).pack(side="left"); self._button(actions,"Open / queue",self._library_open_or_queue,secondary=True).pack(side="left",padx=(8,0)); self._button(actions,"Refresh",self._refresh_library,secondary=True).pack(side="right")

    def _refresh_library(self):
        if not hasattr(self,"library_tree"): return
        for iid in self.library_tree.get_children(): self.library_tree.delete(iid)
        self.library_rows={}; project=self.project_var.get() or "Inbox"; query=(self.library_search_var.get().strip() if hasattr(self,"library_search_var") else "").casefold()
        with sqlite3.connect(DB_FILE) as db:
            jobs=db.execute("SELECT id,created_at,COALESCE(title,''),source,transcript,output_dir,COALESCE(language,'') FROM jobs WHERE project=? AND status='Done' ORDER BY created_at DESC LIMIT 300",(project,)).fetchall()
            research=db.execute("SELECT storage_id,scanned_at,title,source_url,platform,view_count,raw_json FROM research_items WHERE project=? ORDER BY scanned_at DESC LIMIT 500",(project,)).fetchall()
            runs=db.execute("SELECT id,created_at,task,COALESCE(source_title,''),provider,model,prompt,result FROM ai_runs WHERE project=? ORDER BY created_at DESC LIMIT 300",(project,)).fetchall()
        for row in jobs:
            hay=f"{row[2]} {row[3]}".casefold()
            if query and query not in hay: continue
            iid="job:"+row[0]; self.library_rows[iid]={"kind":"Transcript","title":row[2] or Path(row[3]).name,"source":row[3],"transcript":row[4],"output":row[5],"language":row[6]}; self.library_tree.insert("","end",iid=iid,values=("Transcript",self.library_rows[iid]["title"],project,"",row[6] or "—",row[1]))
        for row in research:
            hay=f"{row[2]} {row[3]} {row[4]}".casefold()
            if query and query not in hay: continue
            iid="research:"+row[0]; self.library_rows[iid]={"kind":"Research","title":row[2],"source":row[3],"platform":row[4],"views":row[5],"raw_json":row[6]}; self.library_tree.insert("","end",iid=iid,values=("Research",row[2][:160],project,row[4],self._metric(row[5]),row[1]))
        for row in runs:
            hay=f"{row[2]} {row[3]} {row[4]} {row[5]} {row[7] or ''}".casefold()
            if query and query not in hay: continue
            iid="ai:"+row[0]; self.library_rows[iid]={"kind":"AI","title":row[3] or row[2],"task":row[2],"prompt":row[6],"result":row[7] or ""}; self.library_tree.insert("","end",iid=iid,values=("AI",self.library_rows[iid]["title"][:160],project,row[4],row[2],row[1]))

    def _selected_library(self):
        sel=self.library_tree.selection() if hasattr(self,"library_tree") else (); return self.library_rows.get(sel[0]) if sel else None

    def _library_use_in_ai(self):
        item=self._selected_library()
        if not item: return
        text=""
        if item["kind"]=="Transcript" and item.get("transcript") and Path(item["transcript"]).is_file(): text=Path(item["transcript"]).read_text(encoding="utf-8")
        elif item["kind"]=="Research":
            text=item.get("title","")
            try:
                raw=json.loads(item.get("raw_json") or "{}"); desc=str(raw.get("description") or "").strip()
                if desc and desc!=text: text += "\n\n"+desc
            except Exception: pass
        elif item["kind"]=="AI": text=item.get("result") or item.get("prompt") or ""
        self.ai_source_text.delete("1.0","end"); self.ai_source_text.insert("1.0",text); self.ai_topic_var.set(item.get("title","")[:180]); self.notebook.select(self.ai_tab); self._update_script_timer()

    def _library_open_or_queue(self):
        item=self._selected_library()
        if not item:return
        if item["kind"]=="Transcript":
            if item.get("transcript") and Path(item["transcript"]).is_file(): self._load_transcript(item["transcript"],item.get("language"),item.get("title")); self.notebook.select(self.transcript_tab)
        elif item["kind"]=="Research" and item.get("source"): self._enqueue(self._new_job("url",item["source"])); self.notebook.select(self.queue_tab)
        elif item["kind"]=="AI": self._library_use_in_ai()

    def _on_close(self):
        if self.current_job_id:
            if not messagebox.askyesno(APP, "A transcription is still running. Cancel it and quit?"):
                return
        self._save_settings()
        self._kill_worker_group()
        if self.research_proc and self.research_proc.poll() is None:
            try:
                os.killpg(os.getpgid(self.research_proc.pid), signal.SIGTERM)
            except Exception:
                self.research_proc.terminate()
        self.destroy()
