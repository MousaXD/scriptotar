use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{Job, Media, RepositoryResult, Source, Transcript};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Watchlist {
    pub id: Uuid,
    pub project_id: Uuid,
    pub label: String,
    pub profile_url: String,
    pub limit_count: u32,
    pub last_scan_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TranscriptBundle {
    pub project_id: Uuid,
    pub source: Source,
    pub media: Media,
    pub transcript: Transcript,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct LegacyImportReport {
    pub skipped: bool,
    pub backup_path: Option<String>,
    pub projects: usize,
    pub jobs: usize,
    pub transcripts: usize,
    pub research_items: usize,
    pub watchlists: usize,
    pub ai_runs: usize,
}

pub trait JobRuntimeRepository: Send + Sync {
    fn update_job_progress(&self, id: Uuid, progress: Option<f32>) -> RepositoryResult<Job>;
    fn fail_job(&self, id: Uuid, error: &str) -> RepositoryResult<Job>;
}

pub trait ContentRepository: Send + Sync {
    fn persist_transcription(
        &self,
        job_id: Uuid,
        source: &Source,
        media: &Media,
        transcript: &Transcript,
    ) -> RepositoryResult<Job>;

    fn list_transcripts(&self, project_id: Option<Uuid>) -> RepositoryResult<Vec<TranscriptBundle>>;
}

pub trait WatchlistRepository: Send + Sync {
    fn list_watchlists(&self, project_id: Option<Uuid>) -> RepositoryResult<Vec<Watchlist>>;
}
