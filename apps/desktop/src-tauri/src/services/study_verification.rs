//! Read-only, scoped bridge from canonical Study evidence into the verification
//! engine. No schema, DTO, provider or publication side effects are added.
use std::collections::HashMap;

use chrono::DateTime;
use scriptotar_ai::writing::{self, verification::{
    self, EvidenceRelation, EvidenceState, ExactLocator, Modality, ResolvedEvidence, SourceKind,
    VerificationContext, VerificationReport,
}};
use scriptotar_core::{EvidenceCertainty, RepositoryError, RepositoryResult, StudySourceKind};
use uuid::Uuid;

use super::AppServices;

fn collected_epoch(value: &str) -> Option<u64> {
    DateTime::parse_from_rfc3339(value).ok()
        .and_then(|parsed| u64::try_from(parsed.timestamp()).ok())
}

/// The v9 table records creator observations and disputes, not authenticated
/// independent claim-level fact reviews. Everything mapped here is UNREVIEWED
/// (or unavailable/disputed), even if a legacy certainty field says otherwise.
fn from_study_evidence(
    evidence: &scriptotar_core::EvidenceReference,
    source: &scriptotar_core::StudySource,
) -> ResolvedEvidence {
    let kind = match source.kind {
        StudySourceKind::Transcript => SourceKind::SocialTranscript,
        StudySourceKind::ResearchItem => SourceKind::ResearchMetadata,
        StudySourceKind::Manual => SourceKind::ManualNote,
        StudySourceKind::AiRun => SourceKind::AiGenerated,
        // Source/media/job origin is not independently authenticated document
        // evidence; require a separate adapter after provenance review.
        _ => SourceKind::ManualNote,
    };
    let modality = match kind {
        SourceKind::SocialTranscript => Modality::SpokenTranscript,
        SourceKind::ResearchMetadata => Modality::Metric,
        _ => Modality::DocumentText,
    };
    let state = if !source.available {
        EvidenceState::Unavailable
    } else if evidence.input.certainty == EvidenceCertainty::Disputed {
        EvidenceState::Disputed
    } else {
        EvidenceState::Unreviewed
    };
    let timestamp_millis = evidence.input.timestamp_seconds
        .and_then(|seconds| (seconds.is_finite() && seconds >= 0.0)
            .then(|| (seconds * 1000.0) as u64));
    ResolvedEvidence {
        evidence_id: evidence.id.to_string(),
        source_id: source.id.to_string(),
        project_id: evidence.project_id.to_string(),
        study_id: evidence.study_id.to_string(),
        // This is only a declared revision, not a verified content digest.
        // A missing revision deliberately cannot pass strict verification.
        source_version: source.source_revision.clone(),
        locator: ExactLocator {
            locator: evidence.input.locator.clone(),
            // User-entered locator strings are NOT authenticated transcript
            // segment IDs. Future segment adapter must cross-check actual JSON.
            segment_id: None,
            timestamp_millis,
            modality,
        },
        // The creator's observation is NOT a checked verbatim source quote.
        excerpt: evidence.input.claim.clone(),
        kind,
        state,
        relation: EvidenceRelation::Unassessed,
        independent_origin: None,
        collected_at_epoch: collected_epoch(&source.collected_at),
        expires_at_epoch: None,
    }
}

impl AppServices {
    /// Report uncertainty for an already scoped Study and candidate script.
    ///
    /// This is NOT publication authorization: evidence can change between
    /// reads, Study revisions are not draft versions, and current storage has
    /// no independently authenticated human fact-review records. Agent 00
    /// must add an atomic snapshot/attestation ledger before any Verified UI.
    pub fn verify_studio_claims_read_only(
        &self,
        project_id: Uuid,
        study_id: Uuid,
        expected_study_revision: i64,
        draft_revision: u64,
        draft: &writing::ScriptDraft,
    ) -> RepositoryResult<VerificationReport> {
        if draft_revision == 0 {
            return Err(RepositoryError::Validation("invalid draft revision".into()));
        }
        let study = self.store.get_study(project_id, study_id)?;
        if study.revision != expected_study_revision {
            return Err(RepositoryError::Conflict("Study changed before verification".into()));
        }
        let sources = self.store.list_study_sources(project_id, study_id)?;
        let references = self.store.list_evidence_references(project_id, study_id)?;
        let source_map: HashMap<Uuid, _> = sources.iter().map(|src| (src.id, src)).collect();
        let mut resolved = Vec::new();
        for evidence in &references {
            let source = source_map.get(&evidence.input.study_source_id)
                .ok_or_else(|| RepositoryError::Storage("orphan Study evidence source".into()))?;
            resolved.push(from_study_evidence(evidence, source));
        }
        let fresh_study = self.store.get_study(project_id, study_id)?;
        let fresh_sources = self.store.list_study_sources(project_id, study_id)?;
        let fresh_references = self.store.list_evidence_references(project_id, study_id)?;
        if fresh_study.revision != expected_study_revision
            || fresh_sources != sources || fresh_references != references
        {
            return Err(RepositoryError::Conflict(
                "Study/evidence changed during verification; retry with a fresh snapshot".into(),
            ));
        }
        let now = chrono::Utc::now().timestamp();
        let ctx = VerificationContext {
            project_id: project_id.to_string(),
            study_id: study_id.to_string(),
            draft_id: draft.id.clone(),
            draft_revision,
            checked_at_epoch: u64::try_from(now).unwrap_or_default(),
            evidence: resolved,
            // No authenticated independent review table exists in v9.
            human_reviews: Vec::new(),
        };
        let claims = verification::extract_draft_claims(draft);
        Ok(verification::verify_claims(&ctx, &claims))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use scriptotar_core::{EvidenceInput, EvidenceReference, StudySource};
    use scriptotar_core::StudySourceKind;

    #[test]
    fn creator_observation_never_becomes_independently_reviewed() {
        let project = Uuid::new_v4();
        let study = Uuid::new_v4();
        let attached = Uuid::new_v4();
        let source = StudySource {
            id: attached, project_id: project, study_id: study,
            kind: StudySourceKind::Transcript, reference_id: Some(Uuid::new_v4()),
            locator: "test".into(), manual_text: None,
            fingerprint: "identity-only".into(), source_revision: None,
            collected_at: "2026-11-16T00:00:00Z".into(), uncertainty: None,
            available: true, created_at: "2026-11-16T00:00:00Z".into(),
        };
        let evidence = EvidenceReference {
            id: Uuid::new_v4(), project_id: project, study_id: study,
            input: EvidenceInput {
                study_source_id: attached, claim: "looks verified".into(),
                locator: "user-entered segment 1".into(), timestamp_seconds: Some(2.5),
                certainty: EvidenceCertainty::Corroborated,
                uncertainty_note: None,
            },
            created_at: "2026-11-16T00:00:00Z".into(),
        };
        let mapped = from_study_evidence(&evidence, &source);
        assert_eq!(mapped.state, EvidenceState::Unreviewed);
        assert_eq!(mapped.relation, EvidenceRelation::Unassessed);
        assert!(mapped.independent_origin.is_none());
        assert!(mapped.source_version.is_none());
        assert!(mapped.locator.segment_id.is_none());
        assert_eq!(mapped.locator.timestamp_millis, Some(2500));
    }
}
