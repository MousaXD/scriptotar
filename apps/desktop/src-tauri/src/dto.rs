use scriptotar_core::LegacyImportReport;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UiProject {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub updated_at: String,
    pub item_count: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UiCreator {
    pub id: String,
    pub name: String,
    pub handle: String,
    pub platform: String,
    pub avatar: Option<String>,
    pub watchlisted: bool,
    pub last_scanned_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UiWatchlistStatus {
    pub watchlist_id: String,
    pub project_id: String,
    pub label: String,
    pub state: String,
    pub last_attempt_at: Option<String>,
    pub last_successful_scan_at: Option<String>,
    pub last_error: Option<String>,
    pub next_retry_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UiMigrationCandidate {
    pub id: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UiMigrationStatus {
    pub state: String,
    pub message: String,
    pub candidates: Vec<UiMigrationCandidate>,
    pub report: Option<LegacyImportReport>,
}

impl UiMigrationStatus {
    pub fn no_legacy_database() -> Self {
        Self {
            state: "no_legacy_db".to_owned(),
            message: "No Scriptotar Classic database was found in the standard legacy locations."
                .to_owned(),
            candidates: Vec::new(),
            report: None,
        }
    }

    pub fn ready() -> Self {
        Self {
            state: "ready".to_owned(),
            message: "A safe legacy database snapshot is prepared for import.".to_owned(),
            candidates: Vec::new(),
            report: None,
        }
    }

    pub fn in_progress() -> Self {
        Self {
            state: "in_progress".to_owned(),
            message: "Scriptotar is importing the prepared legacy snapshot. The source database remains untouched."
                .to_owned(),
            candidates: Vec::new(),
            report: None,
        }
    }

    pub fn requires_choice(candidates: Vec<UiMigrationCandidate>) -> Self {
        Self {
            state: "requires_choice".to_owned(),
            message: "Multiple legacy databases were found. Choose which snapshot Scriptotar should import; no source database will be overwritten."
                .to_owned(),
            candidates,
            report: None,
        }
    }

    pub fn invalid_database(message: impl Into<String>) -> Self {
        Self {
            state: "invalid_db".to_owned(),
            message: message.into(),
            candidates: Vec::new(),
            report: None,
        }
    }

    pub fn failed(message: impl Into<String>) -> Self {
        Self {
            state: "failed".to_owned(),
            message: message.into(),
            candidates: Vec::new(),
            report: None,
        }
    }

    pub fn previously_completed() -> Self {
        Self {
            state: "completed".to_owned(),
            message: "Legacy migration was already completed on this installation. Scriptotar will not automatically import the source database again."
                .to_owned(),
            candidates: Vec::new(),
            report: None,
        }
    }

    pub fn completed(report: LegacyImportReport) -> Self {
        let message = if report.skipped {
            "Legacy migration was already completed for this snapshot; no duplicate rows were created."
                .to_owned()
        } else {
            format!(
                "Legacy migration completed: {} projects, {} jobs, {} transcripts, {} research items, {} watchlists, {} AI runs.",
                report.projects,
                report.jobs,
                report.transcripts,
                report.research_items,
                report.watchlists,
                report.ai_runs,
            )
        };
        Self {
            state: "completed".to_owned(),
            message,
            candidates: Vec::new(),
            report: Some(report),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UiResearchItem {
    pub id: String,
    pub creator_id: String,
    pub creator: String,
    pub title: String,
    pub source_url: String,
    pub platform: String,
    pub views: Option<i64>,
    pub likes: Option<i64>,
    pub comments: Option<i64>,
    pub published_at: Option<String>,
    pub duration_seconds: Option<f64>,
    pub thumbnail: Option<String>,
    pub queued: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UiJob {
    pub id: String,
    pub title: String,
    pub source: String,
    pub state: String,
    pub stage_label: String,
    pub progress: Option<u8>,
    pub updated_at: String,
    pub detail: Option<String>,
    pub completed_transcript_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UiTranscriptSegment {
    pub id: String,
    pub start_seconds: f64,
    pub end_seconds: f64,
    pub text: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UiTranscript {
    pub id: String,
    pub project_id: String,
    pub title: String,
    pub language: String,
    pub direction: String,
    pub source: String,
    pub platform: String,
    pub duration_seconds: f64,
    pub created_at: String,
    pub text: String,
    pub segments: Vec<UiTranscriptSegment>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UiAiRun {
    pub id: String,
    pub task: String,
    pub mode: String,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub title: String,
    pub created_at: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UiLibraryItem {
    pub id: String,
    pub kind: String,
    pub title: String,
    pub subtitle: String,
    pub project_id: String,
    pub platform: Option<String>,
    pub metric: Option<String>,
    pub date: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiSettings {
    pub output_directory: Option<String>,
    pub whisper_model: String,
    pub device: String,
    pub language: String,
    pub quality: String,
    pub cookies: String,
    pub max_duration: String,
    pub copy_local_source: bool,
    pub translate: bool,
    pub batched: bool,
    pub keep_failed: bool,
    pub auto_watch: bool,
    pub watch_interval: String,
    pub appearance: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BootstrapData {
    pub projects: Vec<UiProject>,
    pub active_project_id: String,
    pub creators: Vec<UiCreator>,
    pub research: Vec<UiResearchItem>,
    pub jobs: Vec<UiJob>,
    pub transcripts: Vec<UiTranscript>,
    pub ai_runs: Vec<UiAiRun>,
    pub library: Vec<UiLibraryItem>,
    pub settings: UiSettings,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResearchQuery {
    pub profile_url: String,
    pub limit: u16,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiPromptInput {
    pub mode: String,
    pub provider: String,
    pub model: String,
    pub task: String,
    pub source_text: String,
    pub topic: String,
    pub audience: String,
    pub duration: String,
    pub cta: String,
    pub voice: String,
    pub base_url: Option<String>,
    pub api_key: Option<String>,
}
