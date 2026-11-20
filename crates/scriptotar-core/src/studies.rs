//! Persistent, project-scoped content study domain objects.
//! Never put provider credentials or untrusted source instructions in these records.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StudyStatus {
    Active,
    Paused,
    Completed,
}

impl StudyStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Paused => "paused",
            Self::Completed => "completed",
        }
    }
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "active" => Some(Self::Active),
            "paused" => Some(Self::Paused),
            "completed" => Some(Self::Completed),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContentStudy {
    pub id: Uuid,
    pub project_id: Uuid,
    pub title: String,
    pub goal: String,
    pub status: StudyStatus,
    pub revision: i64,
    pub created_at: String,
    pub updated_at: String,
    pub archived_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StudyEdit {
    pub title: String,
    pub goal: String,
    pub status: StudyStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StudySourceKind {
    ResearchItem,
    Source,
    Media,
    Transcript,
    Job,
    AiRun,
    Manual,
}

impl StudySourceKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ResearchItem => "research_item",
            Self::Source => "source",
            Self::Media => "media",
            Self::Transcript => "transcript",
            Self::Job => "job",
            Self::AiRun => "ai_run",
            Self::Manual => "manual",
        }
    }
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "research_item" => Some(Self::ResearchItem),
            "source" => Some(Self::Source),
            "media" => Some(Self::Media),
            "transcript" => Some(Self::Transcript),
            "job" => Some(Self::Job),
            "ai_run" => Some(Self::AiRun),
            "manual" => Some(Self::Manual),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StudySourceInput {
    pub kind: StudySourceKind,
    /// Required for existing entities; forbidden for pasted/manual notes.
    pub reference_id: Option<Uuid>,
    /// Optional label or locator for manual notes. Other locators come from SQLite.
    pub locator: Option<String>,
    pub manual_text: Option<String>,
    /// Caller-known revision/etag, not a claim of source verification.
    pub source_revision: Option<String>,
    pub uncertainty: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StudySource {
    pub id: Uuid,
    pub project_id: Uuid,
    pub study_id: Uuid,
    pub kind: StudySourceKind,
    /// Original reference ID retained even if the source was later deleted.
    pub reference_id: Option<Uuid>,
    pub locator: String,
    pub manual_text: Option<String>,
    pub fingerprint: String,
    pub source_revision: Option<String>,
    pub collected_at: String,
    pub uncertainty: Option<String>,
    /// False means the original item was deleted/detached; snapshot is not live evidence.
    pub available: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceCertainty {
    Unverified,
    Estimated,
    Corroborated,
    Disputed,
}

impl EvidenceCertainty {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Unverified => "unverified",
            Self::Estimated => "estimated",
            Self::Corroborated => "corroborated",
            Self::Disputed => "disputed",
        }
    }
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "unverified" => Some(Self::Unverified),
            "estimated" => Some(Self::Estimated),
            "corroborated" => Some(Self::Corroborated),
            "disputed" => Some(Self::Disputed),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvidenceInput {
    pub study_source_id: Uuid,
    pub claim: String,
    /// Exact source segment, timestamp range, or document page/excerpt locator.
    pub locator: String,
    pub timestamp_seconds: Option<f64>,
    pub certainty: EvidenceCertainty,
    pub uncertainty_note: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvidenceReference {
    pub id: Uuid,
    pub project_id: Uuid,
    pub study_id: Uuid,
    pub input: EvidenceInput,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreativeBriefInput {
    pub audience: String,
    pub goal: String,
    pub language: String,
    pub region: Option<String>,
    pub platform: String,
    pub tone: String,
    pub duration_seconds: Option<u32>,
    pub objective: String,
    pub cta: Option<String>,
    pub constraints: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreativeBrief {
    pub study_id: Uuid,
    pub project_id: Uuid,
    pub input: CreativeBriefInput,
    pub revision: i64,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreativeDraft {
    pub id: Uuid,
    pub project_id: Uuid,
    pub study_id: Uuid,
    pub title: String,
    pub body: String,
    pub revision: i64,
    pub created_at: String,
    pub updated_at: String,
    pub archived_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DraftVersion {
    pub id: Uuid,
    pub draft_id: Uuid,
    pub version: i64,
    pub title: String,
    pub body: String,
    pub saved_at: String,
}
