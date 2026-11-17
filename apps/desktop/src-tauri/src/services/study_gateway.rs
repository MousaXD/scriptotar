//! Project-scoped adapters from canonical SQLite Content Studies to Studio UI.
use super::AppServices;
use crate::dto::{
    UiStudioAngleOption, UiStudioBrief, UiStudioDraftSummary, UiStudioDraftVersion,
    UiStudioSourceRef, UiStudioStudy, UiStudioStudyPatch, UiStudioStudySummary,
    UiStudyAngleInterpretation, UiStudyAnglePrompt, UiStudyEvidence, UiStudyEvidenceInput,
    UiStudyEvidenceReview, UiStudyScriptInput, UiStudyScriptReview,
};
use scriptotar_ai::writing::{
    self, ClaimStatus, CreativeBrief as WritingBrief, EvidenceKind as WritingEvidenceKind,
    EvidenceLocator as WritingLocator, EvidenceRef as WritingEvidence,
    EvidenceStatus as WritingEvidenceStatus, WritingInput, WritingTask,
};
use scriptotar_core::{
    CreativeBriefInput, EvidenceCertainty, EvidenceInput, EvidenceReference, RepositoryError,
    RepositoryResult, StudyEdit, StudySource, StudySourceInput, StudySourceKind,
};
use std::collections::HashSet;
use uuid::Uuid;

fn invalid(label: &str) -> RepositoryError {
    RepositoryError::Validation(format!("invalid {label}"))
}

fn brief_input(value: &UiStudioBrief) -> RepositoryResult<CreativeBriefInput> {
    let seconds = value
        .duration
        .trim()
        .parse::<u32>()
        .map_err(|_| invalid("duration"))?;
    if seconds == 0 || seconds > 3600 {
        return Err(invalid("duration"));
    }
    Ok(CreativeBriefInput {
        audience: value.audience.clone(),
        goal: value.goal.clone(),
        language: value.language.clone(),
        region: None,
        platform: value.platform.clone(),
        tone: value.style.clone(),
        duration_seconds: Some(seconds),
        objective: value.goal.clone(),
        cta: None,
        constraints: Vec::new(),
    })
}

fn source_input(source: &UiStudioSourceRef) -> RepositoryResult<StudySourceInput> {
    let kind = match source.kind.as_str() {
        "research" => StudySourceKind::ResearchItem,
        "transcript" => StudySourceKind::Transcript,
        _ => return Err(invalid("source kind")),
    };
    let id = Uuid::parse_str(&source.id).map_err(|_| invalid("source id"))?;
    Ok(StudySourceInput {
        kind,
        reference_id: Some(id),
        locator: None,
        manual_text: None,
        source_revision: None,
        uncertainty: None,
    })
}

fn ui_source(source: &scriptotar_core::StudySource) -> Option<UiStudioSourceRef> {
    if !source.available {
        return None;
    }
    let kind = match source.kind {
        StudySourceKind::ResearchItem => "research",
        StudySourceKind::Transcript => "transcript",
        _ => return None,
    };
    source.reference_id.map(|id| UiStudioSourceRef {
        kind: kind.to_owned(),
        id: id.to_string(),
    })
}

/// Evidence retains stable source identity when the origin becomes unavailable.
fn ui_evidence(
    value: EvidenceReference,
    source: &StudySource,
    reviews: Vec<scriptotar_db::EvidenceReview>,
) -> UiStudyEvidence {
    let origin = match (source.kind, source.reference_id) {
        (StudySourceKind::ResearchItem, Some(id)) => Some(UiStudioSourceRef {
            kind: "research".into(),
            id: id.to_string(),
        }),
        (StudySourceKind::Transcript, Some(id)) => Some(UiStudioSourceRef {
            kind: "transcript".into(),
            id: id.to_string(),
        }),
        _ => None,
    };
    UiStudyEvidence {
        id: value.id.to_string(),
        project_id: value.project_id.to_string(),
        study_id: value.study_id.to_string(),
        source: origin,
        source_label: source.locator.clone(),
        source_available: source.available,
        claim: value.input.claim,
        locator: value.input.locator,
        timestamp_seconds: value.input.timestamp_seconds,
        certainty: value.input.certainty.as_str().into(),
        uncertainty_note: value.input.uncertainty_note,
        created_at: value.created_at,
        review_history: reviews
            .into_iter()
            .map(|review| UiStudyEvidenceReview {
                id: review.id.to_string(),
                previous_certainty: review.previous_certainty,
                next_certainty: review.next_certainty,
                note: review.reason,
                reviewed_at: review.reviewed_at,
            })
            .collect(),
    }
}

/// Presentation preserves every model-proposed claim's uncertainty and locator.
fn render_reviewed_script(script: &writing::ScriptDraft) -> String {
    let mut parts = vec![
        format!("# {}", script.title.trim()),
        format!("Creative angle ID: {}", script.angle_id),
    ];
    for beat in &script.beats {
        parts.push(format!("[{:?}]\n{}", beat.role, beat.narration.trim()));
        if let Some(text) = &beat.on_screen_text {
            parts.push(format!("On screen: {}", text.trim()));
        }
        for shot in &beat.shots {
            parts.push(format!(
                "Shot suggestion: {} — {}",
                shot.visual.trim(),
                shot.purpose.trim()
            ));
        }
        if !beat.evidence_ids.is_empty() {
            parts.push(format!(
                "Referenced source IDs (not fact-checked): {}",
                beat.evidence_ids.join(", ")
            ));
        }
    }
    if let Some(cta) = &script.cta {
        parts.push(format!("CTA: {}", cta.trim()));
    }
    if let Some(caption) = &script.caption {
        parts.push(format!("Caption: {}", caption.trim()));
    }
    if !script.claims.is_empty() {
        parts.push("Model-proposed claims — independently verify before publication:".into());
        for claim in &script.claims {
            parts.push(format!(
                "[{:?}] {} — source IDs: {}",
                claim.status,
                claim.assertion.trim(),
                if claim.evidence_ids.is_empty() {
                    "none".into()
                } else {
                    claim.evidence_ids.join(", ")
                }
            ));
        }
    }
    parts.join("\n\n")
}

/// A list row must never hydrate draft bodies, revisions or source transcripts.
fn summarize_studio_brief(
    study: &scriptotar_core::ContentStudy,
    saved: Option<&scriptotar_core::CreativeBrief>,
) -> UiStudioBrief {
    if let Some(value) = saved {
        UiStudioBrief {
            title: study.title.clone(),
            goal: value.input.goal.clone(),
            audience: value.input.audience.clone(),
            platform: value.input.platform.clone(),
            language: value.input.language.clone(),
            duration: value
                .input
                .duration_seconds
                .map_or(String::new(), |v| v.to_string()),
            style: value.input.tone.clone(),
        }
    } else {
        UiStudioBrief {
            title: study.title.clone(),
            goal: study.goal.clone(),
            audience: String::new(),
            platform: String::new(),
            language: String::new(),
            duration: "40".into(),
            style: String::new(),
        }
    }
}

impl AppServices {
    fn lock_studio_project(
        &self,
        project_id: Uuid,
    ) -> RepositoryResult<std::sync::MutexGuard<'_, Uuid>> {
        let guard = self
            .active_project
            .lock()
            .map_err(|_| RepositoryError::Storage("active project lock poisoned".into()))?;
        if *guard != project_id {
            return Err(RepositoryError::NotFound("project is not active".into()));
        }
        Ok(guard)
    }

    fn read_studio_bundle(
        &self,
        project_id: Uuid,
        study_id: Uuid,
    ) -> RepositoryResult<UiStudioStudy> {
        self.read_studio_bundle_for(project_id, study_id, None)
    }

    fn read_studio_bundle_for(
        &self,
        project_id: Uuid,
        study_id: Uuid,
        requested_draft_id: Option<Uuid>,
    ) -> RepositoryResult<UiStudioStudy> {
        let study = self.store.get_study(project_id, study_id)?;
        let saved_brief = self.store.get_creative_brief(project_id, study_id)?;
        let brief = summarize_studio_brief(&study, saved_brief.as_ref());
        let sources = self
            .store
            .list_study_sources(project_id, study_id)?
            .iter()
            .filter_map(ui_source)
            .collect();
        let drafts = self.store.list_draft_summaries(project_id, study_id, 200)?;
        let archived_drafts = self
            .store
            .list_archived_draft_summaries(project_id, study_id, 200)?;
        let selected =
            if let Some(id) = requested_draft_id {
                Some(drafts.iter().find(|item| item.id == id).ok_or_else(|| {
                    RepositoryError::NotFound("selected draft in this Study".into())
                })?)
            } else {
                drafts.first()
            };
        let (draft, draft_versions) = if let Some(value) = selected {
            let full = self.store.get_draft(project_id, value.id)?;
            if full.study_id != study_id
                || full.archived_at.is_some()
                || full.revision != value.revision
            {
                return Err(RepositoryError::Conflict(
                    "selected draft changed during metadata retrieval".into(),
                ));
            }
            let versions = self
                .store
                .list_draft_versions(project_id, value.id)?
                .into_iter()
                .map(|v| UiStudioDraftVersion {
                    id: v.id.to_string(),
                    revision: v.version,
                    content: v.body,
                    created_at: v.saved_at,
                })
                .collect();
            (full.body, versions)
        } else {
            (String::new(), Vec::new())
        };
        Ok(UiStudioStudy {
            id: study_id.to_string(),
            project_id: project_id.to_string(),
            revision: study.revision,
            brief,
            sources,
            draft,
            draft_versions,
            draft_id: selected.map(|item| item.id.to_string()),
            draft_revision: selected.map(|item| item.revision),
            drafts: drafts
                .iter()
                .map(|item| UiStudioDraftSummary {
                    id: item.id.to_string(),
                    title: item.title.clone(),
                    revision: item.revision,
                })
                .collect(),
            archived_drafts: archived_drafts
                .iter()
                .map(|item| UiStudioDraftSummary {
                    id: item.id.to_string(),
                    title: item.title.clone(),
                    revision: item.revision,
                })
                .collect(),
        })
    }

    pub fn list_studio_studies(
        &self,
        project_id: Uuid,
    ) -> RepositoryResult<Vec<UiStudioStudySummary>> {
        let _project_guard = self.lock_studio_project(project_id)?;
        self.store
            .search_studies(project_id, "", false, 200)?
            .into_iter()
            .map(|study| {
                // Listing is metadata-only: defer draft bodies, full version
                // history and source provenance until get_studio_study(id).
                let saved_brief = self.store.get_creative_brief(project_id, study.id)?;
                Ok(UiStudioStudySummary {
                    id: study.id.to_string(),
                    project_id: study.project_id.to_string(),
                    revision: study.revision,
                    brief: summarize_studio_brief(&study, saved_brief.as_ref()),
                })
            })
            .collect()
    }

    pub fn get_studio_study(
        &self,
        project_id: Uuid,
        study_id: Uuid,
    ) -> RepositoryResult<UiStudioStudy> {
        let _project_guard = self.lock_studio_project(project_id)?;
        self.read_studio_bundle(project_id, study_id)
    }

    pub fn get_studio_study_draft(
        &self,
        project_id: Uuid,
        study_id: Uuid,
        draft_id: Uuid,
    ) -> RepositoryResult<UiStudioStudy> {
        let _project_guard = self.lock_studio_project(project_id)?;
        self.read_studio_bundle_for(project_id, study_id, Some(draft_id))
    }

    pub fn duplicate_studio_draft(
        &self,
        project_id: Uuid,
        study_id: Uuid,
        draft_id: Uuid,
        expected_draft_revision: i64,
    ) -> RepositoryResult<UiStudioStudy> {
        let _project_guard = self.lock_studio_project(project_id)?;
        let copied = self.store.duplicate_study_draft_checked(
            project_id,
            study_id,
            draft_id,
            expected_draft_revision,
        )?;
        self.read_studio_bundle_for(project_id, study_id, Some(copied.id))
    }

    pub fn rename_studio_draft(
        &self,
        project_id: Uuid,
        study_id: Uuid,
        draft_id: Uuid,
        expected_draft_revision: i64,
        title: &str,
    ) -> RepositoryResult<UiStudioStudy> {
        let _project_guard = self.lock_studio_project(project_id)?;
        let draft = self.store.get_draft(project_id, draft_id)?;
        if draft.study_id != study_id || draft.archived_at.is_some() {
            return Err(RepositoryError::NotFound(
                "draft in this active Study".into(),
            ));
        }
        let renamed =
            self.store
                .rename_draft(project_id, draft_id, expected_draft_revision, title)?;
        self.read_studio_bundle_for(project_id, study_id, Some(renamed.id))
    }

    pub fn set_studio_draft_archived(
        &self,
        project_id: Uuid,
        study_id: Uuid,
        draft_id: Uuid,
        expected_draft_revision: i64,
        archived: bool,
    ) -> RepositoryResult<UiStudioStudy> {
        let _project_guard = self.lock_studio_project(project_id)?;
        self.store.set_study_draft_archived_checked(
            project_id,
            study_id,
            draft_id,
            expected_draft_revision,
            archived,
        )?;
        if archived {
            self.read_studio_bundle(project_id, study_id)
        } else {
            self.read_studio_bundle_for(project_id, study_id, Some(draft_id))
        }
    }

    /// Private production entrypoint: reconstruct canonical request material
    /// directly from the active Study, never from a browser-supplied context.
    /// The returned plan is offline; no paid call or Tauri IPC is registered.
    #[allow(dead_code)]
    pub(super) fn prepare_three_writing_plan(
        &self,
        project_id: Uuid,
        study_id: Uuid,
        run_id: &str,
    ) -> super::study_writing::WriteResult<super::study_writing::GenerationPlan> {
        use super::study_writing::CurrentStudy;
        let current = CurrentStudy::load(self, project_id, study_id)?;
        super::study_writing::prepare_three(run_id, current)
    }

    /// Production service orchestration seam: canonical CurrentStudy +
    /// four independently authorized provider steps + atomic v11 persistence.
    /// The caller-supplied boundary MUST enforce signed approval, exact
    /// provider bytes and the v10 durable effect claim for EACH step.
    /// A caller cannot substitute browser-provided source data: all fresh
    /// dispatches recheck plan/context against this AppServices instance.
    /// This is deliberately private until a certified provider, pricing
    /// and complete four-step consent lifecycle are registered in Tauri.
    #[allow(dead_code)]
    pub(super) fn generate_three_writing_with_boundary<B, P, S>(
        &self,
        plan: &super::study_writing::GenerationPlan,
        boundary: &mut B,
        provider: &mut P,
        stop: &S,
    ) -> super::study_writing::WriteResult<[Uuid; 3]>
    where
        B: super::study_writing::DurablePermissionBoundary,
        P: super::study_writing::WritingTextProvider,
        S: super::study_writing::StopSignal,
    {
        let mut sink = super::study_writing_persistence::CanonicalSqliteCandidateStore {
            store: &self.store,
            current: self,
        };
        super::study_writing::generate_three(plan, self, boundary, provider, stop, &mut sink)
    }

    /// Private production v11 sink: three new drafts, immutable versions,
    /// canonical source links and one receipt in a single SQLite transaction.
    /// This MUST NOT be surfaced as a client-controlled direct-save command.
    /// A future caller must prove consent/effect/receipt identity first.
    #[allow(dead_code)]
    pub(super) fn persist_three_writing_candidates(
        &self,
        batch: &super::study_writing::GeneratedBatch,
    ) -> super::study_writing::WriteResult<[Uuid; 3]> {
        use super::study_writing::AtomicCandidateStore;
        let mut sink = super::study_writing_persistence::CanonicalSqliteCandidateStore {
            store: &self.store,
            current: self,
        };
        sink.commit_three(batch)
    }

    /// P1 writing bridge: scoped, bounded, no provider call and no side effects.
    /// Real transcript excerpts are attributed, not fact-checked. User notes
    /// are unverified, and disputed observations never enter the prompt.
    fn scoped_writing_input(
        &self,
        project_id: Uuid,
        study_id: Uuid,
    ) -> RepositoryResult<(WritingInput, Vec<String>, usize, usize, usize)> {
        let _project_guard = self.lock_studio_project(project_id)?;
        self.scoped_writing_input_locked(project_id, study_id)
    }

    /// Caller must hold the project lock throughout validation and any write.
    pub(super) fn scoped_writing_input_locked(
        &self,
        project_id: Uuid,
        study_id: Uuid,
    ) -> RepositoryResult<(WritingInput, Vec<String>, usize, usize, usize)> {
        // Only the persisted brief is needed here; do not hydrate historical
        // draft bodies/versions just to prepare a local writing request.
        let study = self.store.get_study(project_id, study_id)?;
        let saved_brief = self.store.get_creative_brief(project_id, study_id)?;
        let brief = summarize_studio_brief(&study, saved_brief.as_ref());
        let saved_sources = self.store.list_study_sources(project_id, study_id)?;
        let mut limitations = Vec::new();
        let active: Vec<_> = saved_sources
            .iter()
            .filter(|source| source.available)
            .collect();
        if active.len() < 3 {
            limitations.push("Fewer than three available Study sources were selected.".into());
        }
        if active.len() > 8 {
            limitations.push("Only the first eight selected sources were inspected.".into());
        }
        if saved_sources.len() != active.len() {
            limitations.push("Unavailable sources were omitted.".into());
        }
        if active
            .iter()
            .take(8)
            .any(|source| source.kind != StudySourceKind::Transcript)
        {
            limitations.push("Research metadata is not proof of spoken or visual claims.".into());
        }
        let mut evidence = Vec::new();
        let mut excerpt_count = 0;
        for source in active.iter().take(8) {
            if source.kind != StudySourceKind::Transcript {
                continue;
            }
            let Some(transcript_id) = source.reference_id else {
                limitations.push("Transcript source has no durable reference ID.".into());
                continue;
            };
            let transcript = self.store.get_transcript_bundle(transcript_id)?;
            if transcript.project_id != project_id {
                return Err(RepositoryError::NotFound(
                    "Study transcript in this project".into(),
                ));
            }
            let segments = super::parse_segments(transcript.transcript.segments_json.as_deref());
            let mut selected = Vec::new();
            if segments.is_empty() {
                let sample: String = transcript.transcript.text.chars().take(960).collect();
                if !sample.trim().is_empty() {
                    evidence.push(WritingEvidence {
                        id: format!("transcript:{transcript_id}:text"),
                        locator: WritingLocator {
                            source_id: transcript_id.to_string(),
                            canonical_url: None,
                            timestamp_seconds: None,
                            document_section: Some("Untimed transcript beginning".into()),
                            collected_at: Some(transcript.transcript.created_at.clone()),
                        },
                        excerpt: sample,
                        kind: WritingEvidenceKind::SocialTranscript,
                        status: WritingEvidenceStatus::Attributed,
                    });
                    excerpt_count += 1;
                    limitations.push("One transcript lacked segment timecodes; its opening text is not timestamped.".into());
                }
                continue;
            }
            for index in [0, segments.len() / 2, segments.len() - 1] {
                if !selected.contains(&index) {
                    selected.push(index);
                }
            }
            for index in selected {
                let segment = &segments[index];
                let excerpt: String = segment.text.chars().take(960).collect();
                if excerpt.trim().is_empty() {
                    continue;
                }
                let seconds = segment.start_seconds;
                let timestamp_seconds =
                    (seconds.is_finite() && seconds >= 0.0 && seconds <= f64::from(u32::MAX))
                        .then_some(seconds as u32);
                let locator: String = format!(
                    "Transcript segment {} ({:.3}-{:.3}s)",
                    segment.id, segment.start_seconds, segment.end_seconds
                )
                .chars()
                .take(250)
                .collect();
                evidence.push(WritingEvidence {
                    id: format!("transcript:{transcript_id}:{index}"),
                    locator: WritingLocator {
                        source_id: transcript_id.to_string(),
                        canonical_url: None,
                        timestamp_seconds,
                        document_section: Some(locator),
                        collected_at: Some(transcript.transcript.created_at.clone()),
                    },
                    excerpt,
                    kind: WritingEvidenceKind::SocialTranscript,
                    status: WritingEvidenceStatus::Attributed,
                });
                excerpt_count += 1;
            }
        }
        let attached: HashSet<Uuid> = active.iter().take(8).map(|source| source.id).collect();
        let observations = self.store.list_evidence_references(project_id, study_id)?;
        let mut note_count = 0;
        for item in observations {
            if item.input.certainty == EvidenceCertainty::Disputed
                || !attached.contains(&item.input.study_source_id)
                || evidence.len() >= 40
            {
                continue;
            }
            let note: String = item.input.claim.chars().take(960).collect();
            evidence.push(WritingEvidence {
                id: format!("observation:{}", item.id),
                locator: WritingLocator {
                    source_id: item.input.study_source_id.to_string(),
                    canonical_url: None,
                    timestamp_seconds: item.input.timestamp_seconds.and_then(|value| {
                        (value.is_finite() && value >= 0.0 && value <= f64::from(u32::MAX))
                            .then_some(value as u32)
                    }),
                    document_section: Some(item.input.locator.chars().take(500).collect()),
                    collected_at: Some(item.created_at),
                },
                excerpt: format!("Unverified user observation: {note}"),
                kind: WritingEvidenceKind::CreativeObservation,
                status: WritingEvidenceStatus::Unverified,
            });
            note_count += 1;
        }
        if evidence.is_empty() {
            limitations.push(
                "No attributable transcript excerpts or active observations were available.".into(),
            );
        }
        limitations.push(
            "Transcript excerpts are not independently checked facts; no frames, audience retention or viral performance were analyzed.".into(),
        );
        let duration_seconds = brief
            .duration
            .parse::<u16>()
            .map_err(|_| invalid("writing duration"))?;
        let input = WritingInput {
            brief: WritingBrief {
                goal: brief.goal,
                audience: brief.audience,
                platform: brief.platform,
                language: brief.language,
                region: None,
                dialect: None,
                tone: Some(brief.style),
                duration_seconds,
                objective: "Original short-form creative angles".into(),
                desired_cta: None,
                constraints: vec![
                    "Do not present source text or user observations as verified facts.".into(),
                ],
                style_guide: None,
            },
            evidence,
            findings: Vec::new(),
            task: WritingTask::ProposeAngles { count: 3 },
        };
        Ok((input, limitations, active.len(), excerpt_count, note_count))
    }

    pub fn prepare_studio_angle_prompt(
        &self,
        project_id: Uuid,
        study_id: Uuid,
    ) -> RepositoryResult<UiStudyAnglePrompt> {
        let (input, limitations, source_count, excerpt_count, note_count) =
            self.scoped_writing_input(project_id, study_id)?;
        let prompt = writing::prepare_writing_request(&input)
            .map_err(|err| RepositoryError::Validation(format!("writing request: {err}")))?
            .prompt;
        Ok(UiStudyAnglePrompt {
            prompt,
            attached_source_count: source_count,
            transcript_excerpt_count: excerpt_count,
            unverified_observation_count: note_count,
            limitations,
        })
    }

    /// Validate a pasted model response against *current* attached evidence.
    /// The exact prepared prompt is required so changing a review without
    /// bumping the Study version invalidates old proposals, too. No egress.
    pub fn interpret_studio_angles(
        &self,
        project_id: Uuid,
        study_id: Uuid,
        expected_study_revision: i64,
        prepared_prompt: &str,
        response_json: &str,
        selected_angle_id: Option<&str>,
    ) -> RepositoryResult<UiStudyAngleInterpretation> {
        let _project_guard = self.lock_studio_project(project_id)?;
        self.interpret_studio_angles_locked(
            project_id,
            study_id,
            expected_study_revision,
            prepared_prompt,
            response_json,
            selected_angle_id,
        )
    }

    /// This locked helper is also used by script acceptance so the evidence
    /// checked for an angle cannot change before a candidate is committed.
    fn interpret_studio_angles_locked(
        &self,
        project_id: Uuid,
        study_id: Uuid,
        expected_study_revision: i64,
        prepared_prompt: &str,
        response_json: &str,
        selected_angle_id: Option<&str>,
    ) -> RepositoryResult<UiStudyAngleInterpretation> {
        if prepared_prompt.len() > 100_000 || response_json.len() > 100_000 {
            return Err(invalid("angle payload size"));
        }
        let (mut input, limitations, _, _, _) =
            self.scoped_writing_input_locked(project_id, study_id)?;
        if self.store.get_study(project_id, study_id)?.revision != expected_study_revision {
            return Err(RepositoryError::Conflict(
                "Study changed since angle preparation".into(),
            ));
        }
        let current_prompt = writing::prepare_writing_request(&input)
            .map_err(|err| RepositoryError::Validation(format!("writing request: {err}")))?
            .prompt;
        if current_prompt != prepared_prompt {
            return Err(RepositoryError::Conflict(
                "Study evidence or brief changed since angle preparation".into(),
            ));
        }
        let angles = writing::parse_angles(response_json, &input)
            .map_err(|err| RepositoryError::Validation(format!("invalid angle response: {err}")))?;
        let draft_prompt = if let Some(id) = selected_angle_id {
            let angle = angles
                .iter()
                .find(|angle| angle.id == id)
                .ok_or_else(|| invalid("selected angle ID"))?
                .clone();
            input.task = WritingTask::Draft { angle };
            Some(
                writing::prepare_writing_request(&input)
                    .map_err(|err| RepositoryError::Validation(format!("draft request: {err}")))?
                    .prompt,
            )
        } else {
            None
        };
        let angles = angles
            .into_iter()
            .map(|angle| UiStudioAngleOption {
                id: angle.id,
                approach: angle.approach,
                hook: angle.hook,
                promise: angle.promise,
                estimated_seconds: angle
                    .outline
                    .iter()
                    .map(|beat| u32::from(beat.estimated_seconds))
                    .sum(),
                outline: angle.outline.into_iter().map(|beat| beat.intent).collect(),
                evidence_ids: angle.evidence_ids,
            })
            .collect();
        Ok(UiStudyAngleInterpretation {
            angles,
            draft_prompt,
            limitations,
        })
    }

    /// Parse a pasted ScriptDraft against exact Study context and the selected angle.
    /// Preview is read-only; acceptance creates a separate draft, never overwriting
    /// the human editor. This method makes no AI provider request.
    pub fn review_studio_script(
        &self,
        project_id: Uuid,
        study_id: Uuid,
        request: &UiStudyScriptInput,
    ) -> RepositoryResult<UiStudyScriptReview> {
        // Hold the same project lock for re-deriving source/evidence context,
        // parsing the model response and the final atomic SQLite write. A
        // concurrent review or edit must not invalidate the checked prompt.
        let _project_guard = self.lock_studio_project(project_id)?;
        let expected_study_revision = request.expected_study_revision;
        let angle_prompt = request.angle_prompt.as_str();
        let angles_response_json = request.angles_response_json.as_str();
        let selected_angle_id = request.selected_angle_id.as_str();
        let prepared_draft_prompt = request.prepared_draft_prompt.as_str();
        let script_response_json = request.script_response_json.as_str();
        let accept_as_new_candidate = request.accept_as_new_candidate;
        if angle_prompt.len() > 100_000
            || angles_response_json.len() > 100_000
            || prepared_draft_prompt.len() > 100_000
            || script_response_json.len() > 100_000
        {
            return Err(invalid("script payload size"));
        }
        let reviewed = self.interpret_studio_angles_locked(
            project_id,
            study_id,
            expected_study_revision,
            angle_prompt,
            angles_response_json,
            Some(selected_angle_id),
        )?;
        if reviewed.draft_prompt.as_deref() != Some(prepared_draft_prompt) {
            return Err(RepositoryError::Conflict(
                "selected script prompt changed since preparation".into(),
            ));
        }
        let (mut input, mut limitations, _, _, _) =
            self.scoped_writing_input_locked(project_id, study_id)?;
        let angles = writing::parse_angles(angles_response_json, &input)
            .map_err(|err| RepositoryError::Validation(format!("invalid angles: {err}")))?;
        let selected = angles
            .into_iter()
            .find(|angle| angle.id == selected_angle_id)
            .ok_or_else(|| invalid("selected angle"))?;
        input.task = WritingTask::Draft { angle: selected };
        let script = writing::parse_draft(script_response_json, &input)
            .map_err(|err| RepositoryError::Validation(format!("invalid script: {err}")))?;
        let body = render_reviewed_script(&script);
        let title: String = script.title.trim().chars().take(200).collect();
        let needs_fact_review_count = script
            .claims
            .iter()
            .filter(|claim| claim.status != ClaimStatus::Opinion)
            .count();
        if needs_fact_review_count > 0 {
            limitations.push(
                "Model-proposed factual claims require independent review before publication."
                    .into(),
            );
        }
        limitations.push(
            "Valid JSON and matching source IDs do not establish factual accuracy or originality."
                .into(),
        );
        let attached = self.store.list_study_sources(project_id, study_id)?;
        let source_ids: Vec<Uuid> = attached
            .iter()
            .filter(|source| source.available)
            .take(8)
            .map(|source| source.id)
            .collect();
        let saved_study = if accept_as_new_candidate {
            let saved = self.store.create_reviewed_study_draft(
                project_id,
                study_id,
                expected_study_revision,
                &title,
                &body,
                &source_ids,
            )?;
            Some(self.read_studio_bundle_for(project_id, study_id, Some(saved.id))?)
        } else {
            None
        };
        Ok(UiStudyScriptReview {
            title,
            body,
            angle_id: script.angle_id,
            referenced_source_count: source_ids.len(),
            claim_count: script.claims.len(),
            needs_fact_review_count,
            limitations,
            saved_study,
        })
    }

    pub fn create_studio_study(
        &self,
        project_id: Uuid,
        brief: UiStudioBrief,
    ) -> RepositoryResult<UiStudioStudy> {
        let _project_guard = self.lock_studio_project(project_id)?;
        let input = brief_input(&brief)?;
        let study_id =
            self.store
                .create_studio_bundle(project_id, &brief.title, &brief.goal, &input)?;
        self.read_studio_bundle(project_id, study_id)
    }

    /// Read evidence scoped to the active project and canonical Study.
    pub fn list_studio_evidence(
        &self,
        project_id: Uuid,
        study_id: Uuid,
    ) -> RepositoryResult<Vec<UiStudyEvidence>> {
        let _project_guard = self.lock_studio_project(project_id)?;
        let sources = self.store.list_study_sources(project_id, study_id)?;
        let references = self.store.list_evidence_references(project_id, study_id)?;
        references
            .into_iter()
            .map(|reference| {
                let source = sources
                    .iter()
                    .find(|item| item.id == reference.input.study_source_id)
                    .ok_or_else(|| RepositoryError::Storage("evidence source missing".into()))?;
                let reviews =
                    self.store
                        .list_evidence_reviews(project_id, study_id, reference.id)?;
                Ok(ui_evidence(reference, source, reviews))
            })
            .collect()
    }

    /// User-created observations are never automatically promoted to verified.
    /// Only an origin already attached to an active Study can receive evidence.
    pub fn add_studio_evidence(
        &self,
        project_id: Uuid,
        study_id: Uuid,
        value: UiStudyEvidenceInput,
    ) -> RepositoryResult<UiStudyEvidence> {
        let _project_guard = self.lock_studio_project(project_id)?;
        let requested = source_input(&value.source)?;
        let reference_id = requested
            .reference_id
            .ok_or_else(|| invalid("evidence source"))?;
        let source = self
            .store
            .list_study_sources(project_id, study_id)?
            .into_iter()
            .find(|source| {
                source.kind == requested.kind
                    && source.reference_id == Some(reference_id)
                    && source.available
            })
            .ok_or_else(|| RepositoryError::NotFound("attached evidence source".into()))?;
        let reference = self.store.add_evidence_reference(
            project_id,
            study_id,
            &EvidenceInput {
                study_source_id: source.id,
                claim: value.claim,
                locator: value.locator,
                timestamp_seconds: value.timestamp_seconds,
                certainty: EvidenceCertainty::Unverified,
                uncertainty_note: value.uncertainty_note,
            },
        )?;
        let reviews = self
            .store
            .list_evidence_reviews(project_id, study_id, reference.id)?;
        Ok(ui_evidence(reference, &source, reviews))
    }

    /// Human review never certifies a claim as corroborated. Both transitions
    /// require an explicit reason and a stale-state compare-and-swap.
    pub fn review_studio_evidence(
        &self,
        project_id: Uuid,
        study_id: Uuid,
        evidence_id: Uuid,
        expected_certainty: &str,
        action: &str,
        note: &str,
    ) -> RepositoryResult<UiStudyEvidence> {
        let _project_guard = self.lock_studio_project(project_id)?;
        let (expected, next) = match (expected_certainty, action) {
            ("unverified", "dispute") => {
                (EvidenceCertainty::Unverified, EvidenceCertainty::Disputed)
            }
            ("disputed", "reopen") => (EvidenceCertainty::Disputed, EvidenceCertainty::Unverified),
            _ => return Err(invalid("evidence review transition")),
        };
        let changed = self.store.review_evidence_reference(
            project_id,
            study_id,
            evidence_id,
            expected,
            next,
            note,
        )?;
        let source = self
            .store
            .list_study_sources(project_id, study_id)?
            .into_iter()
            .find(|item| item.id == changed.input.study_source_id)
            .ok_or_else(|| RepositoryError::Storage("evidence source missing".into()))?;
        let reviews = self
            .store
            .list_evidence_reviews(project_id, study_id, changed.id)?;
        Ok(ui_evidence(changed, &source, reviews))
    }

    pub fn save_studio_study(
        &self,
        project_id: Uuid,
        study_id: Uuid,
        expected_revision: i64,
        patch: UiStudioStudyPatch,
    ) -> RepositoryResult<UiStudioStudy> {
        let _project_guard = self.lock_studio_project(project_id)?;
        let study = self.store.get_study(project_id, study_id)?;
        if study.revision != expected_revision {
            return Err(RepositoryError::Conflict(
                "study changed by another editor".into(),
            ));
        }
        let selected_id = patch
            .draft_id
            .as_deref()
            .map(|value| Uuid::parse_str(value).map_err(|_| invalid("draft id")))
            .transpose()?;
        let old = self.read_studio_bundle_for(project_id, study_id, selected_id)?;
        if patch
            .draft_revision
            .is_some_and(|value| Some(value) != old.draft_revision)
        {
            return Err(RepositoryError::Conflict(
                "selected draft revision changed".into(),
            ));
        }
        let brief = patch.brief.unwrap_or_else(|| old.brief.clone());
        let sources = patch.sources.unwrap_or_else(|| old.sources.clone());
        let draft = patch.draft.unwrap_or_else(|| old.draft.clone());
        let input = brief_input(&brief)?;
        if sources.len() > 100 {
            return Err(invalid("source count"));
        }
        let prior_sources = self.store.list_study_sources(project_id, study_id)?;
        let present: HashSet<(String, String)> = prior_sources
            .iter()
            .filter_map(ui_source)
            .map(|item| (item.kind, item.id))
            .collect();
        let mut unique = HashSet::new();
        for source in &sources {
            let input = source_input(source)?;
            if !unique.insert((source.kind.as_str(), source.id.as_str())) {
                return Err(invalid("duplicate source"));
            }
            if !present.contains(&(source.kind.clone(), source.id.clone())) {
                self.store.validate_study_origin(
                    project_id,
                    input.kind,
                    input.reference_id.ok_or_else(|| invalid("source id"))?,
                )?;
            }
        }

        let canonical_sources = sources
            .iter()
            .map(source_input)
            .collect::<RepositoryResult<Vec<_>>>()?;
        let selected_sources = canonical_sources
            .iter()
            .map(|value| {
                Ok((
                    value.kind,
                    value.reference_id.ok_or_else(|| invalid("source id"))?,
                ))
            })
            .collect::<RepositoryResult<Vec<_>>>()?;
        self.store.save_studio_bundle(
            project_id,
            study_id,
            scriptotar_db::StudioBundleUpdate {
                expected_revision,
                edit: &StudyEdit {
                    title: brief.title.clone(),
                    goal: brief.goal.clone(),
                    status: study.status,
                },
                brief: &input,
                brief_changed: brief != old.brief,
                sources: &selected_sources,
                draft: &draft,
                draft_id: old
                    .draft_id
                    .as_deref()
                    .and_then(|value| Uuid::parse_str(value).ok()),
                expected_draft_revision: old.draft_revision,
            },
        )?;
        self.read_studio_bundle_for(project_id, study_id, selected_id)
    }
}

/*
 * Production CurrentStudy loader. This intentionally reuses the same bounded,
 * Study-scoped writing bridge as Copy Prompt; no untrusted DTO, provider call,
 * or synthetic source fixture can populate the generation context.
 *
 * The active-project mutex excludes project switches while this snapshot is
 * built. SQLite's batch sink independently reloads this context under its
 * IMMEDIATE write transaction before committing any of the three drafts.
 *
 * NOTE: This is NOT a paid-egress authorization epoch. Source mutation and
 * transport admission still need Agent 01's host-wide shared guard.
 */
impl super::study_writing::CurrentStudy for AppServices {
    fn load(
        &self,
        project_id: Uuid,
        study_id: Uuid,
    ) -> super::study_writing::WriteResult<super::study_writing::StudyWritingContext> {
        use super::study_writing::{StudyWritingContext, WritingWorkflowError};

        let _project_guard = self
            .lock_studio_project(project_id)
            .map_err(|_| WritingWorkflowError::Denied("project is not active".into()))?;
        let study = self
            .store
            .get_study(project_id, study_id)
            .map_err(|_| WritingWorkflowError::Denied("Study is not accessible".into()))?;
        if study.archived_at.is_some() {
            return Err(WritingWorkflowError::Denied("Study is archived".into()));
        }

        let canonical_sources = self
            .store
            .list_study_sources(project_id, study_id)
            .map_err(|_| WritingWorkflowError::Denied("Study sources are unavailable".into()))?;
        let (input, _, _, _, _) = self
            .scoped_writing_input_locked(project_id, study_id)
            .map_err(|_| WritingWorkflowError::Denied("Study evidence is unavailable".into()))?;

        // The gateway uses multiple repository reads. Refuse a changing Study
        // snapshot rather than signing a mixed revision/source selection.
        let latest = self
            .store
            .get_study(project_id, study_id)
            .map_err(|_| WritingWorkflowError::StaleContext)?;
        if latest.revision != study.revision || latest.archived_at.is_some() {
            return Err(WritingWorkflowError::StaleContext);
        }
        let latest_sources = self
            .store
            .list_study_sources(project_id, study_id)
            .map_err(|_| WritingWorkflowError::StaleContext)?;
        let signature = |rows: &[StudySource]| {
            rows.iter()
                .map(|source| {
                    (
                        source.id,
                        source.reference_id,
                        source.available,
                        source.fingerprint.clone(),
                        source.source_revision.clone(),
                        source.collected_at.clone(),
                    )
                })
                .collect::<Vec<_>>()
        };
        if signature(&canonical_sources) != signature(&latest_sources) {
            return Err(WritingWorkflowError::StaleContext);
        }

        StudyWritingContext::from_scoped_study(
            project_id,
            study_id,
            study.revision,
            &canonical_sources,
            input,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn example_brief() -> UiStudioBrief {
        UiStudioBrief {
            title: "Content study".into(),
            goal: "Write an original script about a product".into(),
            audience: "Regional creators".into(),
            platform: "TikTok".into(),
            language: "Arabic".into(),
            duration: "40".into(),
            style: "Clear".into(),
        }
    }

    #[test]
    fn current_writing_context_uses_real_scoped_sources_and_rejects_revocation() {
        use super::super::study_writing::{prepare_three, CurrentStudy};

        let dir = TempDir::new().unwrap();
        let services = AppServices::new(dir.path()).unwrap();
        let project = services.active_project_id().unwrap();
        let study = services
            .create_studio_study(project, example_brief())
            .unwrap();
        let study_id = Uuid::parse_str(&study.id).unwrap();
        let conn = rusqlite::Connection::open(services.store.path()).unwrap();
        conn.execute_batch("PRAGMA foreign_keys=ON;").unwrap();

        let mut references = Vec::new();
        for index in 0..3 {
            let source = Uuid::new_v4();
            let media = Uuid::new_v4();
            let transcript = Uuid::new_v4();
            conn.execute(
                "INSERT INTO sources(id,project_id,creator_id,source_type,locator,created_at)
                 VALUES(?1,?2,NULL,'url',?3,'2026-11-16T00:00:00Z')",
                rusqlite::params![
                    source.to_string(),
                    project.to_string(),
                    format!("https://example.org/current-study/{index}"),
                ],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO media(id,source_id,local_path,created_at)
                 VALUES(?1,?2,?3,'2026-11-16T00:00:00Z')",
                rusqlite::params![
                    media.to_string(),
                    source.to_string(),
                    format!("/tmp/scriptotar-current-study-{index}.mp4"),
                ],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO transcripts(id,media_id,text,created_at,updated_at)
                 VALUES(?1,?2,?3,'2026-11-16T00:00:00Z','2026-11-16T00:00:00Z')",
                rusqlite::params![
                    transcript.to_string(),
                    media.to_string(),
                    format!("Distinct attributable source excerpt number {index}"),
                ],
            )
            .unwrap();
            references.push(UiStudioSourceRef {
                kind: "transcript".into(),
                id: transcript.to_string(),
            });
        }

        let saved = services
            .save_studio_study(
                project,
                study_id,
                study.revision,
                UiStudioStudyPatch {
                    brief: None,
                    sources: Some(references.clone()),
                    draft: None,
                    draft_id: None,
                    draft_revision: None,
                },
            )
            .unwrap();

        let current = CurrentStudy::load(&services, project, study_id).unwrap();
        assert_eq!(current.study_revision, saved.revision);
        assert_eq!(current.sources.len(), 3);
        assert_eq!(current.evidence.len(), 3);
        assert!(current.sources.iter().all(|source| {
            !source.fingerprint.is_empty()
                && references
                    .iter()
                    .any(|reference| reference.id == source.origin_id.to_string())
        }));
        let plan = services
            .prepare_three_writing_plan(project, study_id, "canonical-study-1")
            .unwrap();
        assert_eq!(plan.context, current);
        assert!(prepare_three("canonical-study-1", current.clone()).is_ok());
        assert!(CurrentStudy::load(&services, Uuid::new_v4(), study_id).is_err());

        let first_origin = Uuid::parse_str(&references[0].id).unwrap();
        conn.execute(
            "UPDATE transcripts SET text=?1, updated_at=?2 WHERE id=?3",
            rusqlite::params![
                "A revised source excerpt that replaces original text",
                "2026-11-16T00:01:00Z",
                first_origin.to_string(),
            ],
        )
        .unwrap();
        let edited = CurrentStudy::load(&services, project, study_id).unwrap();
        assert_eq!(edited.study_revision, current.study_revision);
        assert_ne!(edited, current);

        services
            .save_studio_study(
                project,
                study_id,
                saved.revision,
                UiStudioStudyPatch {
                    brief: None,
                    sources: Some(references[..2].to_vec()),
                    draft: None,
                    draft_id: None,
                    draft_revision: None,
                },
            )
            .unwrap();
        assert!(CurrentStudy::load(&services, project, study_id).is_err());
    }

    /// Genuine AppServices + canonical transcripts + four controlled no-cost
    /// provider steps + v11 atomic three-draft receipt. No public Tauri
    /// handler, API keys, network or synthetic persisted Study context.
    #[test]
    fn host_generates_three_original_candidates_atomically_and_reopens_without_replay() {
        use super::super::study_writing::{
            DurablePermissionBoundary, GenerationPlan, NeverStop, StepAdmission,
            StudyWritingContext, WriteResult, WritingTextProvider, WritingWorkflowError,
        };
        use scriptotar_ai::{AiRequest, AiResponse};
        use std::collections::{HashMap, HashSet, VecDeque};

        struct TestPermit {
            approved: bool,
            pending: HashSet<String>,
            saved: HashMap<String, String>,
            steps: usize,
        }
        impl DurablePermissionBoundary for TestPermit {
            fn begin(
                &mut self,
                run_id: &str,
                _context: &StudyWritingContext,
                step: &str,
                request: &AiRequest,
            ) -> WriteResult<StepAdmission> {
                if !self.approved {
                    return Err(WritingWorkflowError::Denied(
                        "explicit approval required".into(),
                    ));
                }
                let key = format!(
                    "{run_id}:{step}:{}",
                    scriptotar_ai::provider_consent::sha256_hex(request.prompt.as_bytes())
                );
                if let Some(response_json) = self.saved.get(&key) {
                    return Ok(StepAdmission::Recorded {
                        response_json: response_json.clone(),
                    });
                }
                if !self.pending.insert(key.clone()) {
                    return Err(WritingWorkflowError::UncertainEffect);
                }
                self.steps += 1;
                Ok(StepAdmission::Dispatch { effect_key: key })
            }
            fn record_response(&mut self, key: &str, value: &str) -> WriteResult<()> {
                if !self.pending.remove(key) {
                    return Err(WritingWorkflowError::UncertainEffect);
                }
                self.saved.insert(key.into(), value.into());
                Ok(())
            }
            fn mark_uncertain(&mut self, _key: &str) -> WriteResult<()> {
                Err(WritingWorkflowError::UncertainEffect)
            }
            fn abandon_before_dispatch(&mut self, key: &str) -> WriteResult<()> {
                self.pending.remove(key);
                Ok(())
            }
        }
        struct NoCostProvider {
            responses: VecDeque<String>,
            calls: usize,
        }
        impl WritingTextProvider for NoCostProvider {
            fn generate(&mut self, _request: &AiRequest) -> Result<AiResponse, ()> {
                self.calls += 1;
                self.responses
                    .pop_front()
                    .map(|text| AiResponse { text })
                    .ok_or(())
            }
        }

        let dir = TempDir::new().unwrap();
        let services = AppServices::new(dir.path()).unwrap();
        let project = services.active_project_id().unwrap();
        let created = services
            .create_studio_study(project, example_brief())
            .unwrap();
        let study = Uuid::parse_str(&created.id).unwrap();
        let conn = rusqlite::Connection::open(services.store.path()).unwrap();
        conn.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
        let mut attached = Vec::new();
        for index in 0..3 {
            let source = Uuid::new_v4();
            let media = Uuid::new_v4();
            let transcript = Uuid::new_v4();
            conn.execute(
                "INSERT INTO sources(id,project_id,creator_id,source_type,locator,created_at)
                 VALUES(?1,?2,NULL,'url',?3,'2026-11-17T00:00:00Z')",
                rusqlite::params![
                    source.to_string(),
                    project.to_string(),
                    format!("https://example.org/controlled-writing/{index}")
                ],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO media(id,source_id,local_path,created_at)
                 VALUES(?1,?2,?3,'2026-11-17T00:00:00Z')",
                rusqlite::params![
                    media.to_string(),
                    source.to_string(),
                    format!("/tmp/controlled-writing-{index}.mp4")
                ],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO transcripts(id,media_id,text,created_at,updated_at)
                 VALUES(?1,?2,?3,'2026-11-17T00:00:00Z','2026-11-17T00:00:00Z')",
                rusqlite::params![
                    transcript.to_string(),
                    media.to_string(),
                    format!("Authentic controlled source excerpt {index}")
                ],
            )
            .unwrap();
            attached.push(UiStudioSourceRef {
                kind: "transcript".into(),
                id: transcript.to_string(),
            });
        }
        services
            .save_studio_study(
                project,
                study,
                created.revision,
                UiStudioStudyPatch {
                    brief: None,
                    sources: Some(attached),
                    draft: None,
                    draft_id: None,
                    draft_revision: None,
                },
            )
            .unwrap();
        let plan: GenerationPlan = services
            .prepare_three_writing_plan(project, study, "controlled-four-step")
            .unwrap();
        let ids = plan
            .context
            .evidence
            .iter()
            .map(|e| e.id.clone())
            .collect::<Vec<_>>();
        assert_eq!(ids.len(), 3);
        let angles = serde_json::json!({ "angles": [
            { "id":"contrast", "approach":"contrast two options",
              "hook":"One neighborhood, two different paths",
              "promise":"See an unexpected choice",
              "outline":[{"role":"hook","intent":"contrast","estimated_seconds":10}],
              "evidence_ids":[ids[0]] },
            { "id":"journey", "approach":"first person diary",
              "hook":"The notebook changed my walking route",
              "promise":"A grounded human journey",
              "outline":[{"role":"hook","intent":"journey","estimated_seconds":12}],
              "evidence_ids":[ids[1]] },
            { "id":"guide", "approach":"practical how to",
              "hook":"A useful three-step preparation checklist",
              "promise":"Actionable practical tips",
              "outline":[{"role":"hook","intent":"guide","estimated_seconds":9}],
              "evidence_ids":[ids[2]] }
        ]})
        .to_string();
        let script = |index: usize, angle: &str, evidence: &str, narration: &str| {
            serde_json::json!({
                "id":format!("draft-{index}"), "angle_id":angle,
                "title":format!("Original content angle {index}"), "language":"Arabic",
                "beats":[
                    {"id":"hook","role":"hook","narration":narration,
                     "on_screen_text":null,"shots":[],"evidence_ids":[evidence],
                     "manually_edited":true},
                    {"id":"cta","role":"cta",
                     "narration":"Invite viewers to discuss their own experiences.",
                     "on_screen_text":null,"shots":[],"evidence_ids":[],
                     "manually_edited":false}
                ],
                "cta":"Discuss your own ideas", "caption":"An original controlled script",
                "claims":[{"id":"claim-a","beat_id":"hook",
                           "assertion":"A source discusses this theme",
                           "status":"unverified","evidence_ids":[evidence]}]
            })
            .to_string()
        };
        let mut provider = NoCostProvider {
            responses: VecDeque::from([
                angles,
                script(
                    1,
                    "contrast",
                    &ids[0],
                    "Two routes lead to very different community stories.",
                ),
                script(
                    2,
                    "journey",
                    &ids[1],
                    "I followed a personal diary and discovered a quiet path.",
                ),
                script(
                    3,
                    "guide",
                    &ids[2],
                    "First gather evidence, then compare it, then share advice.",
                ),
            ]),
            calls: 0,
        };
        let mut permit = TestPermit {
            approved: false,
            pending: HashSet::new(),
            saved: HashMap::new(),
            steps: 0,
        };
        assert!(matches!(
            services.generate_three_writing_with_boundary(
                &plan,
                &mut permit,
                &mut provider,
                &NeverStop
            ),
            Err(WritingWorkflowError::Denied(_))
        ));
        assert_eq!(provider.calls, 0);
        permit.approved = true;
        let stored = services
            .generate_three_writing_with_boundary(&plan, &mut permit, &mut provider, &NeverStop)
            .unwrap();
        assert_eq!(provider.calls, 4);
        assert_eq!(permit.steps, 4);
        assert_eq!(stored.len(), 3);
        assert_eq!(stored.iter().copied().collect::<HashSet<_>>().len(), 3);
        let batches: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM study_writing_batches WHERE run_id='controlled-four-step'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(batches, 1);
        for id in stored {
            let versions: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM draft_versions WHERE draft_id=?1 AND version=1",
                    rusqlite::params![id.to_string()],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(versions, 1);
            let lineage: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM study_writing_revisions WHERE draft_id=?1 AND version=1",
                    rusqlite::params![id.to_string()],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(
                lineage, 1,
                "generated candidate must retain structured provenance"
            );
        }
        let chosen = services
            .get_studio_study_draft(project, study, stored[0])
            .unwrap();
        // Creating a Study may already create a manual candidate. The atomic
        // batch must add exactly its own three drafts without deleting it.
        assert!(chosen.drafts.len() >= 3);
        assert!(stored.iter().all(|id| {
            chosen.drafts.iter().any(|draft| draft.id == id.to_string())
        }));
        let edited_body = format!(
            "{}\\n\\nReviewer manual edit: preserve this wording after restart.",
            chosen.draft
        );
        let edited = services
            .save_studio_study(
                project,
                study,
                chosen.revision,
                UiStudioStudyPatch {
                    brief: None,
                    sources: None,
                    draft: Some(edited_body.clone()),
                    draft_id: Some(stored[0].to_string()),
                    draft_revision: chosen.draft_revision,
                },
            )
            .unwrap();
        assert_eq!(edited.draft, edited_body);
        assert!(services
            .get_studio_study_draft(Uuid::new_v4(), study, stored[0])
            .is_err());
        drop(conn);
        drop(services);

        // A restart with NO provider responses and NO approval must recover
        // exactly the committed receipt. Never bill or generate again.
        let reopened = AppServices::new(dir.path()).unwrap();
        let mut no_approval = TestPermit {
            approved: false,
            pending: HashSet::new(),
            saved: HashMap::new(),
            steps: 0,
        };
        let mut no_provider = NoCostProvider {
            responses: VecDeque::new(),
            calls: 0,
        };
        let recovered = reopened
            .generate_three_writing_with_boundary(
                &plan,
                &mut no_approval,
                &mut no_provider,
                &NeverStop,
            )
            .unwrap();
        assert_eq!(recovered, stored);
        assert_eq!(no_approval.steps, 0);
        assert_eq!(no_provider.calls, 0);
        let manual = reopened
            .get_studio_study_draft(project, study, stored[0])
            .unwrap();
        assert_eq!(
            manual.draft, edited_body,
            "recovery must not overwrite a manual edit"
        );
        assert!(manual.draft_versions.len() >= 2);
        let untouched = reopened
            .get_studio_study_draft(project, study, stored[1])
            .unwrap();
        assert_eq!(untouched.draft_versions.len(), 1);
        assert!(untouched.draft.contains("personal diary"));
    }

    #[test]
    fn title_only_study_can_be_saved_and_reopened_without_inventing_brief_fields() {
        let dir = TempDir::new().unwrap();
        let project_id;
        let study_id;
        let mut partial = example_brief();
        partial.title = "A new idea".into();
        partial.goal.clear();
        partial.audience.clear();
        partial.style.clear();
        {
            let services = AppServices::new(dir.path()).unwrap();
            project_id = services.active_project_id().unwrap();
            let created = services
                .create_studio_study(project_id, partial.clone())
                .unwrap();
            study_id = Uuid::parse_str(&created.id).unwrap();
            assert_eq!(created.brief, partial);
            assert!(created.draft.is_empty());
            assert_eq!(created.revision, 1);
        }
        let services = AppServices::new(dir.path()).unwrap();
        let opened = services.get_studio_study(project_id, study_id).unwrap();
        assert_eq!(opened.brief, partial);
        assert_eq!(opened.revision, 1);
    }

    /// A late SQL error must not consume the study's optimistic-concurrency
    /// revision or strand half-written brief/draft state. Restart-proof.
    #[test]
    fn studio_save_rolls_back_all_resources_after_late_sql_failure() {
        let dir = TempDir::new().unwrap();
        let services = AppServices::new(dir.path()).unwrap();
        let project_id = services.active_project_id().unwrap();
        let initial = services
            .create_studio_study(project_id, example_brief())
            .unwrap();
        let study_id = Uuid::parse_str(&initial.id).unwrap();
        let db = rusqlite::Connection::open(services.store.path()).unwrap();
        db.execute_batch(
            "CREATE TRIGGER test_fail_draft_write BEFORE UPDATE ON creative_drafts
             BEGIN SELECT RAISE(ABORT, 'simulated draft write interruption'); END;",
        )
        .unwrap();
        let mut revised = example_brief();
        revised.title = "Changed title".into();
        revised.audience = "Creators and producers".into();
        assert!(services
            .save_studio_study(
                project_id,
                study_id,
                initial.revision,
                UiStudioStudyPatch {
                    brief: Some(revised.clone()),
                    sources: Some(Vec::new()),
                    draft: Some("new script that must roll back".into()),
                    draft_id: None,
                    draft_revision: None,
                },
            )
            .is_err());
        let unchanged = services.get_studio_study(project_id, study_id).unwrap();
        assert_eq!(unchanged.revision, initial.revision);
        assert_eq!(unchanged.brief, initial.brief);
        assert_eq!(unchanged.draft, "");
        assert_eq!(unchanged.draft_versions.len(), 1);
        db.execute_batch("DROP TRIGGER test_fail_draft_write;")
            .unwrap();
        let saved = services
            .save_studio_study(
                project_id,
                study_id,
                initial.revision,
                UiStudioStudyPatch {
                    brief: Some(revised.clone()),
                    sources: Some(Vec::new()),
                    draft: Some("persisted after recovery".into()),
                    draft_id: None,
                    draft_revision: None,
                },
            )
            .unwrap();
        assert_eq!(saved.revision, initial.revision + 1);
        assert_eq!(saved.brief, revised);
        assert_eq!(saved.draft_versions.len(), 2);
    }

    #[test]
    fn studio_create_rolls_back_study_when_initial_draft_fails() {
        let dir = TempDir::new().unwrap();
        let services = AppServices::new(dir.path()).unwrap();
        let project_id = services.active_project_id().unwrap();
        let db = rusqlite::Connection::open(services.store.path()).unwrap();
        db.execute_batch(
            "CREATE TRIGGER test_fail_initial_version BEFORE INSERT ON draft_versions
             BEGIN SELECT RAISE(ABORT, 'simulated failed first version'); END;",
        )
        .unwrap();
        assert!(services
            .create_studio_study(project_id, example_brief())
            .is_err());
        assert!(services.list_studio_studies(project_id).unwrap().is_empty());
        db.execute_batch("DROP TRIGGER test_fail_initial_version;")
            .unwrap();
        let created = services
            .create_studio_study(project_id, example_brief())
            .unwrap();
        assert_eq!(created.draft_versions.len(), 1);
        assert_eq!(services.list_studio_studies(project_id).unwrap().len(), 1);
    }

    #[test]
    fn studio_persists_draft_versions_and_reopens_after_restart() {
        let dir = TempDir::new().unwrap();
        let (project_id, study_id) = {
            let services = AppServices::new(dir.path()).unwrap();
            let project_id = services.active_project_id().unwrap();
            let initial = services
                .create_studio_study(project_id, example_brief())
                .unwrap();
            let study_id = Uuid::parse_str(&initial.id).unwrap();
            assert_eq!(initial.revision, 1);
            assert_eq!(initial.brief, example_brief());
            let saved = services
                .save_studio_study(
                    project_id,
                    study_id,
                    1,
                    UiStudioStudyPatch {
                        brief: None,
                        sources: Some(Vec::new()),
                        draft: Some("An original saved script".into()),
                        draft_id: None,
                        draft_revision: None,
                    },
                )
                .unwrap();
            assert_eq!(saved.revision, 2);
            assert_eq!(saved.draft, "An original saved script");
            assert_eq!(
                saved.draft_versions.last().unwrap().content,
                "An original saved script"
            );
            assert!(matches!(
                services.save_studio_study(
                    project_id,
                    study_id,
                    1,
                    UiStudioStudyPatch {
                        brief: None,
                        sources: None,
                        draft: Some("stale writer".into()),
                        draft_id: None,
                        draft_revision: None,
                    },
                ),
                Err(RepositoryError::Conflict(_))
            ));
            assert!(services.get_studio_study(Uuid::new_v4(), study_id).is_err());
            (project_id, study_id)
        };
        let reopened = AppServices::new(dir.path()).unwrap();
        let restored = reopened.get_studio_study(project_id, study_id).unwrap();
        assert_eq!(restored.revision, 2);
        assert_eq!(restored.draft, "An original saved script");
        assert_eq!(restored.draft_versions.len(), 2);
    }

    #[test]
    fn studio_keeps_real_source_provenance_and_rejects_linked_source_removal_atomically() {
        let dir = TempDir::new().unwrap();
        let project_id;
        let study_id;
        let research_id = Uuid::new_v4();
        let transcript_id = Uuid::new_v4();
        let evidence_id;
        {
            let services = AppServices::new(dir.path()).unwrap();
            project_id = services.active_project_id().unwrap();
            let source_id = Uuid::new_v4();
            let media_id = Uuid::new_v4();
            let conn = rusqlite::Connection::open(services.store.path()).unwrap();
            conn.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
            conn.execute(
                "INSERT INTO research_items(id,project_id,source_url,platform,scanned_at)
                 VALUES(?1,?2,'https://example.org/study/source','sample','2026-11-13T10:00:00Z')",
                rusqlite::params![research_id.to_string(), project_id.to_string()],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO sources(id,project_id,creator_id,source_type,locator,created_at)
                 VALUES(?1,?2,NULL,'url','https://example.org/study/source','2026-11-13T10:00:00Z')",
                rusqlite::params![source_id.to_string(), project_id.to_string()],
            ).unwrap();
            conn.execute(
                "INSERT INTO media(id,source_id,local_path,created_at)
                 VALUES(?1,?2,'/tmp/study-fixture.mp4','2026-11-13T10:00:00Z')",
                rusqlite::params![media_id.to_string(), source_id.to_string()],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO transcripts(id,media_id,text,created_at,updated_at)
                 VALUES(?1,?2,'Source with a timecode','2026-11-13T10:00:00Z','2026-11-13T10:00:00Z')",
                rusqlite::params![transcript_id.to_string(), media_id.to_string()],
            ).unwrap();
            let study = services
                .create_studio_study(project_id, example_brief())
                .unwrap();
            study_id = Uuid::parse_str(&study.id).unwrap();
            let sources = vec![
                UiStudioSourceRef {
                    kind: "research".into(),
                    id: research_id.to_string(),
                },
                UiStudioSourceRef {
                    kind: "transcript".into(),
                    id: transcript_id.to_string(),
                },
            ];
            let saved = services
                .save_studio_study(
                    project_id,
                    study_id,
                    study.revision,
                    UiStudioStudyPatch {
                        brief: None,
                        sources: Some(sources.clone()),
                        draft: Some("Original referenced draft".into()),
                        draft_id: None,
                        draft_revision: None,
                    },
                )
                .unwrap();
            assert_eq!(saved.sources.len(), sources.len());
            assert!(sources.iter().all(|source| saved.sources.contains(source)));
            assert_eq!(saved.draft, "Original referenced draft");
            let stored_sources = services
                .store
                .list_study_sources(project_id, study_id)
                .unwrap();
            assert_eq!(stored_sources.len(), 2);
            let evidence = services
                .add_studio_evidence(
                    project_id,
                    study_id,
                    UiStudyEvidenceInput {
                        source: sources[1].clone(),
                        claim: "Transcript contains a literal spoken example".into(),
                        locator: "0:00-0:05".into(),
                        timestamp_seconds: Some(0.0),
                        uncertainty_note: None,
                    },
                )
                .unwrap();
            assert_eq!(evidence.certainty, "unverified");
            assert_eq!(evidence.source, Some(sources[1].clone()));
            conn.execute(
                "UPDATE transcripts SET segments_json=?1 WHERE id=?2",
                rusqlite::params![
                    r#"[{"index":0,"start":0.5,"end":3.25,"text":"Literal opening from timestamp"}]"#,
                    transcript_id.to_string()
                ],
            )
            .unwrap();
            let angles = services
                .prepare_studio_angle_prompt(project_id, study_id)
                .unwrap();
            assert_eq!(angles.attached_source_count, 2);
            assert_eq!(angles.transcript_excerpt_count, 1);
            assert_eq!(angles.unverified_observation_count, 1);
            assert!(angles.prompt.contains("Propose exactly 3"));
            assert!(angles.prompt.contains("Literal opening from timestamp"));
            assert!(angles
                .prompt
                .contains("Transcript segment 0 (0.500-3.250s)"));
            assert!(angles
                .prompt
                .contains("Transcript contains a literal spoken example"));
            assert!(angles.prompt.contains("unverified"));
            assert!(angles.prompt.contains("untrusted context"));
            let three_angles = serde_json::json!({
                "angles": [
                    {
                        "id": "angle-a",
                        "approach": "Show the everyday problem",
                        "hook": "Open on the problem",
                        "promise": "Show a specific useful alternative",
                        "outline": [{ "role": "hook", "intent": "Present a clear problem", "estimated_seconds": 7 }],
                        "evidence_ids": [format!("observation:{}", evidence.id)]
                    },
                    {
                        "id": "angle-b",
                        "approach": "A before-and-after account",
                        "hook": "Contrast two scenarios",
                        "promise": "Present an original practical payoff",
                        "outline": [{ "role": "hook", "intent": "Present a contrast", "estimated_seconds": 6 }],
                        "evidence_ids": []
                    },
                    {
                        "id": "angle-c",
                        "approach": "A practical checklist",
                        "hook": "Lead with a concrete action",
                        "promise": "Deliver a concise checklist",
                        "outline": [{ "role": "hook", "intent": "Introduce the checklist", "estimated_seconds": 5 }],
                        "evidence_ids": []
                    }
                ]
            })
            .to_string();
            let reviewed = services
                .interpret_studio_angles(
                    project_id,
                    study_id,
                    saved.revision,
                    &angles.prompt,
                    &three_angles,
                    None,
                )
                .unwrap();
            assert_eq!(reviewed.angles.len(), 3);
            assert_eq!(reviewed.angles[0].evidence_ids.len(), 1);
            assert!(reviewed.draft_prompt.is_none());
            let chosen = services
                .interpret_studio_angles(
                    project_id,
                    study_id,
                    saved.revision,
                    &angles.prompt,
                    &three_angles,
                    Some("angle-a"),
                )
                .unwrap();
            assert!(chosen
                .draft_prompt
                .as_deref()
                .unwrap()
                .contains("Write one original ScriptDraft"));
            assert!(chosen.draft_prompt.as_deref().unwrap().contains("angle-a"));
            let structured_script = serde_json::json!({
                "id": "generated-1",
                "angle_id": "angle-a",
                "title": "An original opening proposal",
                "language": "Arabic",
                "beats": [{
                    "id": "hook-1",
                    "role": "hook",
                    "narration": "An original opening with one concrete example",
                    "on_screen_text": null,
                    "shots": [],
                    "evidence_ids": [format!("observation:{}", evidence.id)],
                    "manually_edited": true
                }],
                "cta": null,
                "caption": null,
                "claims": [{
                    "id": "claim-1",
                    "beat_id": "hook-1",
                    "assertion": "One model-proposed observation needs checking",
                    "status": "unverified",
                    "evidence_ids": [format!("observation:{}", evidence.id)]
                }]
            })
            .to_string();
            let draft_prompt = chosen.draft_prompt.as_deref().unwrap();
            let request = UiStudyScriptInput {
                expected_study_revision: saved.revision,
                angle_prompt: angles.prompt.clone(),
                angles_response_json: three_angles.clone(),
                selected_angle_id: "angle-a".into(),
                prepared_draft_prompt: draft_prompt.into(),
                script_response_json: structured_script.clone(),
                accept_as_new_candidate: false,
            };
            let preview = services
                .review_studio_script(project_id, study_id, &request)
                .unwrap();
            assert!(preview.saved_study.is_none());
            assert!(preview.body.contains("An original opening"));
            assert_eq!(preview.needs_fact_review_count, 1);
            assert_eq!(
                services
                    .get_studio_study(project_id, study_id)
                    .unwrap()
                    .drafts
                    .len(),
                1
            );
            let accepted = UiStudyScriptInput {
                accept_as_new_candidate: true,
                ..request.clone()
            };
            assert!(services
                .review_studio_script(
                    project_id,
                    study_id,
                    &UiStudyScriptInput {
                        prepared_draft_prompt: "stale draft prompt".into(),
                        ..accepted.clone()
                    },
                )
                .is_err());
            assert!(services
                .review_studio_script(Uuid::new_v4(), study_id, &accepted)
                .is_err());
            assert!(services
                .review_studio_script(
                    project_id,
                    study_id,
                    &UiStudyScriptInput {
                        script_response_json: structured_script
                            .replace("\"unverified\"", "\"supported\""),
                        ..accepted.clone()
                    },
                )
                .is_err());
            let saved_script = services
                .review_studio_script(project_id, study_id, &accepted)
                .unwrap();
            let candidate = saved_script.saved_study.unwrap();
            assert_eq!(candidate.drafts.len(), 2);
            assert_eq!(candidate.revision, saved.revision + 1);
            assert_eq!(candidate.draft_versions.len(), 1);
            // Replaying an acceptance after a successful commit is denied.
            assert!(services
                .review_studio_script(project_id, study_id, &accepted)
                .is_err());
            assert!(candidate.draft.contains("source IDs"));
            assert!(candidate.draft.contains("Unverified"));
            let generated_id = Uuid::parse_str(candidate.draft_id.as_deref().unwrap()).unwrap();
            let links = services
                .store
                .list_draft_source_ids(project_id, generated_id)
                .unwrap();
            assert_eq!(links.len(), 2);
            assert_eq!(
                services
                    .get_studio_study_draft(project_id, study_id, generated_id)
                    .unwrap()
                    .draft,
                candidate.draft
            );
            assert_eq!(
                services
                    .get_studio_study_draft(
                        project_id,
                        study_id,
                        Uuid::parse_str(saved.draft_id.as_deref().unwrap()).unwrap()
                    )
                    .unwrap()
                    .draft,
                "Original referenced draft"
            );
            assert!(services
                .interpret_studio_angles(
                    project_id,
                    study_id,
                    saved.revision,
                    &angles.prompt,
                    &three_angles,
                    Some("missing")
                )
                .is_err());
            assert!(services
                .interpret_studio_angles(
                    Uuid::new_v4(),
                    study_id,
                    saved.revision,
                    &angles.prompt,
                    &three_angles,
                    None
                )
                .is_err());
            assert!(services
                .interpret_studio_angles(
                    project_id,
                    study_id,
                    saved.revision - 1,
                    &angles.prompt,
                    &three_angles,
                    None
                )
                .is_err());
            assert!(services
                .interpret_studio_angles(
                    project_id,
                    study_id,
                    saved.revision,
                    &angles.prompt,
                    r#"{"angles":[]}"#,
                    None
                )
                .is_err());
            assert!(services
                .prepare_studio_angle_prompt(Uuid::new_v4(), study_id)
                .is_err());
            assert_eq!(
                services
                    .list_studio_evidence(project_id, study_id)
                    .unwrap()
                    .len(),
                1
            );
            assert!(services
                .add_studio_evidence(
                    Uuid::new_v4(),
                    study_id,
                    UiStudyEvidenceInput {
                        source: sources[1].clone(),
                        claim: "Wrong project".into(),
                        locator: "0:00".into(),
                        timestamp_seconds: None,
                        uncertainty_note: None,
                    },
                )
                .is_err());
            assert!(services
                .add_studio_evidence(
                    project_id,
                    study_id,
                    UiStudyEvidenceInput {
                        source: UiStudioSourceRef {
                            kind: "transcript".into(),
                            id: Uuid::new_v4().to_string(),
                        },
                        claim: "Unattached source".into(),
                        locator: "0:00".into(),
                        timestamp_seconds: None,
                        uncertainty_note: None,
                    },
                )
                .is_err());
            evidence_id = Uuid::parse_str(&evidence.id).unwrap();
            assert!(services
                .review_studio_evidence(
                    Uuid::new_v4(),
                    study_id,
                    evidence_id,
                    "unverified",
                    "dispute",
                    "Wrong project",
                )
                .is_err());
            assert!(services
                .review_studio_evidence(
                    project_id,
                    study_id,
                    evidence_id,
                    "unverified",
                    "dispute",
                    " ",
                )
                .is_err());
            assert!(services
                .review_studio_evidence(
                    project_id,
                    study_id,
                    evidence_id,
                    "unverified",
                    "corroborate",
                    "Cannot certify source",
                )
                .is_err());
            let disputed = services
                .review_studio_evidence(
                    project_id,
                    study_id,
                    evidence_id,
                    "unverified",
                    "dispute",
                    "Full transcript contradicts this claim",
                )
                .unwrap();
            assert_eq!(disputed.certainty, "disputed");
            let after_dispute = services
                .prepare_studio_angle_prompt(project_id, study_id)
                .unwrap();
            assert_eq!(after_dispute.unverified_observation_count, 0);
            assert!(services
                .interpret_studio_angles(
                    project_id,
                    study_id,
                    saved.revision,
                    &angles.prompt,
                    &three_angles,
                    None
                )
                .is_err());
            assert!(services
                .interpret_studio_angles(
                    project_id,
                    study_id,
                    saved.revision,
                    &after_dispute.prompt,
                    &three_angles,
                    None
                )
                .is_err());
            assert!(!after_dispute
                .prompt
                .contains("Transcript contains a literal spoken example"));
            assert!(after_dispute
                .prompt
                .contains("Literal opening from timestamp"));
            assert_eq!(disputed.review_history.len(), 1);
            assert_eq!(disputed.review_history[0].previous_certainty, "unverified");
            assert_eq!(disputed.review_history[0].next_certainty, "disputed");
            assert_eq!(
                disputed.review_history[0].note,
                "Full transcript contradicts this claim"
            );
            assert_eq!(
                disputed.uncertainty_note.as_deref(),
                Some("Full transcript contradicts this claim")
            );
            assert_eq!(disputed.locator, evidence.locator);
            assert!(matches!(
                services.review_studio_evidence(
                    project_id,
                    study_id,
                    evidence_id,
                    "unverified",
                    "dispute",
                    "Stale review",
                ),
                Err(RepositoryError::Conflict(_))
            ));
            assert!(matches!(
                services.save_studio_study(
                    project_id,
                    study_id,
                    saved.revision,
                    UiStudioStudyPatch {
                        brief: None,
                        sources: Some(vec![sources[0].clone()]),
                        draft: Some("Must not commit this edit".into()),
                        draft_id: None,
                        draft_revision: None,
                    },
                ),
                Err(RepositoryError::Conflict(_))
            ));
            let unchanged = services.get_studio_study(project_id, study_id).unwrap();
            assert_eq!(unchanged.revision, saved.revision + 1);
            assert_eq!(unchanged.sources.len(), sources.len());
            assert!(sources
                .iter()
                .all(|source| unchanged.sources.contains(source)));
            assert!(unchanged
                .draft
                .starts_with("# An original opening proposal"));
        }
        let reopened = AppServices::new(dir.path()).unwrap();
        let restored = reopened.get_studio_study(project_id, study_id).unwrap();
        assert_eq!(restored.sources.len(), 2);
        assert!(restored.draft.starts_with("# An original opening proposal"));
        assert_eq!(restored.drafts.len(), 2);
        let evidence = reopened
            .store
            .get_evidence_reference(project_id, study_id, evidence_id)
            .unwrap();
        assert_eq!(evidence.input.locator, "0:00-0:05");
        let restored_refs = reopened.list_studio_evidence(project_id, study_id).unwrap();
        assert_eq!(restored_refs.len(), 1);
        assert_eq!(restored_refs[0].locator, "0:00-0:05");
        assert_eq!(restored_refs[0].certainty, "disputed");
        assert_eq!(restored_refs[0].review_history.len(), 1);
        assert_eq!(
            restored_refs[0].uncertainty_note.as_deref(),
            Some("Full transcript contradicts this claim")
        );
        let reopened_review = reopened
            .review_studio_evidence(
                project_id,
                study_id,
                evidence_id,
                "disputed",
                "reopen",
                "Reviewed full recording again",
            )
            .unwrap();
        assert_eq!(reopened_review.certainty, "unverified");
        assert_eq!(reopened_review.review_history.len(), 2);
        assert_eq!(
            reopened_review.review_history[0].note,
            "Full transcript contradicts this claim"
        );
        assert_eq!(
            reopened_review.review_history[1].note,
            "Reviewed full recording again"
        );
        assert_eq!(
            reopened_review.review_history[1].previous_certainty,
            "disputed"
        );
        assert_eq!(
            reopened_review.review_history[1].next_certainty,
            "unverified"
        );
        assert!(reopened
            .store
            .list_evidence_reviews(Uuid::new_v4(), study_id, evidence_id)
            .is_err());
        assert_eq!(
            reopened
                .store
                .list_evidence_reviews(project_id, study_id, evidence_id)
                .unwrap()
                .len(),
            2
        );
        let conn = rusqlite::Connection::open(reopened.store.path()).unwrap();
        assert!(conn
            .execute(
                "UPDATE evidence_reviews SET reason='tampered' WHERE evidence_id=?1",
                rusqlite::params![evidence_id.to_string()]
            )
            .is_err());
        assert!(conn
            .execute(
                "DELETE FROM evidence_reviews WHERE evidence_id=?1",
                rusqlite::params![evidence_id.to_string()]
            )
            .is_err());
        assert_eq!(
            reopened
                .store
                .get_evidence_reference(project_id, study_id, evidence_id)
                .unwrap()
                .input
                .certainty,
            EvidenceCertainty::Unverified
        );
    }

    #[test]
    fn invalid_source_is_rejected_before_study_revision_changes() {
        let dir = TempDir::new().unwrap();
        let services = AppServices::new(dir.path()).unwrap();
        let project_id = services.active_project_id().unwrap();
        let study = services
            .create_studio_study(project_id, example_brief())
            .unwrap();
        let study_id = Uuid::parse_str(&study.id).unwrap();
        let attempt = services.save_studio_study(
            project_id,
            study_id,
            study.revision,
            UiStudioStudyPatch {
                brief: None,
                sources: Some(vec![UiStudioSourceRef {
                    kind: "transcript".into(),
                    id: Uuid::new_v4().to_string(),
                }]),
                draft: Some("must not be saved".into()),
                draft_id: None,
                draft_revision: None,
            },
        );
        assert!(attempt.is_err());
        let unchanged = services.get_studio_study(project_id, study_id).unwrap();
        assert_eq!(unchanged.revision, study.revision);
        assert_eq!(unchanged.draft, "");
        assert!(unchanged.sources.is_empty());
    }

    #[test]
    fn selected_draft_candidates_keep_independent_versions_after_restart() {
        let dir = TempDir::new().unwrap();
        let project_id;
        let study_id;
        let original_id;
        let copy_id;
        {
            let services = AppServices::new(dir.path()).unwrap();
            project_id = services.active_project_id().unwrap();
            let initial = services
                .create_studio_study(project_id, example_brief())
                .unwrap();
            study_id = Uuid::parse_str(&initial.id).unwrap();
            original_id = Uuid::parse_str(initial.draft_id.as_deref().unwrap()).unwrap();
            let original = services
                .save_studio_study(
                    project_id,
                    study_id,
                    initial.revision,
                    UiStudioStudyPatch {
                        brief: None,
                        sources: None,
                        draft: Some("An independent original script".into()),
                        draft_id: initial.draft_id.clone(),
                        draft_revision: initial.draft_revision,
                    },
                )
                .unwrap();
            assert_eq!(original.draft, "An independent original script");
            assert_eq!(original.drafts.len(), 1);
            let copied = services
                .duplicate_studio_draft(
                    project_id,
                    study_id,
                    original_id,
                    original.draft_revision.unwrap(),
                )
                .unwrap();
            copy_id = Uuid::parse_str(copied.draft_id.as_deref().unwrap()).unwrap();
            assert_ne!(copy_id, original_id);
            assert_eq!(copied.drafts.len(), 2);
            assert_eq!(copied.draft, original.draft);
            assert_eq!(copied.draft_versions.len(), 1);
            assert!(copied
                .drafts
                .iter()
                .any(|item| item.id == original_id.to_string()));
            assert!(matches!(
                services.duplicate_studio_draft(project_id, study_id, original_id, 1),
                Err(RepositoryError::Conflict(_))
            ));
            assert!(services
                .duplicate_studio_draft(Uuid::new_v4(), study_id, original_id, 2)
                .is_err());
            let foreign_study = services
                .create_studio_study(project_id, example_brief())
                .unwrap();
            let foreign_id = Uuid::parse_str(&foreign_study.id).unwrap();
            assert!(services
                .get_studio_study_draft(project_id, foreign_id, original_id)
                .is_err());
            assert!(services
                .duplicate_studio_draft(project_id, foreign_id, original_id, 2)
                .is_err());
            let changed = services
                .save_studio_study(
                    project_id,
                    study_id,
                    copied.revision,
                    UiStudioStudyPatch {
                        brief: None,
                        sources: None,
                        draft: Some("A DIFFERENT second script".into()),
                        draft_id: copied.draft_id.clone(),
                        draft_revision: copied.draft_revision,
                    },
                )
                .unwrap();
            assert_eq!(changed.draft_id, copied.draft_id);
            assert_eq!(changed.draft_versions.len(), 2);
            assert!(matches!(
                services.save_studio_study(
                    project_id,
                    study_id,
                    changed.revision,
                    UiStudioStudyPatch {
                        brief: None,
                        sources: None,
                        draft: Some("Must fail stale draft revision".into()),
                        draft_id: changed.draft_id.clone(),
                        draft_revision: copied.draft_revision,
                    },
                ),
                Err(RepositoryError::Conflict(_))
            ));
            let original_after = services
                .get_studio_study_draft(project_id, study_id, original_id)
                .unwrap();
            assert_eq!(original_after.draft, original.draft);
            assert_eq!(
                original_after.draft_versions.len(),
                original.draft_versions.len()
            );
            assert_eq!(changed.drafts.len(), 2);
        }
        let restarted = AppServices::new(dir.path()).unwrap();
        let older = restarted
            .get_studio_study_draft(project_id, study_id, original_id)
            .unwrap();
        let newer = restarted
            .get_studio_study_draft(project_id, study_id, copy_id)
            .unwrap();
        assert_eq!(older.draft, "An independent original script");
        assert_eq!(newer.draft, "A DIFFERENT second script");
        assert_eq!(older.draft_versions.len(), 2);
        assert_eq!(newer.draft_versions.len(), 2);
        assert_eq!(newer.drafts.len(), 2);
        let renamed = restarted
            .rename_studio_draft(
                project_id,
                study_id,
                copy_id,
                newer.draft_revision.unwrap(),
                "Arabic follow-up",
            )
            .unwrap();
        assert_eq!(renamed.draft, newer.draft);
        assert_eq!(renamed.draft_versions.len(), 3);
        assert!(renamed
            .drafts
            .iter()
            .any(|item| item.id == copy_id.to_string() && item.title == "Arabic follow-up"));
        assert!(matches!(
            restarted.rename_studio_draft(project_id, study_id, copy_id, 1, "Stale title"),
            Err(RepositoryError::Conflict(_))
        ));
        assert!(restarted
            .rename_studio_draft(Uuid::new_v4(), study_id, copy_id, 3, "Foreign")
            .is_err());
        let original = restarted
            .get_studio_study_draft(project_id, study_id, original_id)
            .unwrap();
        assert_eq!(original.draft, "An independent original script");
        let archived = restarted
            .set_studio_draft_archived(
                project_id,
                study_id,
                copy_id,
                renamed.draft_revision.unwrap(),
                true,
            )
            .unwrap();
        assert_eq!(archived.draft_id, Some(original_id.to_string()));
        assert_eq!(archived.drafts.len(), 1);
        assert_eq!(archived.archived_drafts.len(), 1);
        drop(restarted);
        let reopened = AppServices::new(dir.path()).unwrap();
        let archived_after_restart = reopened.get_studio_study(project_id, study_id).unwrap();
        assert_eq!(archived_after_restart.archived_drafts.len(), 1);
        assert_eq!(archived_after_restart.drafts.len(), 1);
        assert!(reopened
            .set_studio_draft_archived(
                project_id,
                study_id,
                original_id,
                original.draft_revision.unwrap(),
                true,
            )
            .is_err());
        assert!(reopened
            .set_studio_draft_archived(Uuid::new_v4(), study_id, copy_id, 4, false)
            .is_err());
        let restored = reopened
            .set_studio_draft_archived(
                project_id,
                study_id,
                copy_id,
                archived.archived_drafts[0].revision,
                false,
            )
            .unwrap();
        assert_eq!(restored.draft_id, Some(copy_id.to_string()));
        assert_eq!(restored.draft, "A DIFFERENT second script");
        assert!(restored.archived_drafts.is_empty());
        assert_eq!(restored.drafts.len(), 2);
        assert_eq!(restored.draft_versions.len(), 5);
    }
}
