from __future__ import annotations

import subprocess
from pathlib import Path

BASE_COMMIT = "31df74e452781673efc4135f09eddbd53183344e"
SERVICES_PATH = "apps/desktop/src-tauri/src/services.rs"


def git_show(path: str) -> str:
    result = subprocess.run(
        ["git", "show", f"{BASE_COMMIT}:{path}"],
        check=True,
        capture_output=True,
        text=True,
    )
    return result.stdout


def replace_once(text: str, old: str, new: str, label: str) -> str:
    if old not in text:
        raise RuntimeError(f"expected services.rs fragment not found: {label}")
    return text.replace(old, new, 1)


def main() -> None:
    text = git_show(SERVICES_PATH)

    text = replace_once(
        text,
        '''    pub fn list_jobs(&self) -> RepositoryResult<Vec<UiJob>> {
        let active_project = self.active_project_id()?;
        self.store
            .list_jobs(Some(active_project))
            .map(|jobs| jobs.iter().map(job_to_ui).collect())
    }
''',
        '''    pub fn list_jobs(&self) -> RepositoryResult<Vec<UiJob>> {
        let active_project = self.active_project_id()?;
        let jobs = self.store.list_jobs(Some(active_project))?;
        let transcript_links = self.store.list_job_transcript_links(Some(active_project))?;
        Ok(jobs
            .iter()
            .map(|job| job_to_ui(job, transcript_links.get(&job.id).copied()))
            .collect())
    }
''',
        "list_jobs lineage map",
    )

    text = replace_once(
        text,
        '''        let transcripts = self.store.list_transcripts(Some(active_project))?;
        let research_items = self.store.list_research_items(Some(active_project))?;
''',
        '''        let transcripts = self.store.list_transcripts(Some(active_project))?;
        let transcript_links = self.store.list_job_transcript_links(Some(active_project))?;
        let research_items = self.store.list_research_items(Some(active_project))?;
''',
        "bootstrap lineage map",
    )

    text = replace_once(
        text,
        '''        let jobs = all_jobs
            .iter()
            .filter(|job| job.project_id == active_project)
            .map(job_to_ui)
            .collect::<Vec<_>>();
''',
        '''        let jobs = all_jobs
            .iter()
            .filter(|job| job.project_id == active_project)
            .map(|job| job_to_ui(job, transcript_links.get(&job.id).copied()))
            .collect::<Vec<_>>();
''',
        "bootstrap job mapping",
    )

    text = replace_once(
        text,
        "fn job_to_ui(job: &Job) -> UiJob {",
        "fn job_to_ui(job: &Job, completed_transcript_id: Option<Uuid>) -> UiJob {",
        "job_to_ui signature",
    )

    text = replace_once(
        text,
        '''        updated_at: job.updated_at.clone(),
        detail: job.last_error.clone(),
    }
''',
        '''        updated_at: job.updated_at.clone(),
        detail: job.last_error.clone(),
        completed_transcript_id: completed_transcript_id.map(|id| id.to_string()),
    }
''',
        "job_to_ui completed transcript field",
    )

    Path(SERVICES_PATH).write_text(text)


if __name__ == "__main__":
    main()
