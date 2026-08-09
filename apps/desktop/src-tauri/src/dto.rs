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
