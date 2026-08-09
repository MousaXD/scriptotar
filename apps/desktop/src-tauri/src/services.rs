use std::path::Path;

use scriptotar_ai::{EndpointPolicy, ProviderConfig};
use scriptotar_core::{
    ApplicationSettings, Job, JobInput, Project, ProjectRepository, RepositoryResult,
    SettingsRepository,
};
use scriptotar_db::SqliteStore;
use scriptotar_jobs::JobService;
use scriptotar_research::NetworkPolicy;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct AppServices {
    store: SqliteStore,
}

impl AppServices {
    pub fn new(database_path: impl AsRef<Path>) -> RepositoryResult<Self> {
        let store = SqliteStore::open(database_path)?;
        JobService::new(store.clone()).recover_after_unclean_shutdown()?;
        Ok(Self { store })
    }

    pub fn schema_version(&self) -> RepositoryResult<u32> {
        self.store.schema_version()
    }

    pub fn create_project(&self, name: String) -> RepositoryResult<Project> {
        let project = Project::new(name.trim());
        self.store.create_project(&project)?;
        Ok(project)
    }

    pub fn list_projects(&self) -> RepositoryResult<Vec<Project>> {
        self.store.list_projects()
    }

    pub fn enqueue_job(&self, project_id: Uuid, input: JobInput) -> RepositoryResult<Job> {
        self.store.get_project(project_id)?;
        JobService::new(self.store.clone()).enqueue(project_id, input)
    }

    pub fn list_jobs(&self, project_id: Option<Uuid>) -> RepositoryResult<Vec<Job>> {
        JobService::new(self.store.clone()).list(project_id)
    }

    pub fn cancel_job(&self, job_id: Uuid) -> RepositoryResult<Job> {
        JobService::new(self.store.clone()).cancel(job_id)
    }

    pub fn retry_job(&self, job_id: Uuid) -> RepositoryResult<Job> {
        JobService::new(self.store.clone()).retry(job_id)
    }

    pub fn load_settings(&self) -> RepositoryResult<ApplicationSettings> {
        self.store.load_settings()
    }

    pub fn save_settings(&self, settings: &ApplicationSettings) -> RepositoryResult<()> {
        self.store.save_settings(settings)
    }

    pub fn validate_research_url(&self, url: &str) -> Result<String, String> {
        NetworkPolicy
            .validate(url)
            .map(|validated| validated.as_url().to_string())
            .map_err(|error| error.to_string())
    }

    pub fn validate_ai_endpoint(&self, config: &ProviderConfig) -> Result<String, String> {
        EndpointPolicy
            .endpoint_for(config)
            .map(|endpoint| endpoint.as_url().to_string())
            .map_err(|error| error.to_string())
    }
}
