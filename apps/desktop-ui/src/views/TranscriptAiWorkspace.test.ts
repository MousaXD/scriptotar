import { act, fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createMockClient, mockBootstrap } from '../api/mockClient';
import type { Transcript } from '../types';
import TranscriptView from './TranscriptView.svelte';
import { setLocale } from '../i18n';
import AiStudioView from './AiStudioView.svelte';

beforeEach(() => { window.localStorage.clear(); setLocale('en'); });

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
    await fireEvent.click(screen.getByRole('combobox', { name: 'Choose transcript' }));
    await fireEvent.click(screen.getByRole('option', { name: 'Caption pacing breakdown' }));
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
  it('localizes ownership mismatch and focuses its real denial without displaying foreign content', async () => {
    setLocale('ar');
    const original = mockBootstrap.transcripts[0];
    const api = createMockClient({
      getTranscript: async () => ({ ...original, projectId: 'foreign-project' })
    });
    render(TranscriptView, { props: { api, transcripts: [original], selectedId: original.id } });
    const alert = await screen.findByRole('alert');
    expect(alert).toHaveTextContent('لا يطابق المشروع');
    await waitFor(() => expect(alert).toHaveFocus());
    expect(screen.queryByTestId('transcript-content')).not.toBeInTheDocument();
  });

  it('keeps Arabic reader labels translated and isolates mixed-script metadata and timestamps', async () => {
    setLocale('ar');
    const transcript: Transcript = {
      ...mockBootstrap.transcripts[0],
      title: 'Scriptotar — مقابلة المبدع',
      segments: Array.from({ length: 142 }, (_, index) => ({
        id: 'ar-segment-' + index,
        startSeconds: index * 4,
        endSeconds: index * 4 + 3,
        text: 'المقطع Scriptotar ' + index
      }))
    };
    const onUseInStudio = vi.fn();
    const api = createMockClient({ getTranscript: async () => transcript });
    render(TranscriptView, { props: { api, transcripts: [transcript], selectedId: transcript.id, onUseInStudio } });
    const content = await screen.findByTestId('transcript-content');
    expect(screen.getByRole('heading', { name: transcript.title })).toHaveAttribute('dir', 'auto');
    expect(content.querySelector('button.timestamp')).toHaveAttribute('dir', 'ltr');
    expect(screen.getByRole('textbox', { name: 'البحث في النص المفرغ' })).toHaveAttribute('dir', 'auto');
    expect(screen.getByRole('button', { name: 'عرض مقاطع تفريغ إضافية' })).toBeInTheDocument();
    await fireEvent.click(screen.getByRole('button', { name: 'استخدام في الاستوديو' }));
    expect(onUseInStudio).toHaveBeenCalledWith(transcript.id);
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
    await fireEvent.click(screen.getByRole('combobox', { name: 'Transcript source' }));
    await fireEvent.click(screen.getByRole('option', { name: original.title }));
    const source = screen.getByPlaceholderText('Paste or load transcript/research text…');
    await waitFor(() => expect(source).toHaveValue(original.text));
    const moved: Transcript = { ...original, projectId: 'another-project' };
    await rerender({ api, transcripts: [moved] });
    await waitFor(() => expect(screen.getByRole('combobox', { name: 'Transcript source' })).toHaveTextContent('Manual / pasted text'));
    expect(source).toHaveValue('');
    await waitFor(() => expect(screen.queryByTestId('ai-source-lineage')).not.toBeInTheDocument());
  });
  it('keeps uncertified BYOK execution blocked without collecting credentials', async () => {
    let calls = 0;
    const api = createMockClient({
      runAi: async () => {
        calls += 1;
        return 'Uncertified provider output';
      }
    });
    render(AiStudioView, { props: { api, transcripts: mockBootstrap.transcripts } });

    await fireEvent.click(screen.getByRole('button', { name: 'BYOK' }));
    expect(screen.getByRole('status')).toHaveTextContent('BYOK sending is unavailable');
    expect(screen.getByLabelText('API key')).toBeDisabled();
    const send = screen.getByRole('button', { name: 'Run with API (unavailable)' });
    expect(send).toBeDisabled();
    await fireEvent.click(send);
    expect(calls).toBe(0);
    expect(screen.queryByText('Uncertified provider output')).not.toBeInTheDocument();

    await fireEvent.click(screen.getByRole('button', { name: 'Copy Prompt' }));
    expect(screen.getByRole('button', { name: 'Build prompt' })).toBeEnabled();
  });
});
