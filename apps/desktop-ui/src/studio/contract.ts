/**
 * Agent 03 frontend port, NOT a Tauri command binding.
 * Agent 00 owns the final DTO and ScriptotarApi contract. The optional adapter
 * allows component tests to exercise durable workflows while production keeps
 * unsupported mutations unavailable until the backend is integrated.
 */
export type StudySourceKind = 'research' | 'transcript';
export interface StudySourceRef {
  kind: StudySourceKind;
  id: string;
}
export interface CreativeBrief {
  title: string;
  goal: string;
  audience: string;
  platform: string;
  language: string;
  duration: string;
  style: string;
}
export interface DraftSnapshot {
  id: string;
  revision: number;
  content: string;
  createdAt: string;
}
/** Local request preview, not an AI result or provider authorization. */
export interface StudyAnglePrompt {
  prompt: string;
  attachedSourceCount: number;
  transcriptExcerptCount: number;
  unverifiedObservationCount: number;
  limitations: string[];
}
/** Returned text is model-proposed, not independently verified. */
export interface StudyAngleOption {
  id: string;
  approach: string;
  hook: string;
  promise: string;
  outline: string[];
  estimatedSeconds: number;
  evidenceIds: string[];
}
export interface StudyAngleInterpretation {
  angles: StudyAngleOption[];
  draftPrompt: string | null;
  limitations: string[];
}
/** User-pasted model output remains unverified until reviewed by a person. */
export interface StudyScriptReview {
  title: string;
  body: string;
  angleId: string;
  referencedSourceCount: number;
  claimCount: number;
  needsFactReviewCount: number;
  limitations: string[];
  savedStudy: ContentStudy | null;
}
/** The three immutable canonical draft IDs from a verified v11 receipt.
 * Read-only history. This is neither consent nor proof that paid egress is enabled.
 */
export interface StudyWritingBatch {
  runId: string;
  projectId: string;
  studyId: string;
  draftIds: [string, string, string];
  committedAt: string;
}

export interface DraftCandidate {
  id: string;
  title: string;
  revision: number;
}
export interface ContentStudy {
  id: string;
  projectId: string;
  revision: number;
  brief: CreativeBrief;
  sources: StudySourceRef[];
  draft: string;
  draftVersions: DraftSnapshot[];
  /** Exact saved candidate; older injected Studio fixtures may omit metadata. */
  draftId?: string | null;
  draftRevision?: number | null;
  drafts?: DraftCandidate[];
  archivedDrafts?: DraftCandidate[];
}
export interface StudyEvidenceInput {
  source: StudySourceRef;
  claim: string;
  locator: string;
  timestampSeconds?: number | null;
  uncertaintyNote?: string | null;
}
export interface StudyEvidenceReviewInput {
  expectedCertainty: 'unverified' | 'disputed';
  action: 'dispute' | 'reopen';
  note: string;
}
export interface StudyEvidenceReview {
  id: string;
  previousCertainty: 'unverified' | 'disputed';
  nextCertainty: 'unverified' | 'disputed';
  note: string;
  reviewedAt: string;
}
export interface StudyEvidence {
  id: string;
  projectId: string;
  studyId: string;
  source: StudySourceRef | null;
  sourceLabel: string;
  sourceAvailable: boolean;
  claim: string;
  locator: string;
  timestampSeconds: number | null;
  certainty: 'unverified' | 'estimated' | 'corroborated' | 'disputed';
  uncertaintyNote: string | null;
  createdAt: string;
  /** Last 50 human reviews since SQLite v9; no independent fact checking. */
  reviewHistory?: StudyEvidenceReview[];
}
export interface HistoricalAiArtifact {
  id: string;
  projectId: string;
  task: string;
  title: string;
  prompt: string;
  result: string | null;
  createdAt: string;
}
import type { LocalInspectionRun } from './agent';

export interface StudioGateway {
  /** Optional until Agent 02's project-scoped exact-run detail endpoint is integrated. */
  getAiArtifact?(projectId: string, runId: string): Promise<HistoricalAiArtifact>;
  listLocalInspections?(projectId: string, studyId: string): Promise<LocalInspectionRun[]>;
  startLocalInspection?(projectId: string, studyId: string, transcriptId: string): Promise<LocalInspectionRun>;
  decideLocalInspection?(projectId: string, studyId: string, runId: string, action: 'approve' | 'deny' | 'cancel' | 'recover'): Promise<LocalInspectionRun>;
  listStudyEvidence?(projectId: string, studyId: string): Promise<StudyEvidence[]>;
  addStudyEvidence?(projectId: string, studyId: string, input: StudyEvidenceInput): Promise<StudyEvidence>;
  reviewStudyEvidence?(projectId: string, studyId: string, evidenceId: string, input: StudyEvidenceReviewInput): Promise<StudyEvidence>;
  listStudies(projectId: string): Promise<Pick<ContentStudy, 'id' | 'projectId' | 'brief' | 'revision'>[]>;
  createStudy(projectId: string, brief: CreativeBrief): Promise<ContentStudy>;
  getStudy(projectId: string, studyId: string, draftId?: string): Promise<ContentStudy>;
  /** Read-only history of committed three-script v11 batches, no AI/network. */
  listStudyWritingBatches?(projectId: string, studyId: string): Promise<StudyWritingBatch[]>;
  duplicateStudyDraft?(projectId: string, studyId: string, draftId: string, expectedDraftRevision: number): Promise<ContentStudy>;
  renameStudyDraft?(projectId: string, studyId: string, draftId: string, expectedDraftRevision: number, title: string): Promise<ContentStudy>;
  setStudyDraftArchived?(projectId: string, studyId: string, draftId: string, expectedDraftRevision: number, archived: boolean): Promise<ContentStudy>;
  prepareStudyAngles?(projectId: string, studyId: string): Promise<StudyAnglePrompt>;
  interpretStudyAngles?(projectId: string, studyId: string, expectedStudyRevision: number,
    preparedPrompt: string, responseJson: string, selectedAngleId?: string,
    requestId?: string): Promise<StudyAngleInterpretation>;
  reviewStudyScript?(projectId: string, studyId: string, expectedStudyRevision: number,
    anglePrompt: string, anglesResponseJson: string, selectedAngleId: string,
    preparedDraftPrompt: string, scriptResponseJson: string,
    acceptAsNewCandidate: boolean): Promise<StudyScriptReview>;
  /** Standard user action, preserving canonical source provenance. */
  revokeStudySourceAccess?(projectId: string, studyId: string, expectedRevision: number,
    source: StudySourceRef): Promise<ContentStudy>;
  saveStudy(projectId: string, studyId: string, expectedRevision: number, patch: {
    brief?: CreativeBrief;
    sources?: StudySourceRef[];
    draft?: string;
    draftId?: string;
    draftRevision?: number;
  }): Promise<ContentStudy>;
}
export const emptyBrief = (): CreativeBrief => ({
  title: '', goal: '', audience: '', platform: 'TikTok', language: 'Arabic',
  duration: '40', style: ''
});
