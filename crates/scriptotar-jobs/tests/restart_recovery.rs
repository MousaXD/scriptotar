use scriptotar_core::{JobInput, JobRepository, JobState, Project, ProjectRepository};
use scriptotar_db::SqliteStore;
use scriptotar_jobs::JobService;
use tempfile::TempDir;

#[test]
fn persisted_running_job_is_interrupted_after_restart_and_requires_explicit_retry() {
    let temp = TempDir::new().unwrap();
    let database = temp.path().join("scriptotar.sqlite3");

    let store = SqliteStore::open(&database).unwrap();
    let project = Project::new("Restart proof");
    store.create_project(&project).unwrap();
    let service = JobService::new(store.clone());
    let job = service
        .enqueue(
            project.id,
            JobInput::LocalFile(temp.path().join("clip.mp4").to_string_lossy().into_owned()),
        )
        .unwrap();
    service.advance(job.id, JobState::Preparing).unwrap();
    service.advance(job.id, JobState::Transcribing).unwrap();
    drop(service);
    drop(store);

    let reopened = SqliteStore::open(&database).unwrap();
    let restarted = JobService::new(reopened.clone());
    assert_eq!(restarted.recover_after_unclean_shutdown().unwrap(), 1);
    assert_eq!(
        reopened.get_job(job.id).unwrap().state,
        JobState::Interrupted
    );

    let retried = restarted.retry(job.id).unwrap();
    assert_eq!(retried.state, JobState::Queued);
    assert_eq!(reopened.get_job(job.id).unwrap().state, JobState::Queued);
}
