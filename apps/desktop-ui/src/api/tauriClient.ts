import type { AiPromptInput, ResearchQuery, ScriptotarApi } from './client';
import type {
  AppSettings,
  BackendJob,
  BootstrapData,
  LegacyImportReport
} from '../types';

export type TauriInvoke = <T>(command: string, args?: Record<string, unknown>) => Promise<T>;

export function createTauriClient(invoke: TauriInvoke): ScriptotarApi {
  return {
    bootstrap: () => invoke<BootstrapData>('bootstrap_app'),
    selectProject: (projectId) => invoke<BootstrapData>('select_project', { projectId }),
    createProject: (name) => invoke<BootstrapData>('create_project', { name }),
    enqueueLocalMedia: (projectId, path) => invoke<BackendJob>('enqueue_local_media', { projectId, path }),
    enqueueUrl: (projectId, url) => invoke<BackendJob>('enqueue_url', { projectId, url }),
    retryJob: (jobId) => invoke<BackendJob>('retry_job', { jobId }),
    saveWatchlist: (query: ResearchQuery) => invoke<BootstrapData>('save_watchlist', { query }),
    scanCreator: (query: ResearchQuery) => invoke<void>('scan_creator', { query }),
    queueResearch: (ids) => invoke<void>('queue_research', { ids }),
    cancelJob: (jobId) => invoke<void>('cancel_job', { jobId }),
    buildAiPrompt: (input: AiPromptInput) => invoke<string>('build_ai_prompt', { input }),
    runAi: (input: AiPromptInput) => invoke<string>('run_ai', { input }),
    getSettings: () => invoke<AppSettings>('get_settings'),
    saveSettings: (settings: AppSettings) => invoke<void>('save_settings', { settings }),
    importLegacyData: () => invoke<LegacyImportReport>('import_legacy_data')
  };
}
