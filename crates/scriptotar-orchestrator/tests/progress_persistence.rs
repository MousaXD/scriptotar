use std::{fs, path::Path, thread, time::{Duration, Instant}};

use scriptotar_core::{
    Job, JobInput, JobRepository, JobState, Project, ProjectRepository, SettingsRepository,
};
use scriptotar_db::SqliteStore;
use scriptotar_orchestrator::{JobOrchestrator, RuntimeConfig};
use tempfile::TempDir;

#[test]
fn sidecar_progress_is_persisted_before_completion() {
    let temp = TempDir::new().unwrap();
    let store = SqliteStore::open(temp.path().join("progress.sqlite3")).unwrap();
    store.run_integration_migrations().unwrap();
    let project = Project::new("Progress persistence");
    store.create_project(&project).unwrap();
    let mut settings = store.load_settings().unwrap();
    settings.output_directory = Some(temp.path().join("output").to_string_lossy().into_owned());
    store.save_settings(&settings).unwrap();

    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixture_sidecar.py");
    let orchestrator = JobOrchestrator::start(
        store.clone(),
        RuntimeConfig::new("python3", fixture, temp.path().join("output")),
    );

    let input = temp.path().join("progress-hold.mp4");
    fs::write(&input, b"fixture").unwrap();
    let job = Job::new(
        project.id,
        JobInput::LocalFile(input.to_string_lossy().into_owned()),
    );
    store.insert_job(&job).unwrap();
    orchestrator.enqueue(job.id).unwrap();

    // The actual background orchestrator must commit the sidecar's event
    // before the fixture is permitted to emit a result. This is a handshake
    // over a test-only file, not a mocked repository write or sleep window.
    let deadline = Instant::now() + Duration::from_secs(15);
    let observed = loop {
        let current = store.get_job(job.id).unwrap();
        if current.state == JobState::Transcribing && current.progress == Some(0.5) {
            break current;
        }
        if matches!(
            current.state,
            JobState::Completed | JobState::Failed | JobState::Cancelled | JobState::Interrupted
        ) || Instant::now() >= deadline {
            panic!(
                "transcribing progress was not durably observable: state={:?}, progress={:?}",
                current.state, current.progress
            );
        }
        thread::sleep(Duration::from_millis(10));
    };
    assert_eq!(observed.progress, Some(0.5));
    let observation_ack = input.with_extension("progress-observed");
    fs::write(&observation_ack, b"observed committed transcribing progress").unwrap();

    for _ in 0..300 {
        let current = store.get_job(job.id).unwrap();
        if current.state == JobState::Completed {
            assert_eq!(current.progress, Some(1.0));
            return;
        }
        thread::sleep(Duration::from_millis(10));
    }
    panic!("job did not complete after persisted progress");
}
