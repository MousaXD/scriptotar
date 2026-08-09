use std::{
    collections::HashMap,
    fs,
    path::Path,
    str::FromStr,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use rusqlite::{params, Connection, OptionalExtension, Transaction, TransactionBehavior};
use scriptotar_core::{
    now_rfc3339, AiRunMode, ContentRepository, Job, JobRepository, JobRuntimeRepository,
    JobState, LegacyImportReport, Media, RepositoryError, RepositoryResult, Source, SourceType,
    Transcript, TranscriptBundle, Watchlist, WatchlistRepository,
};
use uuid::Uuid;

use crate::SqliteStore;

pub const LATEST_INTEGRATION_SCHEMA_VERSION: u32 = 1;

fn storage_error(error: rusqlite::Error) -> RepositoryError {
    RepositoryError::Storage(error.to_string())
}

fn io_error(error: std::io::Error) -> RepositoryError {
    RepositoryError::Storage(error.to_string())
}

fn connection(store: &SqliteStore) -> RepositoryResult<Connection> {
    let connection = Connection::open(store.path()).map_err(storage_error)?;
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
    Ok(connection)
}

impl SqliteStore {
    pub fn run_integration_migrations(&self) -> RepositoryResult<()> {
        let mut connection = connection(self)?;
        connection
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS integration_schema_migrations (
                    version INTEGER PRIMARY KEY,
                    name TEXT NOT NULL,
                    applied_at TEXT NOT NULL
                );",
            )
            .map_err(storage_error)?;
        let current: u32 = connection
            .query_row(
                "SELECT COALESCE(MAX(version), 0) FROM integration_schema_migrations",
                [],
                |row| row.get(0),
            )
            .map_err(storage_error)?;
        if current > LATEST_INTEGRATION_SCHEMA_VERSION {
            return Err(RepositoryError::Storage(format!(
                "unsupported integration schema version {current}; expected at most {LATEST_INTEGRATION_SCHEMA_VERSION}"
            )));
        }
        if current < 1 {
            let tx = connection
                .transaction_with_behavior(TransactionBehavior::Immediate)
                .map_err(storage_error)?;
            tx.execute_batch(
                "CREATE TABLE watchlists (
                    id TEXT PRIMARY KEY,
                    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
                    label TEXT NOT NULL,
                    profile_url TEXT NOT NULL,
                    limit_count INTEGER NOT NULL DEFAULT 25 CHECK(limit_count > 0),
                    last_scan_at TEXT,
                    UNIQUE(project_id, profile_url)
                );
                CREATE INDEX idx_watchlists_project ON watchlists(project_id, label COLLATE NOCASE);
                CREATE TABLE legacy_imports (
                    source_path TEXT NOT NULL,
                    fingerprint TEXT NOT NULL,
                    backup_path TEXT NOT NULL,
                    imported_at TEXT NOT NULL,
                    PRIMARY KEY(source_path, fingerprint)
                );
                ALTER TABLE research_items ADD COLUMN legacy_thumbnail TEXT;
                ALTER TABLE research_items ADD COLUMN legacy_engagement_rate REAL;
                ALTER TABLE ai_runs ADD COLUMN legacy_source_title TEXT;",
            )
            .map_err(storage_error)?;
            tx.execute(
                "INSERT INTO integration_schema_migrations(version, name, applied_at) VALUES(1, 'legacy_import_support', ?1)",
                params![now_rfc3339()],
            )
            .map_err(storage_error)?;
            tx.commit().map_err(storage_error)?;
        }
        Ok(())
    }

    pub fn import_legacy_database(
        &self,
        legacy_path: impl AsRef<Path>,
    ) -> RepositoryResult<LegacyImportReport> {
        self.run_integration_migrations()?;
        let legacy_path = legacy_path.as_ref();
        if !legacy_path.is_file() {
            return Err(RepositoryError::NotFound(format!(
                "legacy database {}",
                legacy_path.display()
            )));
        }
        if legacy_path == self.path() {
            return Err(RepositoryError::Validation(
                "legacy database must be different from the Scriptotar Next database".to_owned(),
            ));
        }

        let metadata = fs::metadata(legacy_path).map_err(io_error)?;
        let modified = metadata
            .modified()
            .unwrap_or(SystemTime::UNIX_EPOCH)
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let canonical = fs::canonicalize(legacy_path).map_err(io_error)?;
        let fingerprint = format!("{}:{}:{}", canonical.display(), metadata.len(), modified);
        let file_name = legacy_path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("history.sqlite3");
        let backup_path = legacy_path.with_file_name(format!(
            "{file_name}.scriptotar-next-{modified}-{}.bak",
            metadata.len()
        ));
        if !backup_path.exists() {
            fs::copy(legacy_path, &backup_path).map_err(io_error)?;
        }

        let legacy = Connection::open(legacy_path).map_err(storage_error)?;
        let mut destination = connection(self)?;
        let already_imported: bool = destination
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM legacy_imports WHERE source_path = ?1 AND fingerprint = ?2)",
                params![canonical.to_string_lossy(), fingerprint],
                |row| row.get(0),
            )
            .map_err(storage_error)?;
        if already_imported {
            return Ok(LegacyImportReport {
                skipped: true,
                backup_path: Some(backup_path.to_string_lossy().into_owned()),
                ..LegacyImportReport::default()
            });
        }

        let tx = destination
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(storage_error)?;
        let mut report = LegacyImportReport {
            backup_path: Some(backup_path.to_string_lossy().into_owned()),
            ..LegacyImportReport::default()
        };
        let mut projects = HashMap::new();

        if table_exists(&legacy, "projects")? {
            let mut statement = legacy
                .prepare("SELECT name, created_at FROM projects ORDER BY name")
                .map_err(storage_error)?;
            let rows = statement
                .query_map([], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                })
                .map_err(storage_error)?;
            for row in rows {
                let (name, created_at) = row.map_err(storage_error)?;
                let id = ensure_project(&tx, &name, &created_at)?;
                projects.insert(name, id);
                report.projects += 1;
            }
        }
        let inbox_id = ensure_project(&tx, "Inbox", &now_rfc3339())?;
        projects.entry("Inbox".to_owned()).or_insert(inbox_id);

        if table_exists(&legacy, "jobs")? {
            let mut statement = legacy
                .prepare(
                    "SELECT id, created_at, source, input_type, title, status, language,
                            output_dir, transcript, error, project FROM jobs ORDER BY created_at",
                )
                .map_err(storage_error)?;
            let rows = statement
                .query_map([], |row| {
                    Ok(LegacyJob {
                        id: row.get(0)?,
                        created_at: row.get(1)?,
                        source: row.get(2)?,
                        input_type: row.get(3)?,
                        title: row.get(4)?,
                        status: row.get(5)?,
                        language: row.get(6)?,
                        output_dir: row.get(7)?,
                        transcript: row.get(8)?,
                        error: row.get(9)?,
                        project: row.get(10)?,
                    })
                })
                .map_err(storage_error)?;
            for row in rows {
                let row = row.map_err(storage_error)?;
                let project_id = project_id_for(&tx, &mut projects, &row.project)?;
                let job_id = legacy_uuid("job", &row.id);
                let input_kind = if row.input_type.eq_ignore_ascii_case("url") {
                    "url"
                } else {
                    "local_file"
                };
                let state = legacy_job_state(&row.status);
                let progress = if state == JobState::Completed {
                    Some(1.0_f32)
                } else {
                    None
                };
                tx.execute(
                    "INSERT INTO jobs(
                        id, project_id, input_kind, input_value, state, progress, last_error,
                        created_at, updated_at
                    ) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)
                    ON CONFLICT(id) DO UPDATE SET
                        project_id = excluded.project_id,
                        input_kind = excluded.input_kind,
                        input_value = excluded.input_value,
                        state = excluded.state,
                        progress = excluded.progress,
                        last_error = excluded.last_error,
                        updated_at = excluded.updated_at",
                    params![
                        job_id.to_string(),
                        project_id.to_string(),
                        input_kind,
                        row.source,
                        state.as_str(),
                        progress,
                        row.error,
                        row.created_at,
                    ],
                )
                .map_err(storage_error)?;
                report.jobs += 1;

                if let Some(text) = row
                    .transcript
                    .as_deref()
                    .filter(|value| !value.trim().is_empty())
                {
                    let source_id = legacy_uuid("source", &row.id);
                    let media_id = legacy_uuid("media", &row.id);
                    let transcript_id = legacy_uuid("transcript", &row.id);
                    let source_type = if input_kind == "url" {
                        "url"
                    } else {
                        "local_file"
                    };
                    tx.execute(
                        "INSERT INTO sources(id, project_id, creator_id, source_type, locator, title, created_at)
                         VALUES(?1, ?2, NULL, ?3, ?4, ?5, ?6)
                         ON CONFLICT(id) DO UPDATE SET title = excluded.title, locator = excluded.locator",
                        params![
                            source_id.to_string(),
                            project_id.to_string(),
                            source_type,
                            row.source,
                            row.title,
                            row.created_at,
                        ],
                    )
                    .map_err(storage_error)?;
                    let local_path = row
                        .output_dir
                        .as_deref()
                        .filter(|value| !value.is_empty())
                        .unwrap_or(&row.source);
                    tx.execute(
                        "INSERT INTO media(id, source_id, local_path, duration_seconds, mime_type, created_at)
                         VALUES(?1, ?2, ?3, NULL, NULL, ?4)
                         ON CONFLICT(id) DO UPDATE SET local_path = excluded.local_path",
                        params![
                            media_id.to_string(),
                            source_id.to_string(),
                            local_path,
                            row.created_at,
                        ],
                    )
                    .map_err(storage_error)?;
                    tx.execute(
                        "INSERT INTO transcripts(
                            id, media_id, language, text, segments_json, words_json, created_at, updated_at
                         ) VALUES(?1, ?2, ?3, ?4, NULL, NULL, ?5, ?5)
                         ON CONFLICT(id) DO UPDATE SET
                            language = excluded.language,
                            text = excluded.text,
                            updated_at = excluded.updated_at",
                        params![
                            transcript_id.to_string(),
                            media_id.to_string(),
                            row.language,
                            text,
                            row.created_at,
                        ],
                    )
                    .map_err(storage_error)?;
                    report.transcripts += 1;
                }
            }
        }

        if table_exists(&legacy, "research_items")? {
            let mut statement = legacy
                .prepare(
                    "SELECT storage_id, project, creator_url, source_url, platform, title,
                            view_count, like_count, comment_count, engagement_rate, published_at,
                            duration, thumbnail, raw_json, scanned_at FROM research_items",
                )
                .map_err(storage_error)?;
            let rows = statement
                .query_map([], |row| {
                    Ok(LegacyResearch {
                        storage_id: row.get(0)?,
                        project: row.get(1)?,
                        creator_url: row.get(2)?,
                        source_url: row.get(3)?,
                        platform: row.get(4)?,
                        title: row.get(5)?,
                        view_count: row.get(6)?,
                        like_count: row.get(7)?,
                        comment_count: row.get(8)?,
                        engagement_rate: row.get(9)?,
                        published_at: row.get(10)?,
                        duration: row.get(11)?,
                        thumbnail: row.get(12)?,
                        raw_json: row.get(13)?,
                        scanned_at: row.get(14)?,
                    })
                })
                .map_err(storage_error)?;
            for row in rows {
                let row = row.map_err(storage_error)?;
                let project_id = project_id_for(&tx, &mut projects, &row.project)?;
                let platform = row.platform.clone().unwrap_or_else(|| "unknown".to_owned());
                let creator_id = if row.creator_url.trim().is_empty() {
                    None
                } else {
                    let id = legacy_uuid("creator", &format!("{}:{}", project_id, row.creator_url));
                    tx.execute(
                        "INSERT INTO creators(id, project_id, platform, profile_url, display_name, created_at)
                         VALUES(?1, ?2, ?3, ?4, NULL, ?5)
                         ON CONFLICT(id) DO NOTHING",
                        params![
                            id.to_string(),
                            project_id.to_string(),
                            platform,
                            row.creator_url,
                            row.scanned_at,
                        ],
                    )
                    .map_err(storage_error)?;
                    Some(id)
                };
                let item_id = legacy_uuid("research", &row.storage_id);
                let source_url = row.source_url.as_deref().unwrap_or(&row.creator_url);
                tx.execute(
                    "INSERT INTO research_items(
                        id, project_id, creator_id, source_url, platform, title, view_count, like_count,
                        comment_count, published_at, duration_seconds, raw_json, scanned_at,
                        legacy_thumbnail, legacy_engagement_rate
                     ) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
                     ON CONFLICT(id) DO UPDATE SET
                        title = excluded.title,
                        view_count = excluded.view_count,
                        like_count = excluded.like_count,
                        comment_count = excluded.comment_count,
                        published_at = excluded.published_at,
                        duration_seconds = excluded.duration_seconds,
                        raw_json = excluded.raw_json,
                        scanned_at = excluded.scanned_at,
                        legacy_thumbnail = excluded.legacy_thumbnail,
                        legacy_engagement_rate = excluded.legacy_engagement_rate",
                    params![
                        item_id.to_string(),
                        project_id.to_string(),
                        creator_id.map(|id| id.to_string()),
                        source_url,
                        row.platform.unwrap_or_else(|| "unknown".to_owned()),
                        row.title,
                        row.view_count,
                        row.like_count,
                        row.comment_count,
                        row.published_at,
                        row.duration,
                        row.raw_json,
                        row.scanned_at,
                        row.thumbnail,
                        row.engagement_rate,
                    ],
                )
                .map_err(storage_error)?;
                report.research_items += 1;
            }
        }

        if table_exists(&legacy, "watchlists")? {
            let mut statement = legacy
                .prepare("SELECT id, project, label, profile_url, limit_count, last_scan_at FROM watchlists")
                .map_err(storage_error)?;
            let rows = statement
                .query_map([], |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, i64>(4)?,
                        row.get::<_, Option<String>>(5)?,
                    ))
                })
                .map_err(storage_error)?;
            for row in rows {
                let (legacy_id, project, label, profile_url, limit_count, last_scan_at) =
                    row.map_err(storage_error)?;
                let project_id = project_id_for(&tx, &mut projects, &project)?;
                let id = legacy_uuid("watchlist", &legacy_id.to_string());
                tx.execute(
                    "INSERT INTO watchlists(id, project_id, label, profile_url, limit_count, last_scan_at)
                     VALUES(?1, ?2, ?3, ?4, ?5, ?6)
                     ON CONFLICT(id) DO UPDATE SET
                        label = excluded.label,
                        profile_url = excluded.profile_url,
                        limit_count = excluded.limit_count,
                        last_scan_at = excluded.last_scan_at",
                    params![
                        id.to_string(),
                        project_id.to_string(),
                        label,
                        profile_url,
                        limit_count.max(1),
                        last_scan_at,
                    ],
                )
                .map_err(storage_error)?;
                report.watchlists += 1;
            }
        }

        if table_exists(&legacy, "ai_runs")? {
            let mut statement = legacy
                .prepare(
                    "SELECT id, created_at, project, task, mode, provider, model, source_title, prompt, result
                     FROM ai_runs ORDER BY created_at",
                )
                .map_err(storage_error)?;
            let rows = statement
                .query_map([], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, Option<String>>(5)?,
                        row.get::<_, Option<String>>(6)?,
                        row.get::<_, Option<String>>(7)?,
                        row.get::<_, String>(8)?,
                        row.get::<_, Option<String>>(9)?,
                    ))
                })
                .map_err(storage_error)?;
            for row in rows {
                let (
                    legacy_id,
                    created_at,
                    project,
                    task,
                    mode,
                    provider,
                    model,
                    source_title,
                    prompt,
                    result,
                ) = row.map_err(storage_error)?;
                let project_id = project_id_for(&tx, &mut projects, &project)?;
                let id = legacy_uuid("ai-run", &legacy_id);
                let mode = if mode.to_ascii_lowercase().contains("copy") {
                    AiRunMode::CopyPrompt
                } else {
                    AiRunMode::Byok
                };
                let mode = match mode {
                    AiRunMode::CopyPrompt => "copy_prompt",
                    AiRunMode::Byok => "byok",
                };
                tx.execute(
                    "INSERT INTO ai_runs(
                        id, project_id, task, mode, provider, model, prompt, result, created_at, legacy_source_title
                     ) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
                     ON CONFLICT(id) DO UPDATE SET
                        task = excluded.task,
                        mode = excluded.mode,
                        provider = excluded.provider,
                        model = excluded.model,
                        prompt = excluded.prompt,
                        result = excluded.result,
                        legacy_source_title = excluded.legacy_source_title",
                    params![
                        id.to_string(),
                        project_id.to_string(),
                        task,
                        mode,
                        provider,
                        model,
                        prompt,
                        result,
                        created_at,
                        source_title,
                    ],
                )
                .map_err(storage_error)?;
                report.ai_runs += 1;
            }
        }

        tx.execute(
            "INSERT INTO legacy_imports(source_path, fingerprint, backup_path, imported_at)
             VALUES(?1, ?2, ?3, ?4)",
            params![
                canonical.to_string_lossy(),
                fingerprint,
                backup_path.to_string_lossy(),
                now_rfc3339(),
            ],
        )
        .map_err(storage_error)?;
        tx.commit().map_err(storage_error)?;
        Ok(report)
    }
}

impl JobRuntimeRepository for SqliteStore {
    fn update_job_progress(&self, id: Uuid, progress: Option<f32>) -> RepositoryResult<Job> {
        if progress.is_some_and(|value| !(0.0..=1.0).contains(&value)) {
            return Err(RepositoryError::Validation(
                "job progress must be between 0 and 1".to_owned(),
            ));
        }
        let connection = connection(self)?;
        let changed = connection
            .execute(
                "UPDATE jobs SET progress = ?1, updated_at = ?2
                 WHERE id = ?3 AND state IN ('preparing', 'downloading', 'transcribing', 'processing')",
                params![progress, now_rfc3339(), id.to_string()],
            )
            .map_err(storage_error)?;
        if changed != 1 {
            return Err(RepositoryError::Conflict(format!(
                "job {id} is not active and cannot accept progress"
            )));
        }
        self.get_job(id)
    }

    fn fail_job(&self, id: Uuid, error: &str) -> RepositoryResult<Job> {
        let mut connection = connection(self)?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(storage_error)?;
        let state: String = tx
            .query_row(
                "SELECT state FROM jobs WHERE id = ?1",
                params![id.to_string()],
                |row| row.get(0),
            )
            .optional()
            .map_err(storage_error)?
            .ok_or_else(|| RepositoryError::NotFound(format!("job {id}")))?;
        let state = JobState::from_str(&state)?;
        state.validate_transition(JobState::Failed)?;
        let changed = tx
            .execute(
                "UPDATE jobs SET state = 'failed', last_error = ?1, updated_at = ?2
                 WHERE id = ?3 AND state = ?4",
                params![error, now_rfc3339(), id.to_string(), state.as_str()],
            )
            .map_err(storage_error)?;
        if changed != 1 {
            return Err(RepositoryError::Conflict(format!(
                "job {id} changed while recording failure"
            )));
        }
        tx.commit().map_err(storage_error)?;
        self.get_job(id)
    }
}

impl ContentRepository for SqliteStore {
    fn persist_transcription(
        &self,
        job_id: Uuid,
        source: &Source,
        media: &Media,
        transcript: &Transcript,
    ) -> RepositoryResult<Job> {
        if media.source_id != source.id || transcript.media_id != media.id {
            return Err(RepositoryError::Validation(
                "transcription object relationships are inconsistent".to_owned(),
            ));
        }
        let mut connection = connection(self)?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(storage_error)?;
        let (project_id, state): (String, String) = tx
            .query_row(
                "SELECT project_id, state FROM jobs WHERE id = ?1",
                params![job_id.to_string()],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(storage_error)?
            .ok_or_else(|| RepositoryError::NotFound(format!("job {job_id}")))?;
        if project_id != source.project_id.to_string() {
            return Err(RepositoryError::Validation(
                "transcription source project does not match job project".to_owned(),
            ));
        }
        let state = JobState::from_str(&state)?;
        if state != JobState::Processing {
            return Err(RepositoryError::Conflict(format!(
                "job {job_id} must be processing before transcript commit"
            )));
        }
        let source_type = match source.source_type {
            SourceType::Url => "url",
            SourceType::LocalFile => "local_file",
        };
        tx.execute(
            "INSERT INTO sources(id, project_id, creator_id, source_type, locator, title, created_at)
             VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                source.id.to_string(),
                source.project_id.to_string(),
                source.creator_id.map(|id| id.to_string()),
                source_type,
                source.locator,
                source.title,
                source.created_at,
            ],
        )
        .map_err(storage_error)?;
        tx.execute(
            "INSERT INTO media(id, source_id, local_path, duration_seconds, mime_type, created_at)
             VALUES(?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                media.id.to_string(),
                media.source_id.to_string(),
                media.local_path,
                media.duration_seconds,
                media.mime_type,
                media.created_at,
            ],
        )
        .map_err(storage_error)?;
        tx.execute(
            "INSERT INTO transcripts(
                id, media_id, language, text, segments_json, words_json, created_at, updated_at
             ) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                transcript.id.to_string(),
                transcript.media_id.to_string(),
                transcript.language,
                transcript.text,
                transcript.segments_json,
                transcript.words_json,
                transcript.created_at,
                transcript.updated_at,
            ],
        )
        .map_err(storage_error)?;
        let changed = tx
            .execute(
                "UPDATE jobs SET state = 'completed', progress = 1.0, last_error = NULL, updated_at = ?1
                 WHERE id = ?2 AND state = 'processing'",
                params![now_rfc3339(), job_id.to_string()],
            )
            .map_err(storage_error)?;
        if changed != 1 {
            return Err(RepositoryError::Conflict(format!(
                "job {job_id} changed while committing transcript"
            )));
        }
        tx.commit().map_err(storage_error)?;
        self.get_job(job_id)
    }

    fn list_transcripts(
        &self,
        project_id: Option<Uuid>,
    ) -> RepositoryResult<Vec<TranscriptBundle>> {
        let connection = connection(self)?;
        let sql = if project_id.is_some() {
            "SELECT s.project_id, s.id, s.creator_id, s.source_type, s.locator, s.title, s.created_at,
                    m.id, m.local_path, m.duration_seconds, m.mime_type, m.created_at,
                    t.id, t.language, t.text, t.segments_json, t.words_json, t.created_at, t.updated_at
             FROM transcripts t
             JOIN media m ON m.id = t.media_id
             JOIN sources s ON s.id = m.source_id
             WHERE s.project_id = ?1
             ORDER BY t.created_at DESC"
        } else {
            "SELECT s.project_id, s.id, s.creator_id, s.source_type, s.locator, s.title, s.created_at,
                    m.id, m.local_path, m.duration_seconds, m.mime_type, m.created_at,
                    t.id, t.language, t.text, t.segments_json, t.words_json, t.created_at, t.updated_at
             FROM transcripts t
             JOIN media m ON m.id = t.media_id
             JOIN sources s ON s.id = m.source_id
             ORDER BY t.created_at DESC"
        };
        let mut statement = connection.prepare(sql).map_err(storage_error)?;
        let map_row = |row: &rusqlite::Row<'_>| -> rusqlite::Result<TranscriptBundle> {
            let project_id = parse_uuid(row.get::<_, String>(0)?)?;
            let source_id = parse_uuid(row.get::<_, String>(1)?)?;
            let creator_id = row
                .get::<_, Option<String>>(2)?
                .map(parse_uuid)
                .transpose()?;
            let source_type: String = row.get(3)?;
            let source = Source {
                id: source_id,
                project_id,
                creator_id,
                source_type: match source_type.as_str() {
                    "url" => SourceType::Url,
                    "local_file" => SourceType::LocalFile,
                    _ => return Err(rusqlite::Error::InvalidQuery),
                },
                locator: row.get(4)?,
                title: row.get(5)?,
                created_at: row.get(6)?,
            };
            let media_id = parse_uuid(row.get::<_, String>(7)?)?;
            let media = Media {
                id: media_id,
                source_id,
                local_path: row.get(8)?,
                duration_seconds: row.get(9)?,
                mime_type: row.get(10)?,
                created_at: row.get(11)?,
            };
            let transcript = Transcript {
                id: parse_uuid(row.get::<_, String>(12)?)?,
                media_id,
                language: row.get(13)?,
                text: row.get(14)?,
                segments_json: row.get(15)?,
                words_json: row.get(16)?,
                created_at: row.get(17)?,
                updated_at: row.get(18)?,
            };
            Ok(TranscriptBundle {
                project_id,
                source,
                media,
                transcript,
            })
        };
        let rows = if let Some(project_id) = project_id {
            statement
                .query_map(params![project_id.to_string()], map_row)
                .map_err(storage_error)?
        } else {
            statement.query_map([], map_row).map_err(storage_error)?
        };
        rows.collect::<Result<Vec<_>, _>>().map_err(storage_error)
    }
}

impl WatchlistRepository for SqliteStore {
    fn list_watchlists(&self, project_id: Option<Uuid>) -> RepositoryResult<Vec<Watchlist>> {
        self.run_integration_migrations()?;
        let connection = connection(self)?;
        let sql = if project_id.is_some() {
            "SELECT id, project_id, label, profile_url, limit_count, last_scan_at
             FROM watchlists WHERE project_id = ?1 ORDER BY label COLLATE NOCASE"
        } else {
            "SELECT id, project_id, label, profile_url, limit_count, last_scan_at
             FROM watchlists ORDER BY label COLLATE NOCASE"
        };
        let mut statement = connection.prepare(sql).map_err(storage_error)?;
        let map_row = |row: &rusqlite::Row<'_>| -> rusqlite::Result<Watchlist> {
            Ok(Watchlist {
                id: parse_uuid(row.get::<_, String>(0)?)?,
                project_id: parse_uuid(row.get::<_, String>(1)?)?,
                label: row.get(2)?,
                profile_url: row.get(3)?,
                limit_count: row.get::<_, u32>(4)?,
                last_scan_at: row.get(5)?,
            })
        };
        let rows = if let Some(project_id) = project_id {
            statement
                .query_map(params![project_id.to_string()], map_row)
                .map_err(storage_error)?
        } else {
            statement.query_map([], map_row).map_err(storage_error)?
        };
        rows.collect::<Result<Vec<_>, _>>().map_err(storage_error)
    }
}

fn parse_uuid(value: String) -> rusqlite::Result<Uuid> {
    Uuid::parse_str(&value).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(error))
    })
}

fn table_exists(connection: &Connection, table: &str) -> RepositoryResult<bool> {
    connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1)",
            params![table],
            |row| row.get(0),
        )
        .map_err(storage_error)
}

fn legacy_uuid(kind: &str, key: &str) -> Uuid {
    Uuid::new_v5(
        &Uuid::NAMESPACE_URL,
        format!("scriptotar://legacy/{kind}/{key}").as_bytes(),
    )
}

fn ensure_project(tx: &Transaction<'_>, name: &str, created_at: &str) -> RepositoryResult<Uuid> {
    let normalized = if name.trim().is_empty() {
        "Inbox"
    } else {
        name.trim()
    };
    if let Some(id) = tx
        .query_row(
            "SELECT id FROM projects WHERE name = ?1 COLLATE NOCASE",
            params![normalized],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(storage_error)?
    {
        return Uuid::parse_str(&id).map_err(|error| RepositoryError::Storage(error.to_string()));
    }
    let id = legacy_uuid("project", &normalized.to_ascii_lowercase());
    tx.execute(
        "INSERT INTO projects(id, name, created_at) VALUES(?1, ?2, ?3)",
        params![id.to_string(), normalized, created_at],
    )
    .map_err(storage_error)?;
    Ok(id)
}

fn project_id_for(
    tx: &Transaction<'_>,
    projects: &mut HashMap<String, Uuid>,
    name: &str,
) -> RepositoryResult<Uuid> {
    if let Some(id) = projects.get(name) {
        return Ok(*id);
    }
    let id = ensure_project(tx, name, &now_rfc3339())?;
    projects.insert(name.to_owned(), id);
    Ok(id)
}

fn legacy_job_state(status: &str) -> JobState {
    match status.trim().to_ascii_lowercase().as_str() {
        "queued" => JobState::Queued,
        "done" | "completed" => JobState::Completed,
        "failed" => JobState::Failed,
        "canceled" | "cancelled" => JobState::Cancelled,
        _ => JobState::Interrupted,
    }
}

#[derive(Debug)]
struct LegacyJob {
    id: String,
    created_at: String,
    source: String,
    input_type: String,
    title: Option<String>,
    status: String,
    language: Option<String>,
    output_dir: Option<String>,
    transcript: Option<String>,
    error: Option<String>,
    project: String,
}

#[derive(Debug)]
struct LegacyResearch {
    storage_id: String,
    project: String,
    creator_url: String,
    source_url: Option<String>,
    platform: Option<String>,
    title: Option<String>,
    view_count: Option<i64>,
    like_count: Option<i64>,
    comment_count: Option<i64>,
    engagement_rate: Option<f64>,
    published_at: Option<String>,
    duration: Option<f64>,
    thumbnail: Option<String>,
    raw_json: Option<String>,
    scanned_at: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    use scriptotar_core::{JobInput, Project, ProjectRepository};
    use tempfile::TempDir;

    fn new_store(temp: &TempDir) -> SqliteStore {
        let store = SqliteStore::open(temp.path().join("next.sqlite3")).unwrap();
        store.run_integration_migrations().unwrap();
        store
    }

    #[test]
    fn integration_migration_is_transactional_and_idempotent() {
        let temp = TempDir::new().unwrap();
        let store = new_store(&temp);
        store.run_integration_migrations().unwrap();
        let connection = connection(&store).unwrap();
        let count: u32 = connection
            .query_row(
                "SELECT COUNT(*) FROM integration_schema_migrations",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, LATEST_INTEGRATION_SCHEMA_VERSION);
    }

    #[test]
    fn transcription_commit_finishes_job_atomically() {
        let temp = TempDir::new().unwrap();
        let store = new_store(&temp);
        let project = Project::new("Inbox");
        store.create_project(&project).unwrap();
        let job = Job::new(project.id, JobInput::LocalFile("/tmp/a.mp4".to_owned()));
        store.insert_job(&job).unwrap();
        store.transition_job(job.id, JobState::Preparing).unwrap();
        store
            .transition_job(job.id, JobState::Transcribing)
            .unwrap();
        store.transition_job(job.id, JobState::Processing).unwrap();
        let now = now_rfc3339();
        let source = Source {
            id: Uuid::new_v4(),
            project_id: project.id,
            creator_id: None,
            source_type: SourceType::LocalFile,
            locator: "/tmp/a.mp4".to_owned(),
            title: Some("a".to_owned()),
            created_at: now.clone(),
        };
        let media = Media {
            id: Uuid::new_v4(),
            source_id: source.id,
            local_path: "/tmp/a.mp4".to_owned(),
            duration_seconds: Some(1.0),
            mime_type: None,
            created_at: now.clone(),
        };
        let transcript = Transcript {
            id: Uuid::new_v4(),
            media_id: media.id,
            language: Some("en".to_owned()),
            text: "hello".to_owned(),
            segments_json: Some("[]".to_owned()),
            words_json: Some("[]".to_owned()),
            created_at: now.clone(),
            updated_at: now,
        };
        let completed = store
            .persist_transcription(job.id, &source, &media, &transcript)
            .unwrap();
        assert_eq!(completed.state, JobState::Completed);
        assert_eq!(store.list_transcripts(Some(project.id)).unwrap().len(), 1);
    }

    #[test]
    fn legacy_import_is_idempotent_and_creates_backup() {
        let temp = TempDir::new().unwrap();
        let legacy_path = temp.path().join("history.sqlite3");
        let legacy = Connection::open(&legacy_path).unwrap();
        legacy.execute_batch(
            "CREATE TABLE projects(name TEXT PRIMARY KEY, created_at TEXT NOT NULL);
             CREATE TABLE jobs(
                id TEXT PRIMARY KEY, created_at TEXT NOT NULL, source TEXT NOT NULL,
                input_type TEXT NOT NULL, title TEXT, status TEXT NOT NULL, language TEXT,
                output_dir TEXT, transcript TEXT, error TEXT, project TEXT NOT NULL
             );
             CREATE TABLE research_items(
                storage_id TEXT PRIMARY KEY, project TEXT NOT NULL, creator_url TEXT NOT NULL,
                source_url TEXT, platform TEXT, title TEXT, view_count INTEGER, like_count INTEGER,
                comment_count INTEGER, engagement_rate REAL, published_at TEXT, duration REAL,
                thumbnail TEXT, raw_json TEXT, scanned_at TEXT NOT NULL
             );
             CREATE TABLE watchlists(
                id INTEGER PRIMARY KEY AUTOINCREMENT, project TEXT NOT NULL, label TEXT NOT NULL,
                profile_url TEXT NOT NULL, limit_count INTEGER NOT NULL DEFAULT 25, last_scan_at TEXT
             );
             CREATE TABLE ai_runs(
                id TEXT PRIMARY KEY, created_at TEXT NOT NULL, project TEXT NOT NULL, task TEXT NOT NULL,
                mode TEXT NOT NULL, provider TEXT, model TEXT, source_title TEXT, prompt TEXT NOT NULL, result TEXT
             );
             INSERT INTO projects VALUES('Inbox','2026-08-09 00:00:00');
             INSERT INTO jobs VALUES('abc123','2026-08-09 00:00:00','/tmp/a.mp4','file','A','Done','en','/tmp/out','hello',NULL,'Inbox');
             INSERT INTO research_items VALUES('r1','Inbox','https://youtube.com/@x','https://youtube.com/watch?v=x','youtube','X',10,2,1,0.3,'20260809',12.0,'thumb',NULL,'2026-08-09 00:00:00');
             INSERT INTO watchlists(project,label,profile_url,limit_count,last_scan_at) VALUES('Inbox','X','https://youtube.com/@x',25,NULL);
             INSERT INTO ai_runs VALUES('ai1','2026-08-09 00:00:00','Inbox','Viral breakdown','Copy prompt only',NULL,NULL,'A','prompt','result');"
        ).unwrap();
        drop(legacy);

        let store = new_store(&temp);
        let first = store.import_legacy_database(&legacy_path).unwrap();
        assert!(!first.skipped);
        assert_eq!(first.jobs, 1);
        assert_eq!(first.transcripts, 1);
        assert_eq!(first.research_items, 1);
        assert_eq!(first.watchlists, 1);
        assert_eq!(first.ai_runs, 1);
        assert!(PathBuf::from(first.backup_path.unwrap()).is_file());
        let second = store.import_legacy_database(&legacy_path).unwrap();
        assert!(second.skipped);
        assert_eq!(store.list_transcripts(None).unwrap().len(), 1);
        assert_eq!(store.list_watchlists(None).unwrap().len(), 1);
    }
}
