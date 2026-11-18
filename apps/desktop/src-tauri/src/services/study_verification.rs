//! Read-only, scoped bridge from canonical Study evidence into the verification
//! engine. No schema, DTO, provider or publication side effects are added.
use std::collections::{HashMap, HashSet};

use chrono::DateTime;
use scriptotar_ai::writing::{
    self,
    verification::{
        self, Citation, EvidenceRelation, EvidenceState, ExactLocator, Modality, ResolvedEvidence,
        SourceKind, VerificationContext, VerificationReport,
    },
};
use scriptotar_core::{EvidenceCertainty, RepositoryError, RepositoryResult, StudySourceKind};
use serde::Deserialize;
use uuid::Uuid;

use super::AppServices;

#[derive(Debug, Clone, PartialEq, Default)]
struct HostSourceBinding {
    /// Cryptographic digest of stored, project-scoped bytes; not the caller's
    /// source_revision, source fingerprint or provider's proposed citation.
    content_sha256: Option<String>,
    segments: Vec<CanonicalTranscriptSegment>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
struct CanonicalTranscriptSegment {
    /// An ID must exist in the stored segments JSON. Array offsets do not
    /// become invented segment IDs.
    id: Option<String>,
    start: f64,
    end: f64,
    text: String,
}

fn load_host_source_binding(
    services: &AppServices,
    project_id: Uuid,
    study_id: Uuid,
    source: &scriptotar_core::StudySource,
) -> RepositoryResult<HostSourceBinding> {
    let mut binding = HostSourceBinding::default();
    if !source.available {
        return Ok(binding);
    }
    match source.kind {
        StudySourceKind::Manual => {
            if let Some(text) = source.manual_text.as_deref() {
                binding.content_sha256 =
                    Some(verification::canonical_source_sha256(&[text.as_bytes()]));
            }
        }
        StudySourceKind::Transcript => {
            let Some(id) = source.reference_id else {
                return Ok(binding);
            };
            // Require a real, active, project-/Study-scoped transcript join
            // BEFORE reading the underlying bundle by its globally unique ID.
            let Ok(byte_length) = services
                .store
                .attached_transcript_byte_length(project_id, study_id, id)
            else {
                return Ok(binding);
            };
            if byte_length > 2_000_000 {
                return Ok(binding);
            }
            let bundle = services.store.get_transcript_bundle(id)?;
            if bundle.project_id != project_id || bundle.transcript.id != id {
                return Ok(binding);
            }
            let transcript = &bundle.transcript;
            let segments = transcript.segments_json.as_deref().unwrap_or("");
            let words = transcript.words_json.as_deref().unwrap_or("");
            if segments.len() > 2_000_000 || words.len() > 2_000_000 {
                return Ok(binding);
            }
            // Changes to the text OR segment/word evidence invalidate the
            // complete source content version without trusting an etag.
            binding.content_sha256 = Some(verification::canonical_source_sha256(&[
                transcript.text.as_bytes(),
                segments.as_bytes(),
                words.as_bytes(),
            ]));
            if let Ok(parsed) = serde_json::from_str::<Vec<CanonicalTranscriptSegment>>(segments) {
                if parsed.len() <= 20_000 {
                    let mut ids = HashSet::new();
                    if parsed.iter().all(|seg| {
                        seg.start.is_finite()
                            && seg.end.is_finite()
                            && seg.start >= 0.0
                            && seg.end >= seg.start
                            && seg.text.len() <= 8_000
                            && seg
                                .id
                                .as_deref()
                                .is_some_and(|id| !id.is_empty() && ids.insert(id.to_owned()))
                    }) {
                        binding.segments = parsed;
                    }
                }
            }
        }
        // Metadata, AI output and stored media locators are not independently
        // attested primary document content.
        _ => {}
    }
    Ok(binding)
}

fn verified_transcript_segment(
    evidence: &scriptotar_core::EvidenceReference,
    binding: &HostSourceBinding,
) -> Option<String> {
    let id = evidence.input.locator.strip_prefix("segment:")?;
    let seconds = evidence.input.timestamp_seconds?;
    if !seconds.is_finite() || seconds < 0.0 {
        return None;
    }
    binding
        .segments
        .iter()
        .find(|segment| {
            segment.id.as_deref() == Some(id)
                && seconds >= segment.start
                && seconds <= segment.end
                // Creator-entered observations are not verbatim evidence.
                // Only an exact match to stored segment text gets an ID.
                && evidence.input.claim == segment.text
        })
        .and_then(|segment| segment.id.clone())
}

fn collected_epoch(value: &str) -> Option<u64> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .and_then(|parsed| u64::try_from(parsed.timestamp()).ok())
}

/// The v9 table records creator observations and disputes, not authenticated
/// independent claim-level fact reviews. Everything mapped here is UNREVIEWED
/// (or unavailable/disputed), even if a legacy certainty field says otherwise.
fn from_study_evidence(
    evidence: &scriptotar_core::EvidenceReference,
    source: &scriptotar_core::StudySource,
    binding: &HostSourceBinding,
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
    let timestamp_millis = evidence.input.timestamp_seconds.and_then(|seconds| {
        (seconds.is_finite() && seconds >= 0.0).then_some((seconds * 1000.0) as u64)
    });
    ResolvedEvidence {
        evidence_id: evidence.id.to_string(),
        source_id: source.id.to_string(),
        project_id: evidence.project_id.to_string(),
        study_id: evidence.study_id.to_string(),
        // Derived from canonical stored bytes, NEVER source.source_revision.
        source_version: binding.content_sha256.clone(),
        locator: ExactLocator {
            locator: evidence.input.locator.clone(),
            // Only a real segment ID, exact quote and bounded timestamp
            // matched against stored transcript JSON can populate this.
            segment_id: if kind == SourceKind::SocialTranscript {
                verified_transcript_segment(evidence, binding)
            } else {
                None
            },
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
    #[allow(dead_code)] // Agent 00 owns production command wiring after review-ledger contract.
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
        // Pin the active project through the complete read. A caller cannot
        // verify claims under a different (otherwise existing) project.
        let project_guard = self
            .active_project
            .lock()
            .map_err(|_| RepositoryError::Storage("active project lock poisoned".into()))?;
        if *project_guard != project_id {
            return Err(RepositoryError::NotFound("project is not active".into()));
        }
        // Model-authored IDs are not sufficient provenance. The draft must be
        // a current canonical saved draft belonging to the exact Study.
        let draft_id = Uuid::parse_str(&draft.id)
            .map_err(|_| RepositoryError::Validation("draft ID is not canonical".into()))?;
        let saved_draft = self.store.get_draft(project_id, draft_id)?;
        if saved_draft.study_id != study_id
            || saved_draft.archived_at.is_some()
            || i64::try_from(draft_revision).ok() != Some(saved_draft.revision)
        {
            return Err(RepositoryError::Conflict(
                "draft Study membership or revision changed".into(),
            ));
        }
        let study = self.store.get_study(project_id, study_id)?;
        if study.archived_at.is_some() {
            return Err(RepositoryError::NotFound("Study is archived".into()));
        }
        if study.revision != expected_study_revision {
            return Err(RepositoryError::Conflict(
                "Study changed before verification".into(),
            ));
        }
        let sources = self.store.list_study_sources(project_id, study_id)?;
        let references = self.store.list_evidence_references(project_id, study_id)?;
        let source_map: HashMap<Uuid, _> = sources.iter().map(|src| (src.id, src)).collect();
        let mut source_bindings = HashMap::new();
        for source in &sources {
            source_bindings.insert(
                source.id,
                load_host_source_binding(self, project_id, study_id, source)?,
            );
        }
        let mut resolved = Vec::new();
        for evidence in &references {
            let source = source_map
                .get(&evidence.input.study_source_id)
                .ok_or_else(|| RepositoryError::Storage("orphan Study evidence source".into()))?;
            let binding = source_bindings
                .get(&source.id)
                .ok_or_else(|| RepositoryError::Storage("missing source content binding".into()))?;
            resolved.push(from_study_evidence(evidence, source, binding));
        }
        let fresh_study = self.store.get_study(project_id, study_id)?;
        let fresh_sources = self.store.list_study_sources(project_id, study_id)?;
        let fresh_references = self.store.list_evidence_references(project_id, study_id)?;
        let final_draft = self.store.get_draft(project_id, draft_id)?;
        let mut fresh_bindings = HashMap::new();
        for source in &fresh_sources {
            fresh_bindings.insert(
                source.id,
                load_host_source_binding(self, project_id, study_id, source)?,
            );
        }
        if fresh_study.revision != expected_study_revision
            || final_draft != saved_draft
            || fresh_sources != sources
            || fresh_references != references
            || fresh_bindings != source_bindings
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
            // The saved CreativeDraft body is currently free-form text, not
            // an immutable typed ScriptDraft/claim manifest. Matching only a
            // valid draft ID/revision must NEVER enable a Verified result.
            draft_content_bound: false,
        };
        // Resolve only declared FactualClaim IDs against real Study evidence.
        // Unknown IDs are deliberately retained as invalid citations so the
        // report records attempted forgery rather than silently dropping them.
        let known: HashMap<&str, &ResolvedEvidence> = ctx
            .evidence
            .iter()
            .map(|entry| (entry.evidence_id.as_str(), entry))
            .collect();
        let mut claims = verification::extract_draft_claims(draft);
        for (proposal, supplied) in claims.iter_mut().zip(draft.claims.iter()) {
            proposal.citations = supplied
                .evidence_ids
                .iter()
                .take(17)
                .map(|id| {
                    if let Some(entry) = known.get(id.as_str()) {
                        Citation {
                            evidence_id: id.clone(),
                            source_id: entry.source_id.clone(),
                            source_version: entry.source_version.clone().unwrap_or_default(),
                            locator: entry.locator.clone(),
                            quote: entry.excerpt.clone(),
                        }
                    } else {
                        Citation {
                            evidence_id: id.clone(),
                            source_id: String::new(),
                            source_version: String::new(),
                            locator: ExactLocator {
                                locator: String::new(),
                                segment_id: None,
                                timestamp_millis: None,
                                modality: Modality::DocumentText,
                            },
                            quote: String::new(),
                        }
                    }
                })
                .collect();
        }
        Ok(verification::verify_claims(&ctx, &claims))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use scriptotar_core::StudySourceKind;
    use scriptotar_core::{EvidenceInput, EvidenceReference, StudySource};

    #[test]
    fn creator_observation_never_becomes_independently_reviewed() {
        let project = Uuid::new_v4();
        let study = Uuid::new_v4();
        let attached = Uuid::new_v4();
        let source = StudySource {
            id: attached,
            project_id: project,
            study_id: study,
            kind: StudySourceKind::Transcript,
            reference_id: Some(Uuid::new_v4()),
            locator: "test".into(),
            manual_text: None,
            fingerprint: "identity-only".into(),
            source_revision: None,
            collected_at: "2026-11-16T00:00:00Z".into(),
            uncertainty: None,
            available: true,
            created_at: "2026-11-16T00:00:00Z".into(),
        };
        let evidence = EvidenceReference {
            id: Uuid::new_v4(),
            project_id: project,
            study_id: study,
            input: EvidenceInput {
                study_source_id: attached,
                claim: "looks verified".into(),
                locator: "user-entered segment 1".into(),
                timestamp_seconds: Some(2.5),
                certainty: EvidenceCertainty::Corroborated,
                uncertainty_note: None,
            },
            created_at: "2026-11-16T00:00:00Z".into(),
        };
        let mapped = from_study_evidence(&evidence, &source, &HostSourceBinding::default());
        assert_eq!(mapped.state, EvidenceState::Unreviewed);
        assert_eq!(mapped.relation, EvidenceRelation::Unassessed);
        assert!(mapped.independent_origin.is_none());
        assert!(mapped.source_version.is_none());
        assert!(mapped.locator.segment_id.is_none());
        assert_eq!(mapped.locator.timestamp_millis, Some(2500));
    }

    #[test]
    fn sqlite_adapter_rejects_foreign_study_and_stale_draft_revision() {
        use crate::dto::UiStudioBrief;
        use scriptotar_ai::writing::{BeatRole, ScriptBeat, ScriptDraft};
        use tempfile::TempDir;

        let tmp = TempDir::new().unwrap();
        let services = AppServices::new(tmp.path()).unwrap();
        let project_id = services.active_project_id().unwrap();
        let brief = UiStudioBrief {
            title: "Verification Study".into(),
            goal: "Verify statements".into(),
            audience: "Writers".into(),
            platform: "Web".into(),
            language: "en".into(),
            duration: "30".into(),
            style: "Precise".into(),
        };
        let first = services
            .create_studio_study(project_id, brief.clone())
            .unwrap();
        let first_id = Uuid::parse_str(&first.id).unwrap();
        let draft_revision = u64::try_from(first.draft_revision.unwrap()).unwrap();
        let candidate = ScriptDraft {
            id: first.draft_id.unwrap(),
            angle_id: "a".into(),
            title: "Draft".into(),
            language: "en".into(),
            beats: vec![ScriptBeat {
                id: "beat-1".into(),
                role: BeatRole::Hook,
                narration: "Unreviewed observation".into(),
                on_screen_text: None,
                shots: vec![],
                evidence_ids: vec![],
                manually_edited: false,
            }],
            cta: None,
            caption: None,
            claims: vec![],
        };

        let report = services
            .verify_studio_claims_read_only(
                project_id,
                first_id,
                first.revision,
                draft_revision,
                &candidate,
            )
            .unwrap();
        assert!(report.publication_review_required);
        assert!(!report.inventory_complete);
        assert!(!report.draft_content_bound);
        assert!(report
            .claims
            .iter()
            .all(|claim| { claim.status != verification::VerificationStatus::Verified }));

        // Matching a real project and draft ID is not enough: the Study itself
        // must be the same and the revisions must still be current.
        let other = services.create_studio_study(project_id, brief).unwrap();
        let other_id = Uuid::parse_str(&other.id).unwrap();
        assert!(services
            .verify_studio_claims_read_only(
                project_id,
                other_id,
                other.revision,
                draft_revision,
                &candidate,
            )
            .is_err());
        assert!(services
            .verify_studio_claims_read_only(
                project_id,
                first_id,
                first.revision + 1,
                draft_revision,
                &candidate,
            )
            .is_err());
        assert!(services
            .verify_studio_claims_read_only(
                project_id,
                first_id,
                first.revision,
                draft_revision + 1,
                &candidate,
            )
            .is_err());
        assert!(services
            .verify_studio_claims_read_only(
                Uuid::new_v4(),
                first_id,
                first.revision,
                draft_revision,
                &candidate,
            )
            .is_err());
    }
}

#[cfg(test)]
mod authenticity_tests {
    use super::*;
    use scriptotar_core::{EvidenceCertainty, EvidenceInput, EvidenceReference};

    fn observation(locator: &str, timestamp_seconds: f64, claim: &str) -> EvidenceReference {
        EvidenceReference {
            id: Uuid::new_v4(),
            project_id: Uuid::new_v4(),
            study_id: Uuid::new_v4(),
            input: EvidenceInput {
                study_source_id: Uuid::new_v4(),
                claim: claim.into(),
                locator: locator.into(),
                timestamp_seconds: Some(timestamp_seconds),
                certainty: EvidenceCertainty::Corroborated,
                uncertainty_note: None,
            },
            created_at: "2026-11-16T00:00:00Z".into(),
        }
    }

    #[test]
    fn sqlite_manual_content_binding_is_host_derived_not_caller_revision() {
        use scriptotar_core::StudySourceInput;
        use tempfile::TempDir;

        let tmp = TempDir::new().unwrap();
        let services = AppServices::new(tmp.path()).unwrap();
        let project = services.active_project_id().unwrap();
        let study = services
            .store
            .create_study(project, "Integrity", "Source binding")
            .unwrap();
        let text = "Persisted primary bytes, not an independently checked fact";
        let source = services
            .store
            .add_study_source(
                project,
                study.id,
                &StudySourceInput {
                    kind: StudySourceKind::Manual,
                    reference_id: None,
                    locator: Some("section 1".into()),
                    manual_text: Some(text.into()),
                    source_revision: Some("sha256:invented-by-caller".into()),
                    uncertainty: None,
                },
            )
            .unwrap();
        let binding = load_host_source_binding(&services, project, study.id, &source).unwrap();
        assert_eq!(
            binding.content_sha256,
            Some(verification::canonical_source_sha256(&[text.as_bytes()]))
        );
        assert_ne!(binding.content_sha256, source.source_revision);
        assert!(binding.segments.is_empty());
    }

    #[test]
    fn exact_stored_segment_id_time_and_quote_are_all_required() {
        let binding = HostSourceBinding {
            content_sha256: Some(verification::canonical_source_sha256(&[b"transcript"])),
            segments: vec![CanonicalTranscriptSegment {
                id: Some("real-17".into()),
                start: 10.0,
                end: 13.0,
                text: "The speaker said this".into(),
            }],
        };
        assert_eq!(
            verified_transcript_segment(
                &observation("segment:real-17", 12.5, "The speaker said this"),
                &binding
            ),
            Some("real-17".into())
        );
        assert!(verified_transcript_segment(
            &observation("segment:real-17", 14.0, "The speaker said this"),
            &binding
        )
        .is_none());
        assert!(verified_transcript_segment(
            &observation("segment:real-17", 12.5, "Altered words"),
            &binding
        )
        .is_none());
        assert!(verified_transcript_segment(
            &observation("segment:invented", 12.5, "The speaker said this"),
            &binding
        )
        .is_none());
    }
}
