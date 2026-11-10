import { act, fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { beforeEach, describe, expect, it } from 'vitest';
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

  it('retains Arabic direction and timestamp controls in the reader', async () => {
    render(TranscriptView, {
      props: { api: createMockClient(), transcripts: [mockBootstrap.transcripts[0]], selectedId: 't-ar' }
    });
    const content = await screen.findByTestId('transcript-content');
    expect(content).toHaveAttribute('dir', 'rtl');
    expect(content).toHaveTextContent('أول ثلاث ثواني');
    expect(content.querySelectorAll('button.timestamp')).toHaveLength(2);
  });
});

describe('AI Studio request isolation', () => {
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
