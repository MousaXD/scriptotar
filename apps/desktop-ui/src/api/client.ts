import type {
  AiMode,
  AiProvider,
  AppSettings,
  BackendJob,
  BootstrapData,
  Job,
  MigrationStatus,
  McpPermissions,
  McpGatewayStatus,
  McpConnectionStatus,
  Transcript,
  WatchlistStatus
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
  listJobs(): Promise<Job[]>;
  searchTranscripts(query: string, limit?: number): Promise<string[]>;
  getTranscript(id: string): Promise<Transcript>;
  subscribeJobChanges(listener: (jobId: string) => void): Promise<() => void>;
  getWatchlistStatuses(): Promise<WatchlistStatus[]>;
  getMigrationStatus(): Promise<MigrationStatus>;
  retryLegacyMigration(): Promise<MigrationStatus>;
  selectLegacyMigrationCandidate(candidateId: string): Promise<MigrationStatus>;
  selectProject(projectId: string): Promise<BootstrapData>;
  createProject(name: string): Promise<BootstrapData>;
  chooseLocalMedia(): Promise<string | null>;
  chooseOutputDirectory(): Promise<string | null>;
  enqueueLocalMedia(projectId: string, path: string): Promise<BackendJob>;
  enqueueUrl(projectId: string, url: string): Promise<BackendJob>;
  retryJob(jobId: string): Promise<BackendJob>;
  saveWatchlist(query: ResearchQuery): Promise<BootstrapData>;
  scanCreator(query: ResearchQuery): Promise<void>;
  queueResearch(ids: string[]): Promise<void>;
  cancelJob(jobId: string): Promise<void>;
  buildAiPrompt(input: AiPromptInput): Promise<string>;
  runAi(input: AiPromptInput): Promise<string>;
  getSettings(): Promise<AppSettings>;
  saveSettings(settings: AppSettings): Promise<void>;
  getMcpPermissions(): Promise<McpPermissions>;
  mcpConnectionStatus(): Promise<McpConnectionStatus>;
  mcpConnectCloudflare(): Promise<McpConnectionStatus>;
  mcpConnectOpenai(tunnelId: string, apiKey: string, rememberKey: boolean): Promise<McpConnectionStatus>;
  mcpConnectManual(hostname: string): Promise<McpConnectionStatus>;
  mcpDisconnect(): Promise<McpConnectionStatus>;
  mcpGatewayStatus(): Promise<McpGatewayStatus>;
  startMcpGateway(): Promise<McpGatewayStatus>;
  stopMcpGateway(): Promise<McpGatewayStatus>;
  saveMcpPermissions(permissions: McpPermissions, acknowledgeSensitiveAccess: boolean): Promise<McpPermissions>;
}
