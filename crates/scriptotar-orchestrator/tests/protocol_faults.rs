use std::{fs, path::Path, thread, time::Duration};

use scriptotar_core::{
    ContentRepository, Job, JobInput, JobRepository, JobState, Project, ProjectRepository,
    SettingsRepository,
};
use scriptotar_db::SqliteStore;
use scriptotar_orchestrator::{JobOrchestrator, RuntimeConfig};
use tempfile::TempDir;
use uuid::Uuid;

fn setup() -> (TempDir, SqliteStore, JobOrchestrator<SqliteStore>) {
    let temp = TempDir::new().unwrap();
    let store = SqliteStore::open(temp.path().join("protocol.sqlite3")).unwrap();
    store.run_integration_migrations().unwrap();
    let project = Project::new("Protocol boundary");
    store.create_project(&project).unwrap();
    let mut settings = store.load_settings().unwrap();
    settings.output_directory = Some(temp.path().join("output").to_string_lossy().into_owned());
    store.save_settings(&settings).unwrap();

    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixture_sidecar.py");
    let config = RuntimeConfig::new("python3", fixture, temp.path().join("output"));
    let orchestrator = JobOrchestrator::start(store.clone(), config);
    (temp, store, orchestrator)
}

fn enqueue_file(
    store: &SqliteStore,
    orchestrator: &JobOrchestrator<SqliteStore>,
    path: &Path,
) -> Job {
    fs::write(path, b"fixture").unwrap();
    let project_id = store.list_projects().unwrap()[0].id;
    let job = Job::new(
        project_id,
        JobInput::LocalFile(path.to_string_lossy().into_owned()),
    );
    store.insert_job(&job).unwrap();
    orchestrator.enqueue(job.id).unwrap();
    job
}

fn wait_for_terminal(store: &SqliteStore, job_id: Uuid) -> Job {
    for _ in 0..300 {
        let job = store.get_job(job_id).unwrap();
        if matches!(
            job.state,
            JobState::Completed | JobState::Failed | JobState::Cancelled | JobState::Interrupted
        ) {
            return job;
        }
        thread::sleep(Duration::from_millis(20));
    }
    panic!("job did not become terminal");
}

fn assert_protocol_fault_recovers(file_name: &str) {
    let (temp, store, orchestrator) = setup();
    let broken = enqueue_file(&store, &orchestrator, &temp.path().join(file_name));
    let next = enqueue_file(&store, &orchestrator, &temp.path().join("normal.mp4"));

    let failed = wait_for_terminal(&store, broken.id);
    assert_eq!(failed.state, JobState::Failed);
    assert!(
        failed
            .last_error
            .as_deref()
            .is_some_and(|message| message.contains("sidecar protocol error")),
        "unexpected persisted protocol error: {:?}",
        failed.last_error
    );
    assert_eq!(wait_for_terminal(&store, next.id).state, JobState::Completed);
    assert_eq!(store.list_transcripts(Some(broken.project_id)).unwrap().len(), 1);
}

#[test]
fn malformed_json_fails_current_job_and_later_job_succeeds() {
    assert_protocol_fault_recovers("malformed.mp4");
}

#[test]
fn wrong_protocol_version_is_rejected_and_later_job_succeeds() {
    assert_protocol_fault_recovers("wrong-protocol.mp4");
}

#[test]
fn wrong_job_id_never_commits_to_the_active_job() {
    assert_protocol_fault_recovers("wrong-job-id.mp4");
}

#[test]
fn nonzero_sidecar_exit_fails_current_job_and_later_job_succeeds() {
    let (temp, store, orchestrator) = setup();
    let broken = enqueue_file(&store, &orchestrator, &temp.path().join("sidecar-exit.mp4"));
    let next = enqueue_file(&store, &orchestrator, &temp.path().join("normal.mp4"));

    let failed = wait_for_terminal(&store, broken.id);
    assert_eq!(failed.state, JobState::Failed);
    assert!(
        failed
            .last_error
            .as_deref()
            .is_some_and(|message| message.contains("sidecar exited")),
        "unexpected persisted exit error: {:?}",
        failed.last_error
    );
    assert_eq!(wait_for_terminal(&store, next.id).state, JobState::Completed);
}
