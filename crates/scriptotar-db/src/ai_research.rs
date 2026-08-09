use std::time::Duration;

use rusqlite::{params, Connection, OptionalExtension, Row, TransactionBehavior};
use scriptotar_core::{
    AiRun, AiRunMode, AiRunRepository, Creator, ProjectRepository, RepositoryError, RepositoryResult,
    ResearchItem, ResearchRepository,
};
use uuid::Uuid;

use crate::SqliteStore;

const MAX_URL_CHARS: usize = 2_048;
const MAX_TITLE_CHARS: usize = 2_000;
const MAX_RAW_JSON_CHARS: usize = 64_000;
const MAX_PROMPT_CHARS: usize = 500_000;
const MAX_RESULT_CHARS: usize = 2_000_000;

fn storage_error(error: rusqlite::Error) -> RepositoryError {
    RepositoryError::Storage(error.to_string())
}

fn open_connection(store: &SqliteStore) -> RepositoryResult<Connection> {
    let connection = Connection::open(store.path()).map_err(storage_error)?;
    connection
        .pragma_update(None, "foreign_keys", true)
        .map_err(storage_error)?;
    connection
        .busy_timeout(Duration::from_secs(5))
        .map_err(storage_error)?;
    Ok(connection)
}

fn validate_text(field: &str, value: &str, max_chars: usize) -> RepositoryResult<()> {
    if value.trim().is_empty() {
        return Err(RepositoryError::Validation(format!("{field} cannot be empty")));
    }
    if value.chars().count() > max_chars {
        return Err(RepositoryError::Validation(format!("{field} is too long")));
    }
    Ok(())
}

fn validate_optional_text(field: &str, value: Option<&str>, max_chars: usize) -> RepositoryResult<()> {
    if value.is_some_and(|value| value.chars().count() > max_chars) {
        return Err(RepositoryError::Validation(format!("{field} is too long")));
    }
    Ok(())
}

fn parse_uuid(value: String) -> rusqlite::Result<Uuid> {
    Uuid::parse_str(&value).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(
            0,
            rusqlite::types::Type::Text,
            Box::new(error),
        )
    })
}

fn row_creator(row: &Row<'_>) -> rusqlite::Result<Creator> {
    Ok(Creator {
        id: parse_uuid(row.get(0)?)?,
        project_id: parse_uuid(row.get(1)?)?,
        platform: row.get(2)?,
        profile_url: row.get(3)?,
        display_name: row.get(4)?,
        created_at: row.get(5)?,
    })
}

fn row_research(row: &Row<'_>) -> rusqlite::Result<ResearchItem> {
    Ok(ResearchItem {
        id: parse_uuid(row.get(0)?)?,
        project_id: parse_uuid(row.get(1)?)?,
        creator_id: row
            .get::<_, Option<String>>(2)?
            .map(parse_uuid)
            .transpose()?,
        source_url: row.get(3)?,
        platform: row.get(4)?,
        title: row.get(5)?,
        view_count: row.get(6)?,
        like_count: row.get(7)?,
        comment_count: row.get(8)?,
        published_at: row.get(9)?,
        duration_seconds: row.get(10)?,
        raw_json: row.get(11)?,
        scanned_at: row.get(12)?,
    })
}

fn ai_mode_text(mode: AiRunMode) -> &'static str {
    match mode {
        AiRunMode::CopyPrompt => "copy_prompt",
        AiRunMode::Byok => "byok",
    }
}

fn row_ai_run(row: &Row<'_>) -> rusqlite::Result<AiRun> {
    let mode: String = row.get(3)?;
    let mode = match mode.as_str() {
        "copy_prompt" => AiRunMode::CopyPrompt,
        "byok" => AiRunMode::Byok,
        other => {
            return Err(rusqlite::Error::FromSqlConversionFailure(
                3,
                rusqlite::types::Type::Text,
                Box::new(RepositoryError::Storage(format!(
                    "unknown persisted AI run mode: {other}"
                ))),
            ))
        }
    };
    Ok(AiRun {
        id: parse_uuid(row.get(0)?)?,
        project_id: parse_uuid(row.get(1)?)?,
        task: row.get(2)?,
        mode,
        provider: row.get(4)?,
        model: row.get(5)?,
        prompt: row.get(6)?,
        result: row.get(7)?,
        created_at: row.get(8)?,
    })
}

impl ResearchRepository for SqliteStore {
    fn upsert_creator(&self, creator: &Creator) -> RepositoryResult<Creator> {
        self.get_project(creator.project_id)?;
        validate_text("creator platform", &creator.platform, 64)?;
        validate_text("creator profile URL", &creator.profile_url, MAX_URL_CHARS)?;
        validate_optional_text("creator display name", creator.display_name.as_deref(), 512)?;
        let connection = open_connection(self)?;
        let existing = connection
            .query_row(
                "SELECT id, project_id, platform, profile_url, display_name, created_at
                 FROM creators WHERE project_id = ?1 AND profile_url = ?2",
                params![creator.project_id.to_string(), creator.profile_url],
                row_creator,
            )
            .optional()
            .map_err(storage_error)?;
        if let Some(existing) = existing {
            connection
                .execute(
                    "UPDATE creators SET platform = ?1, display_name = COALESCE(?2, display_name)
                     WHERE id = ?3",
                    params![creator.platform, creator.display_name, existing.id.to_string()],
                )
                .map_err(storage_error)?;
            return connection
                .query_row(
                    "SELECT id, project_id, platform, profile_url, display_name, created_at
                     FROM creators WHERE id = ?1",
                    params![existing.id.to_string()],
                    row_creator,
                )
                .map_err(storage_error);
        }
        connection
            .execute(
                "INSERT INTO creators(id, project_id, platform, profile_url, display_name, created_at)
                 VALUES(?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    creator.id.to_string(),
                    creator.project_id.to_string(),
                    creator.platform,
                    creator.profile_url,
                    creator.display_name,
                    creator.created_at,
                ],
            )
            .map_err(storage_error)?;
        Ok(creator.clone())
    }

    fn list_creators(&self, project_id: Option<Uuid>) -> RepositoryResult<Vec<Creator>> {
        let connection = open_connection(self)?;
        let sql = if project_id.is_some() {
            "SELECT id, project_id, platform, profile_url, display_name, created_at
             FROM creators WHERE project_id = ?1 ORDER BY created_at DESC"
        } else {
            "SELECT id, project_id, platform, profile_url, display_name, created_at
             FROM creators ORDER BY created_at DESC"
        };
        let mut statement = connection.prepare(sql).map_err(storage_error)?;
        let rows = if let Some(project_id) = project_id {
            statement
                .query_map(params![project_id.to_string()], row_creator)
                .map_err(storage_error)?
        } else {
            statement.query_map([], row_creator).map_err(storage_error)?
        };
        rows.collect::<Result<Vec<_>, _>>().map_err(storage_error)
    }

    fn upsert_research_items(&self, items: &[ResearchItem]) -> RepositoryResult<Vec<ResearchItem>> {
        if items.is_empty() {
            return Ok(Vec::new());
        }
        let mut connection = open_connection(self)?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(storage_error)?;
        let mut persisted = Vec::with_capacity(items.len());
        for item in items {
            validate_text("research source URL", &item.source_url, MAX_URL_CHARS)?;
            validate_text("research platform", &item.platform, 64)?;
            validate_optional_text("research title", item.title.as_deref(), MAX_TITLE_CHARS)?;
            validate_optional_text("research raw metadata", item.raw_json.as_deref(), MAX_RAW_JSON_CHARS)?;
            let existing_id = tx
                .query_row(
                    "SELECT id FROM research_items
                     WHERE project_id = ?1 AND source_url = ?2
                     ORDER BY scanned_at DESC LIMIT 1",
                    params![item.project_id.to_string(), item.source_url],
                    |row| row.get::<_, String>(0),
                )
                .optional()
                .map_err(storage_error)?;
            let id = existing_id
                .map(|value| Uuid::parse_str(&value).map_err(|error| RepositoryError::Storage(error.to_string())))
                .transpose()?
                .unwrap_or(item.id);
            if id == item.id && existing_id.is_none() {
                tx.execute(
                    "INSERT INTO research_items(
                        id, project_id, creator_id, source_url, platform, title, view_count,
                        like_count, comment_count, published_at, duration_seconds, raw_json, scanned_at
                     ) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
                    params![
                        id.to_string(),
                        item.project_id.to_string(),
                        item.creator_id.map(|id| id.to_string()),
                        item.source_url,
                        item.platform,
                        item.title,
                        item.view_count,
                        item.like_count,
                        item.comment_count,
                        item.published_at,
                        item.duration_seconds,
                        item.raw_json,
                        item.scanned_at,
                    ],
                )
                .map_err(storage_error)?;
            } else {
                tx.execute(
                    "UPDATE research_items SET
                        creator_id = ?1, platform = ?2, title = ?3, view_count = ?4,
                        like_count = ?5, comment_count = ?6, published_at = ?7,
                        duration_seconds = ?8, raw_json = ?9, scanned_at = ?10
                     WHERE id = ?11",
                    params![
                        item.creator_id.map(|id| id.to_string()),
                        item.platform,
                        item.title,
                        item.view_count,
                        item.like_count,
                        item.comment_count,
                        item.published_at,
                        item.duration_seconds,
                        item.raw_json,
                        item.scanned_at,
                        id.to_string(),
                    ],
                )
                .map_err(storage_error)?;
            }
            let mut stored = item.clone();
            stored.id = id;
            persisted.push(stored);
        }
        tx.commit().map_err(storage_error)?;
        Ok(persisted)
    }

    fn list_research_items(&self, project_id: Option<Uuid>) -> RepositoryResult<Vec<ResearchItem>> {
        let connection = open_connection(self)?;
        let sql = if project_id.is_some() {
            "SELECT id, project_id, creator_id, source_url, platform, title, view_count,
                    like_count, comment_count, published_at, duration_seconds, raw_json, scanned_at
             FROM research_items WHERE project_id = ?1 ORDER BY scanned_at DESC"
        } else {
            "SELECT id, project_id, creator_id, source_url, platform, title, view_count,
                    like_count, comment_count, published_at, duration_seconds, raw_json, scanned_at
             FROM research_items ORDER BY scanned_at DESC"
        };
        let mut statement = connection.prepare(sql).map_err(storage_error)?;
        let rows = if let Some(project_id) = project_id {
            statement
                .query_map(params![project_id.to_string()], row_research)
                .map_err(storage_error)?
        } else {
            statement.query_map([], row_research).map_err(storage_error)?
        };
        rows.collect::<Result<Vec<_>, _>>().map_err(storage_error)
    }

    fn get_research_items(&self, ids: &[Uuid]) -> RepositoryResult<Vec<ResearchItem>> {
        let connection = open_connection(self)?;
        let mut items = Vec::with_capacity(ids.len());
        for id in ids {
            let item = connection
                .query_row(
                    "SELECT id, project_id, creator_id, source_url, platform, title, view_count,
                            like_count, comment_count, published_at, duration_seconds, raw_json, scanned_at
                     FROM research_items WHERE id = ?1",
                    params![id.to_string()],
                    row_research,
                )
                .optional()
                .map_err(storage_error)?
                .ok_or_else(|| RepositoryError::NotFound(format!("research item {id}")))?;
            items.push(item);
        }
        Ok(items)
    }
}

impl AiRunRepository for SqliteStore {
    fn insert_ai_run(&self, run: &AiRun) -> RepositoryResult<()> {
        self.get_project(run.project_id)?;
        validate_text("AI task", &run.task, 256)?;
        validate_text("AI prompt", &run.prompt, MAX_PROMPT_CHARS)?;
        validate_optional_text("AI provider", run.provider.as_deref(), 128)?;
        validate_optional_text("AI model", run.model.as_deref(), 256)?;
        validate_optional_text("AI result", run.result.as_deref(), MAX_RESULT_CHARS)?;
        let connection = open_connection(self)?;
        connection
            .execute(
                "INSERT INTO ai_runs(
                    id, project_id, task, mode, provider, model, prompt, result, created_at
                 ) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    run.id.to_string(),
                    run.project_id.to_string(),
                    run.task,
                    ai_mode_text(run.mode),
                    run.provider,
                    run.model,
                    run.prompt,
                    run.result,
                    run.created_at,
                ],
            )
            .map_err(storage_error)?;
        Ok(())
    }

    fn list_ai_runs(&self, project_id: Option<Uuid>) -> RepositoryResult<Vec<AiRun>> {
        let connection = open_connection(self)?;
        let sql = if project_id.is_some() {
            "SELECT id, project_id, task, mode, provider, model, prompt, result, created_at
             FROM ai_runs WHERE project_id = ?1 ORDER BY created_at DESC"
        } else {
            "SELECT id, project_id, task, mode, provider, model, prompt, result, created_at
             FROM ai_runs ORDER BY created_at DESC"
        };
        let mut statement = connection.prepare(sql).map_err(storage_error)?;
        let rows = if let Some(project_id) = project_id {
            statement
                .query_map(params![project_id.to_string()], row_ai_run)
                .map_err(storage_error)?
        } else {
            statement.query_map([], row_ai_run).map_err(storage_error)?
        };
        rows.collect::<Result<Vec<_>, _>>().map_err(storage_error)
    }
}

#[cfg(test)]
mod tests {
    use scriptotar_core::{now_rfc3339, Project};
    use tempfile::tempdir;

    use super::*;

    fn setup() -> (tempfile::TempDir, SqliteStore, Project) {
        let temp = tempdir().unwrap();
        let store = SqliteStore::open(temp.path().join("scriptotar.sqlite3")).unwrap();
        store.run_integration_migrations().unwrap();
        let project = Project::new("Research");
        store.create_project(&project).unwrap();
        (temp, store, project)
    }

    #[test]
    fn creator_and_research_upserts_are_project_scoped_and_deduplicated() {
        let (_temp, store, project) = setup();
        let creator = Creator {
            id: Uuid::new_v4(),
            project_id: project.id,
            platform: "YouTube".to_owned(),
            profile_url: "https://www.youtube.com/@creator".to_owned(),
            display_name: Some("Creator".to_owned()),
            created_at: now_rfc3339(),
        };
        let first_creator = store.upsert_creator(&creator).unwrap();
        let mut renamed = creator.clone();
        renamed.id = Uuid::new_v4();
        renamed.display_name = Some("Creator renamed".to_owned());
        let second_creator = store.upsert_creator(&renamed).unwrap();
        assert_eq!(first_creator.id, second_creator.id);
        assert_eq!(second_creator.display_name.as_deref(), Some("Creator renamed"));

        let first = ResearchItem {
            id: Uuid::new_v4(),
            project_id: project.id,
            creator_id: Some(first_creator.id),
            source_url: "https://www.youtube.com/watch?v=abc".to_owned(),
            platform: "YouTube".to_owned(),
            title: Some("First".to_owned()),
            view_count: Some(10),
            like_count: Some(1),
            comment_count: Some(0),
            published_at: Some("2026-08-09".to_owned()),
            duration_seconds: Some(12.0),
            raw_json: None,
            scanned_at: now_rfc3339(),
        };
        let first_saved = store.upsert_research_items(&[first.clone()]).unwrap();
        let mut refreshed = first;
        refreshed.id = Uuid::new_v4();
        refreshed.title = Some("Refreshed".to_owned());
        refreshed.view_count = Some(25);
        let second_saved = store.upsert_research_items(&[refreshed]).unwrap();
        assert_eq!(first_saved[0].id, second_saved[0].id);
        let rows = store.list_research_items(Some(project.id)).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].title.as_deref(), Some("Refreshed"));
        assert_eq!(rows[0].view_count, Some(25));
    }

    #[test]
    fn ai_runs_persist_without_any_key_field() {
        let (temp, store, project) = setup();
        let run = AiRun {
            id: Uuid::new_v4(),
            project_id: project.id,
            task: "Hook ideas".to_owned(),
            mode: AiRunMode::Byok,
            provider: Some("OpenAI".to_owned()),
            model: Some("gpt-test".to_owned()),
            prompt: "prompt".to_owned(),
            result: Some("result".to_owned()),
            created_at: now_rfc3339(),
        };
        store.insert_ai_run(&run).unwrap();
        drop(store);
        let reopened = SqliteStore::open(temp.path().join("scriptotar.sqlite3")).unwrap();
        reopened.run_integration_migrations().unwrap();
        let rows = reopened.list_ai_runs(Some(project.id)).unwrap();
        assert_eq!(rows, vec![run]);
        let serialized = serde_json::to_string(&rows).unwrap().to_ascii_lowercase();
        assert!(!serialized.contains("api_key"));
        assert!(!serialized.contains("apikey"));
    }
}
