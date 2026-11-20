import type {
  AiMode,
  AiRunDetail,
  ResearchItemDetail,
  AiProvider,
  AppSettings,
  BackendJob,
  BootstrapData,
  BootstrapSummaryData,
  Job,
  MigrationStatus,
  McpPermissions,
  McpGatewayStatus,
  McpConnectionStatus,
  Transcript,
  WatchlistStatus
} from '../types';
import type { ContentStudy, CreativeBrief, StudyAnglePrompt, StudyAngleInterpretation, StudyScriptReview, StudySourceRef, StudyEvidence, StudyEvidenceInput, StudyEvidenceReviewInput, StudyWritingBatch } from '../studio/contract';
import type { LocalInspectionRun, StudyResearchSource, StudyResearchRun, StudyResearchReport } from '../studio/agent';

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

/** Host-authored request ceiling; all numbers use nonnegative integer units. */
export interface ProviderRunBudget {
  max_tokens: number;
  max_tool_calls: number;
  max_elapsed_ms: number;
  max_cost_microusd: number;
}

/** This preview is informational only; approval is blocked until trusted pricing exists. */
export interface ProviderConsentPreview {
  run_id: string;
  fingerprint: string;
  provider: string;
  model: string;
  destination: string;
  prompt: string;
  selected_source_ids: string[];
  estimated_input_tokens: number;
  reserved_input_tokens: number;
  max_output_tokens: number;
  max_total_tokens: number;
  worst_case_cost_microusd: number | null;
  limits: ProviderRunBudget;
  expires_at_ms: number;
  egress_disclosure: string;
}

export interface ScriptotarApi {
  backendHealth(): Promise<{schema_version: number}>;
  bootstrap(): Promise<BootstrapData>;
  bootstrapSummary(): Promise<BootstrapSummaryData>;
  listJobs(): Promise<Job[]>;
  searchTranscripts(query: string, limit?: number): Promise<string[]>;
  getTranscript(id: string): Promise<Transcript>;
  getAiRun(projectId: string, aiRunId: string): Promise<AiRunDetail>;
  getResearchItem(projectId: string, researchItemId: string): Promise<ResearchItemDetail>;
  saveCopyPrompt(projectId: string, input: AiPromptInput): Promise<AiRunDetail>;
  listStudies(projectId: string): Promise<Pick<ContentStudy, 'id' | 'projectId' | 'brief' | 'revision'>[]>;
  createStudy(projectId: string, brief: CreativeBrief): Promise<ContentStudy>;
  getStudy(projectId: string, studyId: string, draftId?: string, requestId?: string): Promise<ContentStudy>;
  listStudyWritingBatches(projectId: string, studyId: string): Promise<StudyWritingBatch[]>;
  duplicateStudyDraft(projectId: string, studyId: string, draftId: string, expectedDraftRevision: number): Promise<ContentStudy>;
  renameStudyDraft(projectId: string, studyId: string, draftId: string, expectedDraftRevision: number, title: string): Promise<ContentStudy>;
  setStudyDraftArchived(projectId: string, studyId: string, draftId: string, expectedDraftRevision: number, archived: boolean): Promise<ContentStudy>;
  prepareStudyAngles(projectId: string, studyId: string): Promise<StudyAnglePrompt>;
  interpretStudyAngles(projectId: string, studyId: string, expectedStudyRevision: number,
    preparedPrompt: string, responseJson: string, selectedAngleId?: string,
    requestId?: string): Promise<StudyAngleInterpretation>;
  reviewStudyScript(projectId: string, studyId: string, expectedStudyRevision: number,
    anglePrompt: string, anglesResponseJson: string, selectedAngleId: string,
    preparedDraftPrompt: string, scriptResponseJson: string,
    acceptAsNewCandidate: boolean): Promise<StudyScriptReview>;
  revokeStudySourceAccess(projectId: string, studyId: string, expectedRevision: number,
    source: StudySourceRef): Promise<ContentStudy>;
  saveStudy(projectId: string, studyId: string, expectedRevision: number, patch: {
    brief?: CreativeBrief; sources?: StudySourceRef[]; draft?: string;
    draftId?: string; draftRevision?: number;
  }): Promise<ContentStudy>;
  listStudyEvidence(projectId: string, studyId: string): Promise<StudyEvidence[]>;
  addStudyEvidence(projectId: string, studyId: string, input: StudyEvidenceInput): Promise<StudyEvidence>;
  reviewStudyEvidence(projectId: string, studyId: string, evidenceId: string, input: StudyEvidenceReviewInput): Promise<StudyEvidence>;
  prepareProviderConsentSealed(projectId: string, studyId: string, runId: string,
    model: string, maxOutputTokens: number, limits: ProviderRunBudget): Promise<ProviderConsentPreview>;
  decideProviderConsentSealed(projectId: string, studyId: string, runId: string,
    fingerprint: string, approve: boolean): Promise<void>;
  executeProviderConsentSealed(projectId: string, studyId: string, runId: string,
    fingerprint: string, requestTimeApiKey: string): Promise<string>;
  startLocalInspection(projectId: string, studyId: string, transcriptId: string): Promise<LocalInspectionRun>;
  listLocalInspections(projectId: string, studyId: string): Promise<LocalInspectionRun[]>;
  decideLocalInspection(projectId: string, studyId: string, runId: string, action: 'approve' | 'deny' | 'cancel' | 'recover'): Promise<LocalInspectionRun>;
  listStudyResearchSources(projectId: string, studyId: string): Promise<StudyResearchSource[]>;
  startStudyResearch(projectId: string, studyId: string, sourceIds: string[]): Promise<StudyResearchRun>;
  listStudyResearchRuns(projectId: string, studyId: string): Promise<StudyResearchRun[]>;
  decideStudyResearch(projectId: string, studyId: string, runId: string, action: 'approve' | 'deny' | 'cancel' | 'recover'): Promise<StudyResearchRun>;
  readStudyResearchReport(projectId: string, studyId: string, artifactId: string): Promise<StudyResearchReport>;
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
  enqueueUrl(projectId: string, url: string, requestId?: string): Promise<BackendJob>;
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
