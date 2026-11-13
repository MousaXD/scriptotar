//! Project-scoped, bounded exact-item reads for legacy persisted artifacts.
//!
//! Agent 01 owns DB crate changes. Keep this narrow read-only adapter until
//! the canonical DB repository exposes a single-record AI-run lookup.

use std::time::Duration;

use rusqlite::{params, Connection, OpenFlags, OptionalExtension};
use scriptotar_core::{AiRun, AiRunMode, RepositoryError, RepositoryResult};
use scriptotar_db::SqliteStore;
use uuid::Uuid;

fn read_connection(store: &SqliteStore) -> RepositoryResult<Connection> {
    let connection = Connection::open_with_flags(store.path(), OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|_| RepositoryError::Storage("artifact database is unavailable".to_owned()))?;
    connection
        .busy_timeout(Duration::from_secs(5))
        .map_err(|_| RepositoryError::Storage("artifact database is busy".to_owned()))?;
    Ok(connection)
}

pub(super) fn read_ai_run(
    store: &SqliteStore,
    project_id: Uuid,
    run_id: Uuid,
) -> RepositoryResult<AiRun> {
    let connection = read_connection(store)?;
    connection
        .query_row(
            "SELECT task, mode, provider, model, prompt, result, created_at
             FROM ai_runs WHERE project_id = ?1 AND id = ?2",
            params![project_id.to_string(), run_id.to_string()],
            |row| {
                let mode: String = row.get(1)?;
                let mode = match mode.as_str() {
                    "copy_prompt" => AiRunMode::CopyPrompt,
                    "byok" => AiRunMode::Byok,
                    _ => return Err(rusqlite::Error::InvalidQuery),
                };
                Ok(AiRun {
                    id: run_id,
                    project_id,
                    task: row.get(0)?,
                    mode,
                    provider: row.get(2)?,
                    model: row.get(3)?,
                    prompt: row.get(4)?,
                    result: row.get(5)?,
                    created_at: row.get(6)?,
                })
            },
        )
        .optional()
        .map_err(|_| RepositoryError::Storage("AI run detail could not be loaded".to_owned()))?
        .ok_or_else(|| RepositoryError::NotFound(format!("AI run {run_id} in project")))
}

pub(super) fn is_research_source_queued(
    store: &SqliteStore,
    project_id: Uuid,
    source_url: &str,
) -> RepositoryResult<bool> {
    let connection = read_connection(store)?;
    connection
        .query_row(
            "SELECT EXISTS(
                SELECT 1 FROM jobs
                WHERE project_id = ?1 AND input_kind = 'url' AND input_value = ?2
                  AND state IN ('queued', 'preparing', 'downloading', 'transcribing', 'processing')
             )",
            params![project_id.to_string(), source_url],
            |row| row.get::<_, bool>(0),
        )
        .map_err(|_| RepositoryError::Storage("research queue status could not be loaded".to_owned()))
}
