use std::sync::{Arc, Barrier};
use std::thread;

use scriptotar_core::{JobInput, JobRepository, JobState, Project, ProjectRepository};
use scriptotar_db::SqliteStore;
use scriptotar_jobs::JobService;
use tempfile::TempDir;

#[test]
fn concurrent_job_transitions_cannot_both_advance_a_queued_job() {
    let temp = TempDir::new().unwrap();
    let store = SqliteStore::open(temp.path().join("scriptotar.sqlite3")).unwrap();
    let project = Project::new("Concurrent job transitions");
    store.create_project(&project).unwrap();
    let service = JobService::new(store.clone());
    let job = service
        .enqueue(
            project.id,
            JobInput::LocalFile("/tmp/concurrency.mp4".to_owned()),
        )
        .unwrap();
    let barrier = Arc::new(Barrier::new(3));

    let prepare_service = service.clone();
    let prepare_barrier = barrier.clone();
    let job_id = job.id;
    let prepare = thread::spawn(move || {
        prepare_barrier.wait();
        prepare_service.advance(job_id, JobState::Preparing).is_ok()
    });

    let cancel_service = service.clone();
    let cancel_barrier = barrier.clone();
    let cancel = thread::spawn(move || {
        cancel_barrier.wait();
        cancel_service.cancel(job_id).is_ok()
    });

    barrier.wait();
    let prepared = prepare.join().unwrap();
    let cancelled = cancel.join().unwrap();
    assert_ne!(prepared, cancelled, "exactly one transition must win");

    let persisted = store.get_job(job.id).unwrap();
    assert_eq!(
        persisted.state,
        if prepared {
            JobState::Preparing
        } else {
            JobState::Cancelled
        }
    );
    drop(store);

    let reopened = SqliteStore::open(temp.path().join("scriptotar.sqlite3")).unwrap();
    assert_eq!(reopened.get_job(job.id).unwrap().state, persisted.state);
}

#[test]
fn concurrent_restart_recovery_is_idempotent() {
    let temp = TempDir::new().unwrap();
    let store = SqliteStore::open(temp.path().join("scriptotar.sqlite3")).unwrap();
    let project = Project::new("Recovery concurrency");
    store.create_project(&project).unwrap();
    let service = JobService::new(store.clone());
    let job = service
        .enqueue(
            project.id,
            JobInput::LocalFile("/tmp/recover.mp4".to_owned()),
        )
        .unwrap();
    service.advance(job.id, JobState::Preparing).unwrap();

    let barrier = Arc::new(Barrier::new(3));
    let mut handles = Vec::new();
    for _ in 0..2 {
        let service = service.clone();
        let barrier = barrier.clone();
        handles.push(thread::spawn(move || {
            barrier.wait();
            service.recover_after_unclean_shutdown().unwrap()
        }));
    }
    barrier.wait();
    let total: usize = handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .sum();
    assert_eq!(total, 1);
    assert_eq!(store.get_job(job.id).unwrap().state, JobState::Interrupted);
}
