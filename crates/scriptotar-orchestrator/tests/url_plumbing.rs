use std::{path::Path, thread, time::Duration};

use scriptotar_core::{
    ContentRepository, Job, JobInput, JobRepository, JobState, Project, ProjectRepository,
    SettingsRepository, SourceType,
};
use scriptotar_db::SqliteStore;
use scriptotar_orchestrator::{JobOrchestrator, RuntimeConfig};
use tempfile::TempDir;

#[test]
fn url_job_flows_through_download_stage_and_persists_url_source() {
    let temp = TempDir::new().unwrap();
    let store = SqliteStore::open(temp.path().join("url.sqlite3")).unwrap();
    store.run_integration_migrations().unwrap();
    let project = Project::new("URL plumbing");
    store.create_project(&project).unwrap();
    let mut settings = store.load_settings().unwrap();
    settings.output_directory = Some(temp.path().join("output").to_string_lossy().into_owned());
    store.save_settings(&settings).unwrap();

    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixture_sidecar.py");
    let orchestrator = JobOrchestrator::start(
        store.clone(),
        RuntimeConfig::new("python3", fixture, temp.path().join("output")),
    );
    let source_url = "https://www.youtube.com/watch?v=scriptotar-fixture";
    let job = Job::new(project.id, JobInput::Url(source_url.to_owned()));
    store.insert_job(&job).unwrap();
    orchestrator.enqueue(job.id).unwrap();

    let mut saw_downloading = false;
    let completed = loop {
        let current = store.get_job(job.id).unwrap();
        saw_downloading |= current.state == JobState::Downloading;
        if matches!(
            current.state,
            JobState::Completed | JobState::Failed | JobState::Cancelled | JobState::Interrupted
        ) {
            break current;
        }
        thread::sleep(Duration::from_millis(10));
    };

    assert_eq!(completed.state, JobState::Completed);
    assert!(saw_downloading, "URL job never entered downloading state");
    let transcripts = store.list_transcripts(Some(project.id)).unwrap();
    assert_eq!(transcripts.len(), 1);
    assert_eq!(transcripts[0].source.source_type, SourceType::Url);
    assert_eq!(transcripts[0].source.locator, source_url);
    assert_eq!(
        transcripts[0].media.local_path,
        "/tmp/downloaded-fixture.mp4"
    );
}
