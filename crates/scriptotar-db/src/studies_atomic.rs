//! P0 atomic Content Study bundle writes. Keep all editor resources inside
//! one IMMEDIATE SQLite transaction; never claim the study CAS revision alone.
use crate::{
    studies::{
        database, db_error, fingerprint, optional_text, origin_locator, require_active_study,
        required_text,
    },
    studies_drafts::validate_brief,
    SqliteStore,
};
use rusqlite::{params, OptionalExtension, Transaction, TransactionBehavior};
use scriptotar_core::{
    now_rfc3339, CreativeBriefInput, RepositoryError, RepositoryResult, StudyEdit, StudySourceKind,
};
use std::collections::HashSet;
use uuid::Uuid;

fn insert_brief(
    tx: &Transaction<'_>,
    project: Uuid,
    study: Uuid,
    brief: &CreativeBriefInput,
    json: &str,
    now: &str,
) -> RepositoryResult<()> {
    tx.execute(
        "INSERT INTO study_briefs(
          study_id,project_id,audience,goal,language,region,platform,tone,
          duration_seconds,objective,cta,constraints_json,revision,updated_at
        ) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,1,?13)",
        params![
            study.to_string(),
            project.to_string(),
            brief.audience,
            brief.goal,
            brief.language,
            brief.region,
            brief.platform,
            brief.tone,
            brief.duration_seconds,
            brief.objective,
            brief.cta,
            json,
            now
        ],
    )
    .map_err(db_error)?;
    Ok(())
}

fn insert_draft(
    tx: &Transaction<'_>,
    project: Uuid,
    study: Uuid,
    title: &str,
    body: &str,
    now: &str,
) -> RepositoryResult<()> {
    let draft_id = Uuid::new_v4().to_string();
    tx.execute(
        "INSERT INTO creative_drafts(
          id,project_id,study_id,title,body,revision,created_at,updated_at
        ) VALUES(?1,?2,?3,?4,?5,1,?6,?6)",
        params![
            draft_id,
            project.to_string(),
            study.to_string(),
            title,
            body,
            now
        ],
    )
    .map_err(db_error)?;
    tx.execute(
        "INSERT INTO draft_versions(id,draft_id,version,title,body,saved_at)
         VALUES(?1,?2,1,?3,?4,?5)",
        params![Uuid::new_v4().to_string(), draft_id, title, body, now],
    )
    .map_err(db_error)?;
    Ok(())
}

impl SqliteStore {
    /// The initial study, validated brief, initial draft, and version zero
    /// must either all exist or none exist, including on SQL failure.
    pub fn create_studio_bundle(
        &self,
        project: Uuid,
        title: &str,
        goal: &str,
        brief: &CreativeBriefInput,
    ) -> RepositoryResult<Uuid> {
        required_text("study title", title, 200)?;
        optional_text("study goal", Some(goal), 20_000)?;
        let json = validate_brief(brief)?;
        let mut connection = database(self)?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(db_error)?;
        let study = Uuid::new_v4();
        let now = now_rfc3339();
        let changed = tx
            .execute(
                "INSERT INTO content_studies(
               id,project_id,title,goal,status,revision,created_at,updated_at
             ) SELECT ?1,id,?2,?3,'active',1,?4,?4 FROM projects
               WHERE id=?5 AND archived_at IS NULL",
                params![
                    study.to_string(),
                    title.trim(),
                    goal,
                    now,
                    project.to_string()
                ],
            )
            .map_err(db_error)?;
        if changed != 1 {
            return Err(RepositoryError::NotFound(format!(
                "active project {project}"
            )));
        }
        insert_brief(&tx, project, study, brief, &json, &now)?;
        insert_draft(&tx, project, study, title.trim(), "", &now)?;
        tx.commit().map_err(db_error)?;
        Ok(study)
    }

    /// Apply study CAS, optional brief, source selection and immutable draft
    /// version as one database transaction. A rejected attachment, linked
    /// source or invalid draft body cannot consume the editor revision.
    pub fn save_studio_bundle(
        &self,
        project: Uuid,
        study: Uuid,
        expected_revision: i64,
        edit: &StudyEdit,
        brief: &CreativeBriefInput,
        brief_changed: bool,
        sources: &[(StudySourceKind, Uuid)],
        draft: &str,
    ) -> RepositoryResult<()> {
        required_text("study title", &edit.title, 200)?;
        optional_text("study goal", Some(&edit.goal), 20_000)?;
        optional_text("draft body", Some(draft), 2_000_000)?;
        let json = validate_brief(brief)?;
        if expected_revision < 1 || sources.len() > 100 {
            return Err(RepositoryError::Validation(
                "invalid revision or source count".into(),
            ));
        }
        let mut desired = HashSet::new();
        for (kind, origin) in sources {
            if !matches!(
                kind,
                StudySourceKind::ResearchItem | StudySourceKind::Transcript
            ) {
                return Err(RepositoryError::Validation(
                    "unsupported Studio source kind".into(),
                ));
            }
            if !desired.insert((kind.as_str().to_owned(), origin.to_string())) {
                return Err(RepositoryError::Validation(
                    "duplicate Studio source".into(),
                ));
            }
        }

        let mut connection = database(self)?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(db_error)?;
        require_active_study(&tx, project, study)?;
        let now = now_rfc3339();
        let updated = tx
            .execute(
                "UPDATE content_studies
             SET title=?1,goal=?2,status=?3,revision=revision+1,updated_at=?4
             WHERE id=?5 AND project_id=?6 AND revision=?7 AND archived_at IS NULL
               AND EXISTS(SELECT 1 FROM projects p WHERE p.id=?6 AND p.archived_at IS NULL)",
                params![
                    edit.title.trim(),
                    edit.goal,
                    edit.status.as_str(),
                    now,
                    study.to_string(),
                    project.to_string(),
                    expected_revision
                ],
            )
            .map_err(db_error)?;
        if updated != 1 {
            return Err(RepositoryError::Conflict(
                "study was changed by another editor".into(),
            ));
        }

        if brief_changed {
            let count = tx
                .execute(
                    "UPDATE study_briefs SET
                   audience=?1,goal=?2,language=?3,region=?4,platform=?5,tone=?6,
                   duration_seconds=?7,objective=?8,cta=?9,constraints_json=?10,
                   revision=revision+1,updated_at=?11
                 WHERE study_id=?12 AND project_id=?13",
                    params![
                        brief.audience,
                        brief.goal,
                        brief.language,
                        brief.region,
                        brief.platform,
                        brief.tone,
                        brief.duration_seconds,
                        brief.objective,
                        brief.cta,
                        json,
                        now,
                        study.to_string(),
                        project.to_string()
                    ],
                )
                .map_err(db_error)?;
            if count == 0 {
                insert_brief(&tx, project, study, brief, &json, &now)?;
            }
        }

        // Only the two source kinds exposed by Studio participate in its
        // selection. Other kinds, including manual notes and AI runs, remain
        // intact. Detached provenance tombstones must remain untouched.
        let current = {
            let mut statement = tx
                .prepare(
                    "SELECT id,kind,reference_id FROM study_sources
                 WHERE project_id=?1 AND study_id=?2
                   AND ((kind='research_item' AND research_id IS NOT NULL)
                     OR (kind='transcript' AND transcript_id IS NOT NULL))",
                )
                .map_err(db_error)?;
            let rows = statement
                .query_map(params![project.to_string(), study.to_string()], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                })
                .map_err(db_error)?;
            rows.collect::<Result<Vec<_>, _>>().map_err(db_error)?
        };
        let existing: HashSet<(String, String)> = current
            .iter()
            .map(|(_, kind, origin)| (kind.clone(), origin.clone()))
            .collect();

        for (source_id, kind, origin) in current {
            if desired.contains(&(kind, origin)) {
                continue;
            }
            let removed = tx
                .execute(
                    "DELETE FROM study_sources
                 WHERE id=?1 AND project_id=?2 AND study_id=?3
                   AND NOT EXISTS(SELECT 1 FROM study_evidence WHERE study_source_id=?1)
                   AND NOT EXISTS(SELECT 1 FROM study_source_jobs WHERE study_source_id=?1)
                   AND NOT EXISTS(SELECT 1 FROM draft_source_links WHERE study_source_id=?1)",
                    params![source_id, project.to_string(), study.to_string()],
                )
                .map_err(db_error)?;
            if removed != 1 {
                return Err(RepositoryError::Conflict(
                    "study source is referenced by evidence, a job or a draft".into(),
                ));
            }
        }

        for (kind, origin) in sources {
            if existing.contains(&(kind.as_str().to_owned(), origin.to_string())) {
                continue;
            }
            let locator = origin_locator(&tx, project, *kind, *origin)?;
            required_text("source locator", &locator, 4096)?;
            let (research_id, transcript_id) = match kind {
                StudySourceKind::ResearchItem => (Some(origin.to_string()), None),
                StudySourceKind::Transcript => (None, Some(origin.to_string())),
                _ => unreachable!("validated source kinds"),
            };
            tx.execute(
                "INSERT INTO study_sources (
                   id,project_id,study_id,kind,reference_id,research_id,legacy_source_id,
                   media_id,transcript_id,job_id,ai_run_id,locator,manual_text,fingerprint,
                   source_revision,collected_at,uncertainty,created_at
                 ) VALUES(
                   ?1,?2,?3,?4,?5,?6,NULL,NULL,?7,NULL,NULL,?8,NULL,?9,NULL,?10,NULL,?10
                 )",
                params![
                    Uuid::new_v4().to_string(),
                    project.to_string(),
                    study.to_string(),
                    kind.as_str(),
                    origin.to_string(),
                    research_id,
                    transcript_id,
                    locator,
                    fingerprint(*kind, &origin.to_string()),
                    now
                ],
            )
            .map_err(db_error)?;
        }

        let previous: Option<(String, i64, String, String)> = tx
            .query_row(
                "SELECT id,revision,title,body FROM creative_drafts
                 WHERE project_id=?1 AND study_id=?2 AND archived_at IS NULL
                 ORDER BY updated_at DESC,id DESC LIMIT 1",
                params![project.to_string(), study.to_string()],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .optional()
            .map_err(db_error)?;
        if let Some((id, revision, title, body)) = previous {
            if title != edit.title.trim() || body != draft {
                let count = tx
                    .execute(
                        "UPDATE creative_drafts
                     SET title=?1,body=?2,revision=revision+1,updated_at=?3
                     WHERE id=?4 AND project_id=?5 AND revision=?6 AND archived_at IS NULL",
                        params![
                            edit.title.trim(),
                            draft,
                            now,
                            id,
                            project.to_string(),
                            revision
                        ],
                    )
                    .map_err(db_error)?;
                if count != 1 {
                    return Err(RepositoryError::Conflict(
                        "draft changed during save".into(),
                    ));
                }
                tx.execute(
                    "INSERT INTO draft_versions(id,draft_id,version,title,body,saved_at)
                     VALUES(?1,?2,?3,?4,?5,?6)",
                    params![
                        Uuid::new_v4().to_string(),
                        id,
                        revision + 1,
                        edit.title.trim(),
                        draft,
                        now
                    ],
                )
                .map_err(db_error)?;
            }
        } else {
            insert_draft(&tx, project, study, edit.title.trim(), draft, &now)?;
        }
        tx.commit().map_err(db_error)
    }
}
