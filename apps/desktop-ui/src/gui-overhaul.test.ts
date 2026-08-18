import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { beforeEach, describe, expect, it } from 'vitest';
import App from './App.svelte';
import { createMockClient } from './api/mockClient';

beforeEach(() => {
  window.localStorage.clear();
  delete document.documentElement.dataset.theme;
});

async function ready() {
  render(App, { props: { api: createMockClient() } });
  await screen.findByRole('heading', { name: 'Creator Lab' });
}

describe('Creator Control Room GUI', () => {
  it('persists the compact sidebar preference', async () => {
    await ready();

    const collapse = screen.getByRole('button', { name: 'Collapse sidebar' });
    await fireEvent.click(collapse);

    expect(window.localStorage.getItem('scriptotar.sidebarCollapsed')).toBe('1');
    expect(screen.getByRole('button', { name: 'Expand sidebar' })).toBeInTheDocument();
  });

  it('opens the command palette from the keyboard and executes a command with Enter', async () => {
    await ready();

    await fireEvent.keyDown(window, { key: 'k', ctrlKey: true });
    expect(screen.getByRole('dialog', { name: 'Command palette' })).toBeInTheDocument();

    const input = screen.getByLabelText('Search workspace and commands');
    await fireEvent.input(input, { target: { value: 'Open Settings' } });
    await fireEvent.keyDown(input, { key: 'Enter' });

    expect(await screen.findByRole('heading', { name: 'Settings' })).toBeInTheDocument();
    expect(screen.queryByRole('dialog', { name: 'Command palette' })).not.toBeInTheDocument();
  });

  it('persists transcript rail visibility without changing transcript content', async () => {
    await ready();
    await fireEvent.click(screen.getByRole('button', { name: /Transcript/ }));

    const transcript = screen.getByTestId('transcript-content').textContent;
    await fireEvent.click(screen.getByRole('button', { name: 'Hide transcript list' }));

    expect(window.localStorage.getItem('scriptotar.transcriptListOpen')).toBe('0');
    expect(screen.getByRole('button', { name: 'Show transcript list' })).toBeInTheDocument();
    expect(screen.getByTestId('transcript-content').textContent).toBe(transcript);

    await fireEvent.click(screen.getByRole('button', { name: 'Hide details' }));
    expect(window.localStorage.getItem('scriptotar.transcriptDetailsOpen')).toBe('0');
    expect(screen.getByRole('button', { name: 'Show details' })).toBeInTheDocument();
  });

  it('surfaces unsaved settings after a local preference changes', async () => {
    await ready();
    await fireEvent.click(screen.getByRole('button', { name: /Settings/ }));

    expect(screen.queryByText('Unsaved changes')).not.toBeInTheDocument();
    await fireEvent.change(screen.getByLabelText('Theme'), { target: { value: 'system' } });

    await waitFor(() => expect(screen.getByText('Unsaved changes')).toBeInTheDocument());
  });
});
