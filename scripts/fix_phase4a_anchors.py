from pathlib import Path

path = Path('scripts/apply_technical_futures_phase4a.py')
text = path.read_text()

old_client = '''replace_once(
    CLIENT,
    "import type { AppSettings, BackendJob, BootstrapData, Job, MigrationStatus, WatchlistStatus } from '../types';",
    "import type { AppSettings, BackendJob, BootstrapData, Job, MigrationStatus, Transcript, WatchlistStatus } from '../types';",
    'frontend Transcript type import',
)
'''
new_client = '''replace_once(
    CLIENT,
    """  Job,
  MigrationStatus,
  WatchlistStatus
} from '../types';""",
    """  Job,
  MigrationStatus,
  Transcript,
  WatchlistStatus
} from '../types';""",
    'frontend Transcript type import',
)
'''
if old_client not in text:
    raise SystemExit('phase4a client import anchor block was not found')
text = text.replace(old_client, new_client, 1)

old_tauri = "TAURI = 'apps/desktop/src-tauri/src/lib.rs'\n"
new_tauri = '''TAURI = 'apps/desktop/src-tauri/src/lib.rs'
replace_once(
    TAURI,
    """    AiPromptInput, BootstrapData, ResearchQuery, UiJob, UiMigrationStatus, UiSettings,
    UiWatchlistStatus,
""",
    """    AiPromptInput, BootstrapData, ResearchQuery, UiJob, UiMigrationStatus, UiSettings,
    UiTranscript, UiWatchlistStatus,
""",
    'Tauri UiTranscript import',
)
'''
if old_tauri not in text:
    raise SystemExit('phase4a TAURI declaration was not found')
text = text.replace(old_tauri, new_tauri, 1)

old_test = '''replace_once(
    APP_TEST,
    ''' + "'''" + '''  it('opens the persisted transcript from a completed job', async () => {
    const api = createMockClient();
    await ready(api);
''' + "'''" + ''',
    ''' + "'''" + '''  it('opens the persisted transcript from a completed job', async () => {
    const api = createMockClient();
    const getTranscript = vi.spyOn(api, 'getTranscript');
    await ready(api);
''' + "'''" + ''',
    'completed job lazy lookup spy',
)
'''
new_test = '''replace_once(
    APP_TEST,
    ''' + "'''" + '''  it('opens the persisted transcript from a completed job', async () => {
    await ready();
''' + "'''" + ''',
    ''' + "'''" + '''  it('opens the persisted transcript from a completed job', async () => {
    const api = createMockClient();
    const getTranscript = vi.spyOn(api, 'getTranscript');
    await ready(api);
''' + "'''" + ''',
    'completed job lazy lookup spy',
)
'''
if old_test not in text:
    raise SystemExit('phase4a completed-job test anchor block was not found')
text = text.replace(old_test, new_test, 1)

path.write_text(text)
