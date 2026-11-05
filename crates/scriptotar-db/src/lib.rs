use std::{
    path::{Path, PathBuf},
    str::FromStr,
    sync::Arc,
    time::Duration,
};

use rusqlite::{params, Connection, OptionalExtension, Row, Transaction, TransactionBehavior};
use scriptotar_core::{
    now_rfc3339, ApplicationSettings, Job, JobInput, JobRepository, JobState, Project,
    ProjectRepository, RepositoryError, RepositoryResult, SettingsRepository,
};
use uuid::Uuid;

pub const LATEST_SCHEMA_VERSION: u32 = 4;

#[derive(Debug, Clone)]
pub struct SqliteStore {
    path: Arc<PathBuf>,
}

impl SqliteStore {
    pub fn open(path: impl AsRef<Path>) -> RepositoryResult<Self> {
        let path = path.as_ref().to_path_buf();
        prepare_private_path(&path)?;
        let store = Self {
            path: Arc::new(path),
        };
        let mut connection = store.connection()?;
        migrate(&mut connection)?;
        enforce_private_file_permissions(store.path.as_ref())?;
        Ok(store)
    }

    pub fn path(&self) -> &Path {
        self.path.as_ref()
    }

    pub fn schema_version(&self) -> RepositoryResult<u32> {
        let connection = self.connection()?;
        connection
            .pragma_query_value(None, "user_version", |row| row.get::<_, u32>(0))
            .map_err(storage_error)
    }

    pub fn search_transcript_ids(
        &self,
        project_id: Uuid,
        raw_query: &str,
        limit: usize,
    ) -> RepositoryResult<Vec<Uuid>> {
        let Some(fts_query) = fts5_search_query(raw_query) else {
            return Ok(Vec::new());
        };
        let connection = self.connection()?;
        let fts5_enabled: i64 = connection
            .query_row(
                "SELECT sqlite_compileoption_used('ENABLE_FTS5')",
                [],
                |row| row.get(0),
            )
            .map_err(storage_error)?;
        let limit = limit.clamp(1, 50) as i64;

        if fts5_enabled == 1 {
            let mut statement = connection
                .prepare(
                    "SELECT t.id
                     FROM transcript_fts
                     JOIN transcripts t ON t.id = transcript_fts.transcript_id
                     JOIN media m ON m.id = t.media_id
                     JOIN sources s ON s.id = m.source_id
                     WHERE s.project_id = ?1
                       AND transcript_fts MATCH ?2
                     ORDER BY bm25(transcript_fts), t.updated_at DESC
                     LIMIT ?3",
                )
                .map_err(storage_error)?;
            let rows = statement
                .query_map(params![project_id.to_string(), fts_query, limit], |row| {
                    parse_uuid(row.get::<_, String>(0)?)
                })
                .map_err(storage_error)?;
            let ids = rows.collect::<Result<Vec<_>, _>>().map_err(storage_error)?;
            if !ids.is_empty() {
                return Ok(ids);
            }
        }

        let like_query = format!("%{}%", escape_like(raw_query.trim()));
        let mut statement = connection
            .prepare(
                "SELECT t.id
                 FROM transcript_fts
                 JOIN transcripts t ON t.id = transcript_fts.transcript_id
                 JOIN media m ON m.id = t.media_id
                 JOIN sources s ON s.id = m.source_id
                 WHERE s.project_id = ?1
                   AND transcript_fts.content LIKE ?2 ESCAPE '\\'
                 ORDER BY t.updated_at DESC
                 LIMIT ?3",
            )
            .map_err(storage_error)?;
        let rows = statement
            .query_map(params![project_id.to_string(), like_query, limit], |row| {
                parse_uuid(row.get::<_, String>(0)?)
            })
            .map_err(storage_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(storage_error)
    }

    fn connection(&self) -> RepositoryResult<Connection> {
        let connection = Connection::open(self.path.as_ref()).map_err(storage_error)?;
        configure_connection(&connection)?;
        Ok(connection)
    }
}

fn configure_connection(connection: &Connection) -> RepositoryResult<()> {
    connection
        .pragma_update(None, "foreign_keys", true)
        .map_err(storage_error)?;
    connection
        .pragma_update(None, "journal_mode", "WAL")
        .map_err(storage_error)?;
    connection
        .pragma_update(None, "synchronous", "FULL")
        .map_err(storage_error)?;
    connection
        .busy_timeout(Duration::from_secs(5))
        .map_err(storage_error)?;
    Ok(())
}

fn migrate(connection: &mut Connection) -> RepositoryResult<()> {
    let mut current = connection
        .pragma_query_value(None, "user_version", |row| row.get::<_, u32>(0))
        .map_err(storage_error)?;
    if current > LATEST_SCHEMA_VERSION {
        return Err(RepositoryError::Storage(format!(
            "unsupported database schema version {current}; expected at most {LATEST_SCHEMA_VERSION}"
        )));
    }

    connection
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS schema_migrations (
                version INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                applied_at TEXT NOT NULL
            );",
        )
        .map_err(storage_error)?;
    let (recorded_version, recorded_count): (u32, u32) = connection
        .query_row(
            "SELECT COALESCE(MAX(version), 0), COUNT(*) FROM schema_migrations",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(storage_error)?;
    if current != recorded_version || current != recorded_count {
        return Err(RepositoryError::Storage(format!(
            "database schema version {current} does not match migration ledger (highest {recorded_version}, count {recorded_count}); refusing to modify existing data"
        )));
    }

    for migration in MIGRATIONS {
        if migration.version <= current {
            continue;
        }
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(storage_error)?;
        (migration.apply)(&tx).map_err(storage_error)?;
        tx.execute(
            "INSERT INTO schema_migrations(version, name, applied_at) VALUES(?1, ?2, ?3)",
            params![migration.version, migration.name, now_rfc3339()],
        )
        .map_err(storage_error)?;
        tx.pragma_update(None, "user_version", migration.version)
            .map_err(storage_error)?;
        tx.commit().map_err(storage_error)?;
        current = migration.version;
    }

    if current != LATEST_SCHEMA_VERSION {
        return Err(RepositoryError::Storage(format!(
            "unsupported database schema version {current}; expected {LATEST_SCHEMA_VERSION}"
        )));
    }
    Ok(())
}

struct Migration {
    version: u32,
    name: &'static str,
    apply: fn(&Transaction<'_>) -> rusqlite::Result<()>,
}

const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        name: "initial_domain_schema",
        apply: migration_1,
    },
    Migration {
        version: 2,
        name: "transcript_search",
        apply: migration_2,
    },
    Migration {
        version: 3,
        name: "transcript_search_backfill",
        apply: migration_3,
    },
    Migration {
        version: 4,
        name: "project_archival_and_relationship_guards",
        apply: migration_4,
    },
];

fn migration_1(tx: &Transaction<'_>) -> rusqlite::Result<()> {
    tx.execute_batch(
        "CREATE TABLE projects (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL COLLATE NOCASE UNIQUE,
            created_at TEXT NOT NULL
        );
        CREATE TABLE creators (
            id TEXT PRIMARY KEY,
            project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
            platform TEXT NOT NULL,
            profile_url TEXT NOT NULL,
            display_name TEXT,
            created_at TEXT NOT NULL,
            UNIQUE(project_id, profile_url)
        );
        CREATE TABLE sources (
            id TEXT PRIMARY KEY,
            project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
            creator_id TEXT REFERENCES creators(id) ON DELETE SET NULL,
            source_type TEXT NOT NULL CHECK(source_type IN ('url', 'local_file')),
            locator TEXT NOT NULL,
            title TEXT,
            created_at TEXT NOT NULL
        );
        CREATE TABLE media (
            id TEXT PRIMARY KEY,
            source_id TEXT NOT NULL REFERENCES sources(id) ON DELETE CASCADE,
            local_path TEXT NOT NULL,
            duration_seconds REAL,
            mime_type TEXT,
            created_at TEXT NOT NULL
        );
        CREATE TABLE transcripts (
            id TEXT PRIMARY KEY,
            media_id TEXT NOT NULL REFERENCES media(id) ON DELETE CASCADE,
            language TEXT,
            text TEXT NOT NULL,
            segments_json TEXT,
            words_json TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        CREATE TABLE research_items (
            id TEXT PRIMARY KEY,
            project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
            creator_id TEXT REFERENCES creators(id) ON DELETE SET NULL,
            source_url TEXT NOT NULL,
            platform TEXT NOT NULL,
            title TEXT,
            view_count INTEGER,
            like_count INTEGER,
            comment_count INTEGER,
            published_at TEXT,
            duration_seconds REAL,
            raw_json TEXT,
            scanned_at TEXT NOT NULL
        );
        CREATE TABLE analyses (
            id TEXT PRIMARY KEY,
            project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
            transcript_id TEXT REFERENCES transcripts(id) ON DELETE SET NULL,
            kind TEXT NOT NULL,
            content TEXT NOT NULL,
            created_at TEXT NOT NULL
        );
        CREATE TABLE ai_runs (
            id TEXT PRIMARY KEY,
            project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
            task TEXT NOT NULL,
            mode TEXT NOT NULL CHECK(mode IN ('copy_prompt', 'byok')),
            provider TEXT,
            model TEXT,
            prompt TEXT NOT NULL,
            result TEXT,
            created_at TEXT NOT NULL
        );
        CREATE TABLE jobs (
            id TEXT PRIMARY KEY,
            project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
            input_kind TEXT NOT NULL CHECK(input_kind IN ('url', 'local_file')),
            input_value TEXT NOT NULL,
            state TEXT NOT NULL CHECK(state IN (
                'queued', 'preparing', 'downloading', 'transcribing', 'processing',
                'completed', 'failed', 'cancelled', 'interrupted'
            )),
            progress REAL CHECK(progress IS NULL OR (progress >= 0.0 AND progress <= 1.0)),
            last_error TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        CREATE TABLE application_settings (
            singleton INTEGER PRIMARY KEY CHECK(singleton = 1),
            settings_json TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        CREATE INDEX idx_creators_project ON creators(project_id);
        CREATE INDEX idx_sources_project_created ON sources(project_id, created_at DESC);
        CREATE INDEX idx_media_source ON media(source_id);
        CREATE INDEX idx_transcripts_media ON transcripts(media_id);
        CREATE INDEX idx_research_project_scanned ON research_items(project_id, scanned_at DESC);
        CREATE INDEX idx_research_creator_published ON research_items(creator_id, published_at DESC);
        CREATE INDEX idx_analyses_project_created ON analyses(project_id, created_at DESC);
        CREATE INDEX idx_ai_runs_project_created ON ai_runs(project_id, created_at DESC);
        CREATE INDEX idx_jobs_project_created ON jobs(project_id, created_at DESC);
        CREATE INDEX idx_jobs_state_created ON jobs(state, created_at);",
    )
}

fn migration_2(tx: &Transaction<'_>) -> rusqlite::Result<()> {
    let fts5_enabled: i64 = tx.query_row(
        "SELECT sqlite_compileoption_used('ENABLE_FTS5')",
        [],
        |row| row.get(0),
    )?;

    if fts5_enabled == 1 {
        tx.execute_batch(
            "CREATE VIRTUAL TABLE transcript_fts USING fts5(
                transcript_id UNINDEXED,
                content,
                tokenize = 'unicode61'
            );",
        )?;
    } else {
        tx.execute_batch(
            "CREATE TABLE transcript_fts (
                transcript_id TEXT PRIMARY KEY,
                content TEXT NOT NULL
            );
            CREATE INDEX idx_transcript_search_content ON transcript_fts(content);",
        )?;
    }

    tx.execute_batch(
        "CREATE TRIGGER transcripts_search_insert AFTER INSERT ON transcripts BEGIN
            INSERT INTO transcript_fts(transcript_id, content) VALUES (NEW.id, NEW.text);
        END;
        CREATE TRIGGER transcripts_search_update AFTER UPDATE OF text ON transcripts BEGIN
            DELETE FROM transcript_fts WHERE transcript_id = OLD.id;
            INSERT INTO transcript_fts(transcript_id, content) VALUES (NEW.id, NEW.text);
        END;
        CREATE TRIGGER transcripts_search_delete AFTER DELETE ON transcripts BEGIN
            DELETE FROM transcript_fts WHERE transcript_id = OLD.id;
        END;",
    )
}

fn migration_3(tx: &Transaction<'_>) -> rusqlite::Result<()> {
    tx.execute(
        "INSERT INTO transcript_fts(transcript_id, content)
         SELECT t.id, t.text
         FROM transcripts t
         WHERE NOT EXISTS (
             SELECT 1 FROM transcript_fts existing
             WHERE existing.transcript_id = t.id
         )",
        [],
    )?;
    Ok(())
}

fn migration_4(tx: &Transaction<'_>) -> rusqlite::Result<()> {
    tx.execute_batch(
        "ALTER TABLE projects ADD COLUMN archived_at TEXT;
         CREATE INDEX idx_projects_archived_name
             ON projects(archived_at, name COLLATE NOCASE);
         CREATE TRIGGER sources_creator_project_insert
         BEFORE INSERT ON sources
         WHEN NEW.creator_id IS NOT NULL AND NOT EXISTS (
             SELECT 1 FROM creators
             WHERE id = NEW.creator_id AND project_id = NEW.project_id
         )
         BEGIN
             SELECT RAISE(ABORT, 'source creator must belong to the same project');
         END;
         CREATE TRIGGER sources_creator_project_update
         BEFORE UPDATE OF creator_id, project_id ON sources
         WHEN NEW.creator_id IS NOT NULL AND NOT EXISTS (
             SELECT 1 FROM creators
             WHERE id = NEW.creator_id AND project_id = NEW.project_id
         )
         BEGIN
             SELECT RAISE(ABORT, 'source creator must belong to the same project');
         END;
         CREATE TRIGGER research_creator_project_insert
         BEFORE INSERT ON research_items
         WHEN NEW.creator_id IS NOT NULL AND NOT EXISTS (
             SELECT 1 FROM creators
             WHERE id = NEW.creator_id AND project_id = NEW.project_id
         )
         BEGIN
             SELECT RAISE(ABORT, 'research creator must belong to the same project');
         END;
         CREATE TRIGGER research_creator_project_update
         BEFORE UPDATE OF creator_id, project_id ON research_items
         WHEN NEW.creator_id IS NOT NULL AND NOT EXISTS (
             SELECT 1 FROM creators
             WHERE id = NEW.creator_id AND project_id = NEW.project_id
         )
         BEGIN
             SELECT RAISE(ABORT, 'research creator must belong to the same project');
         END;
         CREATE TRIGGER analyses_transcript_project_insert
         BEFORE INSERT ON analyses
         WHEN NEW.transcript_id IS NOT NULL AND NOT EXISTS (
             SELECT 1 FROM transcripts t
             JOIN media m ON m.id = t.media_id
             JOIN sources s ON s.id = m.source_id
             WHERE t.id = NEW.transcript_id AND s.project_id = NEW.project_id
         )
         BEGIN
             SELECT RAISE(ABORT, 'analysis transcript must belong to the same project');
         END;
         CREATE TRIGGER analyses_transcript_project_update
         BEFORE UPDATE OF transcript_id, project_id ON analyses
         WHEN NEW.transcript_id IS NOT NULL AND NOT EXISTS (
             SELECT 1 FROM transcripts t
             JOIN media m ON m.id = t.media_id
             JOIN sources s ON s.id = m.source_id
             WHERE t.id = NEW.transcript_id AND s.project_id = NEW.project_id
         )
         BEGIN
             SELECT RAISE(ABORT, 'analysis transcript must belong to the same project');
         END;",
    )
}

fn fts5_search_query(raw_query: &str) -> Option<String> {
    let terms = raw_query
        .split(|character: char| !character.is_alphanumeric())
        .filter(|term| !term.is_empty())
        .take(8)
        .map(|term| format!("\"{term}\"*"))
        .collect::<Vec<_>>();
    (!terms.is_empty()).then(|| terms.join(" AND "))
}

fn escape_like(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

fn storage_error(error: rusqlite::Error) -> RepositoryError {
    RepositoryError::Storage(error.to_string())
}

fn validate_project_name(name: &str) -> RepositoryResult<&str> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(RepositoryError::Validation(
            "project name cannot be empty".to_owned(),
        ));
    }
    if trimmed.chars().count() > 120 {
        return Err(RepositoryError::Validation(
            "project name must be 120 characters or fewer".to_owned(),
        ));
    }
    Ok(trimmed)
}

fn row_project(row: &Row<'_>) -> rusqlite::Result<Project> {
    Ok(Project {
        id: parse_uuid(row.get::<_, String>(0)?)?,
        name: row.get(1)?,
        created_at: row.get(2)?,
    })
}

fn row_job(row: &Row<'_>) -> rusqlite::Result<Job> {
    let input_kind: String = row.get(2)?;
    let input_value: String = row.get(3)?;
    let state: String = row.get(4)?;
    Ok(Job {
        id: parse_uuid(row.get::<_, String>(0)?)?,
        project_id: parse_uuid(row.get::<_, String>(1)?)?,
        input: JobInput::from_parts(&input_kind, input_value).map_err(to_sql_conversion_error)?,
        state: JobState::from_str(&state).map_err(to_sql_conversion_error)?,
        progress: row.get(5)?,
        last_error: row.get(6)?,
        created_at: row.get(7)?,
        updated_at: row.get(8)?,
    })
}

fn parse_uuid(value: String) -> rusqlite::Result<Uuid> {
    Uuid::parse_str(&value).map_err(to_sql_conversion_error)
}

fn to_sql_conversion_error<E>(error: E) -> rusqlite::Error
where
    E: std::error::Error + Send + Sync + 'static,
{
    rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(error))
}

impl ProjectRepository for SqliteStore {
    fn create_project(&self, project: &Project) -> RepositoryResult<()> {
        let name = validate_project_name(&project.name)?;
        let connection = self.connection()?;
        connection
            .execute(
                "INSERT INTO projects(id, name, created_at) VALUES(?1, ?2, ?3)",
                params![project.id.to_string(), name, project.created_at],
            )
            .map_err(storage_error)?;
        Ok(())
    }

    fn get_project(&self, id: Uuid) -> RepositoryResult<Project> {
        let connection = self.connection()?;
        connection
            .query_row(
                "SELECT id, name, created_at FROM projects WHERE id = ?1",
                params![id.to_string()],
                row_project,
            )
            .optional()
            .map_err(storage_error)?
            .ok_or_else(|| RepositoryError::NotFound(format!("project {id}")))
    }

    fn list_projects(&self) -> RepositoryResult<Vec<Project>> {
        let connection = self.connection()?;
        let mut statement = connection
            .prepare("SELECT id, name, created_at FROM projects ORDER BY name COLLATE NOCASE")
            .map_err(storage_error)?;
        let rows = statement
            .query_map([], row_project)
            .map_err(storage_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(storage_error)
    }
}

impl SqliteStore {
    /// Renames a project without changing its ID or any related records.
    pub fn rename_project(&self, id: Uuid, name: &str) -> RepositoryResult<Project> {
        let name = validate_project_name(name)?;
        let connection = self.connection()?;
        let changed = connection
            .execute(
                "UPDATE projects SET name = ?1 WHERE id = ?2",
                params![name, id.to_string()],
            )
            .map_err(storage_error)?;
        if changed == 0 {
            return Err(RepositoryError::NotFound(format!("project {id}")));
        }
        self.get_project(id)
    }

    /// Reversible soft archive; children and active selection remain untouched.
    pub fn archive_project(&self, id: Uuid) -> RepositoryResult<()> {
        self.set_project_archived(id, true)
    }

    pub fn restore_project(&self, id: Uuid) -> RepositoryResult<()> {
        self.set_project_archived(id, false)
    }

    fn set_project_archived(&self, id: Uuid, archived: bool) -> RepositoryResult<()> {
        let connection = self.connection()?;
        let changed = if archived {
            connection.execute(
                "UPDATE projects SET archived_at = COALESCE(archived_at, ?1) WHERE id = ?2",
                params![now_rfc3339(), id.to_string()],
            )
        } else {
            connection.execute(
                "UPDATE projects SET archived_at = NULL WHERE id = ?1",
                params![id.to_string()],
            )
        }
        .map_err(storage_error)?;
        if changed == 0 {
            return Err(RepositoryError::NotFound(format!("project {id}")));
        }
        Ok(())
    }

    pub fn list_archived_projects(&self) -> RepositoryResult<Vec<Project>> {
        self.list_projects_by_archive_state(true)
    }

    pub fn list_active_projects(&self) -> RepositoryResult<Vec<Project>> {
        self.list_projects_by_archive_state(false)
    }

    fn list_projects_by_archive_state(&self, archived: bool) -> RepositoryResult<Vec<Project>> {
        let connection = self.connection()?;
        let sql = if archived {
            "SELECT id, name, created_at FROM projects WHERE archived_at IS NOT NULL
             ORDER BY name COLLATE NOCASE"
        } else {
            "SELECT id, name, created_at FROM projects WHERE archived_at IS NULL
             ORDER BY name COLLATE NOCASE"
        };
        let mut statement = connection.prepare(sql).map_err(storage_error)?;
        let rows = statement
            .query_map([], row_project)
            .map_err(storage_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(storage_error)
    }
}

impl JobRepository for SqliteStore {
    fn insert_job(&self, job: &Job) -> RepositoryResult<()> {
        let (input_kind, input_value) = job.input.parts();
        let connection = self.connection()?;
        connection
            .execute(
                "INSERT INTO jobs(
                    id, project_id, input_kind, input_value, state, progress, last_error,
                    created_at, updated_at
                ) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    job.id.to_string(),
                    job.project_id.to_string(),
                    input_kind,
                    input_value,
                    job.state.as_str(),
                    job.progress,
                    job.last_error,
                    job.created_at,
                    job.updated_at,
                ],
            )
            .map_err(storage_error)?;
        Ok(())
    }

    fn get_job(&self, id: Uuid) -> RepositoryResult<Job> {
        let connection = self.connection()?;
        connection
            .query_row(
                "SELECT id, project_id, input_kind, input_value, state, progress,
                        last_error, created_at, updated_at
                 FROM jobs WHERE id = ?1",
                params![id.to_string()],
                row_job,
            )
            .optional()
            .map_err(storage_error)?
            .ok_or_else(|| RepositoryError::NotFound(format!("job {id}")))
    }

    fn list_jobs(&self, project_id: Option<Uuid>) -> RepositoryResult<Vec<Job>> {
        let connection = self.connection()?;
        let sql = if project_id.is_some() {
            "SELECT id, project_id, input_kind, input_value, state, progress,
                    last_error, created_at, updated_at
             FROM jobs WHERE project_id = ?1 ORDER BY created_at DESC"
        } else {
            "SELECT id, project_id, input_kind, input_value, state, progress,
                    last_error, created_at, updated_at
             FROM jobs ORDER BY created_at DESC"
        };
        let mut statement = connection.prepare(sql).map_err(storage_error)?;
        let rows = if let Some(project_id) = project_id {
            statement
                .query_map(params![project_id.to_string()], row_job)
                .map_err(storage_error)?
        } else {
            statement.query_map([], row_job).map_err(storage_error)?
        };
        rows.collect::<Result<Vec<_>, _>>().map_err(storage_error)
    }

    fn transition_job(&self, id: Uuid, next: JobState) -> RepositoryResult<Job> {
        let mut connection = self.connection()?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(storage_error)?;
        let current = tx
            .query_row(
                "SELECT id, project_id, input_kind, input_value, state, progress,
                        last_error, created_at, updated_at
                 FROM jobs WHERE id = ?1",
                params![id.to_string()],
                row_job,
            )
            .optional()
            .map_err(storage_error)?
            .ok_or_else(|| RepositoryError::NotFound(format!("job {id}")))?;
        current.state.validate_transition(next)?;
        let progress = match next {
            JobState::Queued => None,
            JobState::Completed => Some(1.0_f32),
            _ => current.progress,
        };
        // Retain the previous error for diagnosis until a successful completion.
        let last_error = if next == JobState::Completed {
            None
        } else {
            current.last_error
        };
        let updated_at = now_rfc3339();
        let changed = tx
            .execute(
                "UPDATE jobs SET state = ?1, progress = ?2, last_error = ?3, updated_at = ?4
                 WHERE id = ?5 AND state = ?6",
                params![
                    next.as_str(),
                    progress,
                    last_error,
                    updated_at,
                    id.to_string(),
                    current.state.as_str(),
                ],
            )
            .map_err(storage_error)?;
        if changed != 1 {
            return Err(RepositoryError::Conflict(format!(
                "job {id} changed while transitioning"
            )));
        }
        let updated = tx
            .query_row(
                "SELECT id, project_id, input_kind, input_value, state, progress,
                        last_error, created_at, updated_at
                 FROM jobs WHERE id = ?1",
                params![id.to_string()],
                row_job,
            )
            .map_err(storage_error)?;
        tx.commit().map_err(storage_error)?;
        Ok(updated)
    }

    fn mark_active_jobs_interrupted(&self) -> RepositoryResult<usize> {
        let connection = self.connection()?;
        connection
            .execute(
                "UPDATE jobs SET state = 'interrupted', updated_at = ?1
                 WHERE state IN ('preparing', 'downloading', 'transcribing', 'processing')",
                params![now_rfc3339()],
            )
            .map_err(storage_error)
    }
}

impl SettingsRepository for SqliteStore {
    fn load_settings(&self) -> RepositoryResult<ApplicationSettings> {
        let connection = self.connection()?;
        let json = connection
            .query_row(
                "SELECT settings_json FROM application_settings WHERE singleton = 1",
                [],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(storage_error)?;
        match json {
            Some(json) => serde_json::from_str(&json)
                .map_err(|error| RepositoryError::Storage(error.to_string())),
            None => Ok(ApplicationSettings::default()),
        }
    }

    fn save_settings(&self, settings: &ApplicationSettings) -> RepositoryResult<()> {
        let json = serde_json::to_string(settings)
            .map_err(|error| RepositoryError::Storage(error.to_string()))?;
        let connection = self.connection()?;
        connection
            .execute(
                "INSERT INTO application_settings(singleton, settings_json, updated_at)
                 VALUES(1, ?1, ?2)
                 ON CONFLICT(singleton) DO UPDATE SET
                    settings_json = excluded.settings_json,
                    updated_at = excluded.updated_at",
                params![json, now_rfc3339()],
            )
            .map_err(storage_error)?;
        Ok(())
    }
}

fn prepare_private_path(path: &Path) -> RepositoryResult<()> {
    if let Some(parent) = path.parent() {
        let parent_existed = parent.exists();
        std::fs::create_dir_all(parent)
            .map_err(|error| RepositoryError::Storage(error.to_string()))?;
        if !parent_existed {
            enforce_private_directory_permissions(parent)?;
        }
    }
    Ok(())
}

#[cfg(unix)]
fn enforce_private_directory_permissions(path: &Path) -> RepositoryResult<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700))
        .map_err(|error| RepositoryError::Storage(error.to_string()))
}

#[cfg(not(unix))]
fn enforce_private_directory_permissions(_path: &Path) -> RepositoryResult<()> {
    Ok(())
}

#[cfg(unix)]
fn enforce_private_file_permissions(path: &Path) -> RepositoryResult<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
        .map_err(|error| RepositoryError::Storage(error.to_string()))
}

#[cfg(not(unix))]
fn enforce_private_file_permissions(_path: &Path) -> RepositoryResult<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn store() -> (TempDir, SqliteStore) {
        let temp = TempDir::new().unwrap();
        let db = SqliteStore::open(temp.path().join("private").join("scriptotar.sqlite3")).unwrap();
        (temp, db)
    }

    fn project(db: &SqliteStore, name: &str) -> Project {
        let project = Project::new(name);
        db.create_project(&project).unwrap();
        project
    }

    #[test]
    fn migrations_reach_latest_schema() {
        let (_temp, db) = store();
        assert_eq!(db.schema_version().unwrap(), LATEST_SCHEMA_VERSION);
        let connection = db.connection().unwrap();
        let applied: u32 = connection
            .query_row("SELECT COUNT(*) FROM schema_migrations", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(applied, LATEST_SCHEMA_VERSION);
    }

    #[test]
    fn schema_upgrade_from_v1_is_applied_once() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("upgrade.sqlite3");
        {
            let mut connection = Connection::open(&path).unwrap();
            configure_connection(&connection).unwrap();
            connection
                .execute_batch(
                    "CREATE TABLE schema_migrations (
                        version INTEGER PRIMARY KEY,
                        name TEXT NOT NULL,
                        applied_at TEXT NOT NULL
                    );",
                )
                .unwrap();
            let tx = connection.transaction().unwrap();
            migration_1(&tx).unwrap();
            tx.execute(
                "INSERT INTO schema_migrations(version, name, applied_at) VALUES(1, 'initial_domain_schema', ?1)",
                params![now_rfc3339()],
            )
            .unwrap();
            tx.pragma_update(None, "user_version", 1).unwrap();
            tx.commit().unwrap();
        }
        let db = SqliteStore::open(&path).unwrap();
        assert_eq!(db.schema_version().unwrap(), 4);
        let connection = db.connection().unwrap();
        let count: u32 = connection
            .query_row(
                "SELECT COUNT(*) FROM schema_migrations WHERE version IN (2, 3)",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 2);
    }

    #[test]
    fn schema_v3_backfills_existing_transcript_search_index() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("search-upgrade.sqlite3");
        let project_id = Uuid::new_v4();
        let source_id = Uuid::new_v4();
        let media_id = Uuid::new_v4();
        let transcript_id = Uuid::new_v4();
        let now = now_rfc3339();
        {
            let mut connection = Connection::open(&path).unwrap();
            configure_connection(&connection).unwrap();
            connection
                .execute_batch(
                    "CREATE TABLE schema_migrations (
                        version INTEGER PRIMARY KEY,
                        name TEXT NOT NULL,
                        applied_at TEXT NOT NULL
                    );",
                )
                .unwrap();
            let tx = connection.transaction().unwrap();
            migration_1(&tx).unwrap();
            tx.execute(
                "INSERT INTO schema_migrations(version, name, applied_at)
                 VALUES(1, 'initial_domain_schema', ?1)",
                params![now],
            )
            .unwrap();
            tx.pragma_update(None, "user_version", 1).unwrap();
            tx.commit().unwrap();

            connection
                .execute(
                    "INSERT INTO projects(id, name, created_at) VALUES(?1, 'Search Upgrade', ?2)",
                    params![project_id.to_string(), now],
                )
                .unwrap();
            connection
                .execute(
                    "INSERT INTO sources(id, project_id, creator_id, source_type, locator, title, created_at)
                     VALUES(?1, ?2, NULL, 'local_file', '/tmp/search-upgrade.mp4', 'Search Upgrade', ?3)",
                    params![source_id.to_string(), project_id.to_string(), now],
                )
                .unwrap();
            connection
                .execute(
                    "INSERT INTO media(id, source_id, local_path, duration_seconds, mime_type, created_at)
                     VALUES(?1, ?2, '/tmp/search-upgrade.mp4', 12.0, NULL, ?3)",
                    params![media_id.to_string(), source_id.to_string(), now],
                )
                .unwrap();
            connection
                .execute(
                    "INSERT INTO transcripts(
                        id, media_id, language, text, segments_json, words_json, created_at, updated_at
                     ) VALUES(?1, ?2, 'ar', 'ابدأ بالنتيجة ثم اشرح السبب', NULL, NULL, ?3, ?3)",
                    params![transcript_id.to_string(), media_id.to_string(), now],
                )
                .unwrap();

            let tx = connection.transaction().unwrap();
            migration_2(&tx).unwrap();
            tx.execute(
                "INSERT INTO schema_migrations(version, name, applied_at)
                 VALUES(2, 'transcript_search', ?1)",
                params![now],
            )
            .unwrap();
            tx.pragma_update(None, "user_version", 2).unwrap();
            tx.commit().unwrap();
        }

        let db = SqliteStore::open(&path).unwrap();
        assert_eq!(db.schema_version().unwrap(), 4);
        assert_eq!(
            db.search_transcript_ids(project_id, "النتيجة", 10).unwrap(),
            vec![transcript_id]
        );
    }

    #[test]
    fn transcript_search_is_project_scoped_and_safe_for_punctuation() {
        let (_temp, db) = store();
        let first = project(&db, "First");
        let second = project(&db, "Second");
        let first_transcript = Uuid::new_v4();
        let second_transcript = Uuid::new_v4();
        let now = now_rfc3339();

        for (project_id, transcript_id, suffix) in [
            (first.id, first_transcript, "first"),
            (second.id, second_transcript, "second"),
        ] {
            let source_id = Uuid::new_v4();
            let media_id = Uuid::new_v4();
            let connection = db.connection().unwrap();
            connection
                .execute(
                    "INSERT INTO sources(id, project_id, creator_id, source_type, locator, title, created_at)
                     VALUES(?1, ?2, NULL, 'local_file', ?3, NULL, ?4)",
                    params![source_id.to_string(), project_id.to_string(), format!("/tmp/{suffix}.mp4"), now,],
                )
                .unwrap();
            connection
                .execute(
                    "INSERT INTO media(id, source_id, local_path, duration_seconds, mime_type, created_at)
                     VALUES(?1, ?2, ?3, 4.0, NULL, ?4)",
                    params![media_id.to_string(), source_id.to_string(), format!("/tmp/{suffix}.mp4"), now,],
                )
                .unwrap();
            connection
                .execute(
                    "INSERT INTO transcripts(
                        id, media_id, language, text, segments_json, words_json, created_at, updated_at
                     ) VALUES(?1, ?2, 'en', 'visual payoff retention signal', NULL, NULL, ?3, ?3)",
                    params![transcript_id.to_string(), media_id.to_string(), now],
                )
                .unwrap();
        }

        assert_eq!(
            db.search_transcript_ids(first.id, "payoff!!!", 10).unwrap(),
            vec![first_transcript]
        );
        assert!(db
            .search_transcript_ids(first.id, "%%%", 10)
            .unwrap()
            .is_empty());
    }

    #[test]
    fn project_repository_supports_crud_reads() {
        let (_temp, db) = store();
        let created = project(&db, "Creator Lab");
        assert_eq!(db.get_project(created.id).unwrap(), created);
        assert_eq!(db.list_projects().unwrap(), vec![created]);
    }

    #[test]
    fn job_state_change_is_atomic_and_validated() {
        let (_temp, db) = store();
        let project = project(&db, "Inbox");
        let job = Job::new(
            project.id,
            JobInput::Url("https://youtu.be/example".to_owned()),
        );
        db.insert_job(&job).unwrap();

        let error = db.transition_job(job.id, JobState::Completed).unwrap_err();
        assert!(matches!(error, RepositoryError::Conflict(_)));
        assert_eq!(db.get_job(job.id).unwrap().state, JobState::Queued);

        let preparing = db.transition_job(job.id, JobState::Preparing).unwrap();
        assert_eq!(preparing.state, JobState::Preparing);
    }

    #[test]
    fn active_jobs_become_interrupted_after_restart_recovery() {
        let (_temp, db) = store();
        let project = project(&db, "Inbox");
        let job = Job::new(project.id, JobInput::LocalFile("/tmp/video.mp4".to_owned()));
        db.insert_job(&job).unwrap();
        db.transition_job(job.id, JobState::Preparing).unwrap();
        db.transition_job(job.id, JobState::Transcribing).unwrap();

        assert_eq!(db.mark_active_jobs_interrupted().unwrap(), 1);
        assert_eq!(db.get_job(job.id).unwrap().state, JobState::Interrupted);
    }

    #[test]
    fn retry_resets_progress_and_success_clears_previous_error() {
        let (_temp, db) = store();
        let project = project(&db, "Retry preservation");
        let job = Job::new(project.id, JobInput::LocalFile("/tmp/retry.mp4".to_owned()));
        db.insert_job(&job).unwrap();
        db.transition_job(job.id, JobState::Preparing).unwrap();

        db.connection()
            .unwrap()
            .execute(
                "UPDATE jobs SET progress = 0.8, last_error = 'original failure' WHERE id = ?1",
                params![job.id.to_string()],
            )
            .unwrap();
        db.transition_job(job.id, JobState::Failed).unwrap();

        let retried = db.transition_job(job.id, JobState::Queued).unwrap();
        assert_eq!(retried.progress, None);
        assert_eq!(retried.last_error.as_deref(), Some("original failure"));

        for state in [
            JobState::Preparing,
            JobState::Transcribing,
            JobState::Processing,
            JobState::Completed,
        ] {
            db.transition_job(job.id, state).unwrap();
        }
        let completed = db.get_job(job.id).unwrap();
        assert_eq!(completed.progress, Some(1.0));
        assert_eq!(completed.last_error, None);
    }

    #[test]
    fn inconsistent_schema_ledger_is_rejected_without_changing_project_data() {
        let (temp, db) = store();
        let project = project(&db, "Keep me");
        let connection = db.connection().unwrap();
        connection
            .execute("DELETE FROM schema_migrations WHERE version = 3", [])
            .unwrap();
        drop(connection);
        drop(db);

        let error =
            SqliteStore::open(temp.path().join("private").join("scriptotar.sqlite3")).unwrap_err();
        assert!(matches!(error, RepositoryError::Storage(_)));

        let connection =
            Connection::open(temp.path().join("private").join("scriptotar.sqlite3")).unwrap();
        let recorded_version: u32 = connection
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        let retained_project: String = connection
            .query_row(
                "SELECT name FROM projects WHERE id = ?1",
                params![project.id.to_string()],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(recorded_version, LATEST_SCHEMA_VERSION);
        assert_eq!(retained_project, "Keep me");
    }

    #[test]
    fn settings_round_trip_without_secret_storage() {
        let (_temp, db) = store();
        let mut settings = db.load_settings().unwrap();
        settings.ai_provider = "Anthropic".to_owned();
        settings.ai_base_url = Some("https://example.invalid/v1".to_owned());
        db.save_settings(&settings).unwrap();
        assert_eq!(db.load_settings().unwrap(), settings);

        let connection = db.connection().unwrap();
        let raw: String = connection
            .query_row(
                "SELECT settings_json FROM application_settings WHERE singleton = 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(!raw.to_ascii_lowercase().contains("api_key"));
    }

    #[cfg(unix)]
    #[test]
    fn storage_permissions_are_private() {
        use std::os::unix::fs::PermissionsExt;
        let (temp, db) = store();
        let parent_mode = std::fs::metadata(db.path().parent().unwrap())
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        let file_mode = std::fs::metadata(db.path()).unwrap().permissions().mode() & 0o777;
        assert_eq!(parent_mode, 0o700);
        assert_eq!(file_mode, 0o600);
        drop(temp);
    }
}
