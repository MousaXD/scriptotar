use std::{
    collections::{HashMap, HashSet},
    env,
    path::{Path, PathBuf},
    sync::{
        mpsc::{self, RecvTimeoutError, Sender},
        Arc, Mutex,
    },
    thread::{self, JoinHandle},
    time::Duration,
};

use chrono::{DateTime, Utc};
use scriptotar_ai::{
    AiRequest, AiService, EndpointPolicy, HttpAiProvider, ProviderConfig, ProviderKind,
};
use scriptotar_core::{
    now_rfc3339, AiRun, AiRunMode, AiRunRepository, ApplicationSettings, ContentRepository,
    Creator, Job, JobInput, JobRepository, JobState, Project, ProjectRepository, RepositoryError,
    RepositoryResult, ResearchItem, ResearchRepository, SettingsRepository, SourceType,
    TranscriptBundle, Watchlist, WatchlistRepository,
};
use scriptotar_db::{SqliteStore, WatchlistRefreshState};
use scriptotar_jobs::JobService;
use scriptotar_media::MediaPolicy;
use scriptotar_orchestrator::{JobChangeNotifier, JobOrchestrator, RuntimeConfig};
use scriptotar_research::{NetworkPolicy, ResearchService, YtDlpCommand, YtDlpProvider};
use serde_json::Value;
use uuid::Uuid;

use crate::dto::{
    AiPromptInput, BootstrapData, ResearchQuery, UiAiRun, UiCreator, UiJob, UiLibraryItem,
    UiProject, UiResearchItem, UiSettings, UiTranscript, UiTranscriptSegment,
};

const MAX_RESEARCH_QUEUE_ITEMS: usize = 200;
const MAX_AI_SOURCE_CHARS: usize = 450_000;
const MAX_AI_CONTEXT_CHARS: usize = 20_000;
const WATCHLIST_TICK: Duration = Duration::from_secs(60);
const MAX_PERSISTED_RETRY_DELAY: Duration = Duration::from_secs(24 * 60 * 60);

#[derive(Clone)]
struct WatchlistRefresher {
    _inner: Arc<WatchlistRefresherInner>,
}

struct WatchlistRefresherInner {
    stop: Sender<()>,
    handle: Mutex<Option<JoinHandle<()>>>,
}

impl Drop for WatchlistRefresherInner {
    fn drop(&mut self) {
        let _ = self.stop.send(());
        if let Ok(handle) = self.handle.get_mut() {
            if let Some(handle) = handle.take() {
                let _ = handle.join();
            }
        }
    }
}

#[derive(Clone)]
pub struct AppServices {
    store: SqliteStore,
    orchestrator: JobOrchestrator<SqliteStore>,
    active_project: Arc<Mutex<Uuid>>,
    legacy_path: PathBuf,
    research_command: YtDlpCommand,
    _watchlist_refresher: WatchlistRefresher,
}

impl AppServices {
    pub fn new(data_dir: impl AsRef<Path>) -> RepositoryResult<Self> {
        Self::new_with_job_notifier(data_dir, Arc::new(|_| {}))
    }

    pub fn new_with_job_notifier(
        data_dir: impl AsRef<Path>,
        notifier: JobChangeNotifier,
    ) -> RepositoryResult<Self> {
        let data_dir = data_dir.as_ref();
        std::fs::create_dir_all(data_dir)
            .map_err(|error| RepositoryError::Storage(error.to_string()))?;
        let store = SqliteStore::open(data_dir.join("scriptotar.sqlite3"))?;
        store.run_integration_migrations()?;
        store.recover_interrupted_watchlist_refreshes()?;
        JobService::new(store.clone()).recover_after_unclean_shutdown()?;

        let legacy_path = data_dir.join("history.sqlite3");

        let mut projects = store.list_projects()?;
        if projects.is_empty() {
            let inbox = Project::new("Inbox");
            store.create_project(&inbox)?;
            projects.push(inbox);
        }
        let fallback_project = projects
            .iter()
            .find(|project| project.name.eq_ignore_ascii_case("Inbox"))
            .unwrap_or(&projects[0])
            .id;
        let mut settings = store.load_settings()?;
        let active_project = settings
            .active_project_id
            .filter(|project_id| projects.iter().any(|project| project.id == *project_id))
            .unwrap_or(fallback_project);
        if settings.active_project_id != Some(active_project) {
            settings.active_project_id = Some(active_project);
            store.save_settings(&settings)?;
        }
        let orchestrator = JobOrchestrator::start_with_notifier(
            store.clone(),
            runtime_config(data_dir.join("transcription-output")),
            notifier,
        );
        let research_command = YtDlpCommand::from_environment();
        let watchlist_refresher =
            spawn_watchlist_refresher(store.clone(), research_command.clone());

        Ok(Self {
            store,
            orchestrator,
            active_project: Arc::new(Mutex::new(active_project)),
            legacy_path,
            research_command,
            _watchlist_refresher: watchlist_refresher,
        })
    }

    pub fn schema_version(&self) -> RepositoryResult<u32> {
        self.store.schema_version()
    }

    pub fn bootstrap(&self) -> RepositoryResult<BootstrapData> {
        let active_project = self.active_project_id()?;
        self.bootstrap_for(active_project)
    }

    pub fn list_jobs(&self) -> RepositoryResult<Vec<UiJob>> {
        let active_project = self.active_project_id()?;
        let jobs = self.store.list_jobs(Some(active_project))?;
        let transcript_links = self.store.list_job_transcript_links(Some(active_project))?;
        Ok(jobs
            .iter()
            .map(|job| job_to_ui(job, transcript_links.get(&job.id).copied()))
            .collect())
    }

    pub fn select_project(&self, project_id: Uuid) -> RepositoryResult<BootstrapData> {
        self.store.get_project(project_id)?;
        let mut active_project = self
            .active_project
            .lock()
            .map_err(|_| RepositoryError::Storage("active project lock poisoned".to_owned()))?;
        let mut settings = self.store.load_settings()?;
        settings.active_project_id = Some(project_id);
        self.store.save_settings(&settings)?;
        *active_project = project_id;
        drop(active_project);
        self.bootstrap_for(project_id)
    }

    pub fn create_project(&self, name: String) -> RepositoryResult<BootstrapData> {
        let name = name.trim();
        if name.is_empty() {
            return Err(RepositoryError::Validation(
                "project name cannot be empty".to_owned(),
            ));
        }
        let project = Project::new(name);
        self.store.create_project(&project)?;
        self.select_project(project.id)
    }

    pub fn enqueue_local_media(&self, project_id: Uuid, path: String) -> RepositoryResult<Job> {
        self.store.get_project(project_id)?;
        let input_path = Path::new(path.trim());
        if !input_path.is_file() {
            return Err(RepositoryError::Validation(format!(
                "local media does not exist: {}",
                input_path.display()
            )));
        }
        MediaPolicy
            .validate_local_input(input_path)
            .map_err(|error| RepositoryError::Validation(error.to_string()))?;
        let job = JobService::new(self.store.clone()).enqueue(
            project_id,
            JobInput::LocalFile(input_path.to_string_lossy().into_owned()),
        )?;
        self.orchestrator
            .enqueue(job.id)
            .map_err(|error| RepositoryError::Storage(error.to_string()))?;
        Ok(job)
    }

    pub fn enqueue_url(&self, project_id: Uuid, url: String) -> RepositoryResult<Job> {
        self.store.get_project(project_id)?;
        let validated = NetworkPolicy
            .validate(url.trim())
            .map_err(|error| RepositoryError::Validation(error.to_string()))?;
        let job = JobService::new(self.store.clone())
            .enqueue(project_id, JobInput::Url(validated.as_url().to_string()))?;
        self.orchestrator
            .enqueue(job.id)
            .map_err(|error| RepositoryError::Storage(error.to_string()))?;
        Ok(job)
    }

    pub fn cancel_job(&self, job_id: Uuid) -> RepositoryResult<()> {
        self.store.get_job(job_id)?;
        self.orchestrator
            .cancel(job_id)
            .map_err(|error| RepositoryError::Storage(error.to_string()))
    }

    pub fn retry_job(&self, job_id: Uuid) -> RepositoryResult<Job> {
        let job = JobService::new(self.store.clone()).retry(job_id)?;
        self.orchestrator
            .enqueue(job.id)
            .map_err(|error| RepositoryError::Storage(error.to_string()))?;
        Ok(job)
    }

    pub fn load_settings(&self) -> RepositoryResult<UiSettings> {
        Ok(settings_to_ui(&self.store.load_settings()?))
    }

    pub fn save_settings(&self, settings: UiSettings) -> RepositoryResult<()> {
        let active_project = self
            .active_project
            .lock()
            .map_err(|_| RepositoryError::Storage("active project lock poisoned".to_owned()))?;
        let current = self.store.load_settings()?;
        let mut settings = settings_from_ui(settings, current)?;
        settings.active_project_id = Some(*active_project);
        self.store.save_settings(&settings)
    }

    pub fn import_legacy_data(&self) -> RepositoryResult<scriptotar_core::LegacyImportReport> {
        self.store.import_legacy_database(&self.legacy_path)
    }

    pub fn save_watchlist(&self, query: ResearchQuery) -> Result<BootstrapData, String> {
        validate_research_limit(query.limit)?;
        let validated = NetworkPolicy
            .validate(&query.profile_url)
            .map_err(|error| error.to_string())?;
        let profile_url = validated.as_url().to_string();
        let label = profile_label(validated.as_url());
        let active_project = self
            .active_project_id()
            .map_err(|error| error.to_string())?;
        self.store
            .upsert_watchlist(active_project, &label, &profile_url, u32::from(query.limit))
            .map_err(|error| error.to_string())?;
        self.bootstrap_for(active_project)
            .map_err(|error| error.to_string())
    }

    pub fn scan_creator(&self, query: ResearchQuery) -> Result<(), String> {
        validate_research_limit(query.limit)?;
        let active_project = self
            .active_project_id()
            .map_err(|error| error.to_string())?;
        let settings = self
            .store
            .load_settings()
            .map_err(|error| error.to_string())?;
        let validated = NetworkPolicy
            .validate(&query.profile_url)
            .map_err(|error| error.to_string())?;
        let canonical_url = validated.as_url().to_string();
        let watchlist = self
            .store
            .list_watchlists(Some(active_project))
            .map_err(|error| error.to_string())?
            .into_iter()
            .find(|watchlist| watchlist.profile_url == canonical_url);
        let attempted_at = now_rfc3339();
        if let Some(watchlist) = &watchlist {
            let claimed = self
                .store
                .try_begin_watchlist_refresh(watchlist.id, &attempted_at)
                .map_err(|error| error.to_string())?;
            if !claimed {
                return Err(
                    "A refresh for this saved watchlist is already running. Try again after it finishes."
                        .to_owned(),
                );
            }
        }
        let result = scan_and_persist_research(
            &self.store,
            &self.research_command,
            active_project,
            &canonical_url,
            query.limit,
            settings.cookie_browser.as_deref(),
        );
        match result {
            Ok(_) => {
                if let Some(watchlist) = watchlist {
                    self.store
                        .record_watchlist_refresh_success(watchlist.id, &now_rfc3339())
                        .map_err(|error| error.to_string())?;
                }
                Ok(())
            }
            Err(error) => {
                let safe_error = safe_watchlist_error(&error);
                if let Some(watchlist) = watchlist {
                    if let Err(status_error) = self.store.record_watchlist_refresh_failure(
                        watchlist.id,
                        &attempted_at,
                        &safe_error,
                        None,
                    ) {
                        eprintln!(
                            "[scriptotar-watchlist] could not persist manual refresh failure: {status_error}"
                        );
                    }
                }
                Err(safe_error)
            }
        }
    }

    pub fn queue_research(&self, ids: Vec<String>) -> Result<(), String> {
        if ids.is_empty() {
            return Err("select at least one research item".to_owned());
        }
        if ids.len() > MAX_RESEARCH_QUEUE_ITEMS {
            return Err(format!(
                "select at most {MAX_RESEARCH_QUEUE_ITEMS} research items at once"
            ));
        }
        let active_project = self
            .active_project_id()
            .map_err(|error| error.to_string())?;
        let mut unique = Vec::new();
        let mut seen = HashSet::new();
        for raw in ids {
            let id = Uuid::parse_str(raw.trim())
                .map_err(|_| format!("invalid research item ID: {raw}"))?;
            if seen.insert(id) {
                unique.push(id);
            }
        }
        let items = self
            .store
            .get_research_items(&unique)
            .map_err(|error| error.to_string())?;
        if items.iter().any(|item| item.project_id != active_project) {
            return Err("research selection contains an item from another project".to_owned());
        }
        let mut existing_sources = self
            .store
            .list_jobs(Some(active_project))
            .map_err(|error| error.to_string())?
            .into_iter()
            .filter(|job| job.state == JobState::Queued || job.state.is_active())
            .filter_map(|job| match job.input {
                JobInput::Url(url) => Some(url),
                JobInput::LocalFile(_) => None,
            })
            .collect::<HashSet<_>>();

        for item in items {
            let validated = NetworkPolicy
                .validate(&item.source_url)
                .map_err(|error| error.to_string())?;
            let source_url = validated.as_url().to_string();
            if !existing_sources.insert(source_url.clone()) {
                continue;
            }
            let job = JobService::new(self.store.clone())
                .enqueue(active_project, JobInput::Url(source_url))
                .map_err(|error| error.to_string())?;
            self.orchestrator
                .enqueue(job.id)
                .map_err(|error| error.to_string())?;
        }
        Ok(())
    }

    pub fn build_ai_prompt(&self, input: &AiPromptInput) -> Result<String, String> {
        build_ai_prompt_text(input)
    }

    pub fn run_ai(&self, input: &AiPromptInput) -> Result<String, String> {
        let prompt = self.build_ai_prompt(input)?;
        let active_project = self
            .active_project_id()
            .map_err(|error| error.to_string())?;
        if !input.mode.eq_ignore_ascii_case("byok") {
            self.store
                .insert_ai_run(&AiRun {
                    id: Uuid::new_v4(),
                    project_id: active_project,
                    task: input.task.trim().to_owned(),
                    mode: AiRunMode::CopyPrompt,
                    provider: None,
                    model: None,
                    prompt: prompt.clone(),
                    result: None,
                    created_at: now_rfc3339(),
                })
                .map_err(|error| error.to_string())?;
            return Ok(prompt);
        }

        let api_key = input
            .api_key
            .as_deref()
            .map(str::trim)
            .filter(|key| !key.is_empty())
            .ok_or_else(|| "an API key is required for BYOK mode".to_owned())?;
        let provider = provider_kind(&input.provider)?;
        if provider == ProviderKind::Local {
            return Err("local AI provider execution is not enabled yet".to_owned());
        }
        let config = ProviderConfig {
            provider,
            model: input.model.trim().to_owned(),
            base_url: input
                .base_url
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_owned),
        };
        EndpointPolicy
            .endpoint_for(&config)
            .map_err(|error| error.to_string())?;
        let provider_client = HttpAiProvider::new(provider).map_err(|error| error.to_string())?;
        let response = AiService::new(provider_client)
            .generate(
                &config,
                api_key,
                &AiRequest {
                    prompt: prompt.clone(),
                },
            )
            .map_err(|error| error.to_string())?;
        self.store
            .insert_ai_run(&AiRun {
                id: Uuid::new_v4(),
                project_id: active_project,
                task: input.task.trim().to_owned(),
                mode: AiRunMode::Byok,
                provider: Some(input.provider.clone()),
                model: Some(config.model),
                prompt,
                result: Some(response.text.clone()),
                created_at: now_rfc3339(),
            })
            .map_err(|error| error.to_string())?;
        Ok(response.text)
    }

    fn active_project_id(&self) -> RepositoryResult<Uuid> {
        self.active_project
            .lock()
            .map(|value| *value)
            .map_err(|_| RepositoryError::Storage("active project lock poisoned".to_owned()))
    }

    fn bootstrap_for(&self, active_project: Uuid) -> RepositoryResult<BootstrapData> {
        let projects = self.store.list_projects()?;
        let all_jobs = self.store.list_jobs(None)?;
        let transcripts = self.store.list_transcripts(Some(active_project))?;
        let transcript_links = self.store.list_job_transcript_links(Some(active_project))?;
        let research_items = self.store.list_research_items(Some(active_project))?;
        let ai_runs = self.store.list_ai_runs(Some(active_project))?;
        let stored_creators = self.store.list_creators(Some(active_project))?;
        let watchlists = self.store.list_watchlists(Some(active_project))?;
        let watch_by_url = watchlists
            .iter()
            .map(|watchlist| (watchlist.profile_url.clone(), watchlist))
            .collect::<HashMap<_, _>>();
        let creator_by_id = stored_creators
            .iter()
            .map(|creator| (creator.id, creator))
            .collect::<HashMap<_, _>>();
        let mut creator_urls = HashSet::new();
        let mut creators = stored_creators
            .iter()
            .map(|creator| {
                creator_urls.insert(creator.profile_url.clone());
                let watchlist = watch_by_url.get(&creator.profile_url).copied();
                UiCreator {
                    id: creator.id.to_string(),
                    name: creator
                        .display_name
                        .clone()
                        .or_else(|| watchlist.map(|watchlist| watchlist.label.clone()))
                        .unwrap_or_else(|| profile_label_from_raw(&creator.profile_url)),
                    handle: creator.profile_url.clone(),
                    platform: creator.platform.clone(),
                    avatar: None,
                    watchlisted: watchlist.is_some(),
                    last_scanned_at: watchlist.and_then(|watchlist| watchlist.last_scan_at.clone()),
                }
            })
            .collect::<Vec<_>>();
        creators.extend(
            watchlists
                .iter()
                .filter(|watchlist| !creator_urls.contains(&watchlist.profile_url))
                .map(|watchlist| UiCreator {
                    id: watchlist.id.to_string(),
                    name: watchlist.label.clone(),
                    handle: watchlist.profile_url.clone(),
                    platform: source_platform(&watchlist.profile_url, SourceType::Url),
                    avatar: None,
                    watchlisted: true,
                    last_scanned_at: watchlist.last_scan_at.clone(),
                }),
        );

        let jobs = all_jobs
            .iter()
            .filter(|job| job.project_id == active_project)
            .map(|job| job_to_ui(job, transcript_links.get(&job.id).copied()))
            .collect::<Vec<_>>();
        let queued_sources = all_jobs
            .iter()
            .filter(|job| job.project_id == active_project)
            .filter(|job| job.state == JobState::Queued || job.state.is_active())
            .filter_map(|job| match &job.input {
                JobInput::Url(url) => Some(url.clone()),
                JobInput::LocalFile(_) => None,
            })
            .collect::<HashSet<_>>();
        let ui_research = research_items
            .iter()
            .map(|item| research_to_ui(item, &creator_by_id, &queued_sources))
            .collect::<Vec<_>>();
        let ui_ai_runs = ai_runs.iter().map(ai_run_to_ui).collect::<Vec<_>>();
        let ui_transcripts = transcripts.iter().map(transcript_to_ui).collect::<Vec<_>>();
        let ui_projects = projects
            .iter()
            .map(|project| {
                let project_jobs = all_jobs
                    .iter()
                    .filter(|job| job.project_id == project.id)
                    .collect::<Vec<_>>();
                let updated_at = project_jobs
                    .iter()
                    .map(|job| job.updated_at.as_str())
                    .max()
                    .unwrap_or(&project.created_at)
                    .to_owned();
                UiProject {
                    id: project.id.to_string(),
                    name: project.name.clone(),
                    description: None,
                    updated_at,
                    item_count: project_jobs.len(),
                }
            })
            .collect::<Vec<_>>();
        let mut library = ui_projects
            .iter()
            .map(|project| UiLibraryItem {
                id: format!("project:{}", project.id),
                kind: "Project".to_owned(),
                title: project.name.clone(),
                subtitle: format!("{} jobs", project.item_count),
                project_id: project.id.clone(),
                platform: None,
                metric: None,
                date: project.updated_at.clone(),
            })
            .collect::<Vec<_>>();
        library.extend(ui_transcripts.iter().map(|transcript| UiLibraryItem {
            id: format!("transcript:{}", transcript.id),
            kind: "Transcript".to_owned(),
            title: transcript.title.clone(),
            subtitle: transcript.source.clone(),
            project_id: transcript.project_id.clone(),
            platform: Some(transcript.platform.clone()),
            metric: None,
            date: transcript.created_at.clone(),
        }));
        library.extend(ui_research.iter().map(|item| {
            UiLibraryItem {
                id: format!("research:{}", item.id),
                kind: "Research".to_owned(),
                title: item.title.clone(),
                subtitle: item.creator.clone(),
                project_id: active_project.to_string(),
                platform: Some(item.platform.clone()),
                metric: item.views.map(|views| format!("{views} views")),
                date: item
                    .published_at
                    .clone()
                    .unwrap_or_else(|| "Unknown date".to_owned()),
            }
        }));
        library.extend(ai_runs.iter().map(|run| {
            UiLibraryItem {
                id: format!("ai:{}", run.id),
                kind: "AI run".to_owned(),
                title: run.task.clone(),
                subtitle: run
                    .provider
                    .clone()
                    .map(|provider| match &run.model {
                        Some(model) => format!("{provider} · {model}"),
                        None => provider,
                    })
                    .unwrap_or_else(|| "Copy Prompt".to_owned()),
                project_id: run.project_id.to_string(),
                platform: None,
                metric: None,
                date: run.created_at.clone(),
            }
        }));

        Ok(BootstrapData {
            projects: ui_projects,
            active_project_id: active_project.to_string(),
            creators,
            research: ui_research,
            jobs,
            transcripts: ui_transcripts,
            ai_runs: ui_ai_runs,
            library,
            settings: self.load_settings()?,
        })
    }
}

fn scan_and_persist_research(
    store: &SqliteStore,
    command: &YtDlpCommand,
    project_id: Uuid,
    profile_url: &str,
    limit: u16,
    cookie_browser: Option<&str>,
) -> Result<usize, String> {
    validate_research_limit(limit)?;
    store
        .get_project(project_id)
        .map_err(|error| error.to_string())?;
    let validated = NetworkPolicy
        .validate(profile_url.trim())
        .map_err(|error| error.to_string())?;
    if cookie_browser.is_some() && validated.as_url().scheme() != "https" {
        return Err("authenticated creator scanning requires HTTPS".to_owned());
    }
    let canonical_url = validated.as_url().to_string();
    let creator = store
        .upsert_creator(&Creator {
            id: Uuid::new_v4(),
            project_id,
            platform: source_platform(&canonical_url, SourceType::Url),
            profile_url: canonical_url.clone(),
            display_name: Some(profile_label(validated.as_url())),
            created_at: now_rfc3339(),
        })
        .map_err(|error| error.to_string())?;
    let provider = YtDlpProvider::new(command.clone())
        .with_cookie_browser(cookie_browser)
        .map_err(|error| error.to_string())?;
    let items = ResearchService::new(provider)
        .scan(project_id, Some(creator.id), &canonical_url, limit)
        .map_err(|error| error.to_string())?;
    let persisted = store
        .upsert_research_items(&items)
        .map_err(|error| error.to_string())?;
    let scanned_at = now_rfc3339();
    store
        .mark_watchlist_scanned_by_profile(project_id, &canonical_url, &scanned_at)
        .map_err(|error| error.to_string())?;
    Ok(persisted.len())
}

fn persisted_retry_delay(value: &str, now: DateTime<Utc>) -> Option<Duration> {
    let retry_at = DateTime::parse_from_rfc3339(value)
        .ok()?
        .with_timezone(&Utc);
    let remaining = retry_at.signed_duration_since(now).to_std().ok()?;
    if remaining.is_zero() {
        None
    } else {
        Some(remaining.min(MAX_PERSISTED_RETRY_DELAY))
    }
}

fn safe_watchlist_error(error: &str) -> String {
    let lower = error.to_ascii_lowercase();
    if lower.contains("cookie")
        || lower.contains("login")
        || lower.contains("auth")
        || lower.contains("unauthorized")
        || lower.contains("forbidden")
        || lower.contains("401")
        || lower.contains("403")
    {
        "Creator refresh needs valid browser authentication or provider access.".to_owned()
    } else if lower.contains("timeout")
        || lower.contains("network")
        || lower.contains("connection")
        || lower.contains("dns")
    {
        "Creator refresh could not reach the provider. Scriptotar will retry after the configured backoff."
            .to_owned()
    } else if lower.contains("yt-dlp") || lower.contains("executable") || lower.contains("spawn") {
        "The local research runtime could not start. Repair the packaged runtime before retrying."
            .to_owned()
    } else if lower.contains("unsupported") || lower.contains("invalid") {
        "The creator profile could not be scanned because the provider rejected or does not support it."
            .to_owned()
    } else {
        "Creator refresh failed. Open Research and run a manual scan for more detail.".to_owned()
    }
}

fn spawn_watchlist_refresher(store: SqliteStore, command: YtDlpCommand) -> WatchlistRefresher {
    let (stop, receiver) = mpsc::channel();
    let handle = thread::spawn(move || loop {
        match receiver.recv_timeout(WATCHLIST_TICK) {
            Ok(()) | Err(RecvTimeoutError::Disconnected) => break,
            Err(RecvTimeoutError::Timeout) => {}
        }
        let settings = match store.load_settings() {
            Ok(settings) => settings,
            Err(error) => {
                eprintln!("[scriptotar-watchlist] could not load refresh settings: {error}");
                continue;
            }
        };
        if !settings.auto_watch {
            continue;
        }
        let watchlists = match store.list_watchlists(None) {
            Ok(watchlists) => watchlists,
            Err(error) => {
                eprintln!("[scriptotar-watchlist] could not load watchlists: {error}");
                continue;
            }
        };
        let persisted = match store.list_watchlist_refresh_status(None) {
            Ok(statuses) => statuses
                .into_iter()
                .map(|status| (status.watchlist_id, status))
                .collect::<HashMap<_, _>>(),
            Err(error) => {
                eprintln!("[scriptotar-watchlist] could not load refresh status: {error}");
                continue;
            }
        };
        let now = Utc::now();
        for watchlist in watchlists {
            if persisted.get(&watchlist.id).is_some_and(|status| {
                status.state == WatchlistRefreshState::RetryScheduled
                    && status
                        .next_retry_at
                        .as_deref()
                        .and_then(|retry_at| persisted_retry_delay(retry_at, now))
                        .is_some()
            }) {
                continue;
            }
            if !watchlist_is_due(&watchlist, settings.watch_interval_minutes) {
                continue;
            }

            let attempted_at = now_rfc3339();
            match store.try_begin_watchlist_refresh(watchlist.id, &attempted_at) {
                Ok(true) => {}
                Ok(false) => continue,
                Err(error) => {
                    eprintln!(
                        "[scriptotar-watchlist] could not claim refresh for {}: {error}",
                        watchlist.id
                    );
                    continue;
                }
            }

            match scan_and_persist_research(
                &store,
                &command,
                watchlist.project_id,
                &watchlist.profile_url,
                watchlist.limit_count.min(200) as u16,
                settings.cookie_browser.as_deref(),
            ) {
                Ok(_) => {
                    if let Err(error) =
                        store.record_watchlist_refresh_success(watchlist.id, &now_rfc3339())
                    {
                        eprintln!(
                            "[scriptotar-watchlist] could not persist refresh success for {}: {error}",
                            watchlist.id
                        );
                    }
                }
                Err(error) => {
                    let retry = watchlist_failure_retry(settings.watch_interval_minutes);
                    let retry_at = Utc::now()
                        + chrono::Duration::from_std(retry)
                            .unwrap_or_else(|_| chrono::Duration::hours(6));
                    let retry_at = retry_at.to_rfc3339();
                    let safe_error = safe_watchlist_error(&error);
                    if let Err(status_error) = store.record_watchlist_refresh_failure(
                        watchlist.id,
                        &attempted_at,
                        &safe_error,
                        Some(&retry_at),
                    ) {
                        eprintln!(
                            "[scriptotar-watchlist] could not persist refresh failure for {}: {status_error}",
                            watchlist.id
                        );
                    }
                    eprintln!(
                        "[scriptotar-watchlist] refresh failed for {}: {safe_error}; retry scheduled for {retry_at}",
                        watchlist.id
                    );
                }
            }
        }
    });
    WatchlistRefresher {
        _inner: Arc::new(WatchlistRefresherInner {
            stop,
            handle: Mutex::new(Some(handle)),
        }),
    }
}

fn watchlist_failure_retry(interval_minutes: u32) -> Duration {
    Duration::from_secs(u64::from(interval_minutes.max(5)) * 60)
}

fn watchlist_is_due(watchlist: &Watchlist, interval_minutes: u32) -> bool {
    let Some(last_scan_at) = watchlist.last_scan_at.as_deref() else {
        return true;
    };
    let Ok(last_scan_at) = DateTime::parse_from_rfc3339(last_scan_at) else {
        return true;
    };
    Utc::now().signed_duration_since(last_scan_at.with_timezone(&Utc))
        >= chrono::Duration::minutes(i64::from(interval_minutes.max(1)))
}

fn validate_research_limit(limit: u16) -> Result<(), String> {
    if !(1..=200).contains(&limit) {
        Err("research limit must be between 1 and 200".to_owned())
    } else {
        Ok(())
    }
}

fn validate_ai_input(input: &AiPromptInput) -> Result<(), String> {
    if input.task.trim().is_empty() {
        return Err("AI task cannot be empty".to_owned());
    }
    if input.task.chars().count() > 256 {
        return Err("AI task is too long".to_owned());
    }
    if input.source_text.chars().count() > MAX_AI_SOURCE_CHARS {
        return Err("AI source context is too large".to_owned());
    }
    for (field, value) in [
        ("topic", &input.topic),
        ("audience", &input.audience),
        ("duration", &input.duration),
        ("CTA", &input.cta),
        ("voice", &input.voice),
    ] {
        if value.chars().count() > MAX_AI_CONTEXT_CHARS {
            return Err(format!("AI {field} field is too large"));
        }
    }
    Ok(())
}

fn task_instructions(task: &str) -> &'static str {
    match task.trim() {
        "Viral breakdown" => {
            "Analyze why the source may retain attention. Return: Hook, Structure beats, Retention devices, Payoff, and Transferable lessons. Explain techniques without reproducing the source script."
        }
        "Hook ideas" => {
            "Generate 10 distinct hooks grounded in the supplied facts. Vary the angle and mechanism; keep each hook to one or two lines and avoid copying distinctive source phrasing."
        }
        "New short-form script" => {
            "Write an original short-form script for the requested goal and duration. Use a clear hook, concise body beats, payoff, and CTA. Preserve source facts but do not imitate or reproduce distinctive wording."
        }
        "Structure remix" => {
            "First identify the source's high-level structural beats, then propose a materially new structure using the same supported facts. Do not perform sentence-level imitation."
        }
        "Content ideas" => {
            "Generate 10 original content ideas. For each, provide a working title, core angle, audience value, and one concrete execution note grounded in the source context."
        }
        "Caption + CTA" => {
            "Return three concise caption options followed by three CTA options. Keep claims supported by the source and make the variants meaningfully different."
        }
        "Voice profile" => {
            "Describe observable communication traits such as pacing, sentence shape, vocabulary, rhetorical devices, energy, and recurring content patterns. Describe traits for adaptation, not identity impersonation or voice cloning."
        }
        "B-roll shot list" => {
            "Create a practical ordered B-roll shot list. For each shot include the visual, its narrative purpose, and any useful on-screen text or graphic cue. Keep visuals consistent with supported source facts."
        }
        _ => {
            "Complete the requested creator task with a clear, useful result grounded in the supplied source and brief."
        }
    }
}

fn build_ai_prompt_text(input: &AiPromptInput) -> Result<String, String> {
    validate_ai_input(input)?;
    let mut sections = vec![format!(
        "Task: {}\n\nTask instructions:\n{}",
        input.task.trim(),
        task_instructions(&input.task)
    )];
    push_prompt_field(
        &mut sections,
        "Source transcript / context",
        &input.source_text,
    );
    push_prompt_field(&mut sections, "Topic / goal", &input.topic);
    push_prompt_field(&mut sections, "Audience", &input.audience);
    push_prompt_field(&mut sections, "Target duration", &input.duration);
    push_prompt_field(&mut sections, "CTA", &input.cta);
    push_prompt_field(&mut sections, "Voice / style instructions", &input.voice);
    sections.push(
        "Grounding and transformation rules:\n- Preserve factual meaning and uncertainty; do not invent source details, quotes, metrics, or events.\n- Transform, summarize, analyze, or create anew rather than reproducing lengthy or distinctive copyrighted wording from the source.\n- If the source does not support a requested claim, say what is missing.\n- Treat voice/style notes as creative constraints, not permission to impersonate a real person."
            .to_owned(),
    );
    sections.push(
        "Output format:\nReturn clean Markdown suitable for Scriptotar's result panel. Use short headings, bullets, or numbered sections where useful. Do not include process commentary or a generic preamble."
            .to_owned(),
    );
    Ok(sections.join("\n\n"))
}

fn profile_label(url: &url::Url) -> String {
    url.path_segments()
        .and_then(|mut segments| segments.rfind(|segment| !segment.is_empty()))
        .filter(|segment| !segment.is_empty())
        .or_else(|| url.host_str())
        .unwrap_or("Creator")
        .trim_start_matches('@')
        .chars()
        .take(256)
        .collect()
}

fn profile_label_from_raw(raw: &str) -> String {
    NetworkPolicy
        .validate(raw)
        .map(|validated| profile_label(validated.as_url()))
        .unwrap_or_else(|_| "Creator".to_owned())
}

fn research_to_ui(
    item: &ResearchItem,
    creators: &HashMap<Uuid, &Creator>,
    queued_sources: &HashSet<String>,
) -> UiResearchItem {
    let creator = item.creator_id.and_then(|id| creators.get(&id).copied());
    let creator_name = creator
        .and_then(|creator| creator.display_name.clone())
        .or_else(|| creator.map(|creator| profile_label_from_raw(&creator.profile_url)))
        .unwrap_or_else(|| "Creator".to_owned());
    let thumbnail = item
        .raw_json
        .as_deref()
        .and_then(|raw| serde_json::from_str::<Value>(raw).ok())
        .and_then(|value| {
            value
                .get("thumbnail")
                .and_then(Value::as_str)
                .map(str::to_owned)
        });
    UiResearchItem {
        id: item.id.to_string(),
        creator_id: item.creator_id.map(|id| id.to_string()).unwrap_or_default(),
        creator: creator_name,
        title: item
            .title
            .clone()
            .unwrap_or_else(|| "Untitled research item".to_owned()),
        source_url: item.source_url.clone(),
        platform: item.platform.clone(),
        views: item.view_count,
        likes: item.like_count,
        comments: item.comment_count,
        published_at: item.published_at.clone(),
        duration_seconds: item.duration_seconds,
        thumbnail,
        queued: Some(queued_sources.contains(&item.source_url)),
    }
}

fn ai_run_to_ui(run: &AiRun) -> UiAiRun {
    let title = run
        .result
        .as_deref()
        .and_then(|result| result.lines().find(|line| !line.trim().is_empty()))
        .map(|line| line.trim().chars().take(120).collect())
        .unwrap_or_else(|| run.task.clone());
    UiAiRun {
        id: run.id.to_string(),
        task: run.task.clone(),
        mode: match run.mode {
            AiRunMode::CopyPrompt => "copy",
            AiRunMode::Byok => "byok",
        }
        .to_owned(),
        provider: run.provider.clone(),
        model: run.model.clone(),
        title,
        created_at: run.created_at.clone(),
        status: "completed".to_owned(),
    }
}

fn runtime_config(fallback_output_root: PathBuf) -> RuntimeConfig {
    let python = env::var_os("SCRIPTOTAR_SIDECAR_PYTHON")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(if cfg!(windows) { "python" } else { "python3" }));
    let sidecar_script = env::var_os("SCRIPTOTAR_SIDECAR_SCRIPT")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../sidecars/transcription/sidecar.py")
        });
    RuntimeConfig::new(python, sidecar_script, fallback_output_root)
}

fn job_to_ui(job: &Job, completed_transcript_id: Option<Uuid>) -> UiJob {
    let (_, source) = job.input.parts();
    let title = Path::new(source)
        .file_name()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .unwrap_or(source)
        .to_owned();
    UiJob {
        id: job.id.to_string(),
        title,
        source: source.to_owned(),
        state: job.state.as_str().to_owned(),
        stage_label: stage_label(job.state),
        progress: job
            .progress
            .map(|value| (value.clamp(0.0, 1.0) * 100.0).round() as u8),
        updated_at: job.updated_at.clone(),
        detail: job.last_error.clone(),
        completed_transcript_id: completed_transcript_id.map(|id| id.to_string()),
    }
}

fn stage_label(state: JobState) -> String {
    match state {
        JobState::Queued => "Queued",
        JobState::Preparing => "Preparing",
        JobState::Downloading => "Downloading",
        JobState::Transcribing => "Transcribing",
        JobState::Processing => "Processing",
        JobState::Completed => "Completed",
        JobState::Failed => "Failed",
        JobState::Cancelled => "Cancelled",
        JobState::Interrupted => "Interrupted",
    }
    .to_owned()
}

fn transcript_to_ui(bundle: &TranscriptBundle) -> UiTranscript {
    let language = bundle
        .transcript
        .language
        .clone()
        .unwrap_or_else(|| "unknown".to_owned());
    let direction = if language.to_ascii_lowercase().starts_with("ar") {
        "rtl"
    } else {
        "ltr"
    };
    let title = bundle
        .source
        .title
        .clone()
        .or_else(|| {
            Path::new(&bundle.source.locator)
                .file_name()
                .and_then(|value| value.to_str())
                .map(str::to_owned)
        })
        .unwrap_or_else(|| "Transcript".to_owned());
    UiTranscript {
        id: bundle.transcript.id.to_string(),
        project_id: bundle.project_id.to_string(),
        title,
        language,
        direction: direction.to_owned(),
        source: bundle.source.locator.clone(),
        platform: source_platform(&bundle.source.locator, bundle.source.source_type),
        duration_seconds: bundle.media.duration_seconds.unwrap_or_default(),
        created_at: bundle.transcript.created_at.clone(),
        text: bundle.transcript.text.clone(),
        segments: parse_segments(bundle.transcript.segments_json.as_deref()),
    }
}

fn parse_segments(raw: Option<&str>) -> Vec<UiTranscriptSegment> {
    let Some(raw) = raw else {
        return Vec::new();
    };
    let Ok(Value::Array(values)) = serde_json::from_str::<Value>(raw) else {
        return Vec::new();
    };
    values
        .into_iter()
        .enumerate()
        .filter_map(|(index, value)| {
            let object = value.as_object()?;
            let text = object.get("text")?.as_str()?.to_owned();
            Some(UiTranscriptSegment {
                id: object
                    .get("index")
                    .and_then(Value::as_u64)
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| index.to_string()),
                start_seconds: object
                    .get("start")
                    .and_then(Value::as_f64)
                    .unwrap_or_default(),
                end_seconds: object
                    .get("end")
                    .and_then(Value::as_f64)
                    .unwrap_or_default(),
                text,
            })
        })
        .collect()
}

fn source_platform(locator: &str, source_type: SourceType) -> String {
    if source_type == SourceType::LocalFile {
        return "Local".to_owned();
    }
    let lower = locator.to_ascii_lowercase();
    if lower.contains("youtube.com") || lower.contains("youtu.be") {
        "YouTube"
    } else if lower.contains("instagram.com") {
        "Instagram"
    } else if lower.contains("tiktok.com") {
        "TikTok"
    } else {
        "Web"
    }
    .to_owned()
}

fn settings_to_ui(settings: &ApplicationSettings) -> UiSettings {
    UiSettings {
        output_directory: settings.output_directory.clone(),
        whisper_model: settings.transcription_model.clone(),
        device: settings.transcription_device.clone(),
        language: match settings.language.as_str() {
            "ar" | "Arabic" => "Arabic",
            "en" | "English" => "English",
            _ => "auto",
        }
        .to_owned(),
        quality: match settings.download_quality.as_str() {
            "best" | "Best" => "Best",
            "audio-only" | "Audio only" => "Audio only",
            value => value,
        }
        .to_owned(),
        cookies: settings
            .cookie_browser
            .clone()
            .unwrap_or_else(|| "none".to_owned()),
        max_duration: match settings.max_duration_seconds {
            0 => "Unlimited",
            1800 => "30 min",
            7200 => "2 hours",
            _ => "60 min",
        }
        .to_owned(),
        copy_local_source: settings.copy_source,
        translate: settings.translate,
        batched: settings.batched,
        keep_failed: settings.keep_failed_artifacts,
        auto_watch: settings.auto_watch,
        watch_interval: match settings.watch_interval_minutes {
            30 => "30 min",
            120 => "2 hours",
            360 => "6 hours",
            _ => "60 min",
        }
        .to_owned(),
        appearance: settings.appearance.clone(),
    }
}

fn settings_from_ui(
    ui: UiSettings,
    mut settings: ApplicationSettings,
) -> RepositoryResult<ApplicationSettings> {
    settings.output_directory = validate_output_directory(ui.output_directory)?;
    if !["small", "medium", "turbo", "large-v3"].contains(&ui.whisper_model.as_str()) {
        return Err(RepositoryError::Validation(
            "unsupported transcription model".to_owned(),
        ));
    }
    if !["auto", "cpu", "cuda"].contains(&ui.device.as_str()) {
        return Err(RepositoryError::Validation(
            "unsupported transcription device".to_owned(),
        ));
    }
    settings.transcription_model = ui.whisper_model;
    settings.transcription_device = ui.device;
    settings.language = match ui.language.as_str() {
        "auto" => "auto",
        "Arabic" => "ar",
        "English" => "en",
        _ => {
            return Err(RepositoryError::Validation(
                "unsupported transcription language".to_owned(),
            ))
        }
    }
    .to_owned();
    settings.download_quality = match ui.quality.as_str() {
        "720p" => "720p",
        "1080p" => "1080p",
        "Best" => "best",
        "Audio only" => "audio-only",
        _ => {
            return Err(RepositoryError::Validation(
                "unsupported download quality".to_owned(),
            ))
        }
    }
    .to_owned();
    settings.cookie_browser = match ui.cookies.as_str() {
        "none" => None,
        "firefox" | "chrome" | "chromium" | "brave" => Some(ui.cookies),
        _ => {
            return Err(RepositoryError::Validation(
                "unsupported cookie browser".to_owned(),
            ))
        }
    };
    settings.max_duration_seconds = match ui.max_duration.as_str() {
        "30 min" => 1800,
        "60 min" => 3600,
        "2 hours" => 7200,
        "Unlimited" => 0,
        _ => {
            return Err(RepositoryError::Validation(
                "unsupported max duration".to_owned(),
            ))
        }
    };
    settings.copy_source = ui.copy_local_source;
    settings.translate = ui.translate;
    settings.batched = ui.batched;
    settings.keep_failed_artifacts = ui.keep_failed;
    settings.auto_watch = ui.auto_watch;
    settings.watch_interval_minutes = match ui.watch_interval.as_str() {
        "30 min" => 30,
        "60 min" => 60,
        "2 hours" => 120,
        "6 hours" => 360,
        _ => {
            return Err(RepositoryError::Validation(
                "unsupported watch interval".to_owned(),
            ))
        }
    };
    if !["dark", "system"].contains(&ui.appearance.as_str()) {
        return Err(RepositoryError::Validation(
            "unsupported appearance setting".to_owned(),
        ));
    }
    settings.appearance = ui.appearance;
    Ok(settings)
}

fn validate_output_directory(value: Option<String>) -> RepositoryResult<Option<String>> {
    let Some(raw) = value else {
        return Ok(None);
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    let path = Path::new(trimmed);
    if !path.is_dir() {
        return Err(RepositoryError::Validation(format!(
            "output directory does not exist or is not a directory: {}",
            path.display()
        )));
    }
    let canonical = std::fs::canonicalize(path).map_err(|error| {
        RepositoryError::Validation(format!("cannot access output directory: {error}"))
    })?;
    let probe = canonical.join(format!(".scriptotar-write-test-{}", Uuid::new_v4()));
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&probe)
        .map_err(|error| {
            RepositoryError::Validation(format!("output directory is not writable: {error}"))
        })?;
    drop(file);
    std::fs::remove_file(&probe).map_err(|error| {
        RepositoryError::Storage(format!(
            "failed to clean output-directory write test: {error}"
        ))
    })?;
    Ok(Some(canonical.to_string_lossy().into_owned()))
}

fn provider_kind(provider: &str) -> Result<ProviderKind, String> {
    match provider {
        "OpenAI" => Ok(ProviderKind::OpenAi),
        "Anthropic" => Ok(ProviderKind::Anthropic),
        "Gemini" => Ok(ProviderKind::Gemini),
        "OpenAI-compatible" => Ok(ProviderKind::OpenAiCompatible),
        "Local (coming later)" => Ok(ProviderKind::Local),
        _ => Err("unsupported AI provider".to_owned()),
    }
}

fn push_prompt_field(sections: &mut Vec<String>, label: &str, value: &str) {
    let value = value.trim();
    if !value.is_empty() {
        sections.push(format!("{label}:\n{value}"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::{params, Connection};
    use tempfile::TempDir;

    fn prompt_input(task: &str) -> AiPromptInput {
        AiPromptInput {
            mode: "byok".to_owned(),
            provider: "OpenAI".to_owned(),
            model: "model".to_owned(),
            task: task.to_owned(),
            source_text: "A source fact with uncertainty.".to_owned(),
            topic: "Topic".to_owned(),
            audience: "Audience".to_owned(),
            duration: "45 seconds".to_owned(),
            cta: "Follow for more".to_owned(),
            voice: "Direct and concise".to_owned(),
            base_url: None,
            api_key: Some("super-secret".to_owned()),
        }
    }

    fn bootstrap_active_id(bootstrap: &BootstrapData) -> Uuid {
        Uuid::parse_str(&bootstrap.active_project_id).unwrap()
    }

    fn database_path(temp: &TempDir) -> PathBuf {
        temp.path().join("scriptotar.sqlite3")
    }

    #[test]
    fn ai_prompt_never_contains_session_key_and_has_grounding_rules() {
        let prompt = build_ai_prompt_text(&prompt_input("Hook ideas")).unwrap();
        assert!(!prompt.contains("super-secret"));
        assert!(prompt.contains("10 distinct hooks"));
        assert!(prompt.contains("Preserve factual meaning"));
        assert!(prompt.contains("copyrighted wording"));
        assert!(prompt.contains("clean Markdown"));
    }

    #[test]
    fn supported_ai_actions_have_task_specific_instructions() {
        for task in [
            "Viral breakdown",
            "Hook ideas",
            "New short-form script",
            "Structure remix",
            "Content ideas",
            "Caption + CTA",
            "Voice profile",
            "B-roll shot list",
        ] {
            let prompt = build_ai_prompt_text(&prompt_input(task)).unwrap();
            assert!(prompt.contains(&format!("Task: {task}")));
            assert!(!prompt.contains("Task instructions:\nComplete the requested creator task"));
        }
    }

    #[test]
    fn insecure_custom_ai_endpoint_is_rejected_before_execution() {
        let config = ProviderConfig {
            provider: ProviderKind::OpenAiCompatible,
            model: "model".to_owned(),
            base_url: Some("http://example.com/v1".to_owned()),
        };
        assert!(EndpointPolicy.endpoint_for(&config).is_err());
    }

    #[test]
    fn watchlist_interval_and_failed_refresh_backoff_are_bounded() {
        let watchlist = Watchlist {
            id: Uuid::new_v4(),
            project_id: Uuid::new_v4(),
            label: "Creator".to_owned(),
            profile_url: "https://www.youtube.com/@creator".to_owned(),
            limit_count: 25,
            last_scan_at: Some("not-a-time".to_owned()),
        };
        assert!(watchlist_is_due(&watchlist, 60));
        assert_eq!(watchlist_failure_retry(30), Duration::from_secs(30 * 60));
        assert_eq!(watchlist_failure_retry(0), Duration::from_secs(5 * 60));
    }

    #[test]
    fn persisted_retry_timestamps_handle_clock_changes_safely() {
        let now = DateTime::parse_from_rfc3339("2026-08-10T10:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        assert!(persisted_retry_delay("2026-08-10T09:59:59Z", now).is_none());
        assert!(persisted_retry_delay("not-a-time", now).is_none());
        assert_eq!(
            persisted_retry_delay("2036-08-10T10:00:00Z", now),
            Some(MAX_PERSISTED_RETRY_DELAY)
        );
        assert_eq!(
            persisted_retry_delay("2026-08-10T10:05:00Z", now),
            Some(Duration::from_secs(5 * 60))
        );
    }

    #[test]
    fn watchlist_provider_errors_are_safe_for_persistent_and_displayable_status() {
        let raw_errors = [
            "ERROR 403 cookie=/home/user/.mozilla/profile token=super-secret",
            "Authorization: Bearer super-secret https://provider.invalid/?token=query-secret",
            "<html><body>500 provider exploded</body></html> /home/user/private/db.sqlite3",
            "Traceback (most recent call last): /home/user/app.py api_key=super-secret",
        ];
        for raw in raw_errors {
            let safe = safe_watchlist_error(raw);
            assert!(safe.len() < 180);
            assert!(!safe.contains("super-secret"));
            assert!(!safe.contains("query-secret"));
            assert!(!safe.contains("/home/user"));
            assert!(!safe.contains("<html>"));
            assert!(!safe.contains("Traceback"));
            assert!(!safe.contains("Bearer"));
        }
    }

    #[test]
    fn secret_like_provider_error_is_not_persisted_or_displayable() {
        let temp = tempfile::tempdir().unwrap();
        let store = SqliteStore::open(temp.path().join("scriptotar.sqlite3")).unwrap();
        store.run_integration_migrations().unwrap();
        let project = Project::new("Inbox");
        store.create_project(&project).unwrap();
        let watchlist = store
            .upsert_watchlist(
                project.id,
                "Creator",
                "https://www.youtube.com/@creator",
                25,
            )
            .unwrap();
        let raw = "Authorization: Bearer super-secret cookie=/home/user/profile?token=query-secret <html>Traceback</html>";
        let safe = safe_watchlist_error(raw);
        store
            .record_watchlist_refresh_failure(watchlist.id, "2026-08-10T10:00:00Z", &safe, None)
            .unwrap();
        let displayable = store
            .watchlist_refresh_status(watchlist.id)
            .unwrap()
            .unwrap()
            .last_error
            .unwrap();
        assert_eq!(displayable, safe);
        for forbidden in [
            "super-secret",
            "query-secret",
            "/home/user",
            "<html>",
            "Traceback",
            "Bearer",
        ] {
            assert!(!displayable.contains(forbidden));
        }
    }

    #[test]
    fn output_directory_validation_accepts_writable_directory_and_normalizes_empty() {
        let temp =
            std::env::temp_dir().join(format!("scriptotar-output-validation-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&temp).unwrap();
        let expected = std::fs::canonicalize(&temp).unwrap();
        let value = validate_output_directory(Some(temp.to_string_lossy().into_owned())).unwrap();
        assert_eq!(value.as_deref(), Some(expected.to_string_lossy().as_ref()));
        assert_eq!(
            validate_output_directory(Some("   ".to_owned())).unwrap(),
            None
        );
        std::fs::remove_dir_all(&temp).unwrap();
    }

    #[test]
    fn first_start_defaults_to_inbox_and_persists_selection() {
        let temp = TempDir::new().unwrap();
        let services = AppServices::new(temp.path()).unwrap();
        let bootstrap = services.bootstrap().unwrap();
        let active_project = bootstrap_active_id(&bootstrap);
        let inbox = bootstrap
            .projects
            .iter()
            .find(|project| project.name.eq_ignore_ascii_case("Inbox"))
            .unwrap();

        assert_eq!(inbox.id, active_project.to_string());
        assert_eq!(
            services.store.load_settings().unwrap().active_project_id,
            Some(active_project)
        );
    }

    #[test]
    fn selected_project_survives_service_restart() {
        let temp = TempDir::new().unwrap();
        let selected_project = {
            let services = AppServices::new(temp.path()).unwrap();
            let bootstrap = services.create_project("Project X".to_owned()).unwrap();
            let selected_project = bootstrap_active_id(&bootstrap);
            assert_eq!(
                services.store.load_settings().unwrap().active_project_id,
                Some(selected_project)
            );
            selected_project
        };

        let restarted = AppServices::new(temp.path()).unwrap();
        assert_eq!(
            bootstrap_active_id(&restarted.bootstrap().unwrap()),
            selected_project
        );
    }

    #[test]
    fn missing_selected_project_falls_back_to_inbox_and_repairs_settings() {
        let temp = TempDir::new().unwrap();
        let (inbox_id, selected_project) = {
            let services = AppServices::new(temp.path()).unwrap();
            let initial = services.bootstrap().unwrap();
            let inbox_id = bootstrap_active_id(&initial);
            let selected = services.create_project("Temporary".to_owned()).unwrap();
            (inbox_id, bootstrap_active_id(&selected))
        };

        let connection = Connection::open(database_path(&temp)).unwrap();
        connection
            .execute(
                "DELETE FROM projects WHERE id = ?1",
                params![selected_project.to_string()],
            )
            .unwrap();
        drop(connection);

        let restarted = AppServices::new(temp.path()).unwrap();
        assert_eq!(
            bootstrap_active_id(&restarted.bootstrap().unwrap()),
            inbox_id
        );
        assert_eq!(
            restarted.store.load_settings().unwrap().active_project_id,
            Some(inbox_id)
        );
    }

    #[test]
    fn legacy_settings_without_active_project_fall_back_safely() {
        let temp = TempDir::new().unwrap();
        let inbox_id = {
            let services = AppServices::new(temp.path()).unwrap();
            bootstrap_active_id(&services.bootstrap().unwrap())
        };

        let connection = Connection::open(database_path(&temp)).unwrap();
        let raw: String = connection
            .query_row(
                "SELECT settings_json FROM application_settings WHERE singleton = 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let mut value: Value = serde_json::from_str(&raw).unwrap();
        value.as_object_mut().unwrap().remove("active_project_id");
        connection
            .execute(
                "UPDATE application_settings SET settings_json = ?1 WHERE singleton = 1",
                params![serde_json::to_string(&value).unwrap()],
            )
            .unwrap();
        drop(connection);

        let restarted = AppServices::new(temp.path()).unwrap();
        assert_eq!(
            bootstrap_active_id(&restarted.bootstrap().unwrap()),
            inbox_id
        );
        assert_eq!(
            restarted.store.load_settings().unwrap().active_project_id,
            Some(inbox_id)
        );
    }

    #[test]
    fn malformed_active_project_id_does_not_break_startup() {
        let temp = TempDir::new().unwrap();
        let inbox_id = {
            let services = AppServices::new(temp.path()).unwrap();
            bootstrap_active_id(&services.bootstrap().unwrap())
        };

        let connection = Connection::open(database_path(&temp)).unwrap();
        let raw: String = connection
            .query_row(
                "SELECT settings_json FROM application_settings WHERE singleton = 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let mut value: Value = serde_json::from_str(&raw).unwrap();
        value.as_object_mut().unwrap().insert(
            "active_project_id".to_owned(),
            Value::String("broken-id".to_owned()),
        );
        connection
            .execute(
                "UPDATE application_settings SET settings_json = ?1 WHERE singleton = 1",
                params![serde_json::to_string(&value).unwrap()],
            )
            .unwrap();
        drop(connection);

        let restarted = AppServices::new(temp.path()).unwrap();
        assert_eq!(
            bootstrap_active_id(&restarted.bootstrap().unwrap()),
            inbox_id
        );
        assert_eq!(
            restarted.store.load_settings().unwrap().active_project_id,
            Some(inbox_id)
        );
    }
}
