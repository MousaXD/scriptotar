use scriptotar_core::{Job, JobInput, JobRepository, JobState, RepositoryResult};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct JobService<R> {
    repository: R,
}

impl<R> JobService<R>
where
    R: JobRepository,
{
    pub fn new(repository: R) -> Self {
        Self { repository }
    }

    pub fn enqueue(&self, project_id: Uuid, input: JobInput) -> RepositoryResult<Job> {
        let job = Job::new(project_id, input);
        self.repository.insert_job(&job)?;
        Ok(job)
    }

    pub fn list(&self, project_id: Option<Uuid>) -> RepositoryResult<Vec<Job>> {
        self.repository.list_jobs(project_id)
    }

    pub fn advance(&self, job_id: Uuid, next: JobState) -> RepositoryResult<Job> {
        self.repository.transition_job(job_id, next)
    }

    pub fn cancel(&self, job_id: Uuid) -> RepositoryResult<Job> {
        self.repository.transition_job(job_id, JobState::Cancelled)
    }

    pub fn retry(&self, job_id: Uuid) -> RepositoryResult<Job> {
        self.repository.transition_job(job_id, JobState::Queued)
    }

    pub fn recover_after_unclean_shutdown(&self) -> RepositoryResult<usize> {
        self.repository.mark_active_jobs_interrupted()
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::HashMap,
        sync::{Arc, Mutex},
    };

    use scriptotar_core::{RepositoryError, RepositoryResult};

    use super::*;

    #[derive(Clone, Default)]
    struct FakeRepository {
        jobs: Arc<Mutex<HashMap<Uuid, Job>>>,
    }

    impl JobRepository for FakeRepository {
        fn insert_job(&self, job: &Job) -> RepositoryResult<()> {
            self.jobs.lock().unwrap().insert(job.id, job.clone());
            Ok(())
        }

        fn get_job(&self, id: Uuid) -> RepositoryResult<Job> {
            self.jobs
                .lock()
                .unwrap()
                .get(&id)
                .cloned()
                .ok_or_else(|| RepositoryError::NotFound(format!("job {id}")))
        }

        fn list_jobs(&self, project_id: Option<Uuid>) -> RepositoryResult<Vec<Job>> {
            Ok(self
                .jobs
                .lock()
                .unwrap()
                .values()
                .filter(|job| project_id.is_none_or(|project_id| job.project_id == project_id))
                .cloned()
                .collect())
        }

        fn transition_job(&self, id: Uuid, next: JobState) -> RepositoryResult<Job> {
            let mut jobs = self.jobs.lock().unwrap();
            let job = jobs
                .get_mut(&id)
                .ok_or_else(|| RepositoryError::NotFound(format!("job {id}")))?;
            job.state.validate_transition(next)?;
            job.state = next;
            Ok(job.clone())
        }

        fn mark_active_jobs_interrupted(&self) -> RepositoryResult<usize> {
            let mut jobs = self.jobs.lock().unwrap();
            let mut count = 0;
            for job in jobs.values_mut() {
                if job.state.is_active() {
                    job.state = JobState::Interrupted;
                    count += 1;
                }
            }
            Ok(count)
        }
    }

    #[test]
    fn enqueue_starts_queued() {
        let repository = FakeRepository::default();
        let service = JobService::new(repository);
        let job = service
            .enqueue(
                Uuid::new_v4(),
                JobInput::LocalFile("/tmp/a.mp4".to_owned()),
            )
            .unwrap();
        assert_eq!(job.state, JobState::Queued);
    }

    #[test]
    fn interrupted_jobs_are_recoverable_but_not_silently_resumed() {
        let repository = FakeRepository::default();
        let service = JobService::new(repository.clone());
        let job = service
            .enqueue(
                Uuid::new_v4(),
                JobInput::LocalFile("/tmp/a.mp4".to_owned()),
            )
            .unwrap();
        service.advance(job.id, JobState::Preparing).unwrap();
        service.advance(job.id, JobState::Transcribing).unwrap();

        assert_eq!(service.recover_after_unclean_shutdown().unwrap(), 1);
        assert_eq!(
            repository.get_job(job.id).unwrap().state,
            JobState::Interrupted
        );
        assert_eq!(service.retry(job.id).unwrap().state, JobState::Queued);
    }

    #[test]
    fn completed_job_cannot_be_retried() {
        let repository = FakeRepository::default();
        let service = JobService::new(repository);
        let job = service
            .enqueue(
                Uuid::new_v4(),
                JobInput::LocalFile("/tmp/a.mp4".to_owned()),
            )
            .unwrap();
        for next in [
            JobState::Preparing,
            JobState::Transcribing,
            JobState::Processing,
            JobState::Completed,
        ] {
            service.advance(job.id, next).unwrap();
        }
        assert!(service.retry(job.id).is_err());
    }
}
