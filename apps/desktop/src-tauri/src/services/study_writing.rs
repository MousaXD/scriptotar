//! P1 original-writing orchestration. No Tauri command or paid provider is
//! registered here. Agent 00 supplies a project-scoped context loader and an
//! atomic, idempotent canonical draft sink; Agents 01/02 supply the permission
//! and durable effect boundary. These are intentionally non-optional traits.
//!
//! A result returned by a model is NEVER permission to dispatch another tool,
//! reassign a Study, claim factual verification, or replace an editor's text.
use std::collections::{HashMap, HashSet};

use scriptotar_ai::{
    writing::{
        self, AngleOption, BeatRevision, ClaimStatus, CreativeBrief, DraftAudit, EvidenceKind,
        EvidenceRef, EvidenceStatus, ResearchFinding, ReviewFinding, ScriptDraft, WritingInput,
        WritingTask,
    },
    AiProvider, AiRequest, AiResponse, AiService, ProviderConfig,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub type WriteResult<T> = Result<T, WritingWorkflowError>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WritingWorkflowError {
    InvalidContext(&'static str),
    StaleContext,
    InvalidResponse(String),
    DuplicateOrSimilar,
    Denied(String),
    Cancelled,
    ProviderFailed,
    UncertainEffect,
    PersistFailed(String),
}

impl std::fmt::Display for WritingWorkflowError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for WritingWorkflowError {}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SelectedWritingSource {
    /// UUID of the actual attached study_sources row.
    pub study_source_id: Uuid,
    /// UUID of the project-owned original transcript/research item.
    pub origin_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StudyWritingContext {
    pub project_id: Uuid,
    pub study_id: Uuid,
    pub study_revision: i64,
    pub sources: Vec<SelectedWritingSource>,
    pub brief: CreativeBrief,
    pub evidence: Vec<EvidenceRef>,
    pub findings: Vec<ResearchFinding>,
}

impl StudyWritingContext {
    /// Translate the existing scoped Study gateway's already validated writing
    /// input and canonical source rows. This does not load arbitrary IDs, repeat
    /// transcript parsing or call a provider. Agent 00 invokes it while holding
    /// the active-project lock, never from an untrusted Tauri DTO.
    pub fn from_scoped_study(
        project_id: Uuid,
        study_id: Uuid,
        study_revision: i64,
        canonical_sources: &[scriptotar_core::StudySource],
        input: WritingInput,
    ) -> WriteResult<Self> {
        if !matches!(&input.task, WritingTask::ProposeAngles { count: 3 }) {
            return Err(WritingWorkflowError::InvalidContext(
                "expected three-angle input",
            ));
        }
        if canonical_sources
            .iter()
            .any(|source| source.project_id != project_id || source.study_id != study_id)
        {
            return Err(WritingWorkflowError::InvalidContext("foreign source row"));
        }
        let sources =
            canonical_sources
                .iter()
                .filter(|source| source.available)
                .take(8)
                .map(|source| {
                    Ok(SelectedWritingSource {
                        study_source_id: source.id,
                        origin_id: source.reference_id.ok_or(
                            WritingWorkflowError::InvalidContext("unresolved source origin"),
                        )?,
                    })
                })
                .collect::<WriteResult<Vec<_>>>()?;
        let context = Self {
            project_id,
            study_id,
            study_revision,
            sources,
            brief: input.brief,
            evidence: input.evidence,
            findings: input.findings,
        };
        context.validate()?;
        Ok(context)
    }

    /// Requires three distinct, available *actual* sources with attributable
    /// evidence. The host must retrieve/validate these IDs in the canonical
    /// database; user-supplied IDs must never be used to construct this value.
    pub fn validate(&self) -> WriteResult<()> {
        if self.project_id.is_nil() || self.study_id.is_nil() || self.study_revision < 1 {
            return Err(WritingWorkflowError::InvalidContext("scope or revision"));
        }
        if !(3..=8).contains(&self.sources.len()) {
            return Err(WritingWorkflowError::InvalidContext(
                "three to eight selected sources required",
            ));
        }
        let mut attached = HashSet::new();
        let mut origins = HashSet::new();
        for source in &self.sources {
            if source.study_source_id.is_nil()
                || source.origin_id.is_nil()
                || !attached.insert(source.study_source_id)
                || !origins.insert(source.origin_id)
            {
                return Err(WritingWorkflowError::InvalidContext(
                    "duplicate or missing source",
                ));
            }
        }
        let mut covered = HashSet::new();
        for item in &self.evidence {
            let origin = Uuid::parse_str(&item.locator.source_id).map_err(|_| {
                WritingWorkflowError::InvalidContext("noncanonical evidence locator")
            })?;
            let binding = self
                .sources
                .iter()
                .find(|source| source.origin_id == origin || source.study_source_id == origin)
                .ok_or(WritingWorkflowError::InvalidContext(
                    "foreign evidence source",
                ))?;
            covered.insert(binding.study_source_id);
            // The existing Study bridge emits only attributed transcripts and
            // unverified human observations. A creator observation cannot become
            // independent corroboration by supplying a different enum variant.
            if matches!(
                item.kind,
                EvidenceKind::CreativeObservation | EvidenceKind::SocialTranscript
            ) && item.status == EvidenceStatus::Corroborated
            {
                return Err(WritingWorkflowError::InvalidContext(
                    "untrusted corroboration",
                ));
            }
        }
        if covered.len() < 3 {
            return Err(WritingWorkflowError::InvalidContext(
                "insufficient attributable source excerpts",
            ));
        }
        self.angle_input()
            .and_then(|input| {
                writing::prepare_writing_request(&input)
                    .map_err(|error| WritingWorkflowError::InvalidResponse(error.to_string()))
            })
            .map(|_| ())
    }

    fn angle_input(&self) -> WriteResult<WritingInput> {
        Ok(WritingInput {
            brief: self.brief.clone(),
            evidence: self.evidence.clone(),
            findings: self.findings.clone(),
            task: WritingTask::ProposeAngles { count: 3 },
        })
    }

    fn input_for_angle(&self, angle: &AngleOption) -> WriteResult<WritingInput> {
        let mut input = self.angle_input()?;
        input.task = WritingTask::Draft {
            angle: angle.clone(),
        };
        Ok(input)
    }
}

/// The host rebuilds this from canonical Study/brief/evidence rows each time.
/// Full equality intentionally detects evidence review changes without a Study
/// revision bump. AtomicCandidateStore must RECHECK this at commit under its
/// own SQLite IMMEDIATE transaction, not rely only on this preflight.
pub trait CurrentStudy {
    fn load(&self, project_id: Uuid, study_id: Uuid) -> WriteResult<StudyWritingContext>;
}

pub struct FixedStudy<'a>(pub &'a StudyWritingContext);
impl CurrentStudy for FixedStudy<'_> {
    fn load(&self, project_id: Uuid, study_id: Uuid) -> WriteResult<StudyWritingContext> {
        if self.0.project_id != project_id || self.0.study_id != study_id {
            return Err(WritingWorkflowError::InvalidContext(
                "foreign project or study",
            ));
        }
        Ok(self.0.clone())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GenerationPlan {
    pub run_id: String,
    pub context: StudyWritingContext,
    pub angles_prompt: AiRequest,
}

pub fn prepare_three(run_id: &str, context: StudyWritingContext) -> WriteResult<GenerationPlan> {
    if run_id.is_empty()
        || run_id.len() > 128
        || !run_id
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
    {
        return Err(WritingWorkflowError::InvalidContext("run ID"));
    }
    context.validate()?;
    let angles_prompt = writing::prepare_writing_request(&context.angle_input()?)
        .map_err(|e| WritingWorkflowError::InvalidResponse(e.to_string()))?;
    Ok(GenerationPlan {
        run_id: run_id.into(),
        context,
        angles_prompt,
    })
}

/// The *only* provider dispatch interface. Implementation must check Agent 01's
/// specific provider/egress/tokens/US$ approval, then Agent 02's exact run/step
/// checkpoint and effect ledger claim BEFORE returning Dispatch. Existing,
/// uncertain, cancelled, expired or ambiguous claims MUST return an error.
/// Recorded means the exact step/prompt has a durable response artifact.
pub enum StepAdmission {
    Dispatch { effect_key: String },
    Recorded { response_json: String },
}
pub trait DurablePermissionBoundary {
    fn begin(
        &mut self,
        run_id: &str,
        context: &StudyWritingContext,
        step: &str,
        request: &AiRequest,
    ) -> WriteResult<StepAdmission>;
    /// Persist the bounded response artifact AND its receipt before returning.
    fn record_response(&mut self, effect_key: &str, response_json: &str) -> WriteResult<()>;
    /// Outcome after invocation is unknown, including timeouts. No automatic replay.
    fn mark_uncertain(&mut self, effect_key: &str) -> WriteResult<()>;
    /// Only valid after a durable reservation and provably before any dispatch.
    fn abandon_before_dispatch(&mut self, effect_key: &str) -> WriteResult<()>;
}

pub trait WritingTextProvider {
    fn generate(&mut self, request: &AiRequest) -> Result<AiResponse, ()>;
}

/// Reuses the existing audited provider adapter, not a new HTTP stack.
/// MUST ONLY be constructed behind the approved durable permission boundary.
/// No credential/configuration is saved by this adapter.
pub struct ApprovedAiAdapter<'a, P: AiProvider> {
    pub service: &'a AiService<P>,
    pub config: &'a ProviderConfig,
    pub ephemeral_key: &'a str,
}
impl<P: AiProvider> WritingTextProvider for ApprovedAiAdapter<'_, P> {
    fn generate(&mut self, request: &AiRequest) -> Result<AiResponse, ()> {
        self.service
            .generate(self.config, self.ephemeral_key, request)
            .map_err(|_| ())
    }
}

pub trait StopSignal {
    fn cancelled(&self) -> bool;
}
pub struct NeverStop;
impl StopSignal for NeverStop {
    fn cancelled(&self) -> bool {
        false
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CandidateLineage {
    pub source_id: Uuid,
    pub evidence: Vec<EvidenceRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GeneratedCandidate {
    pub angle: AngleOption,
    pub draft: ScriptDraft,
    pub lineage: Vec<CandidateLineage>,
    pub audit: DraftAudit,
    /// AI review is advisory and remains human-review-required.
    pub review_findings: Vec<ReviewFinding>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GeneratedBatch {
    pub run_id: String,
    pub context: StudyWritingContext,
    pub candidates: Vec<GeneratedCandidate>,
}

/// Must commit exactly three immutable draft initial versions, their source
/// links, structured provenance and idempotency receipt in one transaction.
/// Recheck project, Study revision AND exact evidence at transaction time.
/// A duplicate run returns the already committed draft IDs, never new drafts.
pub trait AtomicCandidateStore {
    fn commit_three(&mut self, batch: &GeneratedBatch) -> WriteResult<[Uuid; 3]>;
}

/// Always local and copyable; no provider, gate, key or network is needed.
pub fn copy_angles_prompt(plan: &GenerationPlan) -> &str {
    &plan.angles_prompt.prompt
}

fn check_current<C: CurrentStudy>(loader: &C, plan: &GenerationPlan) -> WriteResult<()> {
    let latest = loader.load(plan.context.project_id, plan.context.study_id)?;
    latest.validate()?;
    if latest != plan.context {
        return Err(WritingWorkflowError::StaleContext);
    }
    Ok(())
}

fn execute_step<B: DurablePermissionBoundary, P: WritingTextProvider, S: StopSignal>(
    gate: &mut B,
    provider: &mut P,
    stop: &S,
    plan: &GenerationPlan,
    step: &str,
    request: &AiRequest,
) -> WriteResult<String> {
    if stop.cancelled() {
        return Err(WritingWorkflowError::Cancelled);
    }
    match gate.begin(&plan.run_id, &plan.context, step, request)? {
        StepAdmission::Recorded { response_json } => Ok(response_json),
        StepAdmission::Dispatch { effect_key } => {
            if stop.cancelled() {
                gate.abandon_before_dispatch(&effect_key)?;
                return Err(WritingWorkflowError::Cancelled);
            }
            match provider.generate(request) {
                Ok(response) => {
                    // Fail closed on malformed/oversized receipt: do not retry
                    // a possibly billable call after an uncertain outcome.
                    if response.text.chars().count() > 100_000 {
                        gate.mark_uncertain(&effect_key)?;
                        return Err(WritingWorkflowError::InvalidResponse(
                            "provider output exceeds bound".into(),
                        ));
                    }
                    if gate.record_response(&effect_key, &response.text).is_err() {
                        // The provider may have charged even when the local
                        // response receipt failed to commit. Never replay it.
                        let _ = gate.mark_uncertain(&effect_key);
                        return Err(WritingWorkflowError::UncertainEffect);
                    }
                    Ok(response.text)
                }
                Err(()) => {
                    gate.mark_uncertain(&effect_key)?;
                    Err(WritingWorkflowError::ProviderFailed)
                }
            }
        }
    }
}

/// All provider outputs are validated *before* the first draft is persisted.
/// Steps have stable IDs; the boundary cannot dispatch an existing paid effect.
/// The sink owns the final atomic 3-draft commit and receipt.
pub fn generate_three<C, B, P, S, D>(
    plan: &GenerationPlan,
    current: &C,
    gate: &mut B,
    provider: &mut P,
    stop: &S,
    sink: &mut D,
) -> WriteResult<[Uuid; 3]>
where
    C: CurrentStudy,
    B: DurablePermissionBoundary,
    P: WritingTextProvider,
    S: StopSignal,
    D: AtomicCandidateStore,
{
    check_current(current, plan)?;
    let angles_json = execute_step(gate, provider, stop, plan, "angles", &plan.angles_prompt)?;
    check_current(current, plan)?;
    let angles = writing::parse_angles(&angles_json, &plan.context.angle_input()?)
        .map_err(|e| WritingWorkflowError::InvalidResponse(e.to_string()))?;
    if !writing::compare_angles(&angles).is_empty() {
        return Err(WritingWorkflowError::DuplicateOrSimilar);
    }
    let mut candidates = Vec::with_capacity(3);
    let mut normalized_scripts = HashSet::new();
    for (index, angle) in angles.iter().enumerate() {
        check_current(current, plan)?;
        let input = plan.context.input_for_angle(angle)?;
        let request = writing::prepare_writing_request(&input)
            .map_err(|e| WritingWorkflowError::InvalidResponse(e.to_string()))?;
        let raw = execute_step(
            gate,
            provider,
            stop,
            plan,
            &format!("draft-{}", index + 1),
            &request,
        )?;
        check_current(current, plan)?;
        let draft = writing::parse_draft(&raw, &input)
            .map_err(|e| WritingWorkflowError::InvalidResponse(e.to_string()))?;
        let prose: String = draft
            .beats
            .iter()
            .map(|beat| beat.narration.to_lowercase())
            .collect::<Vec<_>>()
            .join(" ");
        let prose: String = prose.split_whitespace().collect::<Vec<_>>().join(" ");
        if !normalized_scripts.insert(prose) {
            return Err(WritingWorkflowError::DuplicateOrSimilar);
        }
        let lineage = lineage_for(&plan.context, angle, &draft)?;
        let audit = writing::audit_draft(&draft, &plan.context.brief, &plan.context.evidence);
        candidates.push(GeneratedCandidate {
            angle: angle.clone(),
            draft,
            lineage,
            audit,
            review_findings: Vec::new(),
        });
    }
    if stop.cancelled() {
        return Err(WritingWorkflowError::Cancelled);
    }
    check_current(current, plan)?;
    // The sink is responsible for a second scoped context check INSIDE its
    // transaction, exact-once receipt and immutable initial versions.
    sink.commit_three(&GeneratedBatch {
        run_id: plan.run_id.clone(),
        context: plan.context.clone(),
        candidates,
    })
}

fn lineage_for(
    context: &StudyWritingContext,
    angle: &AngleOption,
    draft: &ScriptDraft,
) -> WriteResult<Vec<CandidateLineage>> {
    let mut evidence_ids: HashSet<&str> = angle.evidence_ids.iter().map(String::as_str).collect();
    for beat in &draft.beats {
        evidence_ids.extend(beat.evidence_ids.iter().map(String::as_str));
        for shot in &beat.shots {
            evidence_ids.extend(shot.evidence_ids.iter().map(String::as_str));
        }
    }
    for claim in &draft.claims {
        evidence_ids.extend(claim.evidence_ids.iter().map(String::as_str));
    }
    let mut lineage: HashMap<Uuid, Vec<EvidenceRef>> = HashMap::new();
    for item in &context.evidence {
        if !evidence_ids.contains(item.id.as_str()) {
            continue;
        }
        let claimed = Uuid::parse_str(&item.locator.source_id)
            .map_err(|_| WritingWorkflowError::InvalidContext("lineage locator"))?;
        let source = context
            .sources
            .iter()
            .find(|source| source.origin_id == claimed || source.study_source_id == claimed)
            .ok_or(WritingWorkflowError::InvalidContext(
                "foreign lineage source",
            ))?;
        lineage
            .entry(source.study_source_id)
            .or_default()
            .push(item.clone());
    }
    let mut ordered = Vec::new();
    for source in &context.sources {
        if let Some(evidence) = lineage.remove(&source.study_source_id) {
            ordered.push(CandidateLineage {
                source_id: source.study_source_id,
                evidence,
            });
        }
    }
    Ok(ordered)
}

/// Only an editor-selected beat and explicit feedback may be revised.
pub struct RevisionRequest<'a> {
    pub candidate: &'a GeneratedCandidate,
    pub beat_id: &'a str,
    pub feedback: &'a str,
}

/// Explicit revision request (never auto-applied or persisted). A provider
/// sees only the selected beat/feedback in a typed task; the existing accepted
/// script remains unchanged until a separate human-approved CAS save.
pub fn propose_beat_revision<C, B, P, S>(
    plan: &GenerationPlan,
    current: &C,
    gate: &mut B,
    provider: &mut P,
    stop: &S,
    revision: RevisionRequest<'_>,
) -> WriteResult<String>
where
    C: CurrentStudy,
    B: DurablePermissionBoundary,
    P: WritingTextProvider,
    S: StopSignal,
{
    let RevisionRequest {
        candidate,
        beat_id,
        feedback,
    } = revision;
    if !candidate.draft.beats.iter().any(|beat| beat.id == beat_id) {
        return Err(WritingWorkflowError::InvalidContext("unknown beat"));
    }
    check_current(current, plan)?;
    let mut input = plan.context.angle_input()?;
    input.task = WritingTask::ReviseBeat {
        draft: candidate.draft.clone(),
        beat_id: beat_id.into(),
        feedback: feedback.into(),
    };
    let request = writing::prepare_writing_request(&input)
        .map_err(|e| WritingWorkflowError::InvalidResponse(e.to_string()))?;
    let step = format!("revise-{}-{}", candidate.draft.id, beat_id);
    let proposed = execute_step(gate, provider, stop, plan, &step, &request)?;
    check_current(current, plan)?;
    writing::parse_revision_proposal(&proposed, &input)
        .map_err(|e| WritingWorkflowError::InvalidResponse(e.to_string()))?;
    Ok(proposed)
}

/// Optional billable model review; findings never count as independent fact
/// verification and every returned finding is forced to human-review-needed.
pub fn review_candidate<C, B, P, S>(
    plan: &GenerationPlan,
    current: &C,
    gate: &mut B,
    provider: &mut P,
    stop: &S,
    candidate: &GeneratedCandidate,
) -> WriteResult<GeneratedCandidate>
where
    C: CurrentStudy,
    B: DurablePermissionBoundary,
    P: WritingTextProvider,
    S: StopSignal,
{
    check_current(current, plan)?;
    let mut input = plan.context.angle_input()?;
    input.task = WritingTask::Review {
        draft: candidate.draft.clone(),
    };
    let request = writing::prepare_writing_request(&input)
        .map_err(|e| WritingWorkflowError::InvalidResponse(e.to_string()))?;
    let step = format!("review-{}", candidate.draft.id);
    let proposed = execute_step(gate, provider, stop, plan, &step, &request)?;
    check_current(current, plan)?;
    let review = writing::parse_review(&proposed, &input)
        .map_err(|e| WritingWorkflowError::InvalidResponse(e.to_string()))?;
    let mut updated = candidate.clone();
    updated.review_findings = review.findings;
    Ok(updated)
}

/// Human-edited revision saves must compare-and-swap the current canonical
/// draft version under an IMMEDIATE transaction, append an immutable version,
/// and reject any source/Study or beat-content change since approval.
pub trait VersionedCandidateStore {
    fn save_revision(
        &mut self,
        context: &StudyWritingContext,
        canonical_draft_id: Uuid,
        expected_draft_revision: i64,
        previous_beat_text: &str,
        beat_id: &str,
        reviewed: &GeneratedCandidate,
    ) -> WriteResult<i64>;
}

/// Model suggestions are never auto-applied. Exact expected narration and
/// editor's manual override choice are supplied by the caller only.
pub fn apply_reviewed_beat_revision(
    candidate: &GeneratedCandidate,
    context: &StudyWritingContext,
    response_json: &str,
    beat_id: &str,
    feedback: &str,
    expected_narration: &str,
    overwrite_manual_edit: bool,
) -> WriteResult<GeneratedCandidate> {
    let mut input = context.angle_input()?;
    input.task = WritingTask::ReviseBeat {
        draft: candidate.draft.clone(),
        beat_id: beat_id.into(),
        feedback: feedback.into(),
    };
    let proposal = writing::parse_revision_proposal(response_json, &input)
        .map_err(|e| WritingWorkflowError::InvalidResponse(e.to_string()))?;
    let updated = writing::apply_beat_revision(
        &candidate.draft,
        &BeatRevision {
            beat_id: beat_id.into(),
            expected_narration: expected_narration.into(),
            replacement_narration: proposal.replacement_narration,
            overwrite_manual_edit,
        },
    )
    .map_err(|e| WritingWorkflowError::InvalidResponse(e.to_string()))?;
    let mut next = candidate.clone();
    next.draft = updated;
    next.audit = writing::audit_draft(&next.draft, &context.brief, &context.evidence);
    next.review_findings
        .retain(|finding| finding.beat_id.as_deref() != Some(beat_id));
    next.lineage = lineage_for(context, &next.angle, &next.draft)?;
    Ok(next)
}

/// A text export with exact declared references and uncertainty. Export is
/// deliberately an explicit caller action; no auto-publishing or network I/O.
pub fn export_candidate(candidate: &GeneratedCandidate) -> String {
    let mut text = format!("# {}\n\n", candidate.draft.title);
    for beat in &candidate.draft.beats {
        text.push_str(&format!("## {:?}\n{}\n\n", beat.role, beat.narration));
        if let Some(on_screen) = &beat.on_screen_text {
            text.push_str(&format!("On screen: {on_screen}\n\n"));
        }
    }
    if let Some(cta) = &candidate.draft.cta {
        text.push_str(&format!("CTA: {cta}\n\n"));
    }
    if let Some(caption) = &candidate.draft.caption {
        text.push_str(&format!("Caption: {caption}\n\n"));
    }
    text.push_str("## Claims (not independently fact-checked)\n");
    for claim in &candidate.draft.claims {
        text.push_str(&format!(
            "- [{:?}] {} (beat {}, evidence: {})\n",
            claim.status,
            claim.assertion,
            claim.beat_id,
            claim.evidence_ids.join(", ")
        ));
    }
    text.push_str("\n## Exact cited source locators\n");
    for source in &candidate.lineage {
        for evidence in &source.evidence {
            let loc = &evidence.locator;
            text.push_str(&format!("- {} | Study source {} | original {} | time {:?} | section {:?} | URL {:?} | status {:?}\n",
                evidence.id, source.source_id, loc.source_id, loc.timestamp_seconds,
                loc.document_section, loc.canonical_url, evidence.status));
        }
    }
    text
}

pub fn needs_human_fact_review(candidate: &GeneratedCandidate) -> bool {
    candidate
        .draft
        .claims
        .iter()
        .any(|claim| claim.status != ClaimStatus::Opinion)
}
