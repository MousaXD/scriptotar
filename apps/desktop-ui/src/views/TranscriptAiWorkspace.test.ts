import { act, fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createMockClient, mockBootstrap } from '../api/mockClient';
import type { Transcript } from '../types';
import TranscriptView from './TranscriptView.svelte';
import AiStudioView from './AiStudioView.svelte';

beforeEach(() => window.localStorage.clear());

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((finish) => { resolve = finish; });
  return { promise, resolve };
}

describe('Transcript reader workspace', () => {
  it('renders long transcripts incrementally while keeping search and late matches reachable', async () => {
    const transcript: Transcript = {
      ...mockBootstrap.transcripts[1],
      id: 't-long',
      title: 'Extended interview',
      segments: Array.from({ length: 350 }, (_, index) => ({
        id: 'segment-' + index,
        startSeconds: index * 5,
        endSeconds: index * 5 + 4,
        text: 'Transcript segment ' + index
      }))
    };
    const api = createMockClient({ getTranscript: async () => transcript });
    render(TranscriptView, { props: { api, transcripts: [transcript], selectedId: transcript.id } });

    const content = await screen.findByTestId('transcript-content');
    expect(content.querySelectorAll('article.segment')).toHaveLength(140);
    await fireEvent.click(screen.getByRole('button', { name: 'Show more transcript segments' }));
    expect(content.querySelectorAll('article.segment')).toHaveLength(280);

    await fireEvent.input(screen.getByRole('textbox'), { target: { value: 'Transcript segment 349' } });
    expect(content).toHaveTextContent('Transcript segment 349');
    expect(content.querySelectorAll('article.segment')).toHaveLength(1);

    await fireEvent.click(content.querySelector('button.timestamp')!);
    await waitFor(() => expect(content.querySelectorAll('article.segment')).toHaveLength(350));
    expect(screen.getByRole('textbox')).toHaveValue('');
  });

  it('keeps the source direction and ignores an older source response after switching', async () => {
    const api = createMockClient();
    const old = deferred<Transcript>();
    const original = api.getTranscript;
    api.getTranscript = (id) => id === 't-ar' ? old.promise : original(id);
    render(TranscriptView, { props: { api, transcripts: mockBootstrap.transcripts, selectedId: 't-ar' } });

    expect(await screen.findByTestId('transcript-loading-skeleton')).toBeInTheDocument();
    await fireEvent.change(screen.getByRole('combobox'), { target: { value: 't-en' } });
    const content = await screen.findByTestId('transcript-content');
    expect(content).toHaveAttribute('dir', 'ltr');
    expect(content).toHaveTextContent('The caption is doing two jobs');

    await act(async () => {
      old.resolve(mockBootstrap.transcripts[0]);
      await old.promise;
    });
    expect(content).toHaveAttribute('dir', 'ltr');
    expect(content).not.toHaveTextContent('أول ثلاث ثواني');
  });

  it('refuses a transcript from the wrong project rather than displaying its text', async () => {
    const api = createMockClient({
      getTranscript: async () => ({ ...mockBootstrap.transcripts[0], projectId: 'different-project' })
    });
    render(TranscriptView, { props: { api, transcripts: [mockBootstrap.transcripts[0]], selectedId: 't-ar' } });
    expect(await screen.findByRole('alert')).toHaveTextContent('did not match the selected project');
    expect(screen.queryByTestId('transcript-content')).not.toBeInTheDocument();
  });

  it('clears old content when the same transcript ID points to a different project', async () => {
    const oldTranscript = mockBootstrap.transcripts[1];
    const replacement: Transcript = {
      ...oldTranscript,
      projectId: 'another-project',
      text: 'New project transcription',
      segments: [{ id: 'replacement', startSeconds: 0, endSeconds: 3, text: 'New project transcription' }]
    };
    const newer = deferred<Transcript>();
    let reads = 0;
    const api = createMockClient({
      getTranscript: () => ++reads === 1 ? Promise.resolve(oldTranscript) : newer.promise
    });
    const { rerender } = render(TranscriptView, {
      props: { api, transcripts: [oldTranscript], selectedId: oldTranscript.id }
    });
    expect(await screen.findByTestId('transcript-content')).toHaveTextContent('The caption is doing two jobs');

    await rerender({ api, transcripts: [replacement], selectedId: oldTranscript.id });
    await waitFor(() => expect(screen.queryByTestId('transcript-content')).not.toBeInTheDocument());
    expect(reads).toBe(2);

    await act(async () => {
      newer.resolve(replacement);
      await newer.promise;
    });
    expect(await screen.findByTestId('transcript-content')).toHaveTextContent('New project transcription');
    expect(screen.getByTestId('transcript-content')).not.toHaveTextContent('The caption is doing two jobs');
  });

  it('keeps all five export actions usable, regardless of transcript segment paging', async () => {
    const originalCreate = URL.createObjectURL;
    const originalRevoke = URL.revokeObjectURL;
    URL.createObjectURL = vi.fn(() => 'blob:transcript-test');
    URL.revokeObjectURL = vi.fn();
    const names: string[] = [];
    const clickSpy = vi.spyOn(HTMLAnchorElement.prototype, 'click').mockImplementation(function (this: HTMLAnchorElement) {
      names.push(this.download);
    });
    try {
      const api = createMockClient();
      const { container } = render(TranscriptView, {
        props: { api, transcripts: [mockBootstrap.transcripts[1]], selectedId: 't-en' }
      });
      await screen.findByTestId('transcript-content');
      const exports = container.querySelectorAll('.export-popover button');
      expect(exports).toHaveLength(5);
      for (const control of exports) await fireEvent.click(control);
      expect(names).toEqual([
        'Caption pacing breakdown.txt',
        'Caption pacing breakdown.timestamps.txt',
        'Caption pacing breakdown.srt',
        'Caption pacing breakdown.vtt',
        'Caption pacing breakdown.json'
      ]);
      expect(URL.revokeObjectURL).toHaveBeenCalledTimes(5);
    } finally {
      URL.createObjectURL = originalCreate;
      URL.revokeObjectURL = originalRevoke;
      clickSpy.mockRestore();
    }
  });

  it('retains Arabic direction and timestamp controls in the reader', async () => {
    render(TranscriptView, {
      props: { api: createMockClient(), transcripts: [mockBootstrap.transcripts[0]], selectedId: 't-ar' }
    });
    const content = await screen.findByTestId('transcript-content');
    expect(content).toHaveAttribute('dir', 'rtl');
    expect(content).toHaveTextContent('أول ثلاث ثواني');
    expect(content.querySelectorAll('button.timestamp')).toHaveLength(2);
    expect(screen.getByText('local://arabic-hook.mp4')).toBeInTheDocument();
  });
});

describe('AI Studio request isolation', () => {
  it('keeps a completed prompt when changing between Copy Prompt and BYOK', async () => {
    const api = createMockClient();
    render(AiStudioView, { props: { api, transcripts: mockBootstrap.transcripts } });
    await fireEvent.input(screen.getByPlaceholderText('Paste or load transcript/research text…'), { target: { value: 'Reusable source' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Build prompt' }));
    const preview = screen.getByPlaceholderText('Build a prompt to preview it here…');
    await waitFor(() => expect((preview as HTMLTextAreaElement).value).toContain('Reusable source'));
    const built = (preview as HTMLTextAreaElement).value;
    await fireEvent.click(screen.getByRole('button', { name: 'BYOK' }));
    expect(preview).toHaveValue(built);
    await fireEvent.click(screen.getByRole('button', { name: 'Copy Prompt' }));
    expect(preview).toHaveValue(built);
  });

  it('invalidates a source when its transcript ID changes ownership project', async () => {
    const api = createMockClient();
    const original = mockBootstrap.transcripts[0];
    const { rerender } = render(AiStudioView, { props: { api, transcripts: [original] } });
    await fireEvent.change(screen.getByLabelText('Transcript source'), { target: { value: original.id } });
    const source = screen.getByPlaceholderText('Paste or load transcript/research text…');
    await waitFor(() => expect(source).toHaveValue(original.text));
    const moved: Transcript = { ...original, projectId: 'another-project' };
    await rerender({ api, transcripts: [moved] });
    await waitFor(() => expect(screen.getByLabelText('Transcript source')).toHaveValue('manual'));
    expect(source).toHaveValue('');
    await waitFor(() => expect(screen.queryByTestId('ai-source-lineage')).not.toBeInTheDocument());
  });
  it('allows a new provider run after inputs change without an old completion clearing busy state', async () => {
    const old = deferred<string>();
    const next = deferred<string>();
    let calls = 0;
    const api = createMockClient({
      runAi: () => ++calls === 1 ? old.promise : next.promise
    });
    render(AiStudioView, { props: { api, transcripts: mockBootstrap.transcripts } });

    await fireEvent.click(screen.getByRole('button', { name: 'BYOK' }));
    await fireEvent.input(screen.getByLabelText('API key'), { target: { value: 'session-secret' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Run with API' }));
    expect(screen.getByRole('button', { name: 'Running…' })).toBeDisabled();

    await fireEvent.change(screen.getByRole('combobox', { name: 'AI provider' }), { target: { value: 'Anthropic' } });
    await fireEvent.input(screen.getByLabelText('Model'), { target: { value: 'custom-model' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Run with API' }));
    expect(calls).toBe(2);

    await act(async () => {
      old.resolve('STALE_RESPONSE');
      await old.promise;
    });
    expect(screen.queryByText('STALE_RESPONSE')).not.toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Running…' })).toBeDisabled();

    await act(async () => {
      next.resolve('Current provider output');
      await next.promise;
    });
    await waitFor(() => expect(screen.getByText('Current provider output')).toBeInTheDocument());
  });
});
