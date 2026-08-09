use std::{
    collections::{HashMap, HashSet},
    env,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    thread,
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
use scriptotar_db::SqliteStore;
use scriptotar_jobs::JobService;
use scriptotar_media::MediaPolicy;
use scriptotar_orchestrator::{JobOrchestrator, RuntimeConfig};
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

#[derive(Debug, Clone)]
pub struct AppServices {
    store: SqliteStore,
    orchestrator: JobOrchestrator<SqliteStore>,
    active_project: Arc<Mutex<Uuid>>,
    legacy_path: PathBuf,
    research_command: YtDlpCommand,
}

impl AppServices {
    pub fn new(data_dir: impl AsRef<Path>) -> RepositoryResult<Self> {
        let data_dir = data_dir.as_ref();
        std::fs::create_dir_all(data_dir)
            .map_err(|error| RepositoryError::Storage(error.to_string()))?;
        let store = SqliteStore::open(data_dir.join("scriptotar.sqlite3"))?;
        store.run_integration_migrations()?;
        JobService::new(store.clone()).recover_after_unclean_shutdown()?;

        let legacy_path = data_dir.join("history.sqlite3");
        if legacy_path.is_file() {
            store.import_legacy_database(&legacy_path)?;
        }

        let mut projects = store.list_projects()?;
        if projects.is_empty() {
            let inbox = Project::new("Inbox");
            store.create_project(&inbox)?;
            projects.push(inbox);
        }
        let active_project = projects
            .iter()
            .find(|project| project.name.eq_ignore_ascii_case("Inbox"))
            .unwrap_or(&projects[0])
            .id;
        let orchestrator = JobOrchestrator::start(
            store.clone(),
            runtime_config(data_dir.join("transcription-output")),
        );
        let research_command = YtDlpCommand::from_environment();
        spawn_watchlist_refresher(store.clone(), research_command.clone());

        Ok(Self {
            store,
            orchestrator,
            active_project: Arc::new(Mutex::new(active_project)),
            legacy_path,
            research_command,
        })
    }

    pub fn schema_version(&self) -> RepositoryResult<u32> {
        self.store.schema_version()
    }

    pub fn bootstrap(&self) -> RepositoryResult<BootstrapData> {
        let active_project = self.active_project_id()?;
        self.bootstrap_for(active_project)
    }

    pub fn select_project(&self, project_id: Uuid) -> RepositoryResult<BootstrapData> {
        self.store.get_project(project_id)?;
        *self
            .active_project
            .lock()
            .map_err(|_| RepositoryError::Storage("active project lock poisoned".to_owned()))? =
            project_id;
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
        let current = self.store.load_settings()?;
        let settings = settings_from_ui(settings, current)?;
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
        let active_project = self.active_project_id().map_err(|error| error.to_string())?;
        self.store
            .upsert_watchlist(active_project, &label, &profile_url, u32::from(query.limit))
            .map_err(|error| error.to_string())?;
        self.bootstrap_for(active_project)
            .map_err(|error| error.to_string())
    }

    pub fn scan_creator(&self, query: ResearchQuery) -> Result<(), String> {
        validate_research_limit(query.limit)?;
        let active_project = self.active_project_id().map_err(|error| error.to_string())?;
        let settings = self.store.load_settings().map_err(|error| error.to_string())?;
        scan_and_persist_research(
            &self.store,
            &self.research_command,
            active_project,
            &query.profile_url,
            query.limit,
            settings.cookie_browser.as_deref(),
        )?;
        Ok(())
    }

    pub fn refresh_watchlists(&self) -> Result<usize, String> {
        let active_project = self.active_project_id().map_err(|error| error.to_string())?;
        let watchlists = self
            .store
            .list_watchlists(Some(active_project))
            .map_err(|error| error.to_string())?;
        let settings = self.store.load_settings().map_err(|error| error.to_string())?;
        let mut saved_items = 0_usize;
        let mut errors = Vec::new();
        for watchlist in watchlists {
            match scan_and_persist_research(
                &self.store,
                &self.research_command,
                watchlist.project_id,
                &watchlist.profile_url,
                watchlist.limit_count.min(200) as u16,
                settings.cookie_browser.as_deref(),
            ) {
                Ok(count) => saved_items += count,
                Err(error) => errors.push(format!("{}: {error}", watchlist.label)),
            }
        }
        if errors.is_empty() {
            Ok(saved_items)
        } else {
            Err(format!(
                "some watchlists could not be refreshed: {}",
                errors.join("; ")
            ))
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
        let active_project = self.active_project_id().map_err(|error| error.to_string())?;
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
        validate_ai_input(input)?;
        let mut sections = vec![format!("Task: {}", input.task.trim())];
        push_prompt_field(&mut sections, "Source context", &input.source_text);
        push_prompt_field(&mut sections, "Topic / goal", &input.topic);
        push_prompt_field(&mut sections, "Audience", &input.audience);
        push_prompt_field(&mut sections, "Target duration", &input.duration);
        push_prompt_field(&mut sections, "CTA", &input.cta);
        push_prompt_field(&mut sections, "Voice / style", &input.voice);
        sections.push(
            "Return a useful creator-workstation result. Preserve factual uncertainty and do not invent source details."
                .to_owned(),
        );
        Ok(sections.join("\n\n"))
    }

    pub fn run_ai(&self, input: &AiPromptInput) -> Result<String, String> {
        let prompt = self.build_ai_prompt(input)?;
        let active_project = self.active_project_id().map_err(|error| error.to_string())?;
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
            .map(job_to_ui)
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
        library.extend(ui_research.iter().map(|item| UiLibraryItem {
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
        }));
        library.extend(ai_runs.iter().map(|run| UiLibraryItem {
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
    store.get_project(project_id).map_err(|error| error.to_string())?;
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

fn spawn_watchlist_refresher(store: SqliteStore, command: YtDlpCommand) {
    thread::spawn(move || loop {
        thread::sleep(Duration::from_secs(60));
        let Ok(settings) = store.load_settings() else {
            continue;
        };
        if !settings.auto_watch {
            continue;
        }
        let Ok(watchlists) = store.list_watchlists(None) else {
            continue;
        };
        for watchlist in watchlists {
            if !watchlist_is_due(&watchlist, settings.watch_interval_minutes) {
                continue;
            }
            let _ = scan_and_persist_research(
                &store,
                &command,
                watchlist.project_id,
                &watchlist.profile_url,
                watchlist.limit_count.min(200) as u16,
                settings.cookie_browser.as_deref(),
            );
        }
    });
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
        .and_then(|value| value.get("thumbnail").and_then(Value::as_str).map(str::to_owned));
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

fn job_to_ui(job: &Job) -> UiJob {
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
        appearance: "dark".to_owned(),
    }
}

fn settings_from_ui(
    ui: UiSettings,
    mut settings: ApplicationSettings,
) -> RepositoryResult<ApplicationSettings> {
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
    Ok(settings)
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

    #[test]
    fn ai_prompt_never_contains_session_key() {
        let input = AiPromptInput {
            mode: "byok".to_owned(),
            provider: "OpenAI".to_owned(),
            model: "model".to_owned(),
            task: "Summarize".to_owned(),
            source_text: "source".to_owned(),
            topic: String::new(),
            audience: String::new(),
            duration: String::new(),
            cta: String::new(),
            voice: String::new(),
            base_url: None,
            api_key: Some("super-secret".to_owned()),
        };
        let mut sections = vec![format!("Task: {}", input.task.trim())];
        push_prompt_field(&mut sections, "Source context", &input.source_text);
        let prompt = sections.join("\n\n");
        assert!(!prompt.contains("super-secret"));
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
    fn malformed_watchlist_timestamp_is_due_for_recovery() {
        let watchlist = Watchlist {
            id: Uuid::new_v4(),
            project_id: Uuid::new_v4(),
            label: "Creator".to_owned(),
            profile_url: "https://www.youtube.com/@creator".to_owned(),
            limit_count: 25,
            last_scan_at: Some("not-a-time".to_owned()),
        };
        assert!(watchlist_is_due(&watchlist, 60));
    }
}
