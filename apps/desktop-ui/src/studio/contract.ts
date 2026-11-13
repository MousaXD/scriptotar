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
export interface ContentStudy {
  id: string;
  projectId: string;
  revision: number;
  brief: CreativeBrief;
  sources: StudySourceRef[];
  draft: string;
  draftVersions: DraftSnapshot[];
}
export interface StudioGateway {
  listStudies(projectId: string): Promise<Pick<ContentStudy, 'id' | 'projectId' | 'brief' | 'revision'>[]>;
  createStudy(projectId: string, brief: CreativeBrief): Promise<ContentStudy>;
  getStudy(projectId: string, studyId: string): Promise<ContentStudy>;
  saveStudy(projectId: string, studyId: string, expectedRevision: number, patch: {
    brief?: CreativeBrief;
    sources?: StudySourceRef[];
    draft?: string;
  }): Promise<ContentStudy>;
}
export const emptyBrief = (): CreativeBrief => ({
  title: '', goal: '', audience: '', platform: 'TikTok', language: 'Arabic',
  duration: '40', style: ''
});
