/** A scoped, durable agent snapshot. No provider response or secret is included. */
export interface LocalInspectionRun {
  schema_version: number;
  revision: number;
  state: 'planned' | 'awaiting_approval' | 'running' | 'cancelling' | 'paused' |
    'needs_input' | 'blocked' | 'completed' | 'failed' | 'cancelled';
  current_step: number;
  plan: {
    id: string;
    project_id: string;
    study_id: string;
    steps: {
      id: string;
      input: { tool: 'read_study_transcript'; transcript_id: string };
      requires_approval: boolean;
      estimate: { tokens: number; cost_microusd: number };
    }[];
  };
  steps: {
    id: string;
    state: string;
    approved: boolean;
    output: { artifact_id: string; bytes: number; tokens: number; cost_microusd: number } | null;
  }[];
  usage: { tool_calls: number; reserved_cost_microusd: number };
  events: { sequence: number; kind: string }[];
}

/** A canonical attached Study source; ID is study_sources.id, not a platform ID. */
export interface StudyResearchSource {
  id: string;
  originId: string;
  kind: 'research_item' | 'transcript';
  locator: string;
  available: boolean;
  collectedAt: string;
}

/** Explicitly authorized offline AnalyzeEvidence run, separate from transcript inspection. */
export interface StudyResearchRun extends Omit<LocalInspectionRun, 'plan'> {
  plan: Omit<LocalInspectionRun['plan'], 'steps'> & {
    steps: Array<{
      id: string;
      input: { tool: 'analyze_evidence'; source_ids: string[] };
      requires_approval: boolean;
      estimate: { tokens: number; cost_microusd: number };
    }>;
  };
}

/** Provenance-bearing report. Observations are not independently verified facts. */
export interface StudyResearchReport {
  project_id: string;
  study_id: string;
  analyzed_at: unknown;
  sources: Array<{
    study_source_id: string;
    reference_id: string;
    kind: 'research_item' | 'transcript';
    locator: string;
    collected_at: string;
    access: 'available' | 'partial' | 'unavailable' | 'deleted' | 'revoked' | 'access_denied' | 'unsupported';
    origin_revision: string | null;
    metrics: Record<string, unknown> | null;
    observations: unknown[];
    limitations: string[];
  }>;
  feature_support: unknown[];
  limitations: string[];
  source_text_is_untrusted_data: boolean;
}
