use std::{fmt, str::FromStr};

use chrono::Utc;
use serde::{Deserialize, Deserializer, Serialize};
use thiserror::Error;
use uuid::Uuid;

pub fn now_rfc3339() -> String {
    Utc::now().to_rfc3339()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Project {
    pub id: Uuid,
    pub name: String,
    pub created_at: String,
}

impl Project {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            created_at: now_rfc3339(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Creator {
    pub id: Uuid,
    pub project_id: Uuid,
    pub platform: String,
    pub profile_url: String,
    pub display_name: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Source {
    pub id: Uuid,
    pub project_id: Uuid,
    pub creator_id: Option<Uuid>,
    pub source_type: SourceType,
    pub locator: String,
    pub title: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SourceType {
    Url,
    LocalFile,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Media {
    pub id: Uuid,
    pub source_id: Uuid,
    pub local_path: String,
    pub duration_seconds: Option<f64>,
    pub mime_type: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Transcript {
    pub id: Uuid,
    pub media_id: Uuid,
    pub language: Option<String>,
    pub text: String,
    pub segments_json: Option<String>,
    pub words_json: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResearchItem {
    pub id: Uuid,
    pub project_id: Uuid,
    pub creator_id: Option<Uuid>,
    pub source_url: String,
    pub platform: String,
    pub title: Option<String>,
    pub view_count: Option<i64>,
    pub like_count: Option<i64>,
    pub comment_count: Option<i64>,
    pub published_at: Option<String>,
    pub duration_seconds: Option<f64>,
    pub raw_json: Option<String>,
    pub scanned_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Analysis {
    pub id: Uuid,
    pub project_id: Uuid,
    pub transcript_id: Option<Uuid>,
    pub kind: String,
    pub content: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AiRun {
    pub id: Uuid,
    pub project_id: Uuid,
    pub task: String,
    pub mode: AiRunMode,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub prompt: String,
    pub result: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AiRunMode {
    CopyPrompt,
    Byok,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum JobInput {
    Url(String),
    LocalFile(String),
}

impl JobInput {
    pub fn parts(&self) -> (&'static str, &str) {
        match self {
            Self::Url(value) => ("url", value),
            Self::LocalFile(value) => ("local_file", value),
        }
    }

    pub fn from_parts(kind: &str, value: String) -> Result<Self, RepositoryError> {
        match kind {
            "url" => Ok(Self::Url(value)),
            "local_file" => Ok(Self::LocalFile(value)),
            other => Err(RepositoryError::Storage(format!(
                "unknown persisted job input kind: {other}"
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum JobState {
    Queued,
    Preparing,
    Downloading,
    Transcribing,
    Processing,
    Completed,
    Failed,
    Cancelled,
    Interrupted,
}

impl JobState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Preparing => "preparing",
            Self::Downloading => "downloading",
            Self::Transcribing => "transcribing",
            Self::Processing => "processing",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
            Self::Interrupted => "interrupted",
        }
    }

    pub const fn is_active(self) -> bool {
        matches!(
            self,
            Self::Preparing | Self::Downloading | Self::Transcribing | Self::Processing
        )
    }

    pub const fn can_transition_to(self, next: Self) -> bool {
        match self {
            Self::Queued => matches!(next, Self::Preparing | Self::Cancelled),
            Self::Preparing => matches!(
                next,
                Self::Downloading
                    | Self::Transcribing
                    | Self::Failed
                    | Self::Cancelled
                    | Self::Interrupted
            ),
            Self::Downloading => matches!(
                next,
                Self::Transcribing | Self::Failed | Self::Cancelled | Self::Interrupted
            ),
            Self::Transcribing => matches!(
                next,
                Self::Processing | Self::Failed | Self::Cancelled | Self::Interrupted
            ),
            Self::Processing => matches!(
                next,
                Self::Completed | Self::Failed | Self::Cancelled | Self::Interrupted
            ),
            Self::Failed | Self::Cancelled | Self::Interrupted => matches!(next, Self::Queued),
            Self::Completed => false,
        }
    }

    pub fn validate_transition(self, next: Self) -> Result<(), RepositoryError> {
        if self.can_transition_to(next) {
            Ok(())
        } else {
            Err(RepositoryError::Conflict(format!(
                "invalid job transition: {self} -> {next}"
            )))
        }
    }
}

impl fmt::Display for JobState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for JobState {
    type Err = RepositoryError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "queued" => Ok(Self::Queued),
            "preparing" => Ok(Self::Preparing),
            "downloading" => Ok(Self::Downloading),
            "transcribing" => Ok(Self::Transcribing),
            "processing" => Ok(Self::Processing),
            "completed" => Ok(Self::Completed),
            "failed" => Ok(Self::Failed),
            "cancelled" => Ok(Self::Cancelled),
            "interrupted" => Ok(Self::Interrupted),
            other => Err(RepositoryError::Storage(format!(
                "unknown persisted job state: {other}"
            ))),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Job {
    pub id: Uuid,
    pub project_id: Uuid,
    pub input: JobInput,
    pub state: JobState,
    pub progress: Option<f32>,
    pub last_error: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl Job {
    pub fn new(project_id: Uuid, input: JobInput) -> Self {
        let now = now_rfc3339();
        Self {
            id: Uuid::new_v4(),
            project_id,
            input,
            state: JobState::Queued,
            progress: None,
            last_error: None,
            created_at: now.clone(),
            updated_at: now,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ApplicationSettings {
    pub output_directory: Option<String>,
    pub transcription_model: String,
    pub transcription_device: String,
    pub language: String,
    pub download_quality: String,
    pub cookie_browser: Option<String>,
    pub max_duration_seconds: u32,
    pub copy_source: bool,
    pub translate: bool,
    pub batched: bool,
    pub keep_failed_artifacts: bool,
    pub ai_mode: AiMode,
    pub ai_provider: String,
    pub ai_model: String,
    pub ai_base_url: Option<String>,
    pub auto_watch: bool,
    pub watch_interval_minutes: u32,
    #[serde(default = "default_appearance")]
    pub appearance: String,
    #[serde(default, deserialize_with = "deserialize_optional_uuid_lenient")]
    pub active_project_id: Option<Uuid>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AiMode {
    CopyPrompt,
    Byok,
}

fn default_appearance() -> String {
    "dark".to_owned()
}

fn deserialize_optional_uuid_lenient<'de, D>(deserializer: D) -> Result<Option<Uuid>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    Ok(match value {
        serde_json::Value::String(raw) => Uuid::parse_str(raw.trim()).ok(),
        _ => None,
    })
}

impl Default for ApplicationSettings {
    fn default() -> Self {
        Self {
            output_directory: None,
            transcription_model: "medium".to_owned(),
            transcription_device: "auto".to_owned(),
            language: "auto".to_owned(),
            download_quality: "720p".to_owned(),
            cookie_browser: None,
            max_duration_seconds: 3600,
            copy_source: true,
            translate: false,
            batched: false,
            keep_failed_artifacts: false,
            ai_mode: AiMode::CopyPrompt,
            ai_provider: "OpenAI".to_owned(),
            ai_model: "gpt-5.2".to_owned(),
            ai_base_url: None,
            auto_watch: false,
            watch_interval_minutes: 60,
            appearance: default_appearance(),
            active_project_id: None,
        }
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum RepositoryError {
    #[error("not found: {0}")]
    NotFound(String),
    #[error("conflict: {0}")]
    Conflict(String),
    #[error("validation failed: {0}")]
    Validation(String),
    #[error("storage error: {0}")]
    Storage(String),
}

pub type RepositoryResult<T> = Result<T, RepositoryError>;

pub trait ProjectRepository: Send + Sync {
    fn create_project(&self, project: &Project) -> RepositoryResult<()>;
    fn get_project(&self, id: Uuid) -> RepositoryResult<Project>;
    fn list_projects(&self) -> RepositoryResult<Vec<Project>>;
}

pub trait JobRepository: Send + Sync {
    fn insert_job(&self, job: &Job) -> RepositoryResult<()>;
    fn get_job(&self, id: Uuid) -> RepositoryResult<Job>;
    fn list_jobs(&self, project_id: Option<Uuid>) -> RepositoryResult<Vec<Job>>;
    fn transition_job(&self, id: Uuid, next: JobState) -> RepositoryResult<Job>;
    fn mark_active_jobs_interrupted(&self) -> RepositoryResult<usize>;
}

pub trait SettingsRepository: Send + Sync {
    fn load_settings(&self) -> RepositoryResult<ApplicationSettings>;
    fn save_settings(&self, settings: &ApplicationSettings) -> RepositoryResult<()>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_job_path_reaches_completion() {
        let states = [
            JobState::Queued,
            JobState::Preparing,
            JobState::Downloading,
            JobState::Transcribing,
            JobState::Processing,
            JobState::Completed,
        ];
        for pair in states.windows(2) {
            assert!(pair[0].validate_transition(pair[1]).is_ok());
        }
    }

    #[test]
    fn local_media_can_skip_downloading() {
        assert!(JobState::Preparing
            .validate_transition(JobState::Transcribing)
            .is_ok());
    }

    #[test]
    fn invalid_transitions_are_rejected() {
        assert!(JobState::Queued
            .validate_transition(JobState::Completed)
            .is_err());
        assert!(JobState::Completed
            .validate_transition(JobState::Queued)
            .is_err());
        assert!(JobState::Downloading
            .validate_transition(JobState::Preparing)
            .is_err());
    }

    #[test]
    fn recoverable_terminal_states_can_be_requeued() {
        for state in [JobState::Failed, JobState::Cancelled, JobState::Interrupted] {
            assert!(state.validate_transition(JobState::Queued).is_ok());
        }
    }

    #[test]
    fn application_settings_never_contain_api_keys() {
        let json = serde_json::to_string(&ApplicationSettings::default()).unwrap();
        assert!(!json.to_ascii_lowercase().contains("api_key"));
        assert!(!json.to_ascii_lowercase().contains("apikey"));
    }

    #[test]
    fn legacy_settings_json_defaults_appearance() {
        let mut value = serde_json::to_value(ApplicationSettings::default()).unwrap();
        value.as_object_mut().unwrap().remove("appearance");
        let settings: ApplicationSettings = serde_json::from_value(value).unwrap();
        assert_eq!(settings.appearance, "dark");
    }

    #[test]
    fn legacy_settings_json_defaults_active_project() {
        let mut value = serde_json::to_value(ApplicationSettings::default()).unwrap();
        value.as_object_mut().unwrap().remove("active_project_id");
        let settings: ApplicationSettings = serde_json::from_value(value).unwrap();
        assert_eq!(settings.active_project_id, None);
    }

    #[test]
    fn malformed_active_project_id_is_treated_as_unset() {
        for malformed in [
            serde_json::Value::String("not-a-uuid".to_owned()),
            serde_json::json!({"unexpected": "shape"}),
            serde_json::json!(42),
        ] {
            let mut value = serde_json::to_value(ApplicationSettings::default()).unwrap();
            value
                .as_object_mut()
                .unwrap()
                .insert("active_project_id".to_owned(), malformed);
            let settings: ApplicationSettings = serde_json::from_value(value).unwrap();
            assert_eq!(settings.active_project_id, None);
        }
    }

    #[test]
    fn active_project_id_round_trips_when_valid() {
        let project_id = Uuid::new_v4();
        let mut settings = ApplicationSettings::default();
        settings.active_project_id = Some(project_id);
        let json = serde_json::to_string(&settings).unwrap();
        let restored: ApplicationSettings = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.active_project_id, Some(project_id));
    }
}
