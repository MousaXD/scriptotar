//! Canonical, all-or-nothing persistence for Agent 05's three-script result.
//!
//! This module is deliberately not registered in the live Tauri host. Agent 00
//! must install RECEIPT_MIGRATION as the next versioned DB migration, expose
//! this adapter through the scoped Study gateway, and supply a *real* CurrentStudy
//! loader. No provider transport or paid execution is enabled here.
use std::{collections::HashSet, time::Duration};

use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use scriptotar_ai::writing::{ClaimStatus, EvidenceKind, EvidenceStatus};
use scriptotar_core::{now_rfc3339, RepositoryError};
use scriptotar_db::SqliteStore;
use uuid::Uuid;

use super::study_writing::{
    export_candidate, AtomicCandidateStore, CurrentStudy, GeneratedBatch, WriteResult,
    WritingWorkflowError,
};

/// Agent 00 must fold this additive schema into a numbered migration, not run
/// CREATE TABLE on arbitrary app startup or silently mutate the migration ledger.
/// The receipt and its three canonical drafts are inserted in ONE transaction.
pub const RECEIPT_MIGRATION: &str = r#"
CREATE TABLE study_writing_batches (
    run_id TEXT PRIMARY KEY CHECK(length(run_id) BETWEEN 1 AND 128),
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    study_id TEXT NOT NULL REFERENCES content_studies(id) ON DELETE CASCADE,
    payload_json TEXT NOT NULL CHECK(json_valid(payload_json) AND length(payload_json) <= 2000000),
    draft_ids_json TEXT NOT NULL CHECK(json_valid(draft_ids_json)),
    committed_at TEXT NOT NULL
);
CREATE INDEX idx_study_writing_batches_scope
    ON study_writing_batches(project_id, study_id, committed_at);
CREATE TRIGGER study_writing_batches_immutable_update
    BEFORE UPDATE ON study_writing_batches
    BEGIN SELECT RAISE(ABORT, 'writing receipt is immutable'); END;
CREATE TRIGGER study_writing_batches_immutable_delete
    BEFORE DELETE ON study_writing_batches
    WHEN EXISTS (SELECT 1 FROM content_studies WHERE id=OLD.study_id)
    BEGIN SELECT RAISE(ABORT, 'writing receipt is immutable'); END;
"#;

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
}

impl<C: CurrentStudy> CanonicalSqliteCandidateStore<'_, C> {
    #[cfg(test)]
    pub fn fail_after_candidate(&mut self, batch: &GeneratedBatch, index: usize) -> WriteResult<[Uuid; 3]> {
        self.commit_inner(batch, Some(index))
    }

    fn commit_inner(&mut self, batch: &GeneratedBatch, fail_after: Option<usize>) -> WriteResult<[Uuid; 3]> {
        batch.context.validate()?;
        if batch.candidates.len() != 3
            || batch.run_id.is_empty()
            || batch.run_id.len() > 128
            || !batch.run_id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        {
            return Err(denied("expected three scripts and a valid run"));
        }

        // Model output may assert 'supported', but that is NEVER a verification
        // decision. Save these claims as unverified until Agent 04 reviews them.
        let mut normalized = batch.clone();
        let evidence_ids: HashSet<_> = batch.context.evidence.iter().map(|e| e.id.as_str()).collect();
        let selected_sources: HashSet<_> =
            batch.context.sources.iter().map(|s| s.study_source_id).collect();
        let mut angles = HashSet::new();
        let mut scripts = HashSet::new();
        for candidate in &mut normalized.candidates {
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
            for claim in &mut candidate.draft.claims {
                if !claim.evidence_ids.iter().all(|id| evidence_ids.contains(id.as_str())) {
                    return Err(denied("unknown claim evidence"));
                }
                if claim.status == ClaimStatus::Supported {
                    claim.status = ClaimStatus::Unverified;
                }
            }
            for beat in &candidate.draft.beats {
                if !beat.evidence_ids.iter().all(|id| evidence_ids.contains(id.as_str()))
                    || !beat.shots.iter().all(|shot|
                        shot.evidence_ids.iter().all(|id| evidence_ids.contains(id.as_str())))
                {
                    return Err(denied("unknown beat or shot evidence"));
                }
            }
            for line in &candidate.lineage {
                if !selected_sources.contains(&line.source_id) {
                    return Err(denied("foreign source in script lineage"));
                }
                for evidence in &line.evidence {
                    if !evidence_ids.contains(evidence.id.as_str())
                        || (matches!(evidence.kind, EvidenceKind::SocialTranscript | EvidenceKind::CreativeObservation)
                            && evidence.status == EvidenceStatus::Corroborated)
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
        db.pragma_update(None, "foreign_keys", true).map_err(error)?;
        db.busy_timeout(Duration::from_secs(5)).map_err(error)?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate).map_err(error)?;

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
                let exists: i64 = tx.query_row(
                    "SELECT COUNT(*) FROM creative_drafts d
                     JOIN draft_versions v ON v.draft_id=d.id AND v.version=1
                     WHERE d.id=?1 AND d.project_id=?2 AND d.study_id=?3",
                    params![id.to_string(), project, study],
                    |row| row.get(0),
                ).map_err(error)?;
                if exists != 1 {
                    return Err(WritingWorkflowError::PersistFailed("receipt has missing initial draft".into()));
                }
            }
            tx.commit().map_err(error)?;
            return Ok(ids);
        }

        let project = batch.context.project_id;
        let study = batch.context.study_id;
        let revision: Option<i64> = tx.query_row(
            "SELECT st.revision FROM content_studies st
             JOIN projects p ON p.id=st.project_id
             WHERE st.id=?1 AND st.project_id=?2
               AND st.archived_at IS NULL AND p.archived_at IS NULL",
            params![study.to_string(), project.to_string()],
            |row| row.get(0),
        ).optional().map_err(error)?;
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
            let belongs: i64 = tx.query_row(
                "SELECT COUNT(*) FROM study_sources ss WHERE
                 ss.id=?1 AND ss.study_id=?2 AND ss.project_id=?3
                 AND ss.reference_id=?4 AND (
                   (ss.kind='research_item' AND ss.research_id=ss.reference_id) OR
                   (ss.kind='source' AND ss.legacy_source_id=ss.reference_id) OR
                   (ss.kind='media' AND ss.media_id=ss.reference_id) OR
                   (ss.kind='transcript' AND ss.transcript_id=ss.reference_id) OR
                   (ss.kind='job' AND ss.job_id=ss.reference_id) OR
                   (ss.kind='ai_run' AND ss.ai_run_id=ss.reference_id)
                 )",
                params![source.study_source_id.to_string(), study.to_string(),
                        project.to_string(), source.origin_id.to_string()],
                |row| row.get(0),
            ).map_err(error)?;
            if belongs != 1 {
                return Err(WritingWorkflowError::StaleContext);
            }
        }
        let existing: i64 = tx.query_row(
            "SELECT COUNT(*) FROM creative_drafts WHERE project_id=?1
             AND study_id=?2 AND archived_at IS NULL",
            params![project.to_string(), study.to_string()],
            |row| row.get(0),
        ).map_err(error)?;
        if existing > 197 {
            return Err(WritingWorkflowError::PersistFailed("Study has reached its 200-draft limit".into()));
        }
        let ids: [Uuid; 3] = std::array::from_fn(|index| {
            let material = format!("scriptotar/writing-batch/v1/{project}/{study}/{}/{}", batch.run_id, index);
            Uuid::new_v5(&Uuid::NAMESPACE_URL, material.as_bytes())
        });
        let now = now_rfc3339();
        let consumed = tx.execute(
            "UPDATE content_studies SET revision=revision+1,updated_at=?1
             WHERE id=?2 AND project_id=?3 AND revision=?4 AND archived_at IS NULL",
            params![now, study.to_string(), project.to_string(), batch.context.study_revision],
        ).map_err(error)?;
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
                params![id.to_string(), project.to_string(), study.to_string(), title, body, now],
            ).map_err(error)?;
            tx.execute(
                "INSERT INTO draft_versions(id,draft_id,version,title,body,saved_at)
                 VALUES(?1,?2,1,?3,?4,?5)",
                params![Uuid::new_v4().to_string(), id.to_string(), title, body, now],
            ).map_err(error)?;
            for source in &batch.context.sources {
                tx.execute(
                    "INSERT INTO draft_source_links(project_id,draft_id,study_source_id,attached_at)
                     VALUES(?1,?2,?3,?4)",
                    params![project.to_string(), id.to_string(),
                            source.study_source_id.to_string(), now],
                ).map_err(error)?;
            }
            if fail_after == Some(index + 1) {
                return Err(WritingWorkflowError::PersistFailed("injected rollback".into()));
            }
        }
        let ids_json = serde_json::to_string(&ids)
            .map_err(|e| WritingWorkflowError::PersistFailed(e.to_string()))?;
        tx.execute(
            "INSERT INTO study_writing_batches(
                run_id,project_id,study_id,payload_json,draft_ids_json,committed_at)
             VALUES(?1,?2,?3,?4,?5,?6)",
            params![batch.run_id, project.to_string(), study.to_string(),
                    payload, ids_json, now],
        ).map_err(error)?;
        tx.commit().map_err(error)?;
        Ok(ids)
    }
}
