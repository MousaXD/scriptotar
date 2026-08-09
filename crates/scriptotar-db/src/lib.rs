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

pub const LATEST_SCHEMA_VERSION: u32 = 2;

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
    connection
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS schema_migrations (
                version INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                applied_at TEXT NOT NULL
            );",
        )
        .map_err(storage_error)?;

    let mut current = connection
        .pragma_query_value(None, "user_version", |row| row.get::<_, u32>(0))
        .map_err(storage_error)?;

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
        let rows = statement.query_map([], row_project).map_err(storage_error)?;
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
        let progress = if next == JobState::Completed {
            Some(1.0_f32)
        } else {
            current.progress
        };
        let updated_at = now_rfc3339();
        let changed = tx
            .execute(
                "UPDATE jobs SET state = ?1, progress = ?2, updated_at = ?3
                 WHERE id = ?4 AND state = ?5",
                params![
                    next.as_str(),
                    progress,
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
            .query_row("SELECT COUNT(*) FROM schema_migrations", [], |row| row.get(0))
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
        assert_eq!(db.schema_version().unwrap(), 2);
        let connection = db.connection().unwrap();
        let count: u32 = connection
            .query_row(
                "SELECT COUNT(*) FROM schema_migrations WHERE version = 2",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
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
        let job = Job::new(
            project.id,
            JobInput::LocalFile("/tmp/video.mp4".to_owned()),
        );
        db.insert_job(&job).unwrap();
        db.transition_job(job.id, JobState::Preparing).unwrap();
        db.transition_job(job.id, JobState::Transcribing).unwrap();

        assert_eq!(db.mark_active_jobs_interrupted().unwrap(), 1);
        assert_eq!(
            db.get_job(job.id).unwrap().state,
            JobState::Interrupted
        );
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
