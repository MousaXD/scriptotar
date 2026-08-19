from pathlib import Path


def replace_once(path: str, old: str, new: str, label: str) -> None:
    target = Path(path)
    text = target.read_text()
    if old not in text:
        raise RuntimeError(f"missing expected fragment for {label} in {path}")
    target.write_text(text.replace(old, new, 1))


DB = 'crates/scriptotar-db/src/integration.rs'
replace_once(
    DB,
    '''        rows.collect::<Result<HashMap<_, _>, _>>()
            .map_err(storage_error)
    }

    pub fn import_legacy_database(
''',
    '''        rows.collect::<Result<HashMap<_, _>, _>>()
            .map_err(storage_error)
    }

    pub fn get_transcript_bundle(&self, transcript_id: Uuid) -> RepositoryResult<TranscriptBundle> {
        let connection = connection(self)?;
        connection
            .query_row(
                "SELECT s.project_id, s.id, s.creator_id, s.source_type, s.locator, s.title, s.created_at,
                        m.id, m.local_path, m.duration_seconds, m.mime_type, m.created_at,
                        t.id, t.language, t.text, t.segments_json, t.words_json, t.created_at, t.updated_at
                 FROM transcripts t
                 JOIN media m ON m.id = t.media_id
                 JOIN sources s ON s.id = m.source_id
                 WHERE t.id = ?1",
                params![transcript_id.to_string()],
                |row| {
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
                },
            )
            .optional()
            .map_err(storage_error)?
            .ok_or_else(|| RepositoryError::NotFound(format!("transcript {transcript_id}")))
    }

    pub fn import_legacy_database(
''',
    'exact transcript lookup',
)
replace_once(
    DB,
    '''        assert_eq!(completed.state, JobState::Completed);
        assert_eq!(store.list_transcripts(Some(project.id)).unwrap().len(), 1);
        assert_eq!(
''',
    '''        assert_eq!(completed.state, JobState::Completed);
        assert_eq!(store.list_transcripts(Some(project.id)).unwrap().len(), 1);
        let loaded = store.get_transcript_bundle(transcript.id).unwrap();
        assert_eq!(loaded.project_id, project.id);
        assert_eq!(loaded.transcript.text, "hello");
        assert_eq!(loaded.source.locator, "/tmp/a.mp4");
        assert_eq!(
''',
    'exact transcript lookup regression test',
)

SERVICES = 'apps/desktop/src-tauri/src/services.rs'
replace_once(
    SERVICES,
    '''    pub fn search_transcripts(&self, query: &str, limit: usize) -> RepositoryResult<Vec<String>> {
        let active_project = self.active_project_id()?;
        self.store
            .search_transcript_ids(active_project, query, limit)
            .map(|ids| ids.into_iter().map(|id| id.to_string()).collect())
    }

    pub fn select_project(&self, project_id: Uuid) -> RepositoryResult<BootstrapData> {
''',
    '''    pub fn search_transcripts(&self, query: &str, limit: usize) -> RepositoryResult<Vec<String>> {
        let active_project = self.active_project_id()?;
        self.store
            .search_transcript_ids(active_project, query, limit)
            .map(|ids| ids.into_iter().map(|id| id.to_string()).collect())
    }

    pub fn get_transcript(&self, transcript_id: Uuid) -> RepositoryResult<UiTranscript> {
        let active_project = self.active_project_id()?;
        let transcript = self.store.get_transcript_bundle(transcript_id)?;
        if transcript.project_id != active_project {
            return Err(RepositoryError::NotFound(format!(
                "transcript {transcript_id} in active project"
            )));
        }
        Ok(transcript_to_ui(&transcript))
    }

    pub fn select_project(&self, project_id: Uuid) -> RepositoryResult<BootstrapData> {
''',
    'AppServices exact transcript lookup',
)

TAURI = 'apps/desktop/src-tauri/src/lib.rs'
replace_once(
    TAURI,
    '''#[tauri::command]
fn search_transcripts(
    query: String,
    limit: Option<usize>,
    state: tauri::State<'_, AppServices>,
) -> Result<Vec<String>, String> {
    state
        .search_transcripts(&query, limit.unwrap_or(10))
        .map_err(command_error)
}

#[tauri::command]
fn get_watchlist_statuses(
''',
    '''#[tauri::command]
fn search_transcripts(
    query: String,
    limit: Option<usize>,
    state: tauri::State<'_, AppServices>,
) -> Result<Vec<String>, String> {
    state
        .search_transcripts(&query, limit.unwrap_or(10))
        .map_err(command_error)
}

#[tauri::command]
fn get_transcript(
    transcript_id: String,
    state: tauri::State<'_, AppServices>,
) -> Result<UiTranscript, String> {
    let transcript_id = Uuid::parse_str(&transcript_id)
        .map_err(|_| "invalid transcript id".to_owned())?;
    state.get_transcript(transcript_id).map_err(command_error)
}

#[tauri::command]
fn get_watchlist_statuses(
''',
    'Tauri get_transcript command',
)
replace_once(
    TAURI,
    '''            bootstrap_app,
            list_jobs,
            search_transcripts,
            get_watchlist_statuses,
''',
    '''            bootstrap_app,
            list_jobs,
            search_transcripts,
            get_transcript,
            get_watchlist_statuses,
''',
    'register get_transcript command',
)

CLIENT = 'apps/desktop-ui/src/api/client.ts'
replace_once(
    CLIENT,
    '''  listJobs(): Promise<Job[]>;
  searchTranscripts(query: string, limit?: number): Promise<string[]>;
''',
    '''  listJobs(): Promise<Job[]>;
  searchTranscripts(query: string, limit?: number): Promise<string[]>;
  getTranscript(id: string): Promise<Transcript>;
''',
    'frontend getTranscript contract',
)
replace_once(
    CLIENT,
    "import type { AppSettings, BackendJob, BootstrapData, Job, MigrationStatus, WatchlistStatus } from '../types';",
    "import type { AppSettings, BackendJob, BootstrapData, Job, MigrationStatus, Transcript, WatchlistStatus } from '../types';",
    'frontend Transcript type import',
)

TAURI_CLIENT = 'apps/desktop-ui/src/api/tauriClient.ts'
replace_once(
    TAURI_CLIENT,
    '''  MigrationStatus,
  WatchlistStatus
''',
    '''  MigrationStatus,
  Transcript,
  WatchlistStatus
''',
    'tauri client transcript import',
)
replace_once(
    TAURI_CLIENT,
    '''    searchTranscripts: (query, limit = 10) =>
      invoke<string[]>('search_transcripts', { query, limit }),
    subscribeJobChanges: (listener) =>
''',
    '''    searchTranscripts: (query, limit = 10) =>
      invoke<string[]>('search_transcripts', { query, limit }),
    getTranscript: (id) => invoke<Transcript>('get_transcript', { transcriptId: id }),
    subscribeJobChanges: (listener) =>
''',
    'tauri client getTranscript',
)

MOCK = 'apps/desktop-ui/src/api/mockClient.ts'
replace_once(
    MOCK,
    '''    async searchTranscripts(rawQuery: string, limit = 10) {
      const query = rawQuery.trim().toLocaleLowerCase();
      if (!query) return [];
      return snapshot().transcripts
        .filter((transcript) => transcript.text.toLocaleLowerCase().includes(query))
        .slice(0, limit)
        .map((transcript) => transcript.id);
    },
    async subscribeJobChanges(_listener: (jobId: string) => void) { return () => {}; },
''',
    '''    async searchTranscripts(rawQuery: string, limit = 10) {
      const query = rawQuery.trim().toLocaleLowerCase();
      if (!query) return [];
      return snapshot().transcripts
        .filter((transcript) => transcript.text.toLocaleLowerCase().includes(query))
        .slice(0, limit)
        .map((transcript) => transcript.id);
    },
    async getTranscript(id: string) {
      const transcript = snapshot().transcripts.find((candidate) => candidate.id === id);
      if (!transcript) throw new Error('Transcript not found');
      return structuredClone(transcript);
    },
    async subscribeJobChanges(_listener: (jobId: string) => void) { return () => {}; },
''',
    'mock getTranscript',
)

APP = 'apps/desktop-ui/src/App.svelte'
replace_once(
    APP,
    '''    {:else if activeView === 'transcript'}
      <TranscriptView transcripts={data.transcripts} bind:selectedId={selectedTranscriptId} />
''',
    '''    {:else if activeView === 'transcript'}
      <TranscriptView {api} transcripts={data.transcripts} bind:selectedId={selectedTranscriptId} />
''',
    'pass api to TranscriptView',
)

TRANSCRIPT_VIEW = 'apps/desktop-ui/src/views/TranscriptView.svelte'
replace_once(
    TRANSCRIPT_VIEW,
    '''  import type { Transcript, TranscriptSegment } from '../types';
  import EmptyState from '../components/EmptyState.svelte';
''',
    '''  import type { Transcript, TranscriptSegment } from '../types';
  import type { ScriptotarApi } from '../api/client';
  import EmptyState from '../components/EmptyState.svelte';
''',
    'TranscriptView API type import',
)
replace_once(
    TRANSCRIPT_VIEW,
    '''  export let transcripts: Transcript[];
  export let selectedId = '';
''',
    '''  export let api: ScriptotarApi;
  export let transcripts: Transcript[];
  export let selectedId = '';
''',
    'TranscriptView api prop',
)
replace_once(
    TRANSCRIPT_VIEW,
    '''  let searchCursor = 0;
  let previousQuery = '';

  $: if (transcripts.length > 0 && !transcripts.some((item) => item.id === selectedId)) selectedId = transcripts[0].id;
  $: selected = transcripts.find((item) => item.id === selectedId) || transcripts[0];
  $: matchingSegments = selected ? selected.segments.filter((segment) => segment.text.toLowerCase().includes(query.trim().toLowerCase())) : [];
''',
    '''  let searchCursor = 0;
  let previousQuery = '';
  let selected: Transcript | undefined;
  let selectedLoadId = '';
  let selectedLoading = false;
  let selectedLoadError = '';
  let selectedGeneration = 0;

  $: if (transcripts.length > 0 && !transcripts.some((item) => item.id === selectedId)) selectedId = transcripts[0].id;
  $: if (selectedId && selectedId !== selectedLoadId) void loadSelected(selectedId);
  $: matchingSegments = selected ? selected.segments.filter((segment) => segment.text.toLowerCase().includes(query.trim().toLowerCase())) : [];
''',
    'TranscriptView lazy state',
)
replace_once(
    TRANSCRIPT_VIEW,
    '''  function readPanel(key: string, fallback: boolean) {
''',
    '''  async function loadSelected(id: string) {
    selectedLoadId = id;
    selectedLoading = true;
    selectedLoadError = '';
    const generation = ++selectedGeneration;
    try {
      const transcript = await api.getTranscript(id);
      if (generation === selectedGeneration && selectedId === id) selected = transcript;
    } catch (cause) {
      if (generation === selectedGeneration && selectedId === id) {
        selected = undefined;
        selectedLoadError = cause instanceof Error ? cause.message : 'Could not load the transcript.';
      }
    } finally {
      if (generation === selectedGeneration) selectedLoading = false;
    }
  }

  function readPanel(key: string, fallback: boolean) {
''',
    'TranscriptView lazy loader',
)
replace_once(
    TRANSCRIPT_VIEW,
    '''  function selectTranscript(id: string) {
    selectedId = id;
    query = '';
''',
    '''  function selectTranscript(id: string) {
    if (selectedId !== id) selected = undefined;
    selectedId = id;
    query = '';
''',
    'TranscriptView selection clears detail',
)
replace_once(
    TRANSCRIPT_VIEW,
    '''{#if !selected}
  <EmptyState title="No transcripts yet" message="Complete a transcription job and it will appear here." />
{:else}
''',
    '''{#if transcripts.length === 0}
  <EmptyState title="No transcripts yet" message="Complete a transcription job and it will appear here." />
{:else if !selected}
  <section class="panel transcript-loading" aria-busy={selectedLoading}>
    <strong>{selectedLoading ? 'Loading transcript…' : 'Transcript unavailable'}</strong>
    {#if selectedLoadError}<p role="alert">{selectedLoadError}</p>{/if}
  </section>
{:else}
''',
    'TranscriptView loading state',
)

AI = 'apps/desktop-ui/src/views/AiStudioView.svelte'
replace_once(
    AI,
    '''<script lang="ts">
  import { translator } from '../i18n/translate';
''',
    '''<script lang="ts">
  import { onMount } from 'svelte';
  import { translator } from '../i18n/translate';
''',
    'AI onMount import',
)
replace_once(
    AI,
    '''  let selectedTranscriptId = transcripts[0]?.id || 'manual';
  let sourceText = transcripts[0]?.text || '';
''',
    '''  let selectedTranscriptId = transcripts[0]?.id || 'manual';
  let sourceText = '';
  let sourceLoadGeneration = 0;
  let sourceLoading = false;
  let sourceError = '';
''',
    'AI lazy source state',
)
replace_once(
    AI,
    '''  function chooseSource(event: Event) {
    const id = (event.currentTarget as HTMLSelectElement).value;
    selectedTranscriptId = id;
    if (id === 'manual') {
      sourceText = '';
      return;
    }
    const transcript = transcripts.find((candidate) => candidate.id === id);
    sourceText = transcript?.text || '';
  }

  async function buildPrompt() {
''',
    '''  async function loadSource(id: string) {
    selectedTranscriptId = id;
    sourceError = '';
    const generation = ++sourceLoadGeneration;
    if (id === 'manual') {
      sourceLoading = false;
      sourceText = '';
      return;
    }
    sourceLoading = true;
    sourceText = '';
    try {
      const transcript = await api.getTranscript(id);
      if (generation === sourceLoadGeneration && selectedTranscriptId === id) sourceText = transcript.text;
    } catch (cause) {
      if (generation === sourceLoadGeneration && selectedTranscriptId === id) {
        sourceError = cause instanceof Error ? cause.message : 'Could not load transcript source.';
      }
    } finally {
      if (generation === sourceLoadGeneration) sourceLoading = false;
    }
  }

  function chooseSource(event: Event) {
    void loadSource((event.currentTarget as HTMLSelectElement).value);
  }

  onMount(() => {
    if (selectedTranscriptId !== 'manual') void loadSource(selectedTranscriptId);
  });

  async function buildPrompt() {
''',
    'AI lazy source loader',
)
replace_once(
    AI,
    '''    <textarea bind:value={sourceText} placeholder="Paste or load transcript/research text…"></textarea>
''',
    '''    {#if sourceLoading}<p class="status-copy" aria-live="polite">Loading transcript source…</p>{/if}
    {#if sourceError}<p class="status-copy" role="alert">{sourceError}</p>{/if}
    <textarea bind:value={sourceText} placeholder="Paste or load transcript/research text…"></textarea>
''',
    'AI source loading feedback',
)

AI_TEST = 'apps/desktop-ui/src/ai-source-selection.test.ts'
replace_once(
    AI_TEST,
    "import { fireEvent, render, screen } from '@testing-library/svelte';",
    "import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';",
    'AI test waitFor import',
)
replace_once(
    AI_TEST,
    '''    await fireEvent.change(selector, { target: { value: 't-en' } });

    expect(selector).toHaveValue('t-en');
    expect(screen.getByTestId('ai-source-lineage')).toHaveTextContent('Caption pacing breakdown');
    expect(screen.getByPlaceholderText('Paste or load transcript/research text…')).toHaveValue(
      mockBootstrap.transcripts.find((transcript) => transcript.id === 't-en')?.text
    );
''',
    '''    await fireEvent.change(selector, { target: { value: 't-en' } });

    expect(selector).toHaveValue('t-en');
    expect(screen.getByTestId('ai-source-lineage')).toHaveTextContent('Caption pacing breakdown');
    await waitFor(() => expect(screen.getByPlaceholderText('Paste or load transcript/research text…')).toHaveValue(
      mockBootstrap.transcripts.find((transcript) => transcript.id === 't-en')?.text
    ));
''',
    'AI test waits for lazy source',
)

APP_TEST = 'apps/desktop-ui/src/App.test.ts'
replace_once(
    APP_TEST,
    '''  it('opens the persisted transcript from a completed job', async () => {
    const api = createMockClient();
    await ready(api);
''',
    '''  it('opens the persisted transcript from a completed job', async () => {
    const api = createMockClient();
    const getTranscript = vi.spyOn(api, 'getTranscript');
    await ready(api);
''',
    'completed job lazy lookup spy',
)
replace_once(
    APP_TEST,
    '''    expect(await screen.findByRole('heading', { name: 'Transcript workspace' })).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: 'Caption pacing breakdown' })).toBeInTheDocument();
  });
''',
    '''    expect(await screen.findByRole('heading', { name: 'Transcript workspace' })).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: 'Caption pacing breakdown' })).toBeInTheDocument();
    await waitFor(() => expect(getTranscript).toHaveBeenCalledWith('t-en'));
  });
''',
    'completed job verifies lazy detail fetch',
)

print('technical futures phase 4a patch applied')
