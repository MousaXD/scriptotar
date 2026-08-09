#![cfg(target_os = "linux")]

use std::{fs, path::Path, thread, time::Duration};

use scriptotar_core::{
    Job, JobInput, JobRepository, JobState, Project, ProjectRepository, SettingsRepository,
};
use scriptotar_db::SqliteStore;
use scriptotar_orchestrator::{JobOrchestrator, RuntimeConfig};
use tempfile::TempDir;
use uuid::Uuid;

fn wait_for_state(store: &SqliteStore, job_id: Uuid, wanted: JobState) -> Job {
    for _ in 0..300 {
        let job = store.get_job(job_id).unwrap();
        if job.state == wanted {
            return job;
        }
        if matches!(
            job.state,
            JobState::Failed | JobState::Completed | JobState::Interrupted
        ) && job.state != wanted
        {
            panic!("job reached unexpected terminal state: {:?}", job.state);
        }
        thread::sleep(Duration::from_millis(20));
    }
    panic!("job did not reach {wanted:?}");
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

fn enqueue_file(
    store: &SqliteStore,
    orchestrator: &JobOrchestrator<SqliteStore>,
    project_id: Uuid,
    path: &Path,
) -> Job {
    fs::write(path, b"fixture").unwrap();
    let job = Job::new(
        project_id,
        JobInput::LocalFile(path.to_string_lossy().into_owned()),
    );
    store.insert_job(&job).unwrap();
    orchestrator.enqueue(job.id).unwrap();
    job
}

#[test]
fn cancellation_reaps_spawned_worker_process_and_next_job_succeeds() {
    let temp = TempDir::new().unwrap();
    let store = SqliteStore::open(temp.path().join("cancel.sqlite3")).unwrap();
    store.run_integration_migrations().unwrap();
    let project = Project::new("Cancellation proof");
    store.create_project(&project).unwrap();
    let mut settings = store.load_settings().unwrap();
    settings.output_directory = Some(temp.path().join("output").to_string_lossy().into_owned());
    store.save_settings(&settings).unwrap();

    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let child_pid_file = temp.path().join("child.pid");
    let config = RuntimeConfig::new(
        "python3",
        root.join("sidecars/transcription/sidecar.py"),
        temp.path().join("output"),
    )
    .with_environment(
        "SCRIPTOTAR_SIDECAR_ENGINE_WORKER",
        root.join("sidecars/transcription/tests/fake_engine_worker.py")
            .to_string_lossy()
            .into_owned(),
    )
    .with_environment(
        "SCRIPTOTAR_TEST_CHILD_PID_FILE",
        child_pid_file.to_string_lossy().into_owned(),
    );
    let orchestrator = JobOrchestrator::start(store.clone(), config);

    let blocked = enqueue_file(
        &store,
        &orchestrator,
        project.id,
        &temp.path().join("spawn-child.mp4"),
    );
    wait_for_state(&store, blocked.id, JobState::Transcribing);

    for _ in 0..200 {
        if child_pid_file.is_file() {
            break;
        }
        thread::sleep(Duration::from_millis(20));
    }
    let child_pid: u32 = fs::read_to_string(&child_pid_file)
        .expect("fake worker must expose its child pid")
        .trim()
        .parse()
        .unwrap();
    assert!(Path::new(&format!("/proc/{child_pid}")).exists());

    orchestrator.cancel(blocked.id).unwrap();
    assert_eq!(
        wait_for_terminal(&store, blocked.id).state,
        JobState::Cancelled
    );

    for _ in 0..200 {
        if !Path::new(&format!("/proc/{child_pid}")).exists() {
            break;
        }
        thread::sleep(Duration::from_millis(20));
    }
    assert!(
        !Path::new(&format!("/proc/{child_pid}")).exists(),
        "cancelled transcription left child process {child_pid} alive"
    );

    let next = enqueue_file(
        &store,
        &orchestrator,
        project.id,
        &temp.path().join("normal.mp4"),
    );
    assert_eq!(
        wait_for_terminal(&store, next.id).state,
        JobState::Completed
    );
}
