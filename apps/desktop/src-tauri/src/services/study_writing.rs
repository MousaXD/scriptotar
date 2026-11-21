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
    AiRequest, AiResponse,
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
    /// Immutable canonical source identity, not a model-provided locator.
    pub fingerprint: String,
    /// Source revision captured by the Study's authoritative source row.
    pub source_revision: Option<String>,
    pub collected_at: String,
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
                        fingerprint: source.fingerprint.clone(),
                        source_revision: source.source_revision.clone(),
                        collected_at: source.collected_at.clone(),
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
                || source.fingerprint.trim().is_empty()
                || source.fingerprint.len() > 1000
                || source.collected_at.trim().is_empty()
                || source.collected_at.len() > 1000
                || source
                    .source_revision
                    .as_ref()
                    .is_some_and(|value| value.len() > 1000)
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
            // Only actual attributable source content counts toward the
            // three-original-source writing floor. Human observations are
            // useful context, but three notes about one transcript must not
            // impersonate three independent source excerpts.
            if original_source_excerpt(item) {
                covered.insert(binding.study_source_id);
            }
            // A disputed/contradicted observation is not an eligible citation
            // or writing input. The canonical gateway already filters these;
            // enforce the exclusion again at the workflow boundary.
            if item.status == EvidenceStatus::Contradicted {
                return Err(WritingWorkflowError::InvalidContext(
                    "disputed evidence is not eligible for generation",
                ));
            }
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

// No legacy AiService transport adapter is exposed here. Existing provider
// adapters lack trusted hard output-token caps and verified usage receipts.
// Agent 01's bounded transport and Agent 02's durable effect fence must be
// implemented and approved before wiring a paid WritingTextProvider.

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

    /// Read-only reconciliation BEFORE any billable step or current-Study CAS.
    /// Only return a committed receipt for the exact original run/plan/context.
    /// Real adapters must verify scope, receipt and all three initial versions.
    /// A missing receipt means generation may proceed ONLY through the separate
    /// permission and paid-effect boundary, never by inference from a timeout.
    fn recover_committed(&mut self, _plan: &GenerationPlan) -> WriteResult<Option<[Uuid; 3]>> {
        Ok(None)
    }
}

/// Always local and copyable; no provider, gate, key or network is needed.
pub fn copy_angles_prompt(plan: &GenerationPlan) -> &str {
    &plan.angles_prompt.prompt
}

fn check_current<C: CurrentStudy>(loader: &C, plan: &GenerationPlan) -> WriteResult<()> {
    // A forged or stale prompt must NEVER be sent just because IDs match.
    if plan.run_id.is_empty()
        || plan.run_id.len() > 128
        || !plan
            .run_id
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
    {
        return Err(WritingWorkflowError::InvalidContext("run ID"));
    }
    plan.context.validate()?;
    let expected = writing::prepare_writing_request(&plan.context.angle_input()?)
        .map_err(|e| WritingWorkflowError::InvalidResponse(e.to_string()))?;
    if plan.angles_prompt != expected {
        return Err(WritingWorkflowError::StaleContext);
    }
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
    // A lost success reply must not cause the already completed Study's CAS
    // revision to block read-only recovery. Revalidate the exact signed-off
    // plan shape and prompt BEFORE looking up the durable receipt. No provider
    // call or permission reservation occurs on this path.
    let prepared = prepare_three(&plan.run_id, plan.context.clone())?;
    if prepared.angles_prompt != plan.angles_prompt {
        return Err(WritingWorkflowError::StaleContext);
    }
    if let Some(ids) = sink.recover_committed(plan)? {
        return Ok(ids);
    }
    check_current(current, plan)?;
    let angles_json = execute_step(gate, provider, stop, plan, "angles", &plan.angles_prompt)?;
    check_current(current, plan)?;
    let angles = writing::parse_angles(&angles_json, &plan.context.angle_input()?)
        .map_err(|e| WritingWorkflowError::InvalidResponse(e.to_string()))?;
    if !writing::compare_angles(&angles).is_empty() {
        return Err(WritingWorkflowError::DuplicateOrSimilar);
    }
    // Reject a source-copying angle before reserving any of the three draft
    // steps. The SQLite sink also enforces this against direct batch callers.
    for angle in &angles {
        for text in [&angle.hook, &angle.approach, &angle.promise] {
            reject_extended_source_copy(&plan.context, text)?;
        }
        for beat in &angle.outline {
            reject_extended_source_copy(&plan.context, &beat.intent)?;
        }
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
    // Reject near-identical scripts and extended copied source passages before
    // asking SQLite to accept any result. The sink repeats this check.
    enforce_candidate_originality(&plan.context, &candidates)?;
    // The sink is responsible for a second scoped context check INSIDE its
    // transaction, exact-once receipt and immutable initial versions.
    sink.commit_three(&GeneratedBatch {
        run_id: plan.run_id.clone(),
        context: plan.context.clone(),
        candidates,
    })
}

/// A provider's originality instruction is not an acceptance guarantee. Check
/// both the orchestration path and the canonical SQLite sink: direct callers
/// must not bypass the same source-copy and candidate-diversity requirements.
/// This is a deterministic safety screen, not a plagiarism determination.
fn original_source_excerpt(item: &EvidenceRef) -> bool {
    !item.excerpt.trim().is_empty()
        && matches!(
            (item.kind, item.status),
            (EvidenceKind::SocialTranscript, EvidenceStatus::Attributed)
                | (
                    EvidenceKind::IndependentDocument,
                    EvidenceStatus::Attributed
                )
                | (
                    EvidenceKind::IndependentDocument,
                    EvidenceStatus::Corroborated
                )
        )
}

/// Targeted revisions are independently screened so a provider cannot put a
/// long copied transcript passage into a later version of a valid original.
/// The direct SQLite revision sink repeats this check under its write lock.
pub(super) fn reject_extended_source_copy(
    context: &StudyWritingContext,
    replacement: &str,
) -> WriteResult<()> {
    const WINDOW: usize = 12;
    let words = originality_words(replacement);
    if words.len() < WINDOW {
        return Ok(());
    }
    let candidate_windows: HashSet<Vec<String>> =
        words.windows(WINDOW).map(<[String]>::to_vec).collect();
    for item in &context.evidence {
        if matches!(
            item.kind,
            EvidenceKind::SocialTranscript | EvidenceKind::IndependentDocument
        ) && originality_words(&item.excerpt)
            .windows(WINDOW)
            .any(|source| candidate_windows.contains(source))
        {
            return Err(WritingWorkflowError::InvalidResponse(
                "generated text reproduces an extended source passage".into(),
            ));
        }
    }
    Ok(())
}

pub(super) fn enforce_candidate_originality(
    context: &StudyWritingContext,
    candidates: &[GeneratedCandidate],
) -> WriteResult<()> {
    const SOURCE_WINDOW: usize = 12;
    const SHORT_SHARED_CTA_WORDS: usize = 10;
    const LONG_SCRIPT_WINDOW: usize = 5;
    const SHORT_SCRIPT_WINDOW: usize = 3;
    const VERY_SHORT_SCRIPT_WINDOW: usize = 2;

    let mut source_windows = HashSet::new();
    for evidence in &context.evidence {
        if !matches!(
            evidence.kind,
            EvidenceKind::SocialTranscript | EvidenceKind::IndependentDocument
        ) {
            continue;
        }
        let words = originality_words(&evidence.excerpt);
        source_windows.extend(words.windows(SOURCE_WINDOW).map(<[String]>::to_vec));
    }

    let mut scripts = Vec::with_capacity(candidates.len());
    let mut cited_sources = HashSet::new();
    for candidate in candidates {
        if candidate.angle.evidence_ids.is_empty() {
            return Err(WritingWorkflowError::InvalidResponse(
                "candidate angle has no supporting evidence".into(),
            ));
        }
        // Count only evidence cited by the actual script beats/claims. An
        // angle may cite a source that never supports any script content.
        let mut script_evidence = HashSet::new();
        for beat in &candidate.draft.beats {
            script_evidence.extend(beat.evidence_ids.iter().map(String::as_str));
            for shot in &beat.shots {
                script_evidence.extend(shot.evidence_ids.iter().map(String::as_str));
            }
        }
        for claim in &candidate.draft.claims {
            script_evidence.extend(claim.evidence_ids.iter().map(String::as_str));
        }
        let mut candidate_covered = false;
        for line in &candidate.lineage {
            // Every candidate must cite at least one attributable original
            // excerpt, not merely a creator note or metadata observation.
            // The batch must additionally cite three distinct such origins.
            if line.evidence.iter().any(|item| {
                script_evidence.contains(item.id.as_str()) && original_source_excerpt(item)
            }) {
                candidate_covered = true;
                cited_sources.insert(line.source_id);
            }
        }
        if !candidate_covered {
            return Err(WritingWorkflowError::InvalidResponse(
                "candidate has no attributable original source citation".into(),
            ));
        }
        let narration = candidate
            .draft
            .beats
            .iter()
            .map(|beat| beat.narration.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        // The source-copy check covers every user-visible generated text field.
        // A model must not hide copied passages in an on-screen line, caption,
        // CTA or shot suggestion instead of the main spoken narration.
        let mut generated_text = vec![narration.as_str(), candidate.draft.title.as_str()];
        if let Some(cta) = candidate.draft.cta.as_deref() {
            generated_text.push(cta);
        }
        if let Some(caption) = candidate.draft.caption.as_deref() {
            generated_text.push(caption);
        }
        for beat in &candidate.draft.beats {
            if let Some(text) = beat.on_screen_text.as_deref() {
                generated_text.push(text);
            }
            for shot in &beat.shots {
                generated_text.push(shot.visual.as_str());
                generated_text.push(shot.purpose.as_str());
            }
        }
        for claim in &candidate.draft.claims {
            generated_text.push(claim.assertion.as_str());
        }
        for finding in &candidate.review_findings {
            generated_text.push(finding.concern.as_str());
            generated_text.push(finding.recommendation.as_str());
        }
        generated_text.push(candidate.angle.hook.as_str());
        generated_text.push(candidate.angle.approach.as_str());
        generated_text.push(candidate.angle.promise.as_str());
        for beat in &candidate.angle.outline {
            generated_text.push(beat.intent.as_str());
        }
        if generated_text.iter().any(|text| {
            originality_words(text)
                .windows(SOURCE_WINDOW)
                .any(|window| source_windows.contains(window))
        }) {
            return Err(WritingWorkflowError::InvalidResponse(
                "script reproduces an extended source passage".into(),
            ));
        }

        // Only a genuinely short FINAL CTA beat may be excluded from
        // diversity checks. A model cannot evade originality by labeling
        // long or nonterminal story narration "cta", or by putting a long
        // copied story into the separately exported CTA field.
        let mut creative_parts = candidate
            .draft
            .beats
            .iter()
            .enumerate()
            .filter(|(index, beat)| {
                !(*index + 1 == candidate.draft.beats.len()
                    && beat.role == scriptotar_ai::writing::BeatRole::Cta
                    && originality_words(&beat.narration).len() <= SHORT_SHARED_CTA_WORDS)
            })
            .map(|(_, beat)| beat.narration.as_str())
            .collect::<Vec<_>>();
        if let Some(cta) = candidate.draft.cta.as_deref() {
            if originality_words(cta).len() > SHORT_SHARED_CTA_WORDS {
                creative_parts.push(cta);
            }
        }
        let creative_narration = creative_parts.join(" ");
        let creative_words = originality_words(&creative_narration);
        // A one- or two-word hook plus a common CTA is not a meaningfully
        // distinct full script. Short-form scripts remain supported, but must
        // contain at least a small amount of original creative narration.
        if creative_words.len() < 6 {
            return Err(WritingWorkflowError::InvalidResponse(
                "candidate creative narration is too short".into(),
            ));
        }
        scripts.push(creative_words);
    }

    if candidates.len() == 3 && cited_sources.len() < 3 {
        return Err(WritingWorkflowError::InvalidResponse(
            "three-script batch does not cite three distinct Study sources".into(),
        ));
    }

    for left in 0..scripts.len() {
        for right in (left + 1)..scripts.len() {
            let left_words = &scripts[left];
            let right_words = &scripts[right];
            if left_words == right_words {
                return Err(WritingWorkflowError::DuplicateOrSimilar);
            }
            // A fixed 15-word floor allowed one-word edits of 10-14 word
            // scripts to bypass diversity checks. Choose a shorter n-gram
            // for shorter narrations instead. The smaller-script overlap
            // ratio handles different-length candidates without using CTA.
            let shorter = left_words.len().min(right_words.len());
            let (window, minimum_shared, minimum_percent) = if shorter >= 15 {
                (LONG_SCRIPT_WINDOW, 3, 60)
            } else if shorter >= 10 {
                (SHORT_SCRIPT_WINDOW, 3, 60)
            } else if shorter >= 6 {
                (VERY_SHORT_SCRIPT_WINDOW, 2, 60)
            } else {
                (VERY_SHORT_SCRIPT_WINDOW, 2, 80)
            };
            let left_shingles: HashSet<Vec<String>> =
                left_words.windows(window).map(<[String]>::to_vec).collect();
            let right_shingles: HashSet<Vec<String>> = right_words
                .windows(window)
                .map(<[String]>::to_vec)
                .collect();
            let smaller = left_shingles.len().min(right_shingles.len());
            if smaller == 0 {
                continue;
            }
            let shared = left_shingles.intersection(&right_shingles).count();
            if shared >= minimum_shared && shared * 100 >= smaller * minimum_percent {
                return Err(WritingWorkflowError::DuplicateOrSimilar);
            }
        }
    }
    Ok(())
}

fn originality_words(text: &str) -> Vec<String> {
    // Arabic vowel marks, tatweel and invisible format characters must not
    // split a copied word into fragments that bypass the lexical safety check.
    // Keep this deterministic and dependency-free for offline evidence tests.
    let mut words = Vec::new();
    let mut current = String::new();
    for character in text.chars() {
        let is_format_mark =
            ['\u{0640}', '\u{200B}', '\u{200C}', '\u{200D}', '\u{FEFF}'].contains(&character);
        let is_arabic_mark = ('\u{0610}'..='\u{061A}').contains(&character)
            || ('\u{064B}'..='\u{065F}').contains(&character)
            || character == '\u{0670}'
            || ('\u{06D6}'..='\u{06ED}').contains(&character)
            || ('\u{08D3}'..='\u{08FF}').contains(&character);
        // Unicode direction controls can be inserted between every letter
        // without changing visible Arabic/English words. Treat these as
        // ignorable for originality admission, not token boundaries.
        let is_bidi_control = character == '\u{061C}'
            || character == '\u{200E}'
            || character == '\u{200F}'
            || character == '\u{2060}'
            || ('\u{202A}'..='\u{202E}').contains(&character)
            || ('\u{2066}'..='\u{2069}').contains(&character);
        if is_format_mark || is_arabic_mark || is_bidi_control {
            continue;
        }
        let normalized = match character {
            'أ' | 'إ' | 'آ' | 'ٱ' => 'ا',
            _ => character,
        };
        if normalized.is_alphanumeric() {
            current.extend(normalized.to_lowercase());
        } else if !current.is_empty() {
            words.push(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        words.push(current);
    }
    words
}

pub(super) fn lineage_for(
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
    reject_extended_source_copy(context, &proposal.replacement_narration)?;
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
        for shot in &beat.shots {
            text.push_str(&format!(
                "Shot suggestion: {} ({}) | evidence: {}\n",
                shot.visual,
                shot.purpose,
                shot.evidence_ids.join(", ")
            ));
        }
        if !beat.evidence_ids.is_empty() {
            text.push_str(&format!(
                "Beat source references (not fact-checked): {}\n",
                beat.evidence_ids.join(", ")
            ));
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
    if !candidate.review_findings.is_empty() {
        text.push_str("\n## Model review suggestions (human review required)\n");
        for finding in &candidate.review_findings {
            text.push_str(&format!(
                "- {} | {} | evidence: {}\n",
                finding.concern,
                finding.recommendation,
                finding.evidence_ids.join(", ")
            ));
        }
    }
    if !candidate.audit.advisories.is_empty() {
        text.push_str("\n## Writing quality advisories (not factual verification)\n");
        for advisory in &candidate.audit.advisories {
            text.push_str(&format!(
                "- {:?}: {}\n",
                advisory.level, advisory.description
            ));
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
