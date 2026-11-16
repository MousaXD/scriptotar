//! Canonical, all-or-nothing persistence for Agent 05's three-script result.
//!
//! Compiled privately in the production Tauri host. SQLite v11 is registered
//! centrally and a real, scoped CurrentStudy loader is implemented in the Study
//! gateway. Public generation IPC and paid-provider dispatch are NOT enabled.
use std::{collections::HashSet, time::Duration};

use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use scriptotar_ai::writing::{self, ClaimStatus, EvidenceKind, EvidenceStatus};
use scriptotar_core::now_rfc3339;
use scriptotar_db::SqliteStore;
use uuid::Uuid;

use super::study_writing::{
    export_candidate, lineage_for, prepare_three, AtomicCandidateStore, CurrentStudy,
    GeneratedBatch, GeneratedCandidate, GenerationPlan, VersionedCandidateStore, WriteResult,
    WritingWorkflowError,
};

/// The additive schema is registered exclusively as the central v11 migration;
/// never re-execute CREATE TABLE at startup or alter prior migration history.
/// The receipt and its three canonical drafts are inserted in ONE transaction.
#[cfg(test)]
pub use scriptotar_db::WRITING_RECEIPT_MIGRATION as RECEIPT_MIGRATION;

fn error(error: rusqlite::Error) -> WritingWorkflowError {
    WritingWorkflowError::PersistFailed(error.to_string())
}

fn denied(reason: &'static str) -> WritingWorkflowError {
    WritingWorkflowError::InvalidContext(reason)
}

/// Requires a canonical, active-project-scoped CurrentStudy implementation.
/// A test FixedStudy is NOT sufficient for live Tauri wiring. This adapter
/// acquires SQLite's write reservation BEFORE refreshing that context, making
/// source/evidence/brief rereads and all inserts an indivisible operation.
pub struct CanonicalSqliteCandidateStore<'a, C: CurrentStudy> {
    pub store: &'a SqliteStore,
    pub current: &'a C,
}

impl<C: CurrentStudy> AtomicCandidateStore for CanonicalSqliteCandidateStore<'_, C> {
    fn commit_three(&mut self, batch: &GeneratedBatch) -> WriteResult<[Uuid; 3]> {
        self.commit_inner(batch, None)
    }

    /// A confirmed receipt bypasses a stale Study CAS ONLY for read-only
    /// recovery of already-existing scripts. Never creates a draft, calls a
    /// provider, claims a paid effect, or restores overwritten manual content.
    fn recover_committed(&mut self, plan: &GenerationPlan) -> WriteResult<Option<[Uuid; 3]>> {
        let reconstructed = prepare_three(&plan.run_id, plan.context.clone())?;
        if reconstructed.angles_prompt != plan.angles_prompt {
            return Err(WritingWorkflowError::StaleContext);
        }
        let db = Connection::open(self.store.path()).map_err(error)?;
        db.pragma_update(None, "foreign_keys", true)
            .map_err(error)?;
        let row: Option<(String, String, String, String)> = db
            .query_row(
                "SELECT project_id,study_id,payload_json,draft_ids_json
                 FROM study_writing_batches WHERE run_id=?1",
                params![plan.run_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .optional()
            .map_err(error)?;
        let Some((project, study, stored, ids_json)) = row else {
            return Ok(None);
        };
        if project != plan.context.project_id.to_string()
            || study != plan.context.study_id.to_string()
        {
            return Err(WritingWorkflowError::StaleContext);
        }
        let original: GeneratedBatch = serde_json::from_str(&stored).map_err(|_| {
            WritingWorkflowError::PersistFailed("corrupt saved writing batch".into())
        })?;
        if original.run_id != plan.run_id
            || original.context != plan.context
            || original.candidates.len() != 3
        {
            return Err(WritingWorkflowError::StaleContext);
        }
        let ids: [Uuid; 3] = serde_json::from_str(&ids_json)
            .map_err(|_| WritingWorkflowError::PersistFailed("corrupt batch receipt".into()))?;
        for id in ids {
            let initial: i64 = db
                .query_row(
                    "SELECT COUNT(*) FROM creative_drafts d
                     JOIN draft_versions v ON v.draft_id=d.id AND v.version=1
                     JOIN study_writing_revisions sr
                       ON sr.draft_id=d.id AND sr.version=1
                     WHERE d.id=?1 AND d.project_id=?2 AND d.study_id=?3",
                    params![id.to_string(), project, study],
                    |row| row.get(0),
                )
                .map_err(error)?;
            if initial != 1 {
                return Err(WritingWorkflowError::PersistFailed(
                    "saved writing receipt missing canonical initial draft".into(),
                ));
            }
        }
        Ok(Some(ids))
    }
}

impl<C: CurrentStudy> CanonicalSqliteCandidateStore<'_, C> {
    #[cfg(test)]
    pub fn fail_after_candidate(
        &mut self,
        batch: &GeneratedBatch,
        index: usize,
    ) -> WriteResult<[Uuid; 3]> {
        self.commit_inner(batch, Some(index))
    }

    fn commit_inner(
        &mut self,
        batch: &GeneratedBatch,
        fail_after: Option<usize>,
    ) -> WriteResult<[Uuid; 3]> {
        batch.context.validate()?;
        if batch.candidates.len() != 3
            || batch.run_id.is_empty()
            || batch.run_id.len() > 128
            || !batch
                .run_id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        {
            return Err(denied("expected three scripts and a valid run"));
        }

        // Model output may assert 'supported', but that is NEVER a verification
        // decision. Save these claims as unverified until Agent 04 reviews them.
        let mut normalized = batch.clone();
        let evidence_ids: HashSet<_> = batch
            .context
            .evidence
            .iter()
            .map(|e| e.id.as_str())
            .collect();
        let selected_sources: HashSet<_> = batch
            .context
            .sources
            .iter()
            .map(|s| s.study_source_id)
            .collect();
        // The database boundary must revalidate diversity independently of
        // generate_three: direct batch acceptance must not bypass the
        // orchestrator's angle/same-prose checks.
        let proposed_angles = normalized
            .candidates
            .iter()
            .map(|candidate| candidate.angle.clone())
            .collect::<Vec<_>>();
        if !writing::compare_angles(&proposed_angles).is_empty() {
            return Err(denied("duplicate or similar script angles"));
        }
        let mut angles = HashSet::new();
        let mut scripts = HashSet::new();
        let mut normalized_prose = HashSet::new();
        for candidate in &mut normalized.candidates {
            let prose = candidate
                .draft
                .beats
                .iter()
                .map(|beat| beat.narration.to_lowercase())
                .collect::<Vec<_>>()
                .join(" ");
            let prose = prose.split_whitespace().collect::<Vec<_>>().join(" ");
            if !normalized_prose.insert(prose) {
                return Err(denied("duplicate script bodies"));
            }
            if !angles.insert(candidate.angle.id.as_str())
                || !scripts.insert(candidate.draft.id.as_str())
                || candidate.draft.angle_id != candidate.angle.id
                || candidate.draft.language != batch.context.brief.language
                || candidate.draft.title.trim().is_empty()
                || candidate.draft.title.chars().count() > 200
                || candidate.draft.beats.is_empty()
            {
                return Err(denied("draft identities, language, or title"));
            }
            if !candidate
                .angle
                .evidence_ids
                .iter()
                .all(|id| evidence_ids.contains(id.as_str()))
            {
                return Err(denied("unknown angle evidence"));
            }
            if candidate.lineage != lineage_for(&batch.context, &candidate.angle, &candidate.draft)?
            {
                return Err(denied("incomplete or forged source lineage"));
            }
            for claim in &mut candidate.draft.claims {
                if !claim
                    .evidence_ids
                    .iter()
                    .all(|id| evidence_ids.contains(id.as_str()))
                {
                    return Err(denied("unknown claim evidence"));
                }
                if claim.status == ClaimStatus::Supported {
                    claim.status = ClaimStatus::Unverified;
                }
            }
            for beat in &candidate.draft.beats {
                if !beat
                    .evidence_ids
                    .iter()
                    .all(|id| evidence_ids.contains(id.as_str()))
                    || !beat.shots.iter().all(|shot| {
                        shot.evidence_ids
                            .iter()
                            .all(|id| evidence_ids.contains(id.as_str()))
                    })
                {
                    return Err(denied("unknown beat or shot evidence"));
                }
            }
            for line in &candidate.lineage {
                if !selected_sources.contains(&line.source_id) {
                    return Err(denied("foreign source in script lineage"));
                }
                for evidence in &line.evidence {
                    if batch
                        .context
                        .evidence
                        .iter()
                        .find(|item| item.id == evidence.id)
                        != Some(evidence)
                        || !evidence_ids.contains(evidence.id.as_str())
                        || (matches!(
                            evidence.kind,
                            EvidenceKind::SocialTranscript | EvidenceKind::CreativeObservation
                        ) && evidence.status == EvidenceStatus::Corroborated)
                    {
                        return Err(denied("forged lineage status"));
                    }
                }
            }
            for finding in &mut candidate.review_findings {
                finding.needs_human_review = true;
            }
        }
        // Exact persisted original inputs (incl. original angles, source
        // locators and reviews) provide durable structured provenance. The
        // editor's mutable body remains readable Markdown.
        let payload = serde_json::to_string(&normalized)
            .map_err(|e| WritingWorkflowError::PersistFailed(e.to_string()))?;
        if payload.len() > 2_000_000 {
            return Err(denied("generation batch too large"));
        }
        let mut db = Connection::open(self.store.path()).map_err(error)?;
        db.pragma_update(None, "foreign_keys", true)
            .map_err(error)?;
        db.busy_timeout(Duration::from_secs(5)).map_err(error)?;
        let tx = db
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(error)?;

        // Check receipt FIRST: after a successful write whose reply is lost,
        // the Study revision has advanced and an exact retry must reconcile
        // WITHOUT consuming another approval or creating any new candidates.
        let receipt: Option<(String, String, String, String)> = tx
            .query_row(
                "SELECT project_id,study_id,payload_json,draft_ids_json
                 FROM study_writing_batches WHERE run_id=?1",
                params![batch.run_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .optional()
            .map_err(error)?;
        if let Some((project, study, stored, ids_json)) = receipt {
            if project != batch.context.project_id.to_string()
                || study != batch.context.study_id.to_string()
                || stored != payload
            {
                return Err(WritingWorkflowError::StaleContext);
            }
            let ids: [Uuid; 3] = serde_json::from_str(&ids_json)
                .map_err(|_| WritingWorkflowError::PersistFailed("corrupt batch receipt".into()))?;
            for id in ids {
                let exists: i64 = tx
                    .query_row(
                        "SELECT COUNT(*) FROM creative_drafts d
                     JOIN draft_versions v ON v.draft_id=d.id AND v.version=1
                     WHERE d.id=?1 AND d.project_id=?2 AND d.study_id=?3",
                        params![id.to_string(), project, study],
                        |row| row.get(0),
                    )
                    .map_err(error)?;
                if exists != 1 {
                    return Err(WritingWorkflowError::PersistFailed(
                        "receipt has missing initial draft".into(),
                    ));
                }
            }
            tx.commit().map_err(error)?;
            return Ok(ids);
        }

        let project = batch.context.project_id;
        let study = batch.context.study_id;
        let revision: Option<i64> = tx
            .query_row(
                "SELECT st.revision FROM content_studies st
             JOIN projects p ON p.id=st.project_id
             WHERE st.id=?1 AND st.project_id=?2
               AND st.archived_at IS NULL AND p.archived_at IS NULL",
                params![study.to_string(), project.to_string()],
                |row| row.get(0),
            )
            .optional()
            .map_err(error)?;
        if revision != Some(batch.context.study_revision) {
            return Err(WritingWorkflowError::StaleContext);
        }

        // Rebuild the actual, scoped gateway input *under the write lock*.
        // This catches edited transcript text, changed observations, withdrawn
        // sources, revised briefs and project switches rather than trusting a
        // stale serialized payload or the Study revision alone.
        if self.current.load(project, study)? != batch.context {
            return Err(WritingWorkflowError::StaleContext);
        }

        for source in &batch.context.sources {
            let belongs: i64 = tx
                .query_row(
                    "SELECT COUNT(*) FROM study_sources ss WHERE
                 ss.id=?1 AND ss.study_id=?2 AND ss.project_id=?3
                 AND ss.reference_id=?4 AND ss.fingerprint=?5
                 AND ss.source_revision IS ?6 AND ss.collected_at=?7 AND (
                   (ss.kind='research_item' AND ss.research_id=ss.reference_id) OR
                   (ss.kind='source' AND ss.legacy_source_id=ss.reference_id) OR
                   (ss.kind='media' AND ss.media_id=ss.reference_id) OR
                   (ss.kind='transcript' AND ss.transcript_id=ss.reference_id) OR
                   (ss.kind='job' AND ss.job_id=ss.reference_id) OR
                   (ss.kind='ai_run' AND ss.ai_run_id=ss.reference_id)
                 )",
                    params![
                        source.study_source_id.to_string(),
                        study.to_string(),
                        project.to_string(),
                        source.origin_id.to_string(),
                        source.fingerprint,
                        source.source_revision,
                        source.collected_at
                    ],
                    |row| row.get(0),
                )
                .map_err(error)?;
            if belongs != 1 {
                return Err(WritingWorkflowError::StaleContext);
            }
        }
        let existing: i64 = tx
            .query_row(
                "SELECT COUNT(*) FROM creative_drafts WHERE project_id=?1
             AND study_id=?2 AND archived_at IS NULL",
                params![project.to_string(), study.to_string()],
                |row| row.get(0),
            )
            .map_err(error)?;
        if existing > 197 {
            return Err(WritingWorkflowError::PersistFailed(
                "Study has reached its 200-draft limit".into(),
            ));
        }
        let ids: [Uuid; 3] = std::array::from_fn(|index| {
            let material = format!(
                "scriptotar/writing-batch/v1/{project}/{study}/{}/{}",
                batch.run_id, index
            );
            Uuid::new_v5(&Uuid::NAMESPACE_URL, material.as_bytes())
        });
        let now = now_rfc3339();
        let consumed = tx
            .execute(
                "UPDATE content_studies SET revision=revision+1,updated_at=?1
             WHERE id=?2 AND project_id=?3 AND revision=?4 AND archived_at IS NULL",
                params![
                    now,
                    study.to_string(),
                    project.to_string(),
                    batch.context.study_revision
                ],
            )
            .map_err(error)?;
        if consumed != 1 {
            return Err(WritingWorkflowError::StaleContext);
        }
        for (index, candidate) in normalized.candidates.iter().enumerate() {
            let id = ids[index];
            let title = candidate.draft.title.trim();
            let body = export_candidate(candidate);
            if body.chars().count() > 2_000_000 {
                return Err(denied("draft body exceeds canonical bound"));
            }
            tx.execute(
                "INSERT INTO creative_drafts(
                  id,project_id,study_id,title,body,revision,created_at,updated_at)
                 VALUES(?1,?2,?3,?4,?5,1,?6,?6)",
                params![
                    id.to_string(),
                    project.to_string(),
                    study.to_string(),
                    title,
                    body,
                    now
                ],
            )
            .map_err(error)?;
            tx.execute(
                "INSERT INTO draft_versions(id,draft_id,version,title,body,saved_at)
                 VALUES(?1,?2,1,?3,?4,?5)",
                params![Uuid::new_v4().to_string(), id.to_string(), title, body, now],
            )
            .map_err(error)?;
            tx.execute(
                "INSERT INTO study_writing_revisions(draft_id,version,structured_json)
                 VALUES(?1,1,?2)",
                params![
                    id.to_string(),
                    serde_json::to_string(candidate)
                        .map_err(|e| WritingWorkflowError::PersistFailed(e.to_string()))?
                ],
            )
            .map_err(error)?;
            // Link only sources actually used by this angle/script. The
            // Study's full selected-source set remains in the immutable batch
            // receipt, but it is not a citation by every individual draft.
            for source in &candidate.lineage {
                tx.execute(
                    "INSERT INTO draft_source_links(project_id,draft_id,study_source_id,attached_at)
                     VALUES(?1,?2,?3,?4)",
                    params![project.to_string(), id.to_string(),
                            source.source_id.to_string(), now],
                ).map_err(error)?;
            }
            if fail_after == Some(index + 1) {
                return Err(WritingWorkflowError::PersistFailed(
                    "injected rollback".into(),
                ));
            }
        }
        let ids_json = serde_json::to_string(&ids)
            .map_err(|e| WritingWorkflowError::PersistFailed(e.to_string()))?;
        tx.execute(
            "INSERT INTO study_writing_batches(
                run_id,project_id,study_id,payload_json,draft_ids_json,committed_at)
             VALUES(?1,?2,?3,?4,?5,?6)",
            params![
                batch.run_id,
                project.to_string(),
                study.to_string(),
                payload,
                ids_json,
                now
            ],
        )
        .map_err(error)?;
        tx.commit().map_err(error)?;
        Ok(ids)
    }
}

/// A targeted model suggestion may only revise the named, still-current
/// narration. The rest of the editor's body is preserved byte for byte.
/// Normal Studio edits still own their versions; when they create a version
/// without structured revision metadata, the agent must ask for a new review.
impl<C: CurrentStudy> VersionedCandidateStore for CanonicalSqliteCandidateStore<'_, C> {
    fn save_revision(
        &mut self,
        context: &super::study_writing::StudyWritingContext,
        canonical_draft_id: Uuid,
        expected_draft_revision: i64,
        previous_beat_text: &str,
        beat_id: &str,
        reviewed: &GeneratedCandidate,
    ) -> WriteResult<i64> {
        context.validate()?;
        if canonical_draft_id.is_nil()
            || expected_draft_revision < 1
            || beat_id.trim().is_empty()
            || previous_beat_text.trim().is_empty()
            || previous_beat_text.chars().count() > 4000
        {
            return Err(denied("invalid targeted revision"));
        }
        let mut db = Connection::open(self.store.path()).map_err(error)?;
        db.pragma_update(None, "foreign_keys", true)
            .map_err(error)?;
        db.busy_timeout(Duration::from_secs(5)).map_err(error)?;
        let tx = db
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(error)?;
        let (body, title, snapshot): (String, String, String) = tx
            .query_row(
                "SELECT d.body,d.title,r.structured_json
             FROM creative_drafts d
             JOIN study_writing_revisions r
               ON r.draft_id=d.id AND r.version=d.revision
             JOIN content_studies s ON s.id=d.study_id AND s.project_id=d.project_id
             JOIN projects p ON p.id=d.project_id
             WHERE d.id=?1 AND d.project_id=?2 AND d.study_id=?3
               AND d.revision=?4 AND d.archived_at IS NULL
               AND s.archived_at IS NULL AND p.archived_at IS NULL",
                params![
                    canonical_draft_id.to_string(),
                    context.project_id.to_string(),
                    context.study_id.to_string(),
                    expected_draft_revision
                ],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()
            .map_err(error)?
            .ok_or(WritingWorkflowError::StaleContext)?;
        if self.current.load(context.project_id, context.study_id)? != *context {
            return Err(WritingWorkflowError::StaleContext);
        }
        let original: GeneratedCandidate = serde_json::from_str(&snapshot).map_err(|_| {
            WritingWorkflowError::PersistFailed("corrupt structured revision".into())
        })?;
        let position = original
            .draft
            .beats
            .iter()
            .position(|beat| beat.id == beat_id)
            .ok_or(denied("beat not in current version"))?;
        let before = &original.draft.beats[position];
        if before.narration != previous_beat_text || before.manually_edited {
            return Err(WritingWorkflowError::StaleContext);
        }
        if reviewed.angle != original.angle
            || reviewed.draft.id != original.draft.id
            || reviewed.draft.angle_id != original.draft.angle_id
            || reviewed.draft.language != original.draft.language
            || reviewed.draft.title != original.draft.title
            || reviewed.draft.beats.len() != original.draft.beats.len()
            || reviewed.draft.beats[position].id != beat_id
        {
            return Err(denied("revision changes identity or other fields"));
        }
        let replacement = &reviewed.draft.beats[position].narration;
        if replacement.trim().is_empty()
            || replacement.chars().count() > 4000
            || replacement == previous_beat_text
        {
            return Err(denied("empty or unchanged revision"));
        }
        // The typed writing primitive clears claims/citations made stale by
        // changed narration, and preserves all other beats. Verify that exact
        // transformation rather than trusting arbitrary model-provided JSON.
        let mut expected = original.draft.clone();
        expected.beats[position].narration.clone_from(replacement);
        expected.beats[position].evidence_ids.clear();
        for claim in &mut expected.claims {
            if claim.beat_id == beat_id {
                claim.status = ClaimStatus::Unverified;
                claim.evidence_ids.clear();
            }
        }
        if reviewed.draft != expected {
            return Err(denied(
                "revision modified untouched beats or citation statuses",
            ));
        }
        if body.matches(previous_beat_text).count() != 1 {
            return Err(WritingWorkflowError::StaleContext);
        }
        let next_body = body.replacen(previous_beat_text, replacement, 1);
        if next_body.chars().count() > 2_000_000 {
            return Err(denied("updated body exceeds canonical bound"));
        }
        let next = expected_draft_revision + 1;
        let now = now_rfc3339();
        let changed = tx
            .execute(
                "UPDATE creative_drafts SET body=?1,revision=?2,updated_at=?3
             WHERE id=?4 AND project_id=?5 AND study_id=?6 AND revision=?7",
                params![
                    next_body,
                    next,
                    now,
                    canonical_draft_id.to_string(),
                    context.project_id.to_string(),
                    context.study_id.to_string(),
                    expected_draft_revision
                ],
            )
            .map_err(error)?;
        if changed != 1 {
            return Err(WritingWorkflowError::StaleContext);
        }
        tx.execute(
            "INSERT INTO draft_versions(id,draft_id,version,title,body,saved_at)
             VALUES(?1,?2,?3,?4,?5,?6)",
            params![
                Uuid::new_v4().to_string(),
                canonical_draft_id.to_string(),
                next,
                title,
                next_body,
                now
            ],
        )
        .map_err(error)?;
        // A revised beat may remove stale evidence, but may not invent new
        // source content or silently change a source's status.
        for lineage in &reviewed.lineage {
            let original_lineage = original
                .lineage
                .iter()
                .find(|item| item.source_id == lineage.source_id)
                .ok_or(denied("revision forged a source"))?;
            if !lineage
                .evidence
                .iter()
                .all(|item| original_lineage.evidence.contains(item))
            {
                return Err(denied("revision forged evidence content"));
            }
        }
        for finding in &reviewed.review_findings {
            if !finding
                .evidence_ids
                .iter()
                .all(|id| context.evidence.iter().any(|item| &item.id == id))
            {
                return Err(denied("revision invented review evidence"));
            }
        }
        let mut saved = reviewed.clone();
        for claim in &mut saved.draft.claims {
            if claim.status == ClaimStatus::Supported {
                claim.status = ClaimStatus::Unverified;
            }
        }
        for finding in &mut saved.review_findings {
            finding.needs_human_review = true;
        }
        tx.execute(
            "INSERT INTO study_writing_revisions(draft_id,version,structured_json)
             VALUES(?1,?2,?3)",
            params![
                canonical_draft_id.to_string(),
                next,
                serde_json::to_string(&saved)
                    .map_err(|e| WritingWorkflowError::PersistFailed(e.to_string()))?
            ],
        )
        .map_err(error)?;
        tx.commit().map_err(error)?;
        Ok(next)
    }
}
