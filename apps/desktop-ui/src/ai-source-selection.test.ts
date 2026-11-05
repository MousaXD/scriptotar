import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { beforeEach, describe, expect, it } from 'vitest';
import App from './App.svelte';
import { createMockClient, mockBootstrap } from './api/mockClient';

beforeEach(() => {
  window.localStorage.clear();
  delete document.documentElement.dataset.theme;
});

async function ready() {
  render(App, { props: { api: createMockClient() } });
  await screen.findByRole('heading', { name: 'Creator Lab' });
  await fireEvent.click(screen.getByRole('button', { name: /AI Studio/ }));
}

describe('AI source selection', () => {
  it('starts from manual input and only attributes an explicitly chosen exact transcript', async () => {
    await ready();

    const selector = screen.getByLabelText('Transcript source');
    expect(selector).toHaveValue('manual');
    expect(screen.queryByTestId('ai-source-lineage')).not.toBeInTheDocument();

    await fireEvent.change(selector, { target: { value: 't-ar' } });
    expect(screen.getByTestId('ai-source-lineage')).toHaveTextContent('Hook breakdown — Arabic sample');
    await waitFor(() => expect(screen.getByTestId('ai-source-lineage')).toHaveTextContent('Exact loaded source'));

    await fireEvent.change(selector, { target: { value: 't-en' } });

    expect(selector).toHaveValue('t-en');
    expect(screen.getByTestId('ai-source-lineage')).toHaveTextContent('Caption pacing breakdown');
    await waitFor(() => expect(screen.getByPlaceholderText('Paste or load transcript/research text…')).toHaveValue(
      mockBootstrap.transcripts.find((transcript) => transcript.id === 't-en')?.text
    ));
  });

  it('supports a deliberate manual source instead of silently keeping another transcript', async () => {
    await ready();

    const selector = screen.getByLabelText('Transcript source');
    await fireEvent.change(selector, { target: { value: 'manual' } });

    expect(selector).toHaveValue('manual');
    expect(screen.queryByTestId('ai-source-lineage')).not.toBeInTheDocument();
    expect(screen.getByPlaceholderText('Paste or load transcript/research text…')).toHaveValue('');
  });

  it('refuses a transcript whose returned ID does not match the selected source', async () => {
    const api = createMockClient();
    api.getTranscript = async () => ({ ...mockBootstrap.transcripts[0], id: 'unrelated-id' });
    render(App, { props: { api } });
    await screen.findByRole('heading', { name: 'Creator Lab' });
    await fireEvent.click(screen.getByRole('button', { name: /AI Studio/ }));

    await fireEvent.change(screen.getByLabelText('Transcript source'), { target: { value: 't-ar' } });
    expect(await screen.findByRole('alert')).toHaveTextContent('did not match the selected project and source ID');
    expect(screen.getByTestId('ai-source-lineage')).toHaveTextContent('Source unavailable');
    expect(screen.getByPlaceholderText('Paste or load transcript/research text…')).toHaveValue('');
    expect(screen.getByRole('button', { name: 'Build prompt' })).toBeDisabled();
  });

  it('marks an edited excerpt as modified instead of claiming it is an exact copy', async () => {
    await ready();
    await fireEvent.change(screen.getByLabelText('Transcript source'), { target: { value: 't-ar' } });
    await waitFor(() => expect(screen.getByTestId('ai-source-lineage')).toHaveTextContent('Exact loaded source'));

    await fireEvent.input(screen.getByPlaceholderText('Paste or load transcript/research text…'), {
      target: { value: 'My adapted excerpt' }
    });
    expect(screen.getByTestId('ai-source-lineage')).toHaveTextContent('Locally edited excerpt');
  });

  it('preserves keyless Copy Prompt and distinguishes the edited BYOK preview from its request', async () => {
    const api = createMockClient();
    const runCalls: string[] = [];
    const runAi = api.runAi;
    api.runAi = async (input) => { runCalls.push(input.sourceText); return runAi(input); };
    render(App, { props: { api } });
    await screen.findByRole('heading', { name: 'Creator Lab' });
    await fireEvent.click(screen.getByRole('button', { name: /AI Studio/ }));

    await fireEvent.input(screen.getByPlaceholderText('Paste or load transcript/research text…'), { target: { value: 'Manual source' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Build prompt' }));
    await waitFor(() => expect((screen.getByPlaceholderText('Build a prompt to preview it here…') as HTMLTextAreaElement).value).toContain('Manual source'));
    expect(runCalls).toHaveLength(0);

    await fireEvent.click(screen.getByRole('button', { name: /BYOK/ }));
    await fireEvent.input(screen.getByLabelText('API key'), { target: { value: 'test-only-key' } });
    await fireEvent.input(screen.getByPlaceholderText('Build a prompt to preview it here…'), { target: { value: 'Changed preview, not a generated request' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Run with API' }));
    expect(screen.getByRole('status')).toHaveTextContent('edited prompt preview is for copying only');
    expect(runCalls).toHaveLength(0);
  });
});
