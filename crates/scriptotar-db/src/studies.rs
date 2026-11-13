//! Project-scoped Content Study persistence. No Tauri or AI-provider dependencies.

use std::time::Duration;

use rusqlite::{params, Connection, OptionalExtension, Row, TransactionBehavior};
use scriptotar_core::{
    now_rfc3339, ContentStudy, EvidenceCertainty, EvidenceInput, EvidenceReference,
    RepositoryError, RepositoryResult, StudyEdit, StudySource, StudySourceInput, StudySourceKind,
    StudyStatus,
};
use uuid::Uuid;

use crate::SqliteStore;

pub(crate) fn database(store: &SqliteStore) -> RepositoryResult<Connection> {
    let db = Connection::open(store.path()).map_err(db_error)?;
    db.pragma_update(None, "foreign_keys", true)
        .map_err(db_error)?;
    db.busy_timeout(Duration::from_secs(5)).map_err(db_error)?;
    Ok(db)
}

pub(crate) fn db_error(error: rusqlite::Error) -> RepositoryError {
    if let rusqlite::Error::SqliteFailure(ref failure, _) = error {
        if failure.code == rusqlite::ErrorCode::ConstraintViolation {
            return RepositoryError::Conflict(format!("database constraint: {error}"));
        }
    }
    RepositoryError::Storage(error.to_string())
}

pub(crate) fn sql_conversion<E>(index: usize, error: E) -> rusqlite::Error
where
    E: std::error::Error + Send + Sync + 'static,
{
    rusqlite::Error::FromSqlConversionFailure(index, rusqlite::types::Type::Text, Box::new(error))
}

pub(crate) fn uuid_at(row: &Row<'_>, index: usize) -> rusqlite::Result<Uuid> {
    let raw: String = row.get(index)?;
    Uuid::parse_str(&raw).map_err(|error| sql_conversion(index, error))
}

fn optional_uuid_at(row: &Row<'_>, index: usize) -> rusqlite::Result<Option<Uuid>> {
    let raw: Option<String> = row.get(index)?;
    raw.map(|value| Uuid::parse_str(&value).map_err(|error| sql_conversion(index, error)))
        .transpose()
}

pub(crate) fn required_text(field: &str, text: &str, max: usize) -> RepositoryResult<()> {
    if text.trim().is_empty() || text.chars().count() > max {
        return Err(RepositoryError::Validation(format!(
            "{field} must be nonempty and no longer than {max} characters"
        )));
    }
    Ok(())
}

pub(crate) fn optional_text(field: &str, text: Option<&str>, max: usize) -> RepositoryResult<()> {
    if text.is_some_and(|value| value.chars().count() > max) {
        return Err(RepositoryError::Validation(format!(
            "{field} is longer than {max} characters"
        )));
    }
    Ok(())
}

/// Require a writable study and its active project. Queries/updates never trust
/// a study ID on its own, even when that ID happens to exist in another project.
pub(crate) fn require_active_study(
    db: &Connection,
    project_id: Uuid,
    study_id: Uuid,
) -> RepositoryResult<()> {
    let exists: bool = db
        .query_row(
            "SELECT EXISTS (
                SELECT 1 FROM content_studies st
                JOIN projects p ON p.id=st.project_id
                WHERE st.id=?1 AND st.project_id=?2 AND st.archived_at IS NULL
                  AND p.archived_at IS NULL
             )",
            params![study_id.to_string(), project_id.to_string()],
            |row| row.get(0),
        )
        .map_err(db_error)?;
    if exists {
        Ok(())
    } else {
        Err(RepositoryError::NotFound(format!(
            "active study {study_id} in project {project_id}"
        )))
    }
}

fn study_row(row: &Row<'_>) -> rusqlite::Result<ContentStudy> {
    let status: String = row.get(4)?;
    Ok(ContentStudy {
        id: uuid_at(row, 0)?,
        project_id: uuid_at(row, 1)?,
        title: row.get(2)?,
        goal: row.get(3)?,
        status: StudyStatus::parse(&status).ok_or(rusqlite::Error::InvalidQuery)?,
        revision: row.get(5)?,
        created_at: row.get(6)?,
        updated_at: row.get(7)?,
        archived_at: row.get(8)?,
    })
}

const STUDY_COLUMNS: &str =
    "id, project_id, title, goal, status, revision, created_at, updated_at, archived_at";

impl SqliteStore {
    pub fn create_study(
        &self,
        project_id: Uuid,
        title: &str,
        goal: &str,
    ) -> RepositoryResult<ContentStudy> {
        required_text("study title", title, 200)?;
        optional_text("study goal", Some(goal), 20_000)?;
        let db = database(self)?;
        let id = Uuid::new_v4();
        let now = now_rfc3339();
        let count = db
            .execute(
                "INSERT INTO content_studies
                 (id,project_id,title,goal,status,revision,created_at,updated_at)
                 SELECT ?1, id, ?2, ?3, 'active', 1, ?4, ?4
                 FROM projects WHERE id=?5 AND archived_at IS NULL",
                params![
                    id.to_string(),
                    title.trim(),
                    goal,
                    now,
                    project_id.to_string()
                ],
            )
            .map_err(db_error)?;
        if count == 0 {
            return Err(RepositoryError::NotFound(format!(
                "active project {project_id}"
            )));
        }
        self.get_study(project_id, id)
    }

    pub fn get_study(&self, project_id: Uuid, study_id: Uuid) -> RepositoryResult<ContentStudy> {
        let db = database(self)?;
        let sql =
            format!("SELECT {STUDY_COLUMNS} FROM content_studies WHERE project_id=?1 AND id=?2");
        db.query_row(
            &sql,
            params![project_id.to_string(), study_id.to_string()],
            study_row,
        )
        .optional()
        .map_err(db_error)?
        .ok_or_else(|| RepositoryError::NotFound(format!("study {study_id}")))
    }

    pub fn search_studies(
        &self,
        project_id: Uuid,
        raw_query: &str,
        include_archived: bool,
        limit: usize,
    ) -> RepositoryResult<Vec<ContentStudy>> {
        let db = database(self)?;
        let query = format!("%{}%", escape_like(raw_query.trim()));
        let sql = format!(
            "SELECT {STUDY_COLUMNS} FROM content_studies
             WHERE project_id=?1 AND (?2=1 OR archived_at IS NULL)
               AND (title LIKE ?3 ESCAPE '\\' OR goal LIKE ?3 ESCAPE '\\')
             ORDER BY updated_at DESC, id DESC LIMIT ?4"
        );
        let mut stmt = db.prepare(&sql).map_err(db_error)?;
        let rows = stmt
            .query_map(
                params![
                    project_id.to_string(),
                    include_archived as i64,
                    query,
                    limit.clamp(1, 200) as i64
                ],
                study_row,
            )
            .map_err(db_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_error)
    }

    pub fn update_study(
        &self,
        project_id: Uuid,
        study_id: Uuid,
        expected_revision: i64,
        edit: &StudyEdit,
    ) -> RepositoryResult<ContentStudy> {
        required_text("study title", &edit.title, 200)?;
        optional_text("study goal", Some(&edit.goal), 20_000)?;
        if expected_revision < 1 {
            return Err(RepositoryError::Validation(
                "expected revision must be positive".into(),
            ));
        }
        let db = database(self)?;
        let count = db
            .execute(
                "UPDATE content_studies SET title=?1, goal=?2, status=?3,
                 revision=revision+1, updated_at=?4
                 WHERE project_id=?5 AND id=?6 AND revision=?7 AND archived_at IS NULL
                   AND EXISTS(SELECT 1 FROM projects p WHERE p.id=?5 AND p.archived_at IS NULL)",
                params![
                    edit.title.trim(),
                    edit.goal,
                    edit.status.as_str(),
                    now_rfc3339(),
                    project_id.to_string(),
                    study_id.to_string(),
                    expected_revision,
                ],
            )
            .map_err(db_error)?;
        if count == 0 {
            self.get_study(project_id, study_id)?;
            return Err(RepositoryError::Conflict(
                "study was modified or archived".into(),
            ));
        }
        self.get_study(project_id, study_id)
    }

    pub fn archive_study(&self, project_id: Uuid, study_id: Uuid) -> RepositoryResult<()> {
        self.set_study_archived(project_id, study_id, true)
    }

    pub fn restore_study(&self, project_id: Uuid, study_id: Uuid) -> RepositoryResult<()> {
        self.set_study_archived(project_id, study_id, false)
    }

    fn set_study_archived(
        &self,
        project_id: Uuid,
        study_id: Uuid,
        archived: bool,
    ) -> RepositoryResult<()> {
        let db = database(self)?;
        let when = if archived { Some(now_rfc3339()) } else { None };
        let count = db
            .execute(
                "UPDATE content_studies
                 SET archived_at=?1, revision=revision+1, updated_at=?2
                 WHERE project_id=?3 AND id=?4 AND
                    EXISTS (SELECT 1 FROM projects WHERE id=?3 AND archived_at IS NULL)",
                params![
                    when,
                    now_rfc3339(),
                    project_id.to_string(),
                    study_id.to_string()
                ],
            )
            .map_err(db_error)?;
        if count == 0 {
            return Err(RepositoryError::NotFound(format!("study {study_id}")));
        }
        Ok(())
    }
}

fn escape_like(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

/// Existing source text is not concatenated or interpreted as instructions.
/// A stable v5 identity digest is sufficient for deduplication, not authenticity.
pub(crate) fn fingerprint(kind: StudySourceKind, identity: &str) -> String {
    let identity = format!("scriptotar/study-source/v1/{}/{}", kind.as_str(), identity);
    format!(
        "uuid-v5:{}",
        Uuid::new_v5(&Uuid::NAMESPACE_URL, identity.as_bytes())
    )
}

pub(crate) fn origin_locator(
    db: &Connection,
    project_id: Uuid,
    kind: StudySourceKind,
    id: Uuid,
) -> RepositoryResult<String> {
    let query = match kind {
        StudySourceKind::ResearchItem => {
            "SELECT source_url FROM research_items WHERE id=?1 AND project_id=?2"
        }
        StudySourceKind::Source => "SELECT locator FROM sources WHERE id=?1 AND project_id=?2",
        StudySourceKind::Media => {
            "SELECT s.locator FROM media m JOIN sources s ON s.id=m.source_id
             WHERE m.id=?1 AND s.project_id=?2"
        }
        StudySourceKind::Transcript => {
            "SELECT s.locator FROM transcripts t JOIN media m ON m.id=t.media_id
             JOIN sources s ON s.id=m.source_id
             WHERE t.id=?1 AND s.project_id=?2"
        }
        StudySourceKind::Job => "SELECT input_value FROM jobs WHERE id=?1 AND project_id=?2",
        StudySourceKind::AiRun => "SELECT task FROM ai_runs WHERE id=?1 AND project_id=?2",
        StudySourceKind::Manual => {
            return Err(RepositoryError::Validation(
                "manual sources have no reference ID".into(),
            ))
        }
    };
    let result: Option<String> = db
        .query_row(
            query,
            params![id.to_string(), project_id.to_string()],
            |row| row.get(0),
        )
        .optional()
        .map_err(db_error)?;
    result
        .map(|locator| {
            if kind == StudySourceKind::AiRun {
                format!("ai-run:{id}")
            } else {
                locator
            }
        })
        .ok_or_else(|| RepositoryError::NotFound(format!("{kind:?} {id} in project {project_id}")))
}

fn source_row(row: &Row<'_>) -> rusqlite::Result<StudySource> {
    let kind: String = row.get(3)?;
    Ok(StudySource {
        id: uuid_at(row, 0)?,
        project_id: uuid_at(row, 1)?,
        study_id: uuid_at(row, 2)?,
        kind: StudySourceKind::parse(&kind).ok_or(rusqlite::Error::InvalidQuery)?,
        reference_id: optional_uuid_at(row, 4)?,
        locator: row.get(5)?,
        manual_text: row.get(6)?,
        fingerprint: row.get(7)?,
        source_revision: row.get(8)?,
        collected_at: row.get(9)?,
        uncertainty: row.get(10)?,
        available: row.get::<_, i64>(11)? != 0,
        created_at: row.get(12)?,
    })
}

const SOURCE_COLUMNS: &str =
    "id,project_id,study_id,kind,reference_id,locator,manual_text,fingerprint,
     source_revision,collected_at,uncertainty,
     CASE kind
       WHEN 'manual' THEN 1
       WHEN 'research_item' THEN research_id IS NOT NULL
       WHEN 'source' THEN legacy_source_id IS NOT NULL
       WHEN 'media' THEN media_id IS NOT NULL
       WHEN 'transcript' THEN transcript_id IS NOT NULL
       WHEN 'job' THEN job_id IS NOT NULL
       WHEN 'ai_run' THEN ai_run_id IS NOT NULL
       ELSE 0
     END AS available,
     created_at";

impl SqliteStore {
    pub fn add_study_source(
        &self,
        project_id: Uuid,
        study_id: Uuid,
        input: &StudySourceInput,
    ) -> RepositoryResult<StudySource> {
        let db = database(self)?;
        require_active_study(&db, project_id, study_id)?;
        optional_text("source revision", input.source_revision.as_deref(), 1000)?;
        optional_text("source uncertainty", input.uncertainty.as_deref(), 4000)?;
        let (locator, manual_text, reference_id, identity) = match input.kind {
            StudySourceKind::Manual => {
                if input.reference_id.is_some() {
                    return Err(RepositoryError::Validation(
                        "manual source must not reference a stored entity".into(),
                    ));
                }
                let text = input.manual_text.as_deref().unwrap_or("");
                required_text("manual source text", text, 500_000)?;
                let locator = input.locator.as_deref().unwrap_or("pasted note").trim();
                required_text("manual source locator", locator, 4096)?;
                (
                    locator.to_owned(),
                    Some(text.to_owned()),
                    None,
                    text.to_owned(),
                )
            }
            other => {
                if input.manual_text.is_some() {
                    return Err(RepositoryError::Validation(
                        "stored sources cannot contain an independent pasted text snapshot".into(),
                    ));
                }
                let id = input.reference_id.ok_or_else(|| {
                    RepositoryError::Validation(
                        "stored source must have an exact reference ID".into(),
                    )
                })?;
                let locator = origin_locator(&db, project_id, other, id)?;
                (locator, None, Some(id), id.to_string())
            }
        };
        required_text("source locator", &locator, 4096)?;
        let fp = fingerprint(input.kind, &identity);
        let id = Uuid::new_v4();
        let now = now_rfc3339();
        // Each existing origin type has exactly one non-NULL foreign key.
        // An indexed array keeps the SQL binding list explicit without
        // introducing a deeply nested, error-prone tuple type.
        let mut origin_keys: [Option<String>; 6] = Default::default();
        let origin_index = match input.kind {
            StudySourceKind::ResearchItem => Some(0),
            StudySourceKind::Source => Some(1),
            StudySourceKind::Media => Some(2),
            StudySourceKind::Transcript => Some(3),
            StudySourceKind::Job => Some(4),
            StudySourceKind::AiRun => Some(5),
            StudySourceKind::Manual => None,
        };
        if let Some(index) = origin_index {
            origin_keys[index] = reference_id.map(|value| value.to_string());
        }
        db.execute(
            "INSERT INTO study_sources (
                 id,project_id,study_id,kind,reference_id,research_id,legacy_source_id,
                 media_id,transcript_id,job_id,ai_run_id,locator,manual_text,fingerprint,
                 source_revision,collected_at,uncertainty,created_at
             ) VALUES(
                 ?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18
             )",
            params![
                id.to_string(),
                project_id.to_string(),
                study_id.to_string(),
                input.kind.as_str(),
                reference_id.map(|v| v.to_string()),
                origin_keys[0],
                origin_keys[1],
                origin_keys[2],
                origin_keys[3],
                origin_keys[4],
                origin_keys[5],
                locator,
                manual_text,
                fp,
                input.source_revision,
                now,
                input.uncertainty,
                now
            ],
        )
        .map_err(db_error)?;
        self.get_study_source(project_id, study_id, id)
    }

    pub fn get_study_source(
        &self,
        project_id: Uuid,
        study_id: Uuid,
        source_id: Uuid,
    ) -> RepositoryResult<StudySource> {
        let db = database(self)?;
        let sql = format!(
            "SELECT {SOURCE_COLUMNS} FROM study_sources
             WHERE project_id=?1 AND study_id=?2 AND id=?3"
        );
        db.query_row(
            &sql,
            params![
                project_id.to_string(),
                study_id.to_string(),
                source_id.to_string()
            ],
            source_row,
        )
        .optional()
        .map_err(db_error)?
        .ok_or_else(|| RepositoryError::NotFound(format!("study source {source_id}")))
    }

    /// Verify this Study's active transcript attachment and UTF-8 byte length
    /// using a scoped SQL query, never loading its raw body into Rust memory.
    pub fn attached_transcript_byte_length(
        &self,
        project_id: Uuid,
        study_id: Uuid,
        transcript_id: Uuid,
    ) -> RepositoryResult<u64> {
        let db = database(self)?;
        let bytes: Option<i64> = db
            .query_row(
                "SELECT length(CAST(t.text AS BLOB))
                 FROM study_sources ss
                 JOIN content_studies st ON st.id=ss.study_id AND st.project_id=ss.project_id
                 JOIN projects p ON p.id=ss.project_id
                 JOIN transcripts t ON t.id=ss.transcript_id
                 JOIN media m ON m.id=t.media_id
                 JOIN sources src ON src.id=m.source_id
                 WHERE ss.project_id=?1 AND ss.study_id=?2
                   AND ss.kind='transcript' AND ss.reference_id=?3
                   AND ss.transcript_id=?3 AND src.project_id=?1
                   AND st.archived_at IS NULL AND p.archived_at IS NULL
                 LIMIT 1",
                params![
                    project_id.to_string(),
                    study_id.to_string(),
                    transcript_id.to_string()
                ],
                |row| row.get(0),
            )
            .optional()
            .map_err(db_error)?;
        bytes
            .and_then(|value| u64::try_from(value).ok())
            .ok_or_else(|| {
                RepositoryError::NotFound("active Study attachment for transcript".into())
            })
    }

    pub fn list_study_sources(
        &self,
        project_id: Uuid,
        study_id: Uuid,
    ) -> RepositoryResult<Vec<StudySource>> {
        self.get_study(project_id, study_id)?;
        let db = database(self)?;
        let sql = format!(
            "SELECT {SOURCE_COLUMNS} FROM study_sources
             WHERE project_id=?1 AND study_id=?2 ORDER BY created_at, id"
        );
        let mut stmt = db.prepare(&sql).map_err(db_error)?;
        let rows = stmt
            .query_map(
                params![project_id.to_string(), study_id.to_string()],
                source_row,
            )
            .map_err(db_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_error)
    }
}

fn evidence_row(row: &Row<'_>) -> rusqlite::Result<EvidenceReference> {
    let certainty: String = row.get(7)?;
    Ok(EvidenceReference {
        id: uuid_at(row, 0)?,
        project_id: uuid_at(row, 1)?,
        study_id: uuid_at(row, 2)?,
        input: EvidenceInput {
            study_source_id: uuid_at(row, 3)?,
            claim: row.get(4)?,
            locator: row.get(5)?,
            timestamp_seconds: row.get(6)?,
            certainty: EvidenceCertainty::parse(&certainty).ok_or(rusqlite::Error::InvalidQuery)?,
            uncertainty_note: row.get(8)?,
        },
        created_at: row.get(9)?,
    })
}

impl SqliteStore {
    pub fn add_evidence_reference(
        &self,
        project_id: Uuid,
        study_id: Uuid,
        input: &EvidenceInput,
    ) -> RepositoryResult<EvidenceReference> {
        required_text("evidence claim", &input.claim, 20_000)?;
        required_text("evidence locator", &input.locator, 4000)?;
        optional_text("uncertainty note", input.uncertainty_note.as_deref(), 4000)?;
        if input
            .timestamp_seconds
            .is_some_and(|value| !(0.0..1e9).contains(&value))
        {
            return Err(RepositoryError::Validation(
                "invalid evidence timestamp".into(),
            ));
        }
        self.get_study_source(project_id, study_id, input.study_source_id)?;
        let db = database(self)?;
        require_active_study(&db, project_id, study_id)?;
        let id = Uuid::new_v4();
        db.execute(
            "INSERT INTO study_evidence (
                id,project_id,study_id,study_source_id,claim,locator,timestamp_seconds,
                certainty,uncertainty_note,created_at
             ) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
            params![
                id.to_string(),
                project_id.to_string(),
                study_id.to_string(),
                input.study_source_id.to_string(),
                input.claim,
                input.locator,
                input.timestamp_seconds,
                input.certainty.as_str(),
                input.uncertainty_note,
                now_rfc3339()
            ],
        )
        .map_err(db_error)?;
        self.get_evidence_reference(project_id, study_id, id)
    }

    pub fn get_evidence_reference(
        &self,
        project_id: Uuid,
        study_id: Uuid,
        evidence_id: Uuid,
    ) -> RepositoryResult<EvidenceReference> {
        let db = database(self)?;
        db.query_row(
            "SELECT id,project_id,study_id,study_source_id,claim,locator,
                    timestamp_seconds,certainty,uncertainty_note,created_at
             FROM study_evidence WHERE project_id=?1 AND study_id=?2 AND id=?3",
            params![
                project_id.to_string(),
                study_id.to_string(),
                evidence_id.to_string()
            ],
            evidence_row,
        )
        .optional()
        .map_err(db_error)?
        .ok_or_else(|| RepositoryError::NotFound(format!("evidence {evidence_id}")))
    }

    pub fn list_evidence_references(
        &self,
        project_id: Uuid,
        study_id: Uuid,
    ) -> RepositoryResult<Vec<EvidenceReference>> {
        self.get_study(project_id, study_id)?;
        let db = database(self)?;
        let mut stmt = db
            .prepare(
                "SELECT id,project_id,study_id,study_source_id,claim,locator,
                    timestamp_seconds,certainty,uncertainty_note,created_at
             FROM study_evidence WHERE project_id=?1 AND study_id=?2 ORDER BY created_at, id",
            )
            .map_err(db_error)?;
        let rows = stmt
            .query_map(
                params![project_id.to_string(), study_id.to_string()],
                evidence_row,
            )
            .map_err(db_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_error)
    }
}

impl SqliteStore {
    /// Explicit association from selected origin to a job. A job's final
    /// transcript remains discoverable via existing job_transcript_links.
    pub fn attach_job_to_study_source(
        &self,
        project_id: Uuid,
        study_id: Uuid,
        study_source_id: Uuid,
        job_id: Uuid,
    ) -> RepositoryResult<()> {
        self.get_study_source(project_id, study_id, study_source_id)?;
        let db = database(self)?;
        require_active_study(&db, project_id, study_id)?;
        let owns_job: bool = db
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM jobs WHERE id=?1 AND project_id=?2)",
                params![job_id.to_string(), project_id.to_string()],
                |row| row.get(0),
            )
            .map_err(db_error)?;
        if !owns_job {
            return Err(RepositoryError::NotFound(format!("job {job_id}")));
        }
        db.execute(
            "INSERT INTO study_source_jobs(project_id,study_id,study_source_id,job_id,linked_at)
             VALUES(?1,?2,?3,?4,?5)",
            params![
                project_id.to_string(),
                study_id.to_string(),
                study_source_id.to_string(),
                job_id.to_string(),
                now_rfc3339()
            ],
        )
        .map_err(db_error)?;
        Ok(())
    }

    pub fn list_study_source_job_ids(
        &self,
        project_id: Uuid,
        study_id: Uuid,
        study_source_id: Uuid,
    ) -> RepositoryResult<Vec<Uuid>> {
        self.get_study_source(project_id, study_id, study_source_id)?;
        let db = database(self)?;
        let mut stmt = db
            .prepare(
                "SELECT j.job_id FROM study_source_jobs j JOIN jobs job ON job.id=j.job_id
             WHERE j.project_id=?1 AND j.study_id=?2 AND j.study_source_id=?3
               AND job.project_id=?1 ORDER BY j.linked_at, j.job_id",
            )
            .map_err(db_error)?;
        let rows = stmt
            .query_map(
                params![
                    project_id.to_string(),
                    study_id.to_string(),
                    study_source_id.to_string()
                ],
                |row| uuid_at(row, 0),
            )
            .map_err(db_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_error)
    }
}

/// Do not cascade-delete linked evidence, jobs or draft attachments when the
/// user deselects a source in Studio.
impl SqliteStore {
    pub fn remove_study_source(
        &self,
        project_id: Uuid,
        study_id: Uuid,
        source_id: Uuid,
    ) -> RepositoryResult<()> {
        let mut db = database(self)?;
        let tx = db
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(db_error)?;
        require_active_study(&tx, project_id, study_id)?;
        let linked: i64 = tx
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM study_evidence WHERE study_source_id=?1)
                     OR EXISTS(SELECT 1 FROM draft_source_links WHERE study_source_id=?1)
                     OR EXISTS(SELECT 1 FROM study_source_jobs WHERE study_source_id=?1)",
                params![source_id.to_string()],
                |row| row.get(0),
            )
            .map_err(db_error)?;
        if linked != 0 {
            return Err(RepositoryError::Conflict(
                "cannot remove a source referenced by evidence, drafts or jobs".into(),
            ));
        }
        let changed = tx
            .execute(
                "DELETE FROM study_sources WHERE id=?1 AND project_id=?2 AND study_id=?3",
                params![
                    source_id.to_string(),
                    project_id.to_string(),
                    study_id.to_string()
                ],
            )
            .map_err(db_error)?;
        if changed != 1 {
            return Err(RepositoryError::NotFound(format!(
                "study source {source_id}"
            )));
        }
        tx.commit().map_err(db_error)
    }
}

impl SqliteStore {
    /// Lightweight project-scoped origin validation before claiming the
    /// editor's CAS token, avoiding partial writes for invalid source IDs.
    pub fn validate_study_origin(
        &self,
        project_id: Uuid,
        kind: StudySourceKind,
        source_id: Uuid,
    ) -> RepositoryResult<()> {
        let db = database(self)?;
        origin_locator(&db, project_id, kind, source_id).map(|_| ())
    }
}
