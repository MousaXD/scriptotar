//! Creative briefs and crash-safe immutable draft version snapshots.
//! All operations require the active project ID; every mutation is transactional.

use rusqlite::{params, OptionalExtension, Row, TransactionBehavior};
use scriptotar_core::{
    now_rfc3339, CreativeBrief, CreativeBriefInput, CreativeDraft, DraftVersion,
    RepositoryError, RepositoryResult,
};
use uuid::Uuid;

use crate::{studies::{database, db_error, optional_text, require_active_study, required_text, uuid_at}, SqliteStore};

fn validate_brief(input: &CreativeBriefInput) -> RepositoryResult<String> {
    for (field, value, max) in [
        ("audience", input.audience.as_str(), 4000),
        ("goal", input.goal.as_str(), 20_000),
        ("language", input.language.as_str(), 120),
        ("platform", input.platform.as_str(), 120),
        ("tone", input.tone.as_str(), 1000),
        ("objective", input.objective.as_str(), 4000),
    ] {
        required_text(field, value, max)?;
    }
    optional_text("region", input.region.as_deref(), 240)?;
    optional_text("CTA", input.cta.as_deref(), 4000)?;
    if input.duration_seconds.is_some_and(|seconds| seconds == 0 || seconds > 86_400) {
        return Err(RepositoryError::Validation("duration must be in 1..=86400 seconds".into()));
    }
    if input.constraints.len() > 100 {
        return Err(RepositoryError::Validation("too many brief constraints".into()));
    }
    for constraint in &input.constraints {
        required_text("brief constraint", constraint, 4000)?;
    }
    let constraints = serde_json::to_string(&input.constraints)
        .map_err(|err| RepositoryError::Validation(err.to_string()))?;
    optional_text("serialized brief constraints", Some(&constraints), 100_000)?;
    Ok(constraints)
}

fn brief_row(row: &Row<'_>) -> rusqlite::Result<CreativeBrief> {
    let raw: String = row.get(11)?;
    let constraints: Vec<String> =
        serde_json::from_str(&raw).map_err(|err| crate::studies::sql_conversion(11, err))?;
    Ok(CreativeBrief {
        study_id: uuid_at(row, 0)?,
        project_id: uuid_at(row, 1)?,
        input: CreativeBriefInput {
            audience: row.get(2)?,
            goal: row.get(3)?,
            language: row.get(4)?,
            region: row.get(5)?,
            platform: row.get(6)?,
            tone: row.get(7)?,
            duration_seconds: row.get(8)?,
            objective: row.get(9)?,
            cta: row.get(10)?,
            constraints,
        },
        revision: row.get(12)?,
        updated_at: row.get(13)?,
    })
}

impl SqliteStore {
    /// For first save pass expected_revision=0; all subsequent saves use the
    /// revision returned by the previous successful commit.
    pub fn save_creative_brief(
        &self,
        project_id: Uuid,
        study_id: Uuid,
        expected_revision: i64,
        input: &CreativeBriefInput,
    ) -> RepositoryResult<CreativeBrief> {
        if expected_revision < 0 {
            return Err(RepositoryError::Validation("negative revision".into()));
        }
        let constraints_json = validate_brief(input)?;
        let mut db = database(self)?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate).map_err(db_error)?;
        require_active_study(&tx, project_id, study_id)?;
        let old: Option<i64> = tx
            .query_row(
                "SELECT revision FROM study_briefs WHERE study_id=?1 AND project_id=?2",
                params![study_id.to_string(), project_id.to_string()],
                |row| row.get(0),
            )
            .optional()
            .map_err(db_error)?;
        if old.unwrap_or(0) != expected_revision {
            return Err(RepositoryError::Conflict("creative brief has a newer revision".into()));
        }
        let now = now_rfc3339();
        match old {
            None => {
                tx.execute(
                    "INSERT INTO study_briefs(
                        study_id,project_id,audience,goal,language,region,platform,tone,
                        duration_seconds,objective,cta,constraints_json,revision,updated_at
                     ) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,1,?13)",
                    params![
                        study_id.to_string(), project_id.to_string(), input.audience,
                        input.goal, input.language, input.region, input.platform, input.tone,
                        input.duration_seconds, input.objective, input.cta, constraints_json, now,
                    ],
                )
                .map_err(db_error)?;
            }
            Some(_) => {
                let changed = tx.execute(
                    "UPDATE study_briefs SET
                        audience=?1,goal=?2,language=?3,region=?4,platform=?5,tone=?6,
                        duration_seconds=?7,objective=?8,cta=?9,constraints_json=?10,
                        revision=revision+1,updated_at=?11
                     WHERE study_id=?12 AND project_id=?13 AND revision=?14",
                    params![
                        input.audience, input.goal, input.language, input.region, input.platform,
                        input.tone, input.duration_seconds, input.objective, input.cta,
                        constraints_json, now, study_id.to_string(), project_id.to_string(),
                        expected_revision,
                    ],
                ).map_err(db_error)?;
                if changed != 1 {
                    return Err(RepositoryError::Conflict("creative brief update conflict".into()));
                }
            }
        }
        tx.commit().map_err(db_error)?;
        self.get_creative_brief(project_id, study_id)?
            .ok_or_else(|| RepositoryError::Storage("saved brief missing".into()))
    }

    pub fn get_creative_brief(
        &self,
        project_id: Uuid,
        study_id: Uuid,
    ) -> RepositoryResult<Option<CreativeBrief>> {
        self.get_study(project_id, study_id)?;
        let db = database(self)?;
        db.query_row(
            "SELECT study_id,project_id,audience,goal,language,region,platform,tone,
                    duration_seconds,objective,cta,constraints_json,revision,updated_at
             FROM study_briefs WHERE study_id=?1 AND project_id=?2",
            params![study_id.to_string(), project_id.to_string()],
            brief_row,
        )
        .optional()
        .map_err(db_error)
    }
}

fn draft_row(row: &Row<'_>) -> rusqlite::Result<CreativeDraft> {
    Ok(CreativeDraft {
        id: uuid_at(row, 0)?,
        project_id: uuid_at(row, 1)?,
        study_id: uuid_at(row, 2)?,
        title: row.get(3)?,
        body: row.get(4)?,
        revision: row.get(5)?,
        created_at: row.get(6)?,
        updated_at: row.get(7)?,
        archived_at: row.get(8)?,
    })
}

fn version_row(row: &Row<'_>) -> rusqlite::Result<DraftVersion> {
    Ok(DraftVersion {
        id: uuid_at(row, 0)?,
        draft_id: uuid_at(row, 1)?,
        version: row.get(2)?,
        title: row.get(3)?,
        body: row.get(4)?,
        saved_at: row.get(5)?,
    })
}

const DRAFT_COLUMNS: &str =
    "id,project_id,study_id,title,body,revision,created_at,updated_at,archived_at";

impl SqliteStore {
    pub fn create_draft(
        &self,
        project_id: Uuid,
        study_id: Uuid,
        title: &str,
        body: &str,
    ) -> RepositoryResult<CreativeDraft> {
        required_text("draft title", title, 200)?;
        optional_text("draft body", Some(body), 2_000_000)?;
        let mut db = database(self)?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate).map_err(db_error)?;
        require_active_study(&tx, project_id, study_id)?;
        let id = Uuid::new_v4();
        let now = now_rfc3339();
        tx.execute(
            "INSERT INTO creative_drafts(
                id,project_id,study_id,title,body,revision,created_at,updated_at
             ) VALUES(?1,?2,?3,?4,?5,1,?6,?6)",
            params![id.to_string(), project_id.to_string(), study_id.to_string(), title.trim(), body, now],
        ).map_err(db_error)?;
        tx.execute(
            "INSERT INTO draft_versions(id,draft_id,version,title,body,saved_at)
             VALUES(?1,?2,1,?3,?4,?5)",
            params![Uuid::new_v4().to_string(), id.to_string(), title.trim(), body, now],
        ).map_err(db_error)?;
        tx.commit().map_err(db_error)?;
        self.get_draft(project_id, id)
    }

    pub fn get_draft(&self, project_id: Uuid, draft_id: Uuid) -> RepositoryResult<CreativeDraft> {
        let db = database(self)?;
        let sql = format!(
            "SELECT {DRAFT_COLUMNS} FROM creative_drafts WHERE project_id=?1 AND id=?2"
        );
        db.query_row(&sql, params![project_id.to_string(), draft_id.to_string()], draft_row)
            .optional()
            .map_err(db_error)?
            .ok_or_else(|| RepositoryError::NotFound(format!("draft {draft_id}")))
    }

    /// Every successful edit writes the next immutable version in the same
    /// SQLite IMMEDIATE transaction. Stale editors receive Conflict.
    pub fn update_draft(
        &self,
        project_id: Uuid,
        draft_id: Uuid,
        expected_revision: i64,
        title: &str,
        body: &str,
    ) -> RepositoryResult<CreativeDraft> {
        required_text("draft title", title, 200)?;
        optional_text("draft body", Some(body), 2_000_000)?;
        if expected_revision < 1 {
            return Err(RepositoryError::Validation("expected revision must be positive".into()));
        }
        let mut db = database(self)?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate).map_err(db_error)?;
        let count = tx.execute(
            "UPDATE creative_drafts SET title=?1,body=?2,revision=revision+1,updated_at=?3
             WHERE id=?4 AND project_id=?5 AND revision=?6 AND archived_at IS NULL
               AND EXISTS (
                 SELECT 1 FROM content_studies st JOIN projects p ON p.id=st.project_id
                 WHERE st.id=creative_drafts.study_id AND st.project_id=?5
                   AND st.archived_at IS NULL AND p.archived_at IS NULL
               )",
            params![title.trim(), body, now_rfc3339(), draft_id.to_string(),
                    project_id.to_string(), expected_revision],
        ).map_err(db_error)?;
        if count != 1 {
            return if tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM creative_drafts WHERE id=?1 AND project_id=?2)",
                params![draft_id.to_string(), project_id.to_string()],
                |row| row.get::<_, bool>(0),
            ).map_err(db_error)? {
                Err(RepositoryError::Conflict("draft changed or became archived".into()))
            } else {
                Err(RepositoryError::NotFound(format!("draft {draft_id}")))
            };
        }
        tx.execute(
            "INSERT INTO draft_versions(id,draft_id,version,title,body,saved_at)
             VALUES(?1,?2,?3,?4,?5,?6)",
            params![
                Uuid::new_v4().to_string(), draft_id.to_string(), expected_revision + 1,
                title.trim(), body, now_rfc3339()
            ],
        ).map_err(db_error)?;
        tx.commit().map_err(db_error)?;
        self.get_draft(project_id, draft_id)
    }

    pub fn rename_draft(
        &self,
        project_id: Uuid,
        draft_id: Uuid,
        expected_revision: i64,
        title: &str,
    ) -> RepositoryResult<CreativeDraft> {
        let draft = self.get_draft(project_id, draft_id)?;
        self.update_draft(project_id, draft_id, expected_revision, title, &draft.body)
    }

    pub fn duplicate_draft(
        &self,
        project_id: Uuid,
        draft_id: Uuid,
    ) -> RepositoryResult<CreativeDraft> {
        let draft = self.get_draft(project_id, draft_id)?;
        let title = format!("Copy of {}", draft.title).chars().take(200).collect::<String>();
        self.create_draft(project_id, draft.study_id, &title, &draft.body)
    }

    pub fn search_drafts(
        &self,
        project_id: Uuid,
        study_id: Uuid,
        query: &str,
        include_archived: bool,
        limit: usize,
    ) -> RepositoryResult<Vec<CreativeDraft>> {
        self.get_study(project_id, study_id)?;
        let db = database(self)?;
        let escaped = query.trim().replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_");
        let like = format!("%{escaped}%");
        let sql = format!(
            "SELECT {DRAFT_COLUMNS} FROM creative_drafts
             WHERE project_id=?1 AND study_id=?2 AND (?3=1 OR archived_at IS NULL)
               AND (title LIKE ?4 ESCAPE '\\' OR body LIKE ?4 ESCAPE '\\')
             ORDER BY updated_at DESC,id DESC LIMIT ?5"
        );
        let mut stmt = db.prepare(&sql).map_err(db_error)?;
        let rows = stmt.query_map(
            params![project_id.to_string(), study_id.to_string(), include_archived as i64,
                    like, limit.clamp(1, 200) as i64], draft_row
        ).map_err(db_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_error)
    }

    pub fn list_draft_versions(
        &self,
        project_id: Uuid,
        draft_id: Uuid,
    ) -> RepositoryResult<Vec<DraftVersion>> {
        self.get_draft(project_id, draft_id)?;
        let db = database(self)?;
        let mut stmt = db.prepare(
            "SELECT v.id,v.draft_id,v.version,v.title,v.body,v.saved_at
             FROM draft_versions v JOIN creative_drafts d ON d.id=v.draft_id
             WHERE d.project_id=?1 AND v.draft_id=?2 ORDER BY v.version"
        ).map_err(db_error)?;
        let rows = stmt.query_map(
            params![project_id.to_string(), draft_id.to_string()], version_row
        ).map_err(db_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_error)
    }

    pub fn archive_draft(&self, project_id: Uuid, draft_id: Uuid) -> RepositoryResult<()> {
        self.set_draft_archived(project_id, draft_id, true)
    }

    pub fn restore_draft(&self, project_id: Uuid, draft_id: Uuid) -> RepositoryResult<()> {
        self.set_draft_archived(project_id, draft_id, false)
    }

    fn set_draft_archived(
        &self,
        project_id: Uuid,
        draft_id: Uuid,
        archived: bool,
    ) -> RepositoryResult<()> {
        let mut db = database(self)?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate).map_err(db_error)?;
        let wanted = if archived { "archived_at IS NULL" } else { "archived_at IS NOT NULL" };
        let sql = format!(
            "UPDATE creative_drafts SET archived_at=?1,revision=revision+1,updated_at=?2
             WHERE project_id=?3 AND id=?4 AND {wanted}
             AND EXISTS (SELECT 1 FROM content_studies st JOIN projects p ON p.id=st.project_id
               WHERE st.id=creative_drafts.study_id AND st.archived_at IS NULL
                 AND p.archived_at IS NULL)"
        );
        let when = if archived { Some(now_rfc3339()) } else { None };
        let now = now_rfc3339();
        let changed = tx.execute(
            &sql, params![when, now, project_id.to_string(), draft_id.to_string()]
        ).map_err(db_error)?;
        if changed != 1 {
            return Err(RepositoryError::NotFound(format!(
                "draft {draft_id} is absent, unchanged or study archived"
            )));
        }
        tx.execute(
            "INSERT INTO draft_versions(id,draft_id,version,title,body,saved_at)
             SELECT ?1,id,revision,title,body,?2 FROM creative_drafts
             WHERE project_id=?3 AND id=?4",
            params![Uuid::new_v4().to_string(), now, project_id.to_string(), draft_id.to_string()],
        ).map_err(db_error)?;
        tx.commit().map_err(db_error)
    }
}


impl SqliteStore {
    pub fn attach_source_to_draft(
        &self,
        project_id: Uuid,
        draft_id: Uuid,
        study_source_id: Uuid,
    ) -> RepositoryResult<()> {
        let draft = self.get_draft(project_id, draft_id)?;
        if draft.archived_at.is_some() {
            return Err(RepositoryError::Conflict("cannot attach to archived draft".into()));
        }
        self.get_study_source(project_id, draft.study_id, study_source_id)?;
        let db = database(self)?;
        require_active_study(&db, project_id, draft.study_id)?;
        db.execute(
            "INSERT INTO draft_source_links(project_id,draft_id,study_source_id,attached_at)
             VALUES(?1,?2,?3,?4)",
            params![project_id.to_string(), draft_id.to_string(),
                    study_source_id.to_string(), now_rfc3339()],
        ).map_err(db_error)?;
        Ok(())
    }

    pub fn list_draft_source_ids(
        &self,
        project_id: Uuid,
        draft_id: Uuid,
    ) -> RepositoryResult<Vec<Uuid>> {
        self.get_draft(project_id, draft_id)?;
        let db = database(self)?;
        let mut stmt = db.prepare(
            "SELECT link.study_source_id FROM draft_source_links link
             JOIN creative_drafts d ON d.id=link.draft_id
             JOIN study_sources ss ON ss.id=link.study_source_id
             WHERE link.project_id=?1 AND link.draft_id=?2 AND d.project_id=?1
               AND ss.project_id=?1 AND ss.study_id=d.study_id
             ORDER BY link.attached_at, link.study_source_id"
        ).map_err(db_error)?;
        let rows = stmt.query_map(
            params![project_id.to_string(), draft_id.to_string()],
            |row| uuid_at(row, 0),
        ).map_err(db_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_error)
    }
}
