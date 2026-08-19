import { fireEvent, render, screen } from '@testing-library/svelte';
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
  it('starts from a visible exact transcript and can switch sources by ID', async () => {
    await ready();

    const selector = screen.getByLabelText('Transcript source');
    expect(selector).toHaveValue('t-ar');
    expect(screen.getByTestId('ai-source-lineage')).toHaveTextContent('Hook breakdown — Arabic sample');

    await fireEvent.change(selector, { target: { value: 't-en' } });

    expect(selector).toHaveValue('t-en');
    expect(screen.getByTestId('ai-source-lineage')).toHaveTextContent('Caption pacing breakdown');
    expect(screen.getByPlaceholderText('Paste or load transcript/research text…')).toHaveValue(
      mockBootstrap.transcripts.find((transcript) => transcript.id === 't-en')?.text
    );
  });

  it('supports a deliberate manual source instead of silently keeping another transcript', async () => {
    await ready();

    const selector = screen.getByLabelText('Transcript source');
    await fireEvent.change(selector, { target: { value: 'manual' } });

    expect(selector).toHaveValue('manual');
    expect(screen.queryByTestId('ai-source-lineage')).not.toBeInTheDocument();
    expect(screen.getByPlaceholderText('Paste or load transcript/research text…')).toHaveValue('');
  });
});
