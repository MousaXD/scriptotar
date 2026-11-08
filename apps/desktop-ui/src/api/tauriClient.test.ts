import { describe, expect, it, vi } from 'vitest';
import { createTauriClient, type TauriInvoke } from './tauriClient';

const aiInput = {
  mode: 'copy' as const,
  provider: 'OpenAI' as const,
  model: 'gpt-5.2',
  task: 'Hook ideas',
  sourceText: 'source',
  topic: 'topic',
  audience: 'audience',
  duration: '30 seconds',
  cta: 'follow',
  voice: 'direct'
};

describe('Tauri client command contract', () => {
  it('uses the Rust command names and camelCase argument names exactly', async () => {
    const invokeMock = vi.fn(async () => undefined);
    const api = createTauriClient(invokeMock as unknown as TauriInvoke);

    await api.backendHealth();
    await api.bootstrap();
    await api.getWatchlistStatuses();
    await api.getMigrationStatus();
    await api.retryLegacyMigration();
    await api.selectLegacyMigrationCandidate('candidate-1');
    await api.selectProject('project-id');
    await api.createProject('Client A');
    await api.chooseLocalMedia();
    await api.chooseOutputDirectory();
    await api.enqueueLocalMedia('project-id', '/tmp/a.mp4');
    await api.enqueueUrl('project-id', 'https://www.youtube.com/watch?v=fixture');
    await api.retryJob('job-id');
    await api.saveWatchlist({ profileUrl: 'https://www.youtube.com/@fixture', limit: 25 });
    await api.scanCreator({ profileUrl: 'https://www.youtube.com/@fixture', limit: 25 });
    await api.queueResearch(['research-id']);
    await api.cancelJob('job-id');
    await api.buildAiPrompt(aiInput);
    await api.runAi(aiInput);
    await api.getSettings();
    await api.saveSettings({
      outputDirectory: null,
      whisperModel: 'medium',
      device: 'auto',
      language: 'auto',
      quality: '720p',
      cookies: 'none',
      maxDuration: '60 min',
      copyLocalSource: false,
      translate: false,
      batched: true,
      keepFailed: false,
      autoWatch: false,
      watchInterval: '60 min',
      appearance: 'dark'
    });
    await api.mcpConnectionStatus();
    await api.mcpConnectCloudflare();
    await api.mcpConnectOpenai('tunnel_0123456789abcdef0123456789abcdef', 'sk-test', false);
    await api.mcpConnectManual('https://mcp.example.com');
    await api.mcpDisconnect();
    await api.searchTranscripts('scriptotar', 6);
    await api.getTranscript('transcript-id');
    await api.getMcpPermissions();
    await api.saveMcpPermissions({
      enabled: false, httpToken: '', urlToken: '', readData: false, writeData: false,
      readFiles: false, writeFiles: false, overwriteFiles: false,
      executeCommands: false, searchCreators: false, downloadVideos: false,
      allowedPaths: [], allowedPrograms: []
    }, false);
    await api.mcpGatewayStatus();
    await api.startMcpGateway();
    await api.stopMcpGateway();
    const unsubscribe = await api.subscribeJobChanges(() => {});
    unsubscribe();
    await api.listJobs();

    expect(invokeMock).toHaveBeenCalledWith('backend_health');
    expect(invokeMock).toHaveBeenCalledWith('bootstrap_app');
    expect(invokeMock).toHaveBeenCalledWith('get_watchlist_statuses');
    expect(invokeMock).toHaveBeenCalledWith('get_migration_status');
    expect(invokeMock).toHaveBeenCalledWith('retry_legacy_migration');
    expect(invokeMock).toHaveBeenCalledWith('select_legacy_migration_candidate', {
      candidateId: 'candidate-1'
    });
    expect(invokeMock).toHaveBeenCalledWith('select_project', { projectId: 'project-id' });
    expect(invokeMock).toHaveBeenCalledWith('create_project', { name: 'Client A' });
    expect(invokeMock).toHaveBeenCalledWith('choose_local_media');
    expect(invokeMock).toHaveBeenCalledWith('choose_output_directory');
    expect(invokeMock).toHaveBeenCalledWith('enqueue_local_media', {
      projectId: 'project-id',
      path: '/tmp/a.mp4'
    });
    expect(invokeMock).toHaveBeenCalledWith('enqueue_url', {
      projectId: 'project-id',
      url: 'https://www.youtube.com/watch?v=fixture'
    });
    expect(invokeMock).toHaveBeenCalledWith('retry_job', { jobId: 'job-id' });
    expect(invokeMock).toHaveBeenCalledWith('save_watchlist', {
      query: { profileUrl: 'https://www.youtube.com/@fixture', limit: 25 }
    });
    expect(invokeMock).toHaveBeenCalledWith('scan_creator', {
      query: { profileUrl: 'https://www.youtube.com/@fixture', limit: 25 }
    });
    expect(invokeMock).toHaveBeenCalledWith('queue_research', { ids: ['research-id'] });
    expect(invokeMock).toHaveBeenCalledWith('cancel_job', { jobId: 'job-id' });
    expect(invokeMock).toHaveBeenCalledWith('build_ai_prompt', { input: aiInput });
    expect(invokeMock).toHaveBeenCalledWith('run_ai', { input: aiInput });
    expect(invokeMock).toHaveBeenCalledWith('get_settings');
    expect(invokeMock).toHaveBeenCalledWith('save_settings', {
      settings: expect.objectContaining({ whisperModel: 'medium', outputDirectory: null })
    });
    expect(invokeMock).toHaveBeenCalledWith('mcp_connection_status');
    expect(invokeMock).toHaveBeenCalledWith('mcp_connect_cloudflare');
    expect(invokeMock).toHaveBeenCalledWith('mcp_connect_openai', { tunnelId: 'tunnel_0123456789abcdef0123456789abcdef', apiKey: 'sk-test', rememberKey: false });
    expect(invokeMock).toHaveBeenCalledWith('mcp_connect_manual', { hostname: 'https://mcp.example.com' });
    expect(invokeMock).toHaveBeenCalledWith('mcp_disconnect');
    expect(invokeMock).toHaveBeenCalledWith('search_transcripts', { query: 'scriptotar', limit: 6 });
    expect(invokeMock).toHaveBeenCalledWith('get_transcript', { transcriptId: 'transcript-id' });
    expect(invokeMock).toHaveBeenCalledWith('get_mcp_permissions');
    expect(invokeMock).toHaveBeenCalledWith('save_mcp_permissions', {
      permissions: expect.objectContaining({ readData: false, enabled: false }),
      acknowledgeSensitiveAccess: false
    });
    expect(invokeMock).toHaveBeenCalledWith('mcp_gateway_status');
    expect(invokeMock).toHaveBeenCalledWith('start_mcp_gateway');
    expect(invokeMock).toHaveBeenCalledWith('stop_mcp_gateway');
    expect(invokeMock).toHaveBeenCalledWith('list_jobs');
    expect(invokeMock).not.toHaveBeenCalledWith('import_legacy_data');
  });
});
