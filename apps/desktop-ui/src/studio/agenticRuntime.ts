import type { StudySourceRef } from './contract';

/**
 * Agent 06 proposed UI-only adapter. Agent 00 owns the production Tauri
 * command and shared API contracts. Methods are optional by design.
 * Never infer provider approval from this interface existing.
 */
export type StudioRunState =
  'planned' | 'awaiting_approval' | 'running' | 'needs_input' |
  'blocked' | 'failed' | 'cancelled' | 'completed';

export type StudioRunDecision = 'approve' | 'deny' | 'cancel' | 'recover';

export interface StudioRunRequest {
  provider: string;
  model: string;
  exactPrompt: string;
  selectedSources: StudySourceRef[];
  maxTokens: number;
  maxUsd: number;
}

/** Server-authoritative response, not a fabricated client-side run. */
export interface StudioRun {
  runId: string;
  projectId: string;
  studyId: string;
  studyRevision: number;
  state: StudioRunState;
  provider: string;
  model: string;
  exactPrompt: string;
  selectedSources: StudySourceRef[];
  maxTokens: number;
  maxUsd: number;
  lastError?: string | null;
  completedSteps?: number;
  totalSteps?: number;
}

export interface StudioAgentRunGateway {
  listStudioRuns?(projectId: string, studyId: string): Promise<StudioRun[]>;
  proposeStudioRun?(projectId: string, studyId: string, expectedStudyRevision: number,
    request: StudioRunRequest): Promise<StudioRun>;
  decideStudioRun?(projectId: string, studyId: string, runId: string,
    action: StudioRunDecision): Promise<StudioRun>;
}

export function isSameSourceSet(a: StudySourceRef[], b: StudySourceRef[]): boolean {
  const keys = (refs: StudySourceRef[]) => refs.map(ref => ref.kind + ':' + ref.id).sort();
  return JSON.stringify(keys(a)) === JSON.stringify(keys(b));
}

export function validatesStudioRun(run: StudioRun, projectId: string, studyId: string): boolean {
  return !!run && !!run.runId && run.projectId === projectId && run.studyId === studyId &&
    Number.isInteger(run.studyRevision) && run.studyRevision > 0 &&
    typeof run.exactPrompt === 'string' && run.exactPrompt.length <= 100_000 &&
    Array.isArray(run.selectedSources) &&
    run.selectedSources.every(ref => (ref.kind === 'transcript' || ref.kind === 'research') && !!ref.id) &&
    !!run.provider && !!run.model && Number.isFinite(run.maxTokens) &&
    Number.isFinite(run.maxUsd) && run.maxTokens > 0 && run.maxUsd >= 0 &&
    ['planned', 'awaiting_approval', 'running', 'needs_input', 'blocked', 'failed', 'cancelled', 'completed']
      .includes(run.state);
}

export function limitsValid(tokens: string, dollars: string): boolean {
  if (!/^[1-9][0-9]*$/.test(tokens) || !/^(0|[1-9][0-9]{0,2})(\.[0-9]{1,2})?$/.test(dollars)) return false;
  return Number(tokens) <= 200000 && Number(dollars) <= 100;
}