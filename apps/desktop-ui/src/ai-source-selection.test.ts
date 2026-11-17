import { act, fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { beforeEach, describe, expect, it } from 'vitest';
import App from './App.svelte';
import { createMockClient, mockBootstrap } from './api/mockClient';
import type { ScriptotarApi } from './api/client';
import type { Transcript } from './types';

beforeEach(() => {
  window.localStorage.clear();
  delete document.documentElement.dataset.theme;
});

async function ready(api: ScriptotarApi = createMockClient()) {
  render(App, { props: { api } });
  await screen.findByRole('heading', { name: 'Creator Lab' });
  await fireEvent.keyDown(window, { key: 'a' });
}

async function chooseSource(name: string) {
  await fireEvent.click(screen.getByRole('combobox', { name: 'Transcript source' }));
  await fireEvent.click(screen.getByRole('option', { name }));
}
async function chooseProvider(name: string) {
  await fireEvent.click(screen.getByRole('combobox', { name: 'AI provider' }));
  await fireEvent.click(screen.getByRole('option', { name }));
}
function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((complete) => { resolve = complete; });
  return { promise, resolve };
}

describe('AI source selection', () => {
  it('starts from manual input and only attributes an explicitly chosen exact transcript', async () => {
    await ready();

    const selector = screen.getByLabelText('Transcript source');
    expect(selector).toHaveTextContent('Manual / pasted text');
    expect(screen.queryByTestId('ai-source-lineage')).not.toBeInTheDocument();

    await chooseSource('Hook breakdown — Arabic sample');
    expect(screen.getByTestId('ai-source-lineage')).toHaveTextContent('Hook breakdown — Arabic sample');
    await waitFor(() => expect(screen.getByTestId('ai-source-lineage')).toHaveTextContent('Exact loaded source'));

    await chooseSource('Caption pacing breakdown');

    expect(selector).toHaveTextContent('Caption pacing breakdown');
    expect(screen.getByTestId('ai-source-lineage')).toHaveTextContent('Caption pacing breakdown');
    await waitFor(() => expect(screen.getByPlaceholderText('Paste or load transcript/research text…')).toHaveValue(
      mockBootstrap.transcripts.find((transcript) => transcript.id === 't-en')?.text
    ));
  });

  it('supports a deliberate manual source instead of silently keeping another transcript', async () => {
    await ready();

    const selector = screen.getByLabelText('Transcript source');
    await chooseSource('Manual / pasted text');

    expect(selector).toHaveTextContent('Manual / pasted text');
    expect(screen.queryByTestId('ai-source-lineage')).not.toBeInTheDocument();
    expect(screen.getByPlaceholderText('Paste or load transcript/research text…')).toHaveValue('');
  });

  it('refuses a transcript whose returned ID does not match the selected source', async () => {
    const api = createMockClient();
    api.getTranscript = async () => ({ ...mockBootstrap.transcripts[0], id: 'unrelated-id' });
    render(App, { props: { api } });
    await screen.findByRole('heading', { name: 'Creator Lab' });
    await fireEvent.keyDown(window, { key: 'a' });

    await chooseSource('Hook breakdown — Arabic sample');
    expect(await screen.findByRole('alert')).toHaveTextContent('did not match the selected project and source ID');
    expect(screen.getByTestId('ai-source-lineage')).toHaveTextContent('Source unavailable');
    expect(screen.getByPlaceholderText('Paste or load transcript/research text…')).toHaveValue('');
    expect(screen.getByRole('button', { name: 'Build prompt' })).toBeDisabled();
  });

  it('marks an edited excerpt as modified instead of claiming it is an exact copy', async () => {
    await ready();
    await chooseSource('Hook breakdown — Arabic sample');
    await waitFor(() => expect(screen.getByTestId('ai-source-lineage')).toHaveTextContent('Exact loaded source'));

    await fireEvent.input(screen.getByPlaceholderText('Paste or load transcript/research text…'), {
      target: { value: 'My adapted excerpt' }
    });
    expect(screen.getByTestId('ai-source-lineage')).toHaveTextContent('Locally edited excerpt');
  });

  it('preserves local Copy Prompt and keeps an edited BYOK preview offline', async () => {
    const api = createMockClient();
    const runCalls: string[] = [];
    api.runAi = async (input) => { runCalls.push(input.sourceText); return 'UNSAFE_REMOTE_OUTPUT'; };
    await ready(api);

    await fireEvent.input(screen.getByPlaceholderText('Paste or load transcript/research text…'), { target: { value: 'Manual source' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Build prompt' }));
    const preview = screen.getByPlaceholderText('Build a prompt to preview it here…');
    await waitFor(() => expect((preview as HTMLTextAreaElement).value).toContain('Manual source'));
    expect(runCalls).toHaveLength(0);

    await fireEvent.click(screen.getByRole('button', { name: /BYOK/ }));
    expect(screen.getByLabelText('API key')).toBeDisabled();
    await fireEvent.input(preview, { target: { value: 'Changed preview, not a generated request' } });
    const send = screen.getByRole('button', { name: 'Run with API (unavailable)' });
    expect(send).toBeDisabled();
    await fireEvent.click(send);
    expect(screen.getByRole('status')).toHaveTextContent('BYOK sending is unavailable');
    expect(preview).toHaveValue('Changed preview, not a generated request');
    expect(runCalls).toHaveLength(0);
  });

  it('ignores an older transcript response after the user selects another source', async () => {
    const api = createMockClient();
    const oldResponse = deferred<Transcript>();
    const fetchTranscript = api.getTranscript;
    api.getTranscript = (id) => id === 't-ar' ? oldResponse.promise : fetchTranscript(id);
    await ready(api);

    const selector = screen.getByLabelText('Transcript source');
    await chooseSource('Hook breakdown — Arabic sample');
    expect(screen.getByTestId('ai-source-lineage')).toHaveTextContent('Loading exact source');
    await chooseSource('Caption pacing breakdown');

    const textarea = screen.getByPlaceholderText('Paste or load transcript/research text…');
    await waitFor(() => expect(textarea).toHaveValue(mockBootstrap.transcripts[1].text));
    await act(async () => {
      oldResponse.resolve(structuredClone(mockBootstrap.transcripts[0]));
      await oldResponse.promise;
    });
    expect(selector).toHaveTextContent('Caption pacing breakdown');
    expect(textarea).toHaveValue(mockBootstrap.transcripts[1].text);
    expect(screen.getByTestId('ai-source-lineage')).toHaveTextContent('Caption pacing breakdown');
  });

  it('discards a prompt that resolves after the source was edited', async () => {
    const api = createMockClient();
    const promptResponse = deferred<string>();
    api.buildAiPrompt = () => promptResponse.promise;
    await ready(api);

    const source = screen.getByPlaceholderText('Paste or load transcript/research text…');
    const preview = screen.getByPlaceholderText('Build a prompt to preview it here…');
    await fireEvent.input(source, { target: { value: 'Old source' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Build prompt' }));
    await fireEvent.input(source, { target: { value: 'Updated source' } });
    await act(async () => {
      promptResponse.resolve('Prompt for stale old source');
      await promptResponse.promise;
    });
    expect(source).toHaveValue('Updated source');
    expect(preview).toHaveValue('');
    expect(screen.getByRole('button', { name: 'Build prompt' })).toBeEnabled();
  });

  it('does not start a provider request after changing provider or mode', async () => {
    const api = createMockClient();
    let calls = 0;
    api.runAi = async () => { calls += 1; return 'STALE_PROVIDER_OUTPUT'; };
    await ready(api);

    await fireEvent.click(screen.getByRole('button', { name: /BYOK/ }));
    const send = screen.getByRole('button', { name: 'Run with API (unavailable)' });
    expect(send).toBeDisabled();
    await chooseProvider('Anthropic');
    await fireEvent.click(send);
    await fireEvent.click(screen.getByRole('button', { name: 'Copy Prompt' }));
    expect(calls).toBe(0);
    expect(screen.queryByText('STALE_PROVIDER_OUTPUT')).not.toBeInTheDocument();
  });

  it('does not accept a provider secret while BYOK egress is uncertified', async () => {
    const api = createMockClient();
    const secret = 'test-provider-secret-123';
    let called = false;
    api.runAi = async () => { called = true; throw new Error(`Provider rejected token ${secret}`); };
    await ready(api);

    await fireEvent.click(screen.getByRole('button', { name: /BYOK/ }));
    const key = screen.getByLabelText('API key');
    expect(key).toBeDisabled();
    expect(key).toHaveValue('');
    const send = screen.getByRole('button', { name: 'Run with API (unavailable)' });
    expect(send).toBeDisabled();
    await fireEvent.click(send);
    expect(called).toBe(false);
    expect(screen.getByRole('status')).not.toHaveTextContent(secret);
    expect(screen.getByRole('status')).not.toHaveTextContent('AI request failed');
  });

  it('rejects an exact transcript ID that belongs to another project', async () => {
    const api = createMockClient();
    api.getTranscript = async () => ({ ...mockBootstrap.transcripts[0], projectId: 'p-client-a' });
    await ready(api);

    await chooseSource('Hook breakdown — Arabic sample');
    expect(await screen.findByRole('alert')).toHaveTextContent('did not match the selected project and source ID');
    expect(screen.getByRole('button', { name: 'Build prompt' })).toBeDisabled();
  });

  it('allows a fresh prompt after editing while an earlier prompt never completes promptly', async () => {
    const api = createMockClient();
    const oldPrompt = deferred<string>();
    const newPrompt = deferred<string>();
    let calls = 0;
    api.buildAiPrompt = () => (++calls === 1 ? oldPrompt.promise : newPrompt.promise);
    await ready(api);

    const source = screen.getByPlaceholderText('Paste or load transcript/research text…');
    const preview = screen.getByPlaceholderText('Build a prompt to preview it here…');
    const build = screen.getByRole('button', { name: 'Build prompt' });
    await fireEvent.input(source, { target: { value: 'Old brief' } });
    await fireEvent.click(build);
    expect(screen.getByRole('button', { name: 'Building…' })).toBeDisabled();

    await fireEvent.input(source, { target: { value: 'New brief' } });
    expect(build).toBeEnabled();
    await fireEvent.click(build);
    expect(calls).toBe(2);
    expect(screen.getByRole('button', { name: 'Building…' })).toBeDisabled();

    await act(async () => {
      oldPrompt.resolve('OUTDATED_PROMPT');
      await oldPrompt.promise;
    });
    expect(preview).toHaveValue('');
    expect(screen.getByRole('button', { name: 'Building…' })).toBeDisabled();

    await act(async () => {
      newPrompt.resolve('New brief prompt');
      await newPrompt.promise;
    });
    await waitFor(() => expect(preview).toHaveValue('New brief prompt'));
    expect(screen.getByRole('button', { name: 'Build prompt' })).toBeEnabled();
  });

  it('never sends BYOK credentials or provider URL to local prompt construction', async () => {
    const api = createMockClient();
    const original = api.buildAiPrompt;
    const inputs: Array<{ apiKey?: string; baseUrl?: string }> = [];
    api.buildAiPrompt = async (input) => {
      inputs.push({ apiKey: input.apiKey, baseUrl: input.baseUrl });
      return original(input);
    };
    await ready(api);

    await fireEvent.click(screen.getByRole('button', { name: /BYOK/ }));
    await chooseProvider('OpenAI-compatible');
    await fireEvent.input(screen.getByLabelText('API key'), { target: { value: 'do-not-forward-secret' } });
    await fireEvent.input(screen.getByPlaceholderText('https://…'), { target: { value: 'https://example.com/v1' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Build prompt' }));
    await waitFor(() => expect(screen.getByRole('status')).toHaveTextContent('Prompt built locally'));
    expect(inputs).toEqual([{ apiKey: undefined, baseUrl: undefined }]);
  });
});
