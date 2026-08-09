import type { AiPromptInput, ResearchQuery, ScriptotarApi } from './client';
import type { AppSettings, BootstrapData } from '../types';

export type TauriInvoke = <T>(command: string, args?: Record<string, unknown>) => Promise<T>;

export function createTauriClient(invoke: TauriInvoke): ScriptotarApi {
  return {
    bootstrap: () => invoke<BootstrapData>('bootstrap_app'),
    selectProject: (projectId) => invoke<void>('select_project', { projectId }),
    scanCreator: (query: ResearchQuery) => invoke<void>('scan_creator', { query }),
    queueResearch: (ids) => invoke<void>('queue_research', { ids }),
    cancelJob: (jobId) => invoke<void>('cancel_job', { jobId }),
    buildAiPrompt: (input: AiPromptInput) => invoke<string>('build_ai_prompt', { input }),
    runAi: (input: AiPromptInput) => invoke<string>('run_ai', { input }),
    saveSettings: (settings: AppSettings) => invoke<void>('save_settings', { settings })
  };
}
