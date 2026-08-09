export type ViewId = 'dashboard' | 'research' | 'jobs' | 'transcript' | 'ai' | 'library' | 'settings';
export type JobState = 'queued' | 'preparing' | 'downloading' | 'transcribing' | 'processing' | 'completed' | 'failed' | 'cancelled' | 'interrupted';
export type LibraryKind = 'Transcript' | 'Research' | 'AI run' | 'Project' | 'Creator';
export type AiMode = 'copy' | 'byok';
export type AiProvider = 'OpenAI' | 'Anthropic' | 'Gemini' | 'OpenAI-compatible' | 'Local (coming later)';

export interface Project {
  id: string;
  name: string;
  description?: string;
  updatedAt: string;
  itemCount: number;
}

export interface Creator {
  id: string;
  name: string;
  handle: string;
  platform: string;
  avatar?: string;
  watchlisted: boolean;
  lastScannedAt?: string;
}

export interface ResearchItem {
  id: string;
  creatorId: string;
  creator: string;
  title: string;
  sourceUrl: string;
  platform: string;
  views?: number;
  likes?: number;
  comments?: number;
  publishedAt?: string;
  durationSeconds?: number;
  thumbnail?: string;
  queued?: boolean;
}

export interface Job {
  id: string;
  title: string;
  source: string;
  state: JobState;
  stageLabel: string;
  progress?: number;
  updatedAt: string;
  detail?: string;
}

export interface TranscriptSegment {
  id: string;
  startSeconds: number;
  endSeconds: number;
  text: string;
}

export interface Transcript {
  id: string;
  projectId: string;
  title: string;
  language: string;
  direction: 'ltr' | 'rtl';
  source: string;
  platform: string;
  durationSeconds: number;
  createdAt: string;
  text: string;
  segments: TranscriptSegment[];
}

export interface AiRun {
  id: string;
  task: string;
  mode: AiMode;
  provider?: string;
  model?: string;
  title: string;
  createdAt: string;
  status: 'completed' | 'failed';
}

export interface LibraryItem {
  id: string;
  kind: LibraryKind;
  title: string;
  subtitle: string;
  projectId: string;
  platform?: string;
  metric?: string;
  date: string;
}

export interface AppSettings {
  whisperModel: 'small' | 'medium' | 'turbo' | 'large-v3';
  device: 'auto' | 'cpu' | 'cuda';
  language: 'auto' | 'Arabic' | 'English';
  quality: '720p' | '1080p' | 'Best' | 'Audio only';
  cookies: 'none' | 'firefox' | 'chrome' | 'chromium' | 'brave';
  maxDuration: '30 min' | '60 min' | '2 hours' | 'Unlimited';
  copyLocalSource: boolean;
  translate: boolean;
  batched: boolean;
  keepFailed: boolean;
  autoWatch: boolean;
  watchInterval: '30 min' | '60 min' | '2 hours' | '6 hours';
  appearance: 'dark' | 'system';
}

export interface BootstrapData {
  projects: Project[];
  activeProjectId: string;
  creators: Creator[];
  research: ResearchItem[];
  jobs: Job[];
  transcripts: Transcript[];
  aiRuns: AiRun[];
  library: LibraryItem[];
  settings: AppSettings;
}
