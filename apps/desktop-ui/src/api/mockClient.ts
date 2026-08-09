import type { AiPromptInput, ResearchQuery, ScriptotarApi } from './client';
import type { BackendJob, BootstrapData } from '../types';

export const mockBootstrap: BootstrapData = {
  activeProjectId: 'p-creator-lab',
  projects: [
    { id: 'p-creator-lab', name: 'Creator Lab', description: 'Short-form competitor research', updatedAt: '2026-08-09T08:24:00+04:00', itemCount: 42 },
    { id: 'p-client-a', name: 'Client A', description: 'Restaurant launch content', updatedAt: '2026-08-08T19:10:00+04:00', itemCount: 18 },
    { id: 'p-inbox', name: 'Inbox', description: 'Unsorted captures', updatedAt: '2026-08-08T16:30:00+04:00', itemCount: 7 }
  ],
  creators: [
    { id: 'c-1', name: 'Nora Edits', handle: '@noraedits', platform: 'TikTok', watchlisted: true, lastScannedAt: '14 min ago' },
    { id: 'c-2', name: 'Frame Foundry', handle: '@framefoundry', platform: 'YouTube', watchlisted: true, lastScannedAt: '2 h ago' },
    { id: 'c-3', name: 'Maya Makes', handle: '@mayamakes', platform: 'Instagram', watchlisted: false, lastScannedAt: 'Yesterday' }
  ],
  research: [
    { id: 'r-1', creatorId: 'c-1', creator: 'Nora Edits', title: 'Three cuts that make a hook feel faster', sourceUrl: 'https://example.com/r/1', platform: 'TikTok', views: 2400000, likes: 184000, comments: 3400, publishedAt: '2026-08-07', durationSeconds: 38 },
    { id: 'r-2', creatorId: 'c-2', creator: 'Frame Foundry', title: 'Why this 17-second reveal keeps retention', sourceUrl: 'https://example.com/r/2', platform: 'YouTube', views: 918000, likes: 62000, comments: 1200, publishedAt: '2026-08-08', durationSeconds: 17 },
    { id: 'r-3', creatorId: 'c-3', creator: 'Maya Makes', title: 'A/B testing the first sentence', sourceUrl: 'https://example.com/r/3', platform: 'Instagram', views: 412000, likes: 31000, comments: 890, publishedAt: '2026-08-06', durationSeconds: 46 },
    { id: 'r-4', creatorId: 'c-1', creator: 'Nora Edits', title: 'Caption pacing breakdown', sourceUrl: 'https://example.com/r/4', platform: 'TikTok', views: 175000, likes: 9400, comments: 510, publishedAt: '2026-08-05', durationSeconds: 29 }
  ],
  jobs: [
    { id: 'j-1', title: 'Three cuts that make a hook feel faster', source: 'TikTok', state: 'transcribing', stageLabel: 'Transcribing', progress: 62, updatedAt: 'now', detail: 'medium · auto device' },
    { id: 'j-2', title: 'Local interview take 04.mp4', source: 'Local file', state: 'downloading', stageLabel: 'Preparing media', updatedAt: '1 min ago', detail: 'Local media normalization' },
    { id: 'j-3', title: 'Why this 17-second reveal keeps retention', source: 'YouTube', state: 'queued', stageLabel: 'Queued', updatedAt: '2 min ago' },
    { id: 'j-4', title: 'A/B testing the first sentence', source: 'Instagram', state: 'failed', stageLabel: 'Failed', updatedAt: '12 min ago', detail: 'Extractor returned an authentication error' },
    { id: 'j-5', title: 'Caption pacing breakdown', source: 'TikTok', state: 'completed', stageLabel: 'Completed', progress: 100, updatedAt: '38 min ago' },
    { id: 'j-6', title: 'Interrupted session recovery', source: 'Local file', state: 'interrupted', stageLabel: 'Interrupted', updatedAt: 'Yesterday', detail: 'Application stopped during transcription' }
  ],
  transcripts: [
    {
      id: 't-ar', projectId: 'p-creator-lab', title: 'Hook breakdown — Arabic sample', language: 'Arabic', direction: 'rtl', source: 'local://arabic-hook.mp4', platform: 'Local file', durationSeconds: 31, createdAt: '2026-08-09T07:58:00+04:00',
      text: 'أول ثلاث ثواني هي المكان الذي يقرر فيه المشاهد إذا كان سيكمل. ابدأ بالنتيجة، ثم ارجع خطوة واحدة واشرح لماذا حدثت.',
      segments: [
        { id: 's1', startSeconds: 0, endSeconds: 7.2, text: 'أول ثلاث ثواني هي المكان الذي يقرر فيه المشاهد إذا كان سيكمل.' },
        { id: 's2', startSeconds: 7.2, endSeconds: 15.4, text: 'ابدأ بالنتيجة، ثم ارجع خطوة واحدة واشرح لماذا حدثت.' }
      ]
    },
    {
      id: 't-en', projectId: 'p-creator-lab', title: 'Caption pacing breakdown', language: 'English', direction: 'ltr', source: 'https://example.com/r/4', platform: 'TikTok', durationSeconds: 29, createdAt: '2026-08-09T07:22:00+04:00',
      text: 'The caption is doing two jobs: creating a second hook and giving the viewer a reason to stay until the visual payoff.',
      segments: [{ id: 's3', startSeconds: 0, endSeconds: 9.8, text: 'The caption is doing two jobs: creating a second hook and giving the viewer a reason to stay until the visual payoff.' }]
    }
  ],
  aiRuns: [
    { id: 'a-1', task: 'Viral breakdown', mode: 'copy', title: 'Caption pacing breakdown', createdAt: '18 min ago', status: 'completed' },
    { id: 'a-2', task: 'Hook ideas', mode: 'byok', provider: 'OpenAI', model: 'gpt-5.2', title: 'Restaurant launch hook set', createdAt: 'Yesterday', status: 'completed' }
  ],
  library: [
    { id: 'l-1', kind: 'Transcript', title: 'Hook breakdown — Arabic sample', subtitle: 'Arabic · 0:31', projectId: 'p-creator-lab', platform: 'Local', metric: 'Arabic', date: 'Today 07:58' },
    { id: 'l-2', kind: 'Research', title: 'Three cuts that make a hook feel faster', subtitle: 'Nora Edits', projectId: 'p-creator-lab', platform: 'TikTok', metric: '2.4M views', date: 'Aug 7' },
    { id: 'l-3', kind: 'AI run', title: 'Caption pacing breakdown', subtitle: 'Viral breakdown · Copy Prompt', projectId: 'p-creator-lab', platform: 'Local', metric: 'Viral breakdown', date: '18 min ago' },
    { id: 'l-4', kind: 'Creator', title: 'Frame Foundry', subtitle: '@framefoundry', projectId: 'p-creator-lab', platform: 'YouTube', metric: 'Watchlisted', date: '2 h ago' }
  ],
  settings: {
    whisperModel: 'medium', device: 'auto', language: 'auto', quality: '720p', cookies: 'none', maxDuration: '60 min',
    copyLocalSource: false, translate: false, batched: true, keepFailed: false, autoWatch: false, watchInterval: '60 min', appearance: 'dark'
  }
};

function backendJob(projectId: string, kind: 'url' | 'local_file', value: string): BackendJob {
  return {
    id: `mock-${kind}`,
    project_id: projectId,
    input: { kind, value },
    state: 'queued',
    progress: null,
    last_error: null,
    created_at: '2026-08-09T09:30:00+04:00',
    updated_at: '2026-08-09T09:30:00+04:00'
  };
}

export function createMockClient(overrides?: Partial<ScriptotarApi>): ScriptotarApi {
  let active = mockBootstrap.activeProjectId;
  const snapshot = (): BootstrapData => ({ ...structuredClone(mockBootstrap), activeProjectId: active });
  const client: ScriptotarApi = {
    async bootstrap() { return snapshot(); },
    async selectProject(projectId: string) { active = projectId; return snapshot(); },
    async createProject(_name: string) { return snapshot(); },
    async enqueueLocalMedia(projectId: string, path: string) { return backendJob(projectId, 'local_file', path); },
    async enqueueUrl(projectId: string, url: string) { return backendJob(projectId, 'url', url); },
    async retryJob(jobId: string) { return { ...backendJob(active, 'local_file', '/mock/retry.mp4'), id: jobId }; },
    async saveWatchlist(_query: ResearchQuery) { return snapshot(); },
    async scanCreator(_query: ResearchQuery) {},
    async queueResearch(_ids: string[]) {},
    async cancelJob(_jobId: string) {},
    async buildAiPrompt(input: AiPromptInput) {
      return [`Task: ${input.task}`, `Topic: ${input.topic || 'Use source context'}`, `Audience: ${input.audience || 'General audience'}`, '', input.sourceText].join('\n');
    },
    async runAi(input: AiPromptInput) { return `Mock ${input.provider} result for ${input.task}.`; },
    async getSettings() { return structuredClone(mockBootstrap.settings); },
    async saveSettings(_settings) {},
    async importLegacyData() {
      return {
        skipped: false,
        backup_path: '/mock/history.sqlite3.scriptotar-next.bak',
        projects: 1,
        jobs: 2,
        transcripts: 1,
        research_items: 3,
        watchlists: 1,
        ai_runs: 1
      };
    }
  };
  return { ...client, ...overrides };
}
