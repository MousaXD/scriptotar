import type { AiPromptInput, ResearchQuery, ScriptotarApi } from './client';
import type {
  AppSettings,
  BackendJob,
  BootstrapData,
  Job,
  MigrationStatus,
  WatchlistStatus
} from '../types';

export type TauriInvoke = <T>(command: string, args?: Record<string, unknown>) => Promise<T>;
export type TauriListen = <T>(
  event: string,
  handler: (event: { payload: T }) => void
) => Promise<() => void>;
type CoreBootstrapData = Omit<BootstrapData, 'watchlistStatuses' | 'migrationStatus'>;

async function hydrateBootstrap(
  invoke: TauriInvoke,
  core: Promise<CoreBootstrapData>
): Promise<BootstrapData> {
  const [snapshot, watchlistStatuses, migrationStatus] = await Promise.all([
    core,
    invoke<WatchlistStatus[]>('get_watchlist_statuses'),
    invoke<MigrationStatus>('get_migration_status')
  ]);
  return { ...snapshot, watchlistStatuses, migrationStatus };
}

export function createTauriClient(invoke: TauriInvoke, listen?: TauriListen): ScriptotarApi {
  return {
    bootstrap: () => hydrateBootstrap(invoke, invoke<CoreBootstrapData>('bootstrap_app')),
    listJobs: () => invoke<Job[]>('list_jobs'),
    searchTranscripts: (query, limit = 10) =>
      invoke<string[]>('search_transcripts', { query, limit }),
    subscribeJobChanges: (listener) =>
      listen
        ? listen<string>('scriptotar://job-changed', (event) => listener(event.payload))
        : Promise.resolve(() => {}),
    getWatchlistStatuses: () => invoke<WatchlistStatus[]>('get_watchlist_statuses'),
    getMigrationStatus: () => invoke<MigrationStatus>('get_migration_status'),
    retryLegacyMigration: () => invoke<MigrationStatus>('retry_legacy_migration'),
    selectLegacyMigrationCandidate: (candidateId) =>
      invoke<MigrationStatus>('select_legacy_migration_candidate', { candidateId }),
    selectProject: (projectId) =>
      hydrateBootstrap(invoke, invoke<CoreBootstrapData>('select_project', { projectId })),
    createProject: (name) =>
      hydrateBootstrap(invoke, invoke<CoreBootstrapData>('create_project', { name })),
    chooseLocalMedia: () => invoke<string | null>('choose_local_media'),
    chooseOutputDirectory: () => invoke<string | null>('choose_output_directory'),
    enqueueLocalMedia: (projectId, path) => invoke<BackendJob>('enqueue_local_media', { projectId, path }),
    enqueueUrl: (projectId, url) => invoke<BackendJob>('enqueue_url', { projectId, url }),
    retryJob: (jobId) => invoke<BackendJob>('retry_job', { jobId }),
    saveWatchlist: (query: ResearchQuery) =>
      hydrateBootstrap(invoke, invoke<CoreBootstrapData>('save_watchlist', { query })),
    scanCreator: (query: ResearchQuery) => invoke<void>('scan_creator', { query }),
    queueResearch: (ids) => invoke<void>('queue_research', { ids }),
    cancelJob: (jobId) => invoke<void>('cancel_job', { jobId }),
    buildAiPrompt: (input: AiPromptInput) => invoke<string>('build_ai_prompt', { input }),
    runAi: (input: AiPromptInput) => invoke<string>('run_ai', { input }),
    getSettings: () => invoke<AppSettings>('get_settings'),
    saveSettings: (settings: AppSettings) => invoke<void>('save_settings', { settings }),
  };
}
