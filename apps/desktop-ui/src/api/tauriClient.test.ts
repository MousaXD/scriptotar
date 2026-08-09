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

    await api.bootstrap();
    await api.selectProject('project-id');
    await api.createProject('Client A');
    await api.chooseLocalMedia();
    await api.chooseOutputDirectory();
    await api.enqueueLocalMedia('project-id', '/tmp/a.mp4');
    await api.enqueueUrl('project-id', 'https://www.youtube.com/watch?v=fixture');
    await api.retryJob('job-id');
    await api.saveWatchlist({ profileUrl: 'https://www.youtube.com/@fixture', limit: 25 });
    await api.scanCreator({ profileUrl: 'https://www.youtube.com/@fixture', limit: 25 });
    await api.refreshWatchlists();
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
    await api.importLegacyData();
    await api.listJobs();

    expect(invokeMock).toHaveBeenNthCalledWith(1, 'bootstrap_app');
    expect(invokeMock).toHaveBeenNthCalledWith(2, 'select_project', { projectId: 'project-id' });
    expect(invokeMock).toHaveBeenNthCalledWith(3, 'create_project', { name: 'Client A' });
    expect(invokeMock).toHaveBeenNthCalledWith(4, 'choose_local_media');
    expect(invokeMock).toHaveBeenNthCalledWith(5, 'choose_output_directory');
    expect(invokeMock).toHaveBeenNthCalledWith(6, 'enqueue_local_media', { projectId: 'project-id', path: '/tmp/a.mp4' });
    expect(invokeMock).toHaveBeenNthCalledWith(7, 'enqueue_url', { projectId: 'project-id', url: 'https://www.youtube.com/watch?v=fixture' });
    expect(invokeMock).toHaveBeenNthCalledWith(8, 'retry_job', { jobId: 'job-id' });
    expect(invokeMock).toHaveBeenNthCalledWith(9, 'save_watchlist', { query: { profileUrl: 'https://www.youtube.com/@fixture', limit: 25 } });
    expect(invokeMock).toHaveBeenNthCalledWith(10, 'scan_creator', { query: { profileUrl: 'https://www.youtube.com/@fixture', limit: 25 } });
    expect(invokeMock).toHaveBeenNthCalledWith(11, 'refresh_watchlists');
    expect(invokeMock).toHaveBeenNthCalledWith(12, 'queue_research', { ids: ['research-id'] });
    expect(invokeMock).toHaveBeenNthCalledWith(13, 'cancel_job', { jobId: 'job-id' });
    expect(invokeMock).toHaveBeenNthCalledWith(14, 'build_ai_prompt', { input: aiInput });
    expect(invokeMock).toHaveBeenNthCalledWith(15, 'run_ai', { input: aiInput });
    expect(invokeMock).toHaveBeenNthCalledWith(16, 'get_settings');
    expect(invokeMock).toHaveBeenNthCalledWith(17, 'save_settings', { settings: expect.objectContaining({ whisperModel: 'medium', outputDirectory: null }) });
    expect(invokeMock).toHaveBeenNthCalledWith(18, 'import_legacy_data');
    expect(invokeMock).toHaveBeenNthCalledWith(19, 'list_jobs');
  });
});
