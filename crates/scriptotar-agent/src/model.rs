//! Stable, serializable workflow contract. IDs are opaque until P0 freezes Study APIs.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolKind {
    DiscoverResearch,
    SelectResearch,
    QueueTranscript,
    ReadTranscript,
    ReadStudyTranscript,
    AnalyzeEvidence,
    DraftScript,
    ReviseScript,
    SaveArtifact,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "tool", rename_all = "snake_case")]
pub enum StepInput {
    DiscoverResearch {
        query: String,
    },
    SelectResearch {
        source_ids: Vec<String>,
    },
    QueueTranscript {
        source_id: String,
    },
    ReadTranscript {
        job_id: String,
    },
    /// A specific already-attached transcript, never an arbitrary job ID.
    ReadStudyTranscript {
        transcript_id: String,
    },
    AnalyzeEvidence {
        source_ids: Vec<String>,
    },
    DraftScript {
        brief_id: String,
    },
    ReviseScript {
        draft_id: String,
    },
    SaveArtifact {
        draft_id: String,
    },
}

impl StepInput {
    pub fn kind(&self) -> ToolKind {
        match self {
            Self::DiscoverResearch { .. } => ToolKind::DiscoverResearch,
            Self::SelectResearch { .. } => ToolKind::SelectResearch,
            Self::QueueTranscript { .. } => ToolKind::QueueTranscript,
            Self::ReadTranscript { .. } => ToolKind::ReadTranscript,
            Self::ReadStudyTranscript { .. } => ToolKind::ReadStudyTranscript,
            Self::AnalyzeEvidence { .. } => ToolKind::AnalyzeEvidence,
            Self::DraftScript { .. } => ToolKind::DraftScript,
            Self::ReviseScript { .. } => ToolKind::ReviseScript,
            Self::SaveArtifact { .. } => ToolKind::SaveArtifact,
        }
    }

    pub fn valid(&self) -> bool {
        match self {
            Self::DiscoverResearch { query } => !query.trim().is_empty() && query.len() <= 1024,
            Self::SelectResearch { source_ids } | Self::AnalyzeEvidence { source_ids } => {
                !source_ids.is_empty()
                    && source_ids.len() <= 64
                    && source_ids.iter().all(|id| valid_id(id))
            }
            Self::QueueTranscript { source_id } => valid_id(source_id),
            Self::ReadTranscript { job_id } => valid_id(job_id),
            Self::ReadStudyTranscript { transcript_id } => valid_id(transcript_id),
            Self::DraftScript { brief_id } => valid_id(brief_id),
            Self::ReviseScript { draft_id } | Self::SaveArtifact { draft_id } => valid_id(draft_id),
        }
    }
}

pub fn valid_id(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 128
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Estimate {
    /// Upper-bound reservation enforced by the application service, not a price quote.
    pub tokens: u64,
    pub cost_microusd: u64,
    pub max_output_bytes: u64,
    pub cpu_ms: u64,
    pub peak_memory_mib: u32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Usage {
    pub tool_calls: u32,
    pub reserved_tokens: u64,
    pub reserved_cost_microusd: u64,
    pub reserved_cpu_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Limits {
    pub max_tool_calls: u32,
    pub max_tokens: u64,
    pub max_cost_microusd: u64,
    pub max_elapsed_ms: u64,
    pub max_output_bytes: u64,
    pub max_cpu_ms: u64,
    pub max_peak_memory_mib: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StepPlan {
    pub id: String,
    pub input: StepInput,
    pub estimate: Estimate,
    pub max_attempts: u8,
    /// Extra gate for locally read-only tools; mutating/network tools always require approval.
    pub requires_approval: bool,
}

impl StepPlan {
    pub fn approval_required(&self) -> bool {
        self.requires_approval
            || self.estimate.cost_microusd > 0
            || !matches!(
                self.input.kind(),
                ToolKind::SelectResearch | ToolKind::ReadTranscript
            )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Plan {
    pub id: String,
    pub project_id: String,
    pub study_id: String,
    pub steps: Vec<StepPlan>,
    pub limits: Limits,
}

impl Plan {
    pub fn validate(&self) -> bool {
        valid_id(&self.id)
            && valid_id(&self.project_id)
            && valid_id(&self.study_id)
            && !self.steps.is_empty()
            && self.steps.len() <= 64
            && self.limits.max_tool_calls > 0
            && self.limits.max_elapsed_ms > 0
            && self.limits.max_output_bytes > 0
            && self.limits.max_cpu_ms > 0
            && self.limits.max_peak_memory_mib > 0
            && self.steps.iter().all(|s| {
                valid_id(&s.id)
                    && s.input.valid()
                    && s.max_attempts > 0
                    && s.max_attempts <= 5
                    && s.estimate.max_output_bytes > 0
                    && s.estimate.max_output_bytes <= self.limits.max_output_bytes
                    && s.estimate.peak_memory_mib <= self.limits.max_peak_memory_mib
            })
            && self
                .steps
                .iter()
                .enumerate()
                .all(|(i, s)| !self.steps[..i].iter().any(|prior| prior.id == s.id))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunState {
    Planned,
    AwaitingApproval,
    Running,
    Cancelling,
    Paused,
    NeedsInput,
    Blocked,
    Completed,
    Failed,
    Cancelled,
}

impl RunState {
    pub fn terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepState {
    Pending,
    Running,
    AwaitingResult,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StepOutput {
    /// An opaque source/job/draft/artifact ID, never provider response text or secrets.
    pub artifact_id: String,
    pub tokens: u64,
    pub cost_microusd: u64,
    pub bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StepRecord {
    pub id: String,
    pub state: StepState,
    pub attempts: u8,
    pub approved: bool,
    pub idempotency_key: String,
    pub pending_effect_id: Option<String>,
    pub output: Option<StepOutput>,
    pub failure_code: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditKind {
    Created,
    AwaitingApproval,
    Approved,
    Denied,
    Dispatched,
    Accepted,
    Reconciled,
    Completed,
    Interrupted,
    RetryReady,
    Paused,
    Resumed,
    CancelRequested,
    Cancelled,
    Blocked,
    Failed,
    LimitReached,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditEvent {
    pub sequence: u64,
    pub at_ms: u64,
    pub kind: AuditKind,
    pub step_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunSnapshot {
    pub schema_version: u32,
    pub plan: Plan,
    pub revision: u64,
    pub state: RunState,
    pub current_step: usize,
    pub steps: Vec<StepRecord>,
    pub usage: Usage,
    pub started_ms: u64,
    pub cancel_requested: bool,
    pub pause_requested: bool,
    pub events: Vec<AuditEvent>,
}

impl RunSnapshot {
    pub fn event(&mut self, now: u64, kind: AuditKind) {
        let step_id = self.steps.get(self.current_step).map(|s| s.id.clone());
        self.events.push(AuditEvent {
            sequence: self.events.len() as u64 + 1,
            at_ms: now,
            kind,
            step_id,
        });
    }
}
