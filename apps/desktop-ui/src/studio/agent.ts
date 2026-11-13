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
