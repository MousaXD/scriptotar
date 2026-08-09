import type { AiMode, AiProvider, AppSettings, BootstrapData } from '../types';

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
  selectProject(projectId: string): Promise<void>;
  scanCreator(query: ResearchQuery): Promise<void>;
  queueResearch(ids: string[]): Promise<void>;
  cancelJob(jobId: string): Promise<void>;
  buildAiPrompt(input: AiPromptInput): Promise<string>;
  runAi(input: AiPromptInput): Promise<string>;
  saveSettings(settings: AppSettings): Promise<void>;
}
