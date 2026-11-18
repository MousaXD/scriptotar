//! Offline claim verification over **host-resolved** evidence and review records.
//!
//! This module never authenticates callers, fetches sources, runs a provider, or
//! upgrades a model's claimed certainty. The host MUST load project-scoped
//! evidence and authenticated reviewer attestations from its own repository.
//! An arbitrary JSON object from a model must never be used as a VerifiedReview.
use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::{ClaimStatus, ScriptDraft};

const MAX_CLAIMS: usize = 128;
const MAX_EVIDENCE: usize = 256;
const MAX_CITATIONS: usize = 16;
const MAX_TEXT: usize = 8_000;

/// SHA-256 over length-delimited canonical bytes loaded by the host, never
/// over model-supplied content versions. This proves local snapshot equality,
/// not external authorship, accuracy, permission or independent corroboration.
pub fn canonical_source_sha256(parts: &[&[u8]]) -> String {
    let mut digest = Sha256::new();
    digest.update(b"scriptotar/source-content/v1\0");
    for part in parts {
        digest.update((part.len() as u64).to_be_bytes());
        digest.update(part);
    }
    format!("sha256:{:x}", digest.finalize())
}

fn is_canonical_source_hash(value: &str) -> bool {
    let Some(hex) = value.strip_prefix("sha256:") else {
        return false;
    };
    hex.len() == 64
        && hex
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ClaimKind {
    Fact,
    Opinion,
    Prediction,
    UnsupportedStatement,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Consequence {
    Ordinary,
    Legal,
    Financial,
    Health,
    OtherHighStakes,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Modality {
    DocumentText,
    SpokenTranscript,
    VisualFrame,
    Metric,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExactLocator {
    /// Human-readable exact source locator or document section.
    pub locator: String,
    /// An actual segment ID, never a fabricated array index.
    pub segment_id: Option<String>,
    pub timestamp_millis: Option<u64>,
    pub modality: Modality,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Citation {
    pub evidence_id: String,
    /// Canonical StudySource ID, not a URL, title, or origin's global ID.
    pub source_id: String,
    /// Content-version identity; not an unversioned source URL.
    pub source_version: String,
    pub locator: ExactLocator,
    /// Exact excerpt, no paraphrase presented as a verbatim quotation.
    pub quote: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClaimProposal {
    pub id: String,
    pub beat_id: String,
    pub statement: String,
    /// This is a proposal, NOT an authoritative classification.
    pub kind: ClaimKind,
    pub consequence: Consequence,
    pub required_modality: Modality,
    pub citations: Vec<Citation>,
    /// Explicitly ignored for promotion; retained as an audit signal.
    pub model_claims_verified: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SourceKind {
    IndependentDocument,
    SocialTranscript,
    ManualNote,
    ResearchMetadata,
    VisualFrame,
    AiGenerated,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceState {
    /// A reference exists but no authenticated support decision is present.
    Unreviewed,
    /// Authenticated reviewer assessed the exact excerpt for this claim.
    Reviewed,
    Disputed,
    Revoked,
    Untrusted,
    Unavailable,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceRelation {
    /// User/model-provided observations MUST remain unassessed.
    Unassessed,
    Supports,
    Contradicts,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ResolvedEvidence {
    pub evidence_id: String,
    pub source_id: String,
    pub project_id: String,
    pub study_id: String,
    pub source_version: Option<String>,
    pub locator: ExactLocator,
    pub excerpt: String,
    pub kind: SourceKind,
    pub state: EvidenceState,
    pub relation: EvidenceRelation,
    /// Independent ownership/origin established by the host, not the model.
    pub independent_origin: Option<String>,
    /// None means freshness/expiry is unknown and cannot be certified.
    pub collected_at_epoch: Option<u64>,
    pub expires_at_epoch: Option<u64>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReviewDecision {
    Accept,
    Reject,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct VerifiedReview {
    /// Only append-only, authenticated, independent HUMAN review records.
    pub review_id: String,
    pub reviewer_id: String,
    pub project_id: String,
    pub study_id: String,
    pub draft_id: String,
    pub draft_revision: u64,
    pub claim_id: String,
    pub exact_statement: String,
    pub evidence_ids: Vec<String>,
    /// Exact attested versions, locators AND quotes. An evidence ID alone can
    /// remain stable while its content changes; an older review must not pass.
    pub exact_citations: Vec<Citation>,
    pub decision: ReviewDecision,
    pub reviewed_at_epoch: u64,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct VerificationContext {
    pub project_id: String,
    pub study_id: String,
    pub draft_id: String,
    pub draft_revision: u64,
    pub checked_at_epoch: u64,
    /// Records must come from an authoritative, scoped, coherent snapshot.
    pub evidence: Vec<ResolvedEvidence>,
    /// Must be loaded from authenticated review storage, never the model.
    pub human_reviews: Vec<VerifiedReview>,
    /// Host-only authority: typed claim manifest equals the canonical saved
    /// draft body. Legacy free-text Studio drafts cannot claim this binding.
    pub draft_content_bound: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VerificationStatus {
    Verified,
    Unverified,
    Contradicted,
    Blocked,
    Opinion,
    Prediction,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum FindingCode {
    InvalidClaim,
    DraftContentNotBound,
    UnannotatedText,
    UnsupportedStatement,
    ModelPromotionAttempt,
    MissingCitation,
    UnknownCitation,
    CrossProjectReference,
    SourceMismatch,
    StaleSourceVersion,
    UnversionedSource,
    ForgedLocatorOrQuote,
    MissingSegmentOrTimestamp,
    WrongEvidenceModality,
    UnavailableSource,
    RevokedSource,
    DisputedEvidence,
    UntrustedSource,
    OutdatedSource,
    UnknownFreshness,
    ContradictoryEvidence,
    InsufficientIndependentSources,
    NotIndependentlyReviewed,
    UnassessedCitation,
    ConsequentialHumanReviewRequired,
    InvalidReviewLineage,
    ReviewRejected,
    InputLimit,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Finding {
    pub code: FindingCode,
    pub claim_id: String,
    pub evidence_id: Option<String>,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClaimResult {
    pub id: String,
    pub beat_id: String,
    pub statement: String,
    pub classification: ClaimKind,
    pub status: VerificationStatus,
    pub exact_citations: Vec<Citation>,
    pub findings: Vec<Finding>,
    /// Qualified review is mandatory for consequential claims; this is not
    /// satisfied by the current generic human decision record.
    pub qualified_human_review_required: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct VerificationReport {
    pub project_id: String,
    pub study_id: String,
    pub draft_id: String,
    pub draft_revision: u64,
    pub checked_at_epoch: u64,
    pub claims: Vec<ClaimResult>,
    pub publication_review_required: bool,
    /// There is no independently attested complete claim inventory in the
    /// current data model. Individual Verified claims do not clear a script.
    pub inventory_complete: bool,
    pub draft_content_bound: bool,
    /// Never discard past decision records when saving a new draft version.
    /// The host must durably persist each revision's report/attestation history.
    pub review_lineage: Vec<VerifiedReview>,
}

/// Structured extraction from model-proposed FactualClaim annotations. No NLP
/// classification is claimed: unannotated narration is conservatively flagged.
/// A separate caller may explicitly propose a Prediction, but it cannot
/// self-certify a fact or bypass the authenticated review gate.
pub fn extract_draft_claims(draft: &ScriptDraft) -> Vec<ClaimProposal> {
    let mut claimed_beats = HashSet::new();
    let mut claims: Vec<ClaimProposal> = draft
        .claims
        .iter()
        .take(MAX_CLAIMS)
        .map(|claim| {
            claimed_beats.insert(claim.beat_id.as_str());
            ClaimProposal {
                id: claim.id.clone(),
                beat_id: claim.beat_id.clone(),
                statement: claim.assertion.clone(),
                kind: if claim.status == ClaimStatus::Opinion {
                    ClaimKind::Opinion
                } else {
                    ClaimKind::Fact
                },
                consequence: Consequence::Ordinary,
                required_modality: Modality::DocumentText,
                citations: Vec::new(), // IDs alone are not verified citations.
                model_claims_verified: claim.status == ClaimStatus::Supported,
            }
        })
        .collect();
    for beat in &draft.beats {
        if claims.len() >= MAX_CLAIMS {
            break;
        }
        if !claimed_beats.contains(beat.id.as_str()) && !beat.narration.trim().is_empty() {
            claims.push(ClaimProposal {
                id: format!("unannotated:{}", beat.id),
                beat_id: beat.id.clone(),
                statement: beat.narration.clone(),
                kind: ClaimKind::UnsupportedStatement,
                consequence: Consequence::Ordinary,
                required_modality: Modality::DocumentText,
                citations: Vec::new(),
                model_claims_verified: false,
            });
        }
    }
    claims
}

fn finding(code: FindingCode, claim: &str, evidence: Option<&str>, detail: &str) -> Finding {
    Finding {
        code,
        claim_id: claim.to_owned(),
        evidence_id: evidence.map(str::to_owned),
        detail: detail.to_owned(),
    }
}

fn matches_modality(kind: SourceKind, modality: Modality) -> bool {
    matches!(
        (kind, modality),
        (SourceKind::IndependentDocument, Modality::DocumentText)
            | (SourceKind::SocialTranscript, Modality::SpokenTranscript)
            | (SourceKind::VisualFrame, Modality::VisualFrame)
            | (SourceKind::ResearchMetadata, Modality::Metric)
    )
}

fn exact_id_set(left: &[String], right: &[String]) -> bool {
    left.len() == right.len()
        && left.iter().collect::<HashSet<_>>() == right.iter().collect::<HashSet<_>>()
}

fn review_matches(
    review: &VerifiedReview,
    claim: &ClaimProposal,
    ctx: &VerificationContext,
    ids: &[String],
) -> bool {
    !review.review_id.trim().is_empty()
        && !review.reviewer_id.trim().is_empty()
        && review.project_id == ctx.project_id
        && review.study_id == ctx.study_id
        && review.draft_id == ctx.draft_id
        && review.draft_revision == ctx.draft_revision
        && review.claim_id == claim.id
        && review.exact_statement == claim.statement
        && review.reviewed_at_epoch <= ctx.checked_at_epoch
        // The record cannot attest to evidence that was not yet collected.
        // A refreshed source needs a new review even if its declared version
        // string and excerpt are identical to the previous snapshot.
        && claim.citations.iter().all(|citation| {
            ctx.evidence
                .iter()
                .find(|item| item.evidence_id == citation.evidence_id)
                .and_then(|item| item.collected_at_epoch)
                .is_some_and(|collected| collected <= review.reviewed_at_epoch)
        })
        && exact_id_set(&review.evidence_ids, ids)
        && review.exact_citations.len() == claim.citations.len()
        && claim
            .citations
            .iter()
            .all(|citation| review.exact_citations.contains(citation))
}

/// Deterministic, fail-closed evaluation. A review record is ONLY credible if
/// the host loaded it from an authenticated append-only store and rechecked it
/// against the same evidence snapshot. This function cannot prove factual truth.
pub fn verify_claims(ctx: &VerificationContext, proposals: &[ClaimProposal]) -> VerificationReport {
    let mut by_id = HashMap::new();
    for evidence in ctx.evidence.iter().take(MAX_EVIDENCE + 1) {
        // Duplicate IDs are deliberately ambiguous, so lookup will fail.
        if by_id
            .insert(evidence.evidence_id.as_str(), Some(evidence))
            .is_some()
        {
            by_id.insert(evidence.evidence_id.as_str(), None);
        }
    }
    let invalid_batch = proposals.len() > MAX_CLAIMS
        || ctx.evidence.len() > MAX_EVIDENCE
        || ctx.draft_revision == 0
        || ctx.project_id.is_empty()
        || ctx.study_id.is_empty()
        || ctx.draft_id.is_empty();
    // Duplicate claim IDs invalidate every occurrence, not just later ones:
    // the first item must not inherit a review with an ambiguous identity.
    let mut unique_claim_ids = HashSet::new();
    let mut duplicate_claim_ids = HashSet::new();
    for proposal in proposals {
        if !unique_claim_ids.insert(proposal.id.as_str()) {
            duplicate_claim_ids.insert(proposal.id.as_str());
        }
    }
    let mut results = Vec::new();
    for claim in proposals.iter().take(MAX_CLAIMS) {
        let mut issues = Vec::new();
        let mut blocked = invalid_batch;
        let mut contradicted = false;
        let mut supporting_origins = HashSet::new();
        let mut supporting_hashes = HashSet::new();
        let mut approved = Vec::new();
        if invalid_batch {
            issues.push(finding(
                FindingCode::InputLimit,
                &claim.id,
                None,
                "Scope, revision or batch limit invalid",
            ));
        }
        if claim.id.trim().is_empty()
            || duplicate_claim_ids.contains(claim.id.as_str())
            || claim.statement.trim().is_empty()
            || claim.statement.chars().count() > MAX_TEXT
            || claim.beat_id.trim().is_empty()
        {
            blocked = true;
            issues.push(finding(
                FindingCode::InvalidClaim,
                &claim.id,
                None,
                "Invalid or repeated claim identity/text",
            ));
        }
        if !ctx.draft_content_bound {
            issues.push(finding(
                FindingCode::DraftContentNotBound,
                &claim.id,
                None,
                "Typed claims are not bound to the canonical saved draft body",
            ));
        }
        if claim.model_claims_verified {
            issues.push(finding(
                FindingCode::ModelPromotionAttempt,
                &claim.id,
                None,
                "Model status is not proof",
            ));
        }
        if claim.citations.is_empty() {
            issues.push(finding(
                FindingCode::MissingCitation,
                &claim.id,
                None,
                "No exact evidence reference",
            ));
        }
        if claim.citations.len() > MAX_CITATIONS {
            blocked = true;
            issues.push(finding(
                FindingCode::InputLimit,
                &claim.id,
                None,
                "Too many citations",
            ));
        }
        let mut used = HashSet::new();
        for citation in claim.citations.iter().take(MAX_CITATIONS) {
            let id = citation.evidence_id.as_str();
            if !used.insert(id) {
                blocked = true;
                issues.push(finding(
                    FindingCode::UnknownCitation,
                    &claim.id,
                    Some(id),
                    "Duplicate citation",
                ));
                continue;
            }
            let Some(Some(evidence)) = by_id.get(id) else {
                blocked = true;
                issues.push(finding(
                    FindingCode::UnknownCitation,
                    &claim.id,
                    Some(id),
                    "Unknown or ambiguous evidence ID",
                ));
                continue;
            };
            if evidence.project_id != ctx.project_id || evidence.study_id != ctx.study_id {
                blocked = true;
                issues.push(finding(
                    FindingCode::CrossProjectReference,
                    &claim.id,
                    Some(id),
                    "Evidence outside current project/Study",
                ));
                continue;
            }
            if citation.source_id != evidence.source_id {
                blocked = true;
                issues.push(finding(
                    FindingCode::SourceMismatch,
                    &claim.id,
                    Some(id),
                    "Citation source ID does not match authoritative record",
                ));
                continue;
            }
            if evidence
                .source_version
                .as_deref()
                .is_none_or(|version| !is_canonical_source_hash(version))
            {
                blocked = true;
                issues.push(finding(
                    FindingCode::UnversionedSource,
                    &claim.id,
                    Some(id),
                    "No independently checked source content version",
                ));
                continue;
            }
            if evidence.source_version.as_deref() != Some(citation.source_version.as_str()) {
                blocked = true;
                issues.push(finding(
                    FindingCode::StaleSourceVersion,
                    &claim.id,
                    Some(id),
                    "Source version changed",
                ));
                continue;
            }
            if citation.locator != evidence.locator
                || citation.quote != evidence.excerpt
                || citation.quote.trim().is_empty()
            {
                blocked = true;
                issues.push(finding(
                    FindingCode::ForgedLocatorOrQuote,
                    &claim.id,
                    Some(id),
                    "Locator or exact source excerpt mismatch",
                ));
                continue;
            }
            if evidence.locator.modality == Modality::SpokenTranscript
                && (evidence
                    .locator
                    .segment_id
                    .as_deref()
                    .is_none_or(str::is_empty)
                    || evidence.locator.timestamp_millis.is_none())
            {
                issues.push(finding(
                    FindingCode::MissingSegmentOrTimestamp,
                    &claim.id,
                    Some(id),
                    "Transcript citation has no verified segment/timecode",
                ));
                continue;
            }
            if !matches_modality(evidence.kind, claim.required_modality)
                || evidence.locator.modality != claim.required_modality
            {
                issues.push(finding(
                    FindingCode::WrongEvidenceModality,
                    &claim.id,
                    Some(id),
                    "Spoken words, metrics and visual frames are not interchangeable",
                ));
                continue;
            }
            match evidence.state {
                EvidenceState::Unavailable => {
                    blocked = true;
                    issues.push(finding(
                        FindingCode::UnavailableSource,
                        &claim.id,
                        Some(id),
                        "Source unavailable",
                    ));
                    continue;
                }
                EvidenceState::Revoked => {
                    blocked = true;
                    issues.push(finding(
                        FindingCode::RevokedSource,
                        &claim.id,
                        Some(id),
                        "Source access revoked",
                    ));
                    continue;
                }
                EvidenceState::Disputed => {
                    blocked = true;
                    issues.push(finding(
                        FindingCode::DisputedEvidence,
                        &claim.id,
                        Some(id),
                        "Evidence has an unresolved dispute",
                    ));
                    continue;
                }
                EvidenceState::Untrusted => {
                    blocked = true;
                    issues.push(finding(
                        FindingCode::UntrustedSource,
                        &claim.id,
                        Some(id),
                        "Evidence is untrusted",
                    ));
                    continue;
                }
                EvidenceState::Unreviewed | EvidenceState::Reviewed => {}
            }
            // Every cited excerpt must be adjudicated; two independently
            // supporting sources cannot launder a third unassessed citation.
            if evidence.state == EvidenceState::Unreviewed
                || evidence.relation == EvidenceRelation::Unassessed
            {
                issues.push(finding(
                    FindingCode::UnassessedCitation,
                    &claim.id,
                    Some(id),
                    "This cited excerpt has no authenticated claim-level assessment",
                ));
            }
            if evidence.collected_at_epoch.is_none() {
                issues.push(finding(
                    FindingCode::UnknownFreshness,
                    &claim.id,
                    Some(id),
                    "Collection date cannot be verified",
                ));
                continue;
            }
            if evidence
                .collected_at_epoch
                .is_some_and(|date| date > ctx.checked_at_epoch)
                || evidence
                    .expires_at_epoch
                    .is_some_and(|expiry| expiry < ctx.checked_at_epoch)
            {
                issues.push(finding(
                    FindingCode::OutdatedSource,
                    &claim.id,
                    Some(id),
                    "Evidence is stale, expired or future dated",
                ));
                continue;
            }
            if evidence.relation == EvidenceRelation::Contradicts {
                contradicted = true;
                issues.push(finding(
                    FindingCode::ContradictoryEvidence,
                    &claim.id,
                    Some(id),
                    "Authenticated source contradicts the claim",
                ));
            }
            if evidence.state == EvidenceState::Reviewed
                && evidence.relation == EvidenceRelation::Supports
                && evidence.kind == SourceKind::IndependentDocument
            {
                if let Some(origin) = evidence
                    .independent_origin
                    .as_deref()
                    .filter(|id| !id.trim().is_empty())
                {
                    supporting_origins.insert(origin.to_owned());
                    // An exact-content mirror cannot serve as a second source.
                    if let Some(version) = evidence.source_version.as_deref() {
                        supporting_hashes.insert(version.to_owned());
                    }
                }
            }
            approved.push(citation.clone());
        }
        let ids: Vec<String> = approved
            .iter()
            .map(|citation| citation.evidence_id.clone())
            .collect();
        // A review cannot predate any cited source's actual collection into
        // the authoritative store, even if its identifiers match later.
        let newest_collection = approved
            .iter()
            .filter_map(|citation| {
                by_id
                    .get(citation.evidence_id.as_str())
                    .and_then(|entry| entry.as_ref())
                    .and_then(|entry| entry.collected_at_epoch)
            })
            .max();
        let relevant: Vec<_> = ctx
            .human_reviews
            .iter()
            .filter(|review| review.claim_id == claim.id)
            .collect();
        let matched_reviews: Vec<_> = relevant
            .iter()
            .filter(|review| {
                review_matches(review, claim, ctx, &ids)
                    && newest_collection.is_none_or(|date| review.reviewed_at_epoch >= date)
            })
            .collect();
        if relevant.len() != matched_reviews.len() {
            issues.push(finding(
                FindingCode::InvalidReviewLineage,
                &claim.id,
                None,
                "A review was for a different claim, version, scope, evidence set, or future time",
            ));
        }
        let human_accepted = matched_reviews
            .iter()
            .any(|review| review.decision == ReviewDecision::Accept);
        if matched_reviews
            .iter()
            .any(|review| review.decision == ReviewDecision::Reject)
        {
            contradicted = true;
            issues.push(finding(
                FindingCode::ReviewRejected,
                &claim.id,
                None,
                "An independent human review rejected this claim",
            ));
        }
        // Current generic reviewer records have no independently authenticated
        // reviewer qualification, so no high-stakes fact is certifiable.
        if claim.consequence != Consequence::Ordinary {
            issues.push(finding(
                FindingCode::ConsequentialHumanReviewRequired,
                &claim.id,
                None,
                "Consequential claims require authenticated qualified human review",
            ));
        }
        if !human_accepted {
            issues.push(finding(
                FindingCode::NotIndependentlyReviewed,
                &claim.id,
                None,
                "No current, authenticated, independent human review",
            ));
        }
        if supporting_origins.len() < 2 || supporting_hashes.len() < 2 {
            issues.push(finding(
                FindingCode::InsufficientIndependentSources,
                &claim.id,
                None,
                "Fewer than two independent reviewed document origins and content snapshots",
            ));
        }
        if claim.kind == ClaimKind::UnsupportedStatement {
            issues.push(finding(
                FindingCode::UnsupportedStatement,
                &claim.id,
                None,
                "Unclassified statement cannot be automatically promoted",
            ));
        }
        let verified = !blocked
            && !contradicted
            && claim.kind == ClaimKind::Fact
            && ctx.draft_content_bound
            && claim.consequence == Consequence::Ordinary
            && human_accepted
            && supporting_origins.len() >= 2
            && supporting_hashes.len() >= 2
            && ids.len() == claim.citations.len()
            && !issues.iter().any(|issue| {
                matches!(
                    issue.code,
                    FindingCode::OutdatedSource
                        | FindingCode::UnknownFreshness
                        | FindingCode::UnassessedCitation
                        | FindingCode::MissingSegmentOrTimestamp
                        | FindingCode::WrongEvidenceModality
                        | FindingCode::InvalidReviewLineage
                )
            });
        let status = if blocked {
            VerificationStatus::Blocked
        } else if contradicted {
            VerificationStatus::Contradicted
        } else if verified {
            VerificationStatus::Verified
        } else if claim.kind == ClaimKind::Opinion {
            VerificationStatus::Opinion
        } else if claim.kind == ClaimKind::Prediction {
            VerificationStatus::Prediction
        } else {
            VerificationStatus::Unverified
        };
        if claim.kind == ClaimKind::UnsupportedStatement {
            issues.push(finding(
                FindingCode::UnannotatedText,
                &claim.id,
                None,
                "No authoritative claim classification available",
            ));
        }
        results.push(ClaimResult {
            id: claim.id.clone(),
            beat_id: claim.beat_id.clone(),
            statement: claim.statement.clone(),
            classification: claim.kind,
            status,
            exact_citations: approved,
            findings: issues,
            qualified_human_review_required: claim.consequence != Consequence::Ordinary,
        });
    }
    // No source-backed, reviewer-certified exhaustiveness record exists for
    // ALL factual assertions in narration. Even fully adjudicated annotations
    // must not be presented as whole-script publication approval.
    let publication_review_required = true;
    VerificationReport {
        project_id: ctx.project_id.clone(),
        study_id: ctx.study_id.clone(),
        draft_id: ctx.draft_id.clone(),
        draft_revision: ctx.draft_revision,
        checked_at_epoch: ctx.checked_at_epoch,
        claims: results,
        publication_review_required,
        inventory_complete: false,
        draft_content_bound: ctx.draft_content_bound,
        // Preserve the draft's own historical decisions, but never leak a
        // foreign project, Study or draft's reviewer data into this report.
        review_lineage: ctx
            .human_reviews
            .iter()
            .filter(|review| {
                review.project_id == ctx.project_id
                    && review.study_id == ctx.study_id
                    && review.draft_id == ctx.draft_id
            })
            .cloned()
            .collect(),
    }
}

#[cfg(test)]
mod tests;
