import type { AiPromptInput, ResearchQuery, ScriptotarApi } from './client';
import type { ContentStudy } from '../studio/contract';
import type { LocalInspectionRun } from '../studio/agent';
import type { BackendJob, BootstrapData, Project, McpPermissions, McpGatewayStatus, McpConnectionStatus } from '../types';

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
  watchlistStatuses: [
    {
      watchlistId: 'w-1', projectId: 'p-creator-lab', label: 'Nora Edits', state: 'healthy',
      lastAttemptAt: '2026-08-09T08:10:00+04:00', lastSuccessfulScanAt: '2026-08-09T08:10:00+04:00'
    },
    {
      watchlistId: 'w-2', projectId: 'p-creator-lab', label: 'Frame Foundry', state: 'retry_scheduled',
      lastAttemptAt: '2026-08-09T08:15:00+04:00', lastSuccessfulScanAt: '2026-08-09T06:10:00+04:00',
      lastError: 'Creator refresh needs valid browser authentication or provider access.',
      nextRetryAt: '2026-08-09T09:15:00+04:00'
    }
  ],
  migrationStatus: {
    state: 'no_legacy_db',
    message: 'No Scriptotar Classic database was found in the standard legacy locations.',
    candidates: []
  },
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
    { id: 'j-5', title: 'Caption pacing breakdown', source: 'https://example.com/r/4', state: 'completed', stageLabel: 'Completed', progress: 100, updatedAt: '38 min ago', completedTranscriptId: 't-en' },
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
    { id: 'transcript:t-ar', kind: 'Transcript', title: 'Hook breakdown — Arabic sample', subtitle: 'Arabic · 0:31', projectId: 'p-creator-lab', platform: 'Local', metric: 'Arabic', date: 'Today 07:58' },
    { id: 'research:r-1', kind: 'Research', title: 'Three cuts that make a hook feel faster', subtitle: 'Nora Edits', projectId: 'p-creator-lab', platform: 'TikTok', metric: '2.4M views', date: 'Aug 7' },
    { id: 'ai:a-1', kind: 'AI run', title: 'Caption pacing breakdown', subtitle: 'Viral breakdown · Copy Prompt', projectId: 'p-creator-lab', platform: 'Local', metric: 'Viral breakdown', date: '18 min ago' },
    { id: 'creator:c-2', kind: 'Creator', title: 'Frame Foundry', subtitle: '@framefoundry', projectId: 'p-creator-lab', platform: 'YouTube', metric: 'Watchlisted', date: '2 h ago' }
  ],
  settings: {
    outputDirectory: null,
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
  let settings = structuredClone(mockBootstrap.settings);
  let connection: McpConnectionStatus = {
    mode: 'none', state: 'disconnected', detail: 'Choose a connection method.',
    publicUrl: null, localUrl: null, tunnelId: null,
    cloudflareAvailable: true, openaiAvailable: true
  };
  let mcpGateway: McpGatewayStatus = { running: false, endpoint: 'http://127.0.0.1:39761/mcp' };
  let mcp: McpPermissions = {
    enabled: false, httpToken: '', urlToken: '',  readData: false, writeData: false, readFiles: false, writeFiles: false,
    overwriteFiles: false, executeCommands: false, searchCreators: false, downloadVideos: false,
    allowedPaths: [], allowedPrograms: []
  };
  let migrationStatus = structuredClone(mockBootstrap.migrationStatus);
  let projects = structuredClone(mockBootstrap.projects);
  let projectCounter = 0;
  let studyCounter = 0;
  const studies = new Map<string, ContentStudy>();
  const inspectionRuns = new Map<string, LocalInspectionRun>();
  let inspectionCounter = 0;
  const studyInProject = (projectId: string, studyId: string): ContentStudy => {
    if (active !== projectId) throw new Error('Active project changed');
    const study = studies.get(studyId);
    if (!study || study.projectId !== projectId) throw new Error('Study not found');
    return study;
  };
  const snapshot = (): BootstrapData => ({
    ...structuredClone(mockBootstrap),
    projects: structuredClone(projects),
    activeProjectId: active,
    settings: structuredClone(settings),
    migrationStatus: structuredClone(migrationStatus)
  });
  const client: ScriptotarApi = {
    async backendHealth() { return {schema_version: 1}; },
    async bootstrap() { return snapshot(); },
    async bootstrapSummary() {
      // Delegate through the public method so test overrides exercise the same startup contract.
      const full = await this.bootstrap();
      return {
        ...full,
        transcripts: full.transcripts.map(({
          id, projectId, title, language, direction, source, platform, durationSeconds, createdAt
        }) => ({ id, projectId, title, language, direction, source, platform, durationSeconds, createdAt }))
      };
    },
    async listJobs() { return structuredClone(snapshot().jobs); },
    async searchTranscripts(rawQuery: string, limit = 10) {
      const query = rawQuery.trim().toLocaleLowerCase();
      if (!query) return [];
      return snapshot().transcripts
        .filter((transcript) => transcript.text.toLocaleLowerCase().includes(query))
        .slice(0, limit)
        .map((transcript) => transcript.id);
    },
    async listStudies(projectId: string) {
      if (active !== projectId) throw new Error('Active project changed');
      return [...studies.values()].filter((study) => study.projectId === projectId).map((study) => ({
        id: study.id, projectId: study.projectId, brief: structuredClone(study.brief), revision: study.revision
      }));
    },
    async createStudy(projectId, brief) {
      if (active !== projectId) throw new Error('Active project changed');
      if (!brief.title.trim()) throw new Error('Study title is required');
      const study: ContentStudy = {
        id: `mock-study-${++studyCounter}`, projectId, revision: 1,
        brief: structuredClone(brief), sources: [], draft: '', draftVersions: []
      };
      studies.set(study.id, structuredClone(study));
      return structuredClone(study);
    },
    async getStudy(projectId: string, studyId: string) {
      return structuredClone(studyInProject(projectId, studyId));
    },
    async saveStudy(projectId, studyId, expectedRevision, patch) {
      const old = studyInProject(projectId, studyId);
      if (old.revision !== expectedRevision) throw new Error('Study changed by another editor');
      const next = structuredClone(old);
      if (patch.brief) {
        if (!patch.brief.title.trim()) throw new Error('Study title is required');
        next.brief = structuredClone(patch.brief);
      }
      if (patch.sources) {
        const identities = patch.sources.map((ref) => `${ref.kind}:${ref.id}`);
        if (new Set(identities).size !== identities.length) throw new Error('Duplicate source');
        next.sources = structuredClone(patch.sources);
      }
      if (patch.draft !== undefined && patch.draft !== old.draft) {
        next.draft = patch.draft;
        next.draftVersions.push({
          id: `${studyId}:v${next.draftVersions.length + 1}`,
          revision: next.draftVersions.length + 1,
          content: patch.draft,
          createdAt: '2026-08-09T09:00:00+04:00'
        });
      }
      next.revision += 1;
      studies.set(studyId, next);
      return structuredClone(next);
    },
    async startLocalInspection(projectId: string, studyId: string, transcriptId: string) {
      const study = studyInProject(projectId, studyId);
      if (!study.sources.some((item) => item.kind === 'transcript' && item.id === transcriptId)) {
        throw new Error('Transcript must be attached to the study');
      }
      const id = 'mock-inspect-' + ++inspectionCounter;
      const created: LocalInspectionRun = {
        schema_version: 1, revision: 1, state: 'awaiting_approval', current_step: 0,
        plan: {
          id, project_id: projectId, study_id: studyId,
          steps: [{ id: 'inspect_transcript', input: { tool: 'read_study_transcript', transcript_id: transcriptId },
            requires_approval: true, estimate: { tokens: 0, cost_microusd: 0 } }]
        },
        steps: [{ id: 'inspect_transcript', state: 'pending', approved: false, output: null }],
        events: [
          { sequence: 1, kind: 'created' },
          { sequence: 2, kind: 'awaiting_approval' }
        ],
        usage: { tool_calls: 0, reserved_cost_microusd: 0 }
      };
      inspectionRuns.set(id, created);
      return structuredClone(created);
    },
    async listLocalInspections(projectId: string, studyId: string) {
      studyInProject(projectId, studyId);
      return [...inspectionRuns.values()]
        .filter((run) => run.plan.project_id === projectId && run.plan.study_id === studyId)
        .map((run) => structuredClone(run));
    },
    async decideLocalInspection(projectId: string, studyId: string, runId: string, action) {
      studyInProject(projectId, studyId);
      const stored = inspectionRuns.get(runId);
      if (!stored || stored.plan.project_id !== projectId || stored.plan.study_id !== studyId) {
        throw new Error('Agent run not found');
      }
      if (action === 'recover') return structuredClone(stored);
      if (action === 'cancel' && !['completed', 'cancelled'].includes(stored.state)) {
        stored.state = 'cancelled';
      } else if (stored.state !== 'awaiting_approval') {
        throw new Error('Agent run requires a different action');
      } else if (action === 'deny') {
        stored.state = 'cancelled';
        stored.events.push({ sequence: stored.events.length + 1, kind: 'denied' });
      } else if (action === 'approve') {
        const sourceId = stored.plan.steps[0].input.transcript_id;
        if (!studyInProject(projectId, studyId).sources.some((ref) => ref.id === sourceId && ref.kind === 'transcript')) {
          throw new Error('Source no longer attached to the study');
        }
        stored.state = 'completed';
        stored.steps[0].approved = true;
        stored.steps[0].state = 'completed';
        stored.steps[0].output = { artifact_id: sourceId, bytes: 0, tokens: 0, cost_microusd: 0 };
        stored.usage.tool_calls = 1;
        stored.events.push({ sequence: stored.events.length + 1, kind: 'approved' });
      } else throw new Error('Unsupported agent action');
      stored.revision += 1;
      inspectionRuns.set(runId, stored);
      return structuredClone(stored);
    },
    async getAiRun(projectId: string, aiRunId: string) {
      if (active !== projectId) throw new Error('Active project changed');
      const run = snapshot().aiRuns.find((candidate) => candidate.id === aiRunId);
      if (!run) throw new Error('AI artifact not found');
      return { ...run, projectId, prompt: `Saved prompt for ${run.task}`,
        result: run.mode === 'copy' ? null : `Saved output for ${run.task}` };
    },
    async getResearchItem(projectId: string, researchItemId: string) {
      if (active !== projectId) throw new Error('Active project changed');
      const item = snapshot().research.find((candidate) => candidate.id === researchItemId);
      if (!item) throw new Error('Research item not found');
      return { ...item, projectId, scannedAt: '2026-08-09T09:00:00+04:00' };
    },
    async saveCopyPrompt(projectId: string, input: AiPromptInput) {
      if (active !== projectId) throw new Error('Active project changed');
      if (input.mode !== 'copy' || input.apiKey?.trim()) throw new Error('Copy mode without an API key is required');
      return { id: 'a-saved-copy', projectId, task: input.task, title: input.topic || input.task,
        mode: 'copy' as const, createdAt: '2026-08-09T09:00:00+04:00',
        status: 'completed' as const, prompt: await this.buildAiPrompt(input), result: null };
    },
    async getTranscript(id: string) {
      const transcript = snapshot().transcripts.find((candidate) => candidate.id === id);
      if (!transcript) throw new Error('Transcript not found');
      return structuredClone(transcript);
    },
    async subscribeJobChanges(_listener: (jobId: string) => void) { return () => {}; },
    async getWatchlistStatuses() { return structuredClone(snapshot().watchlistStatuses); },
    async getMigrationStatus() { return structuredClone(migrationStatus); },
    async retryLegacyMigration() { return structuredClone(migrationStatus); },
    async selectLegacyMigrationCandidate(_candidateId: string) { return structuredClone(migrationStatus); },
    async selectProject(projectId: string) {
      if (!projects.some((project) => project.id === projectId)) throw new Error('Project not found');
      active = projectId;
      return snapshot();
    },
    async createProject(rawName: string) {
      const name = rawName.trim();
      if (!name) throw new Error('Project name cannot be empty');
      if (name.length > 120) throw new Error('Project name must be 120 characters or fewer');
      if (projects.some((project) => project.name.toLocaleLowerCase() === name.toLocaleLowerCase())) {
        throw new Error('A project with that name already exists');
      }
      projectCounter += 1;
      const project: Project = {
        id: `p-created-${projectCounter}`,
        name,
        updatedAt: '2026-08-09T09:30:00+04:00',
        itemCount: 0
      };
      projects = [...projects, project];
      active = project.id;
      return snapshot();
    },
    async chooseLocalMedia() { return '/mock/selected-video.mp4'; },
    async chooseOutputDirectory() { return '/mock/scriptotar-output'; },
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
    async getSettings() { return structuredClone(settings); },
    async saveSettings(next) { settings = structuredClone(next); },
    async getMcpPermissions() { return structuredClone(mcp); },
    async mcpConnectionStatus() { return structuredClone(connection); },
    async mcpConnectCloudflare() {
      if (!mcp.enabled) throw new Error('Enable MCP permissions in Scriptotar first.');
      connection = { ...connection, mode: 'cloudflare', state: 'ready',
        detail: 'Cloudflare URL ready.',
        publicUrl: `https://sample-cloudflare.trycloudflare.com/mcp/p/${mcp.urlToken}`,
        localUrl: `http://127.0.0.1:39761/mcp/p/${mcp.urlToken}`, tunnelId: null };
      return structuredClone(connection);
    },
    async mcpConnectOpenai(tunnelId: string, _apiKey: string, _rememberKey: boolean) {
      if (!mcp.enabled) throw new Error('Enable MCP permissions in Scriptotar first.');
      if (!/^tunnel_[0-9a-f]{32}$/.test(tunnelId)) throw new Error('Invalid tunnel ID');
      connection = { ...connection, mode: 'openai', state: 'ready',
        detail: 'OpenAI tunnel ready.', publicUrl: null, localUrl: null, tunnelId };
      return structuredClone(connection);
    },
    async mcpConnectManual(hostname: string) {
      if (!mcp.enabled) throw new Error('Enable MCP permissions in Scriptotar first.');
      const origin = new URL(hostname);
      if (origin.protocol !== 'https:' || origin.pathname !== '/') throw new Error('HTTPS origin required');
      connection = { ...connection, mode: 'manual', state: 'ready',
        detail: 'Local gateway ready.', publicUrl: `${origin.origin}/mcp/p/${mcp.urlToken}`,
        localUrl: `http://127.0.0.1:39761/mcp/p/${mcp.urlToken}`, tunnelId: null };
      return structuredClone(connection);
    },
    async mcpDisconnect() {
      connection = { ...connection, mode: 'none', state: 'disconnected', publicUrl: null,
        localUrl: null, tunnelId: null, detail: 'Disconnected' };
      mcp.urlToken = 'f'.repeat(64);
      return structuredClone(connection);
    },
    async mcpGatewayStatus() { return structuredClone(mcpGateway); },
    async startMcpGateway() {
      if (!mcp.enabled) throw new Error('Enable MCP first');
      mcpGateway = { ...mcpGateway, running: true };
      return structuredClone(mcpGateway);
    },
    async stopMcpGateway() {
      mcpGateway = { ...mcpGateway, running: false };
      return structuredClone(mcpGateway);
    },
    async saveMcpPermissions(next, acknowledgeSensitiveAccess) {
      if (next.enabled && (next.writeFiles || next.overwriteFiles || next.executeCommands) && !acknowledgeSensitiveAccess) {
        throw new Error('Powerful MCP permissions require acknowledgement in Scriptotar Settings.');
      }
      mcp = structuredClone({ ...next, httpToken: next.httpToken || 'a'.repeat(64), urlToken: next.urlToken || 'b'.repeat(64) });
      if (!mcp.enabled) mcpGateway.running = false;
      return structuredClone(mcp);
    },
  };
  return { ...client, ...overrides };
}
