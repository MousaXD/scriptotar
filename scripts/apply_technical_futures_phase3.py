from pathlib import Path


def replace_once(path: str, old: str, new: str, label: str) -> None:
    target = Path(path)
    text = target.read_text()
    if old not in text:
        raise RuntimeError(f"missing expected fragment for {label} in {path}")
    target.write_text(text.replace(old, new, 1))


# ---------------------------------------------------------------------------
# Core SQLite schema v3: backfill pre-existing transcript bodies into search.
# ---------------------------------------------------------------------------
DB = "crates/scriptotar-db/src/lib.rs"
replace_once(
    DB,
    "pub const LATEST_SCHEMA_VERSION: u32 = 2;",
    "pub const LATEST_SCHEMA_VERSION: u32 = 3;",
    "core schema version 3",
)

replace_once(
    DB,
    """    pub fn schema_version(&self) -> RepositoryResult<u32> {\n        let connection = self.connection()?;\n        connection\n            .pragma_query_value(None, \"user_version\", |row| row.get::<_, u32>(0))\n            .map_err(storage_error)\n    }\n\n    fn connection(&self) -> RepositoryResult<Connection> {\n""",
    """    pub fn schema_version(&self) -> RepositoryResult<u32> {\n        let connection = self.connection()?;\n        connection\n            .pragma_query_value(None, \"user_version\", |row| row.get::<_, u32>(0))\n            .map_err(storage_error)\n    }\n\n    pub fn search_transcript_ids(\n        &self,\n        project_id: Uuid,\n        raw_query: &str,\n        limit: usize,\n    ) -> RepositoryResult<Vec<Uuid>> {\n        let Some(fts_query) = fts5_search_query(raw_query) else {\n            return Ok(Vec::new());\n        };\n        let connection = self.connection()?;\n        let fts5_enabled: i64 = connection\n            .query_row(\n                \"SELECT sqlite_compileoption_used('ENABLE_FTS5')\",\n                [],\n                |row| row.get(0),\n            )\n            .map_err(storage_error)?;\n        let limit = limit.clamp(1, 50) as i64;\n\n        if fts5_enabled == 1 {\n            let mut statement = connection\n                .prepare(\n                    \"SELECT t.id\n                     FROM transcript_fts\n                     JOIN transcripts t ON t.id = transcript_fts.transcript_id\n                     JOIN media m ON m.id = t.media_id\n                     JOIN sources s ON s.id = m.source_id\n                     WHERE s.project_id = ?1\n                       AND transcript_fts MATCH ?2\n                     ORDER BY bm25(transcript_fts), t.updated_at DESC\n                     LIMIT ?3\",\n                )\n                .map_err(storage_error)?;\n            let rows = statement\n                .query_map(\n                    params![project_id.to_string(), fts_query, limit],\n                    |row| parse_uuid(row.get::<_, String>(0)?),\n                )\n                .map_err(storage_error)?;\n            return rows\n                .collect::<Result<Vec<_>, _>>()\n                .map_err(storage_error);\n        }\n\n        let like_query = format!(\"%{}%\", escape_like(raw_query.trim()));\n        let mut statement = connection\n            .prepare(\n                \"SELECT t.id\n                 FROM transcript_fts\n                 JOIN transcripts t ON t.id = transcript_fts.transcript_id\n                 JOIN media m ON m.id = t.media_id\n                 JOIN sources s ON s.id = m.source_id\n                 WHERE s.project_id = ?1\n                   AND transcript_fts.content LIKE ?2 ESCAPE '\\\\'\n                 ORDER BY t.updated_at DESC\n                 LIMIT ?3\",\n            )\n            .map_err(storage_error)?;\n        let rows = statement\n            .query_map(\n                params![project_id.to_string(), like_query, limit],\n                |row| parse_uuid(row.get::<_, String>(0)?),\n            )\n            .map_err(storage_error)?;\n        rows.collect::<Result<Vec<_>, _>>().map_err(storage_error)\n    }\n\n    fn connection(&self) -> RepositoryResult<Connection> {\n""",
    "SQLite transcript search API",
)

replace_once(
    DB,
    """    Migration {\n        version: 2,\n        name: \"transcript_search\",\n        apply: migration_2,\n    },\n];\n""",
    """    Migration {\n        version: 2,\n        name: \"transcript_search\",\n        apply: migration_2,\n    },\n    Migration {\n        version: 3,\n        name: \"transcript_search_backfill\",\n        apply: migration_3,\n    },\n];\n""",
    "register transcript search backfill migration",
)

replace_once(
    DB,
    """    tx.execute_batch(\n        \"CREATE TRIGGER transcripts_search_insert AFTER INSERT ON transcripts BEGIN\n            INSERT INTO transcript_fts(transcript_id, content) VALUES (NEW.id, NEW.text);\n        END;\n        CREATE TRIGGER transcripts_search_update AFTER UPDATE OF text ON transcripts BEGIN\n            DELETE FROM transcript_fts WHERE transcript_id = OLD.id;\n            INSERT INTO transcript_fts(transcript_id, content) VALUES (NEW.id, NEW.text);\n        END;\n        CREATE TRIGGER transcripts_search_delete AFTER DELETE ON transcripts BEGIN\n            DELETE FROM transcript_fts WHERE transcript_id = OLD.id;\n        END;\",\n    )\n}\n\nfn storage_error(error: rusqlite::Error) -> RepositoryError {\n""",
    """    tx.execute_batch(\n        \"CREATE TRIGGER transcripts_search_insert AFTER INSERT ON transcripts BEGIN\n            INSERT INTO transcript_fts(transcript_id, content) VALUES (NEW.id, NEW.text);\n        END;\n        CREATE TRIGGER transcripts_search_update AFTER UPDATE OF text ON transcripts BEGIN\n            DELETE FROM transcript_fts WHERE transcript_id = OLD.id;\n            INSERT INTO transcript_fts(transcript_id, content) VALUES (NEW.id, NEW.text);\n        END;\n        CREATE TRIGGER transcripts_search_delete AFTER DELETE ON transcripts BEGIN\n            DELETE FROM transcript_fts WHERE transcript_id = OLD.id;\n        END;\",\n    )\n}\n\nfn migration_3(tx: &Transaction<'_>) -> rusqlite::Result<()> {\n    tx.execute(\n        \"INSERT INTO transcript_fts(transcript_id, content)\n         SELECT t.id, t.text\n         FROM transcripts t\n         WHERE NOT EXISTS (\n             SELECT 1 FROM transcript_fts existing\n             WHERE existing.transcript_id = t.id\n         )\",\n        [],\n    )?;\n    Ok(())\n}\n\nfn fts5_search_query(raw_query: &str) -> Option<String> {\n    let terms = raw_query\n        .split(|character: char| !character.is_alphanumeric())\n        .filter(|term| !term.is_empty())\n        .take(8)\n        .map(|term| format!(\"\\\"{term}\\\"*\"))\n        .collect::<Vec<_>>();\n    (!terms.is_empty()).then(|| terms.join(\" AND \"))\n}\n\nfn escape_like(value: &str) -> String {\n    value\n        .replace('\\\\', \"\\\\\\\\\")\n        .replace('%', \"\\\\%\")\n        .replace('_', \"\\\\_\")\n}\n\nfn storage_error(error: rusqlite::Error) -> RepositoryError {\n""",
    "search index backfill and safe query helpers",
)

replace_once(
    DB,
    """        let db = SqliteStore::open(&path).unwrap();\n        assert_eq!(db.schema_version().unwrap(), 2);\n        let connection = db.connection().unwrap();\n        let count: u32 = connection\n            .query_row(\n                \"SELECT COUNT(*) FROM schema_migrations WHERE version = 2\",\n                [],\n                |row| row.get(0),\n            )\n            .unwrap();\n        assert_eq!(count, 1);\n    }\n\n    #[test]\n    fn project_repository_supports_crud_reads() {\n""",
    """        let db = SqliteStore::open(&path).unwrap();\n        assert_eq!(db.schema_version().unwrap(), 3);\n        let connection = db.connection().unwrap();\n        let count: u32 = connection\n            .query_row(\n                \"SELECT COUNT(*) FROM schema_migrations WHERE version IN (2, 3)\",\n                [],\n                |row| row.get(0),\n            )\n            .unwrap();\n        assert_eq!(count, 2);\n    }\n\n    #[test]\n    fn schema_v3_backfills_existing_transcript_search_index() {\n        let temp = TempDir::new().unwrap();\n        let path = temp.path().join(\"search-upgrade.sqlite3\");\n        let project_id = Uuid::new_v4();\n        let source_id = Uuid::new_v4();\n        let media_id = Uuid::new_v4();\n        let transcript_id = Uuid::new_v4();\n        let now = now_rfc3339();\n        {\n            let mut connection = Connection::open(&path).unwrap();\n            configure_connection(&connection).unwrap();\n            connection\n                .execute_batch(\n                    \"CREATE TABLE schema_migrations (\n                        version INTEGER PRIMARY KEY,\n                        name TEXT NOT NULL,\n                        applied_at TEXT NOT NULL\n                    );\",\n                )\n                .unwrap();\n            let tx = connection.transaction().unwrap();\n            migration_1(&tx).unwrap();\n            tx.execute(\n                \"INSERT INTO schema_migrations(version, name, applied_at)\n                 VALUES(1, 'initial_domain_schema', ?1)\",\n                params![now],\n            )\n            .unwrap();\n            tx.pragma_update(None, \"user_version\", 1).unwrap();\n            tx.commit().unwrap();\n\n            connection\n                .execute(\n                    \"INSERT INTO projects(id, name, created_at) VALUES(?1, 'Search Upgrade', ?2)\",\n                    params![project_id.to_string(), now],\n                )\n                .unwrap();\n            connection\n                .execute(\n                    \"INSERT INTO sources(id, project_id, creator_id, source_type, locator, title, created_at)\n                     VALUES(?1, ?2, NULL, 'local_file', '/tmp/search-upgrade.mp4', 'Search Upgrade', ?3)\",\n                    params![source_id.to_string(), project_id.to_string(), now],\n                )\n                .unwrap();\n            connection\n                .execute(\n                    \"INSERT INTO media(id, source_id, local_path, duration_seconds, mime_type, created_at)\n                     VALUES(?1, ?2, '/tmp/search-upgrade.mp4', 12.0, NULL, ?3)\",\n                    params![media_id.to_string(), source_id.to_string(), now],\n                )\n                .unwrap();\n            connection\n                .execute(\n                    \"INSERT INTO transcripts(\n                        id, media_id, language, text, segments_json, words_json, created_at, updated_at\n                     ) VALUES(?1, ?2, 'ar', 'ابدأ بالنتيجة ثم اشرح السبب', NULL, NULL, ?3, ?3)\",\n                    params![transcript_id.to_string(), media_id.to_string(), now],\n                )\n                .unwrap();\n\n            let tx = connection.transaction().unwrap();\n            migration_2(&tx).unwrap();\n            tx.execute(\n                \"INSERT INTO schema_migrations(version, name, applied_at)\n                 VALUES(2, 'transcript_search', ?1)\",\n                params![now],\n            )\n            .unwrap();\n            tx.pragma_update(None, \"user_version\", 2).unwrap();\n            tx.commit().unwrap();\n        }\n\n        let db = SqliteStore::open(&path).unwrap();\n        assert_eq!(db.schema_version().unwrap(), 3);\n        assert_eq!(\n            db.search_transcript_ids(project_id, \"النتيجة\", 10).unwrap(),\n            vec![transcript_id]\n        );\n    }\n\n    #[test]\n    fn transcript_search_is_project_scoped_and_safe_for_punctuation() {\n        let (_temp, db) = store();\n        let first = project(&db, \"First\");\n        let second = project(&db, \"Second\");\n        let first_transcript = Uuid::new_v4();\n        let second_transcript = Uuid::new_v4();\n        let now = now_rfc3339();\n\n        for (project_id, transcript_id, suffix) in [\n            (first.id, first_transcript, \"first\"),\n            (second.id, second_transcript, \"second\"),\n        ] {\n            let source_id = Uuid::new_v4();\n            let media_id = Uuid::new_v4();\n            let connection = db.connection().unwrap();\n            connection\n                .execute(\n                    \"INSERT INTO sources(id, project_id, creator_id, source_type, locator, title, created_at)\n                     VALUES(?1, ?2, NULL, 'local_file', ?3, NULL, ?4)\",\n                    params![\n                        source_id.to_string(),\n                        project_id.to_string(),\n                        format!(\"/tmp/{suffix}.mp4\"),\n                        now,\n                    ],\n                )\n                .unwrap();\n            connection\n                .execute(\n                    \"INSERT INTO media(id, source_id, local_path, duration_seconds, mime_type, created_at)\n                     VALUES(?1, ?2, ?3, 4.0, NULL, ?4)\",\n                    params![\n                        media_id.to_string(),\n                        source_id.to_string(),\n                        format!(\"/tmp/{suffix}.mp4\"),\n                        now,\n                    ],\n                )\n                .unwrap();\n            connection\n                .execute(\n                    \"INSERT INTO transcripts(\n                        id, media_id, language, text, segments_json, words_json, created_at, updated_at\n                     ) VALUES(?1, ?2, 'en', 'visual payoff retention signal', NULL, NULL, ?3, ?3)\",\n                    params![transcript_id.to_string(), media_id.to_string(), now],\n                )\n                .unwrap();\n        }\n\n        assert_eq!(\n            db.search_transcript_ids(first.id, \"payoff!!!\", 10).unwrap(),\n            vec![first_transcript]\n        );\n        assert!(db.search_transcript_ids(first.id, \"%%%\", 10).unwrap().is_empty());\n    }\n\n    #[test]\n    fn project_repository_supports_crud_reads() {\n""",
    "core search migration and query tests",
)

# ---------------------------------------------------------------------------
# AppServices/Tauri search endpoint, scoped to the active project.
# ---------------------------------------------------------------------------
SERVICES = "apps/desktop/src-tauri/src/services.rs"
replace_once(
    SERVICES,
    """    pub fn list_jobs(&self) -> RepositoryResult<Vec<UiJob>> {\n        let active_project = self.active_project_id()?;\n        let jobs = self.store.list_jobs(Some(active_project))?;\n        let transcript_links = self.store.list_job_transcript_links(Some(active_project))?;\n        Ok(jobs\n            .iter()\n            .map(|job| job_to_ui(job, transcript_links.get(&job.id).copied()))\n            .collect())\n    }\n\n    pub fn select_project(&self, project_id: Uuid) -> RepositoryResult<BootstrapData> {\n""",
    """    pub fn list_jobs(&self) -> RepositoryResult<Vec<UiJob>> {\n        let active_project = self.active_project_id()?;\n        let jobs = self.store.list_jobs(Some(active_project))?;\n        let transcript_links = self.store.list_job_transcript_links(Some(active_project))?;\n        Ok(jobs\n            .iter()\n            .map(|job| job_to_ui(job, transcript_links.get(&job.id).copied()))\n            .collect())\n    }\n\n    pub fn search_transcripts(&self, query: &str, limit: usize) -> RepositoryResult<Vec<String>> {\n        let active_project = self.active_project_id()?;\n        self.store\n            .search_transcript_ids(active_project, query, limit)\n            .map(|ids| ids.into_iter().map(|id| id.to_string()).collect())\n    }\n\n    pub fn select_project(&self, project_id: Uuid) -> RepositoryResult<BootstrapData> {\n""",
    "services transcript search",
)

TAURI = "apps/desktop/src-tauri/src/lib.rs"
replace_once(
    TAURI,
    """#[tauri::command]\nfn list_jobs(state: tauri::State<'_, AppServices>) -> Result<Vec<UiJob>, String> {\n    state.list_jobs().map_err(command_error)\n}\n\n#[tauri::command]\nfn get_watchlist_statuses(\n""",
    """#[tauri::command]\nfn list_jobs(state: tauri::State<'_, AppServices>) -> Result<Vec<UiJob>, String> {\n    state.list_jobs().map_err(command_error)\n}\n\n#[tauri::command]\nfn search_transcripts(\n    query: String,\n    limit: Option<usize>,\n    state: tauri::State<'_, AppServices>,\n) -> Result<Vec<String>, String> {\n    state\n        .search_transcripts(&query, limit.unwrap_or(10))\n        .map_err(command_error)\n}\n\n#[tauri::command]\nfn get_watchlist_statuses(\n""",
    "Tauri transcript search command",
)
replace_once(
    TAURI,
    """            bootstrap_app,\n            list_jobs,\n            get_watchlist_statuses,\n""",
    """            bootstrap_app,\n            list_jobs,\n            search_transcripts,\n            get_watchlist_statuses,\n""",
    "register transcript search command",
)

# ---------------------------------------------------------------------------
# Typed frontend search API.
# ---------------------------------------------------------------------------
CLIENT = "apps/desktop-ui/src/api/client.ts"
replace_once(
    CLIENT,
    """  listJobs(): Promise<Job[]>;\n  subscribeJobChanges(listener: (jobId: string) => void): Promise<() => void>;\n""",
    """  listJobs(): Promise<Job[]>;\n  searchTranscripts(query: string, limit?: number): Promise<string[]>;\n  subscribeJobChanges(listener: (jobId: string) => void): Promise<() => void>;\n""",
    "frontend transcript search API contract",
)

TAURI_CLIENT = "apps/desktop-ui/src/api/tauriClient.ts"
replace_once(
    TAURI_CLIENT,
    """    bootstrap: () => hydrateBootstrap(invoke, invoke<CoreBootstrapData>('bootstrap_app')),\n    listJobs: () => invoke<Job[]>('list_jobs'),\n    subscribeJobChanges: (listener) =>\n""",
    """    bootstrap: () => hydrateBootstrap(invoke, invoke<CoreBootstrapData>('bootstrap_app')),\n    listJobs: () => invoke<Job[]>('list_jobs'),\n    searchTranscripts: (query, limit = 10) =>\n      invoke<string[]>('search_transcripts', { query, limit }),\n    subscribeJobChanges: (listener) =>\n""",
    "Tauri transcript search client",
)

MOCK = "apps/desktop-ui/src/api/mockClient.ts"
replace_once(
    MOCK,
    """    async bootstrap() { return snapshot(); },\n    async listJobs() { return structuredClone(snapshot().jobs); },\n    async subscribeJobChanges(_listener: (jobId: string) => void) { return () => {}; },\n""",
    """    async bootstrap() { return snapshot(); },\n    async listJobs() { return structuredClone(snapshot().jobs); },\n    async searchTranscripts(rawQuery: string, limit = 10) {\n      const query = rawQuery.trim().toLocaleLowerCase();\n      if (!query) return [];\n      return snapshot().transcripts\n        .filter((transcript) => transcript.text.toLocaleLowerCase().includes(query))\n        .slice(0, limit)\n        .map((transcript) => transcript.id);\n    },\n    async subscribeJobChanges(_listener: (jobId: string) => void) { return () => {}; },\n""",
    "mock transcript search API",
)

# ---------------------------------------------------------------------------
# Global search: metadata stays local; transcript body search goes to SQLite.
# ---------------------------------------------------------------------------
APP = "apps/desktop-ui/src/App.svelte"
replace_once(
    APP,
    """  let selectedTranscriptId = '';\n  let jobRefreshInFlight = false;\n  let jobRefreshTimer: number | undefined;\n  let operationalRefreshInFlight = false;\n\n  const activeStates = new Set(['queued','preparing','downloading','transcribing','processing']);\n  $: activeProject = data?.projects.find((project) => project.id === data?.activeProjectId) || data?.projects[0];\n  $: activeJobs = data?.jobs.filter((job) => activeStates.has(job.state)).length || 0;\n  $: searchResults = data ? buildSearchResults(data, globalSearch) : [];\n""",
    """  let selectedTranscriptId = '';\n  let jobRefreshInFlight = false;\n  let jobRefreshTimer: number | undefined;\n  let operationalRefreshInFlight = false;\n  let transcriptSearchIds: string[] = [];\n  let transcriptSearchTimer: number | undefined;\n  let transcriptSearchGeneration = 0;\n  let transcriptSearchKey = '';\n\n  const activeStates = new Set(['queued','preparing','downloading','transcribing','processing']);\n  $: activeProject = data?.projects.find((project) => project.id === data?.activeProjectId) || data?.projects[0];\n  $: activeJobs = data?.jobs.filter((job) => activeStates.has(job.state)).length || 0;\n  $: searchResults = data ? buildSearchResults(data, globalSearch, new Set(transcriptSearchIds)) : [];\n  $: scheduleTranscriptSearch(globalSearch, data?.activeProjectId || '');\n""",
    "global transcript search state",
)

replace_once(
    APP,
    """  function buildSearchResults(snapshot: BootstrapData, rawQuery: string): WorkspaceSearchResult[] {\n        const query = rawQuery.trim().toLocaleLowerCase();\n""",
    """  function buildSearchResults(\n    snapshot: BootstrapData,\n    rawQuery: string,\n    transcriptMatches: Set<string>\n  ): WorkspaceSearchResult[] {\n    const query = rawQuery.trim().toLocaleLowerCase();\n""",
    "search result signature",
)
replace_once(
    APP,
    """    for (const transcript of snapshot.transcripts) {\n      if (match(transcript.title, transcript.text, transcript.language, transcript.platform)) results.push({ id: `transcript:${transcript.id}`, kind: 'Transcript', title: transcript.title, subtitle: `${transcript.language} · ${transcript.platform}`, view: 'transcript', projectId: transcript.projectId, targetId: transcript.id });\n    }\n""",
    """    for (const transcript of snapshot.transcripts) {\n      if (transcriptMatches.has(transcript.id) || match(transcript.title, transcript.language, transcript.platform)) results.push({ id: `transcript:${transcript.id}`, kind: 'Transcript', title: transcript.title, subtitle: `${transcript.language} · ${transcript.platform}`, view: 'transcript', projectId: transcript.projectId, targetId: transcript.id });\n    }\n""",
    "remove transcript body scan from frontend",
)
replace_once(
    APP,
    """    return results.slice(0, 10);\n  }\n\n  async function load(showLoading = true) {\n""",
    """    return results.slice(0, 10);\n  }\n\n  function scheduleTranscriptSearch(rawQuery: string, projectId: string) {\n    const query = rawQuery.trim();\n    const key = projectId && query ? `${projectId}\\u0000${query}` : '';\n    if (key === transcriptSearchKey) return;\n    transcriptSearchKey = key;\n    transcriptSearchGeneration += 1;\n    const generation = transcriptSearchGeneration;\n    if (transcriptSearchTimer !== undefined) {\n      window.clearTimeout(transcriptSearchTimer);\n      transcriptSearchTimer = undefined;\n    }\n    if (!key) {\n      transcriptSearchIds = [];\n      return;\n    }\n    transcriptSearchTimer = window.setTimeout(async () => {\n      transcriptSearchTimer = undefined;\n      try {\n        const ids = await api.searchTranscripts(query, 8);\n        if (generation === transcriptSearchGeneration && transcriptSearchKey === key) {\n          transcriptSearchIds = ids;\n        }\n      } catch {\n        if (generation === transcriptSearchGeneration && transcriptSearchKey === key) {\n          transcriptSearchIds = [];\n        }\n      }\n    }, 160);\n  }\n\n  async function load(showLoading = true) {\n""",
    "debounced backend transcript search",
)
replace_once(
    APP,
    """      if (jobRefreshTimer !== undefined) window.clearTimeout(jobRefreshTimer);\n      window.clearInterval(jobReconcile);\n""",
    """      if (jobRefreshTimer !== undefined) window.clearTimeout(jobRefreshTimer);\n      if (transcriptSearchTimer !== undefined) window.clearTimeout(transcriptSearchTimer);\n      window.clearInterval(jobReconcile);\n""",
    "clean transcript search timer",
)

TEST = "apps/desktop-ui/src/App.test.ts"
replace_once(
    TEST,
    """  it('uses global search to open a local transcript', async () => {\n    await ready();\n    const search = screen.getByLabelText('Search workspace');\n    await fireEvent.input(search, { target: { value: 'caption pacing' } });\n    const result = await screen.findByRole('button', { name: /Caption pacing breakdown.*Transcript/ });\n    await fireEvent.click(result);\n    expect(await screen.findByRole('heading', { name: 'Transcript workspace' })).toBeInTheDocument();\n    expect(screen.getByRole('heading', { name: 'Caption pacing breakdown' })).toBeInTheDocument();\n  });\n""",
    """  it('uses global search to open a local transcript', async () => {\n    await ready();\n    const search = screen.getByLabelText('Search workspace');\n    await fireEvent.input(search, { target: { value: 'caption pacing' } });\n    const result = await screen.findByRole('button', { name: /Caption pacing breakdown.*Transcript/ });\n    await fireEvent.click(result);\n    expect(await screen.findByRole('heading', { name: 'Transcript workspace' })).toBeInTheDocument();\n    expect(screen.getByRole('heading', { name: 'Caption pacing breakdown' })).toBeInTheDocument();\n  });\n\n  it('delegates transcript-body workspace search to the backend index', async () => {\n    const api = createMockClient();\n    const searchTranscripts = vi.spyOn(api, 'searchTranscripts');\n    await ready(api);\n    const search = screen.getByLabelText('Search workspace');\n    await fireEvent.input(search, { target: { value: 'visual payoff' } });\n    await waitFor(() => expect(searchTranscripts).toHaveBeenCalledWith('visual payoff', 8));\n    expect(await screen.findByRole('button', { name: /Caption pacing breakdown.*Transcript/ })).toBeInTheDocument();\n  });\n\n  it('supports Arabic transcript-body workspace search through the backend index', async () => {\n    const api = createMockClient();\n    const searchTranscripts = vi.spyOn(api, 'searchTranscripts');\n    await ready(api);\n    const search = screen.getByLabelText('Search workspace');\n    await fireEvent.input(search, { target: { value: 'النتيجة' } });\n    await waitFor(() => expect(searchTranscripts).toHaveBeenCalledWith('النتيجة', 8));\n    expect(await screen.findByRole('button', { name: /Hook breakdown — Arabic sample.*Transcript/ })).toBeInTheDocument();\n  });\n""",
    "frontend FTS search regression tests",
)

print('technical futures phase three patch applied')
