use std::{
    env,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use scriptotar_ai::{EndpointPolicy, ProviderConfig, ProviderKind};
use scriptotar_core::{
    ApplicationSettings, ContentRepository, Job, JobInput, JobRepository, Project,
    ProjectRepository, RepositoryError, RepositoryResult, SettingsRepository, SourceType,
    TranscriptBundle,
};
use scriptotar_db::SqliteStore;
use scriptotar_jobs::JobService;
use scriptotar_media::MediaPolicy;
use scriptotar_orchestrator::{JobOrchestrator, RuntimeConfig};
use scriptotar_research::NetworkPolicy;
use serde_json::Value;
use uuid::Uuid;

use crate::dto::{
    AiPromptInput, BootstrapData, ResearchQuery, UiJob, UiLibraryItem, UiProject, UiSettings,
    UiTranscript, UiTranscriptSegment,
};

#[derive(Debug, Clone)]
pub struct AppServices {
    store: SqliteStore,
    orchestrator: JobOrchestrator<SqliteStore>,
    active_project: Arc<Mutex<Uuid>>,
    legacy_path: PathBuf,
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

        Ok(Self {
            store,
            orchestrator,
            active_project: Arc::new(Mutex::new(active_project)),
            legacy_path,
        })
    }

    pub fn schema_version(&self) -> RepositoryResult<u32> {
        self.store.schema_version()
    }

    pub fn bootstrap(&self) -> RepositoryResult<BootstrapData> {
        let active_project = *self
            .active_project
            .lock()
            .map_err(|_| RepositoryError::Storage("active project lock poisoned".to_owned()))?;
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

    pub fn scan_creator(&self, query: ResearchQuery) -> Result<(), String> {
        if !(1..=200).contains(&query.limit) {
            return Err("research limit must be between 1 and 200".to_owned());
        }
        NetworkPolicy
            .validate(&query.profile_url)
            .map_err(|error| error.to_string())?;
        Err("research provider execution is not integrated yet; URL policy passed".to_owned())
    }

    pub fn queue_research(&self, ids: Vec<String>) -> Result<(), String> {
        if ids.is_empty() {
            return Err("select at least one research item".to_owned());
        }
        Err("research queue persistence is not integrated yet".to_owned())
    }

    pub fn build_ai_prompt(&self, input: &AiPromptInput) -> Result<String, String> {
        if input.task.trim().is_empty() {
            return Err("AI task cannot be empty".to_owned());
        }
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
        if !input.mode.eq_ignore_ascii_case("byok") {
            return self.build_ai_prompt(input);
        }
        if input
            .api_key
            .as_deref()
            .is_none_or(|key| key.trim().is_empty())
        {
            return Err("an API key is required for BYOK mode".to_owned());
        }
        let provider = provider_kind(&input.provider)?;
        let config = ProviderConfig {
            provider,
            model: input.model.clone(),
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
        Err("AI provider execution is not integrated yet; endpoint policy passed and no key was persisted".to_owned())
    }

    fn bootstrap_for(&self, active_project: Uuid) -> RepositoryResult<BootstrapData> {
        let projects = self.store.list_projects()?;
        let all_jobs = self.store.list_jobs(None)?;
        let transcripts = self.store.list_transcripts(Some(active_project))?;
        let jobs = all_jobs
            .iter()
            .filter(|job| job.project_id == active_project)
            .map(job_to_ui)
            .collect::<Vec<_>>();
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

        Ok(BootstrapData {
            projects: ui_projects,
            active_project_id: active_project.to_string(),
            creators: Vec::new(),
            research: Vec::new(),
            jobs,
            transcripts: ui_transcripts,
            ai_runs: Vec::new(),
            library,
            settings: self.load_settings()?,
        })
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

fn stage_label(state: scriptotar_core::JobState) -> String {
    match state {
        scriptotar_core::JobState::Queued => "Queued",
        scriptotar_core::JobState::Preparing => "Preparing",
        scriptotar_core::JobState::Downloading => "Downloading",
        scriptotar_core::JobState::Transcribing => "Transcribing",
        scriptotar_core::JobState::Processing => "Processing",
        scriptotar_core::JobState::Completed => "Completed",
        scriptotar_core::JobState::Failed => "Failed",
        scriptotar_core::JobState::Cancelled => "Cancelled",
        scriptotar_core::JobState::Interrupted => "Interrupted",
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
}
