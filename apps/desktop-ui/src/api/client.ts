import type {
  AiMode,
  AiProvider,
  AppSettings,
  BackendJob,
  BootstrapData,
  LegacyImportReport
} from '../types';

export interface ResearchQuery {
  profileUrl: string;
  limit: number;
}

export interface AiPromptInput {
  mode: AiMode;
  provider: AiProvider;
  model: string;
  task: string;
  sourceText: string;
  topic: string;
  audience: string;
  duration: string;
  cta: string;
  voice: string;
  baseUrl?: string;
  apiKey?: string;
}

export interface ScriptotarApi {
  bootstrap(): Promise<BootstrapData>;
  selectProject(projectId: string): Promise<BootstrapData>;
  createProject(name: string): Promise<BootstrapData>;
  enqueueLocalMedia(projectId: string, path: string): Promise<BackendJob>;
  enqueueUrl(projectId: string, url: string): Promise<BackendJob>;
  retryJob(jobId: string): Promise<BackendJob>;
  saveWatchlist(query: ResearchQuery): Promise<BootstrapData>;
  scanCreator(query: ResearchQuery): Promise<void>;
  refreshWatchlists(): Promise<number>;
  queueResearch(ids: string[]): Promise<void>;
  cancelJob(jobId: string): Promise<void>;
  buildAiPrompt(input: AiPromptInput): Promise<string>;
  runAi(input: AiPromptInput): Promise<string>;
  getSettings(): Promise<AppSettings>;
  saveSettings(settings: AppSettings): Promise<void>;
  importLegacyData(): Promise<LegacyImportReport>;
}
