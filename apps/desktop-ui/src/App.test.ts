import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import App from './App.svelte';
import { APPEARANCE_STORAGE_KEY } from './appearance';
import { createMockClient } from './api/mockClient';

beforeEach(() => {
  window.localStorage.clear();
  delete document.documentElement.dataset.theme;
});

async function ready(api = createMockClient()) {
  render(App, { props: { api } });
  await screen.findByRole('heading', { name: 'Creator Lab' });
  return api;
}

describe('desktop workstation', () => {
  it('switches projects through the backend contract', async () => {
    const api = createMockClient();
    const selectProject = vi.spyOn(api, 'selectProject');
    await ready(api);
    const select = screen.getByLabelText('Project');
    await fireEvent.change(select, { target: { value: 'p-client-a' } });
    await waitFor(() => expect(selectProject).toHaveBeenCalledWith('p-client-a'));
    expect(await screen.findByRole('heading', { name: 'Client A' })).toBeInTheDocument();
    expect(select).toHaveValue('p-client-a');
  });

  it('keeps the previous project usable when project switching fails', async () => {
    const api = createMockClient();
    vi.spyOn(api, 'selectProject').mockRejectedValueOnce(new Error('Project unavailable'));
    await ready(api);
    const select = screen.getByLabelText('Project');
    await fireEvent.change(select, { target: { value: 'p-client-a' } });
    expect(await screen.findByRole('alert')).toHaveTextContent('Project unavailable');
    expect(screen.getByRole('heading', { name: 'Creator Lab' })).toBeInTheDocument();
    await waitFor(() => expect(select).toHaveValue('p-creator-lab'));
    await fireEvent.change(select, { target: { value: 'p-client-a' } });
    expect(await screen.findByRole('heading', { name: 'Client A' })).toBeInTheDocument();
    expect(screen.queryByRole('alert')).not.toBeInTheDocument();
  });

  it('uses the native picker for the normal local-media queue flow', async () => {
    const api = createMockClient();
    const chooseLocalMedia = vi.spyOn(api, 'chooseLocalMedia');
    const enqueueLocalMedia = vi.spyOn(api, 'enqueueLocalMedia');
    await ready(api);
    await fireEvent.click(screen.getByRole('button', { name: /Jobs/ }));
    await fireEvent.click(screen.getByRole('button', { name: 'Choose video' }));
    await waitFor(() => expect(chooseLocalMedia).toHaveBeenCalledTimes(1));
    expect(screen.getByText('/mock/selected-video.mp4')).toBeInTheDocument();
    await fireEvent.click(screen.getByRole('button', { name: 'Queue selected' }));
    await waitFor(() => expect(enqueueLocalMedia).toHaveBeenCalledWith('p-creator-lab', '/mock/selected-video.mp4'));
  });

  it('renders persisted job states including failure and interruption', async () => {
    await ready();
    await fireEvent.click(screen.getByRole('button', { name: /Jobs/ }));
    expect(screen.getByTestId('job-transcribing')).toBeInTheDocument();
    expect(screen.getByTestId('job-failed')).toBeInTheDocument();
    expect(screen.getByTestId('job-interrupted')).toBeInTheDocument();
  });

  it('refreshes active jobs without repeatedly bootstrapping the full workspace', async () => {
    const api = createMockClient();
    const bootstrap = vi.spyOn(api, 'bootstrap');
    const listJobs = vi.spyOn(api, 'listJobs');
    await ready(api);
    await waitFor(() => expect(listJobs).toHaveBeenCalled(), { timeout: 1600 });
    expect(bootstrap).toHaveBeenCalledTimes(1);
  });

  it('opens the persisted transcript from a completed job', async () => {
    await ready();
    await fireEvent.click(screen.getByRole('button', { name: /Jobs/ }));
    await fireEvent.click(screen.getByRole('button', { name: 'Open transcript' }));
    expect(await screen.findByRole('heading', { name: 'Transcript workspace' })).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: 'Caption pacing breakdown' })).toBeInTheDocument();
  });

  it('switches AI Studio between Copy Prompt and BYOK without persisting a key in UI state', async () => {
    await ready();
    await fireEvent.click(screen.getByRole('button', { name: /AI Studio/ }));
    expect(screen.queryByLabelText('API key')).not.toBeInTheDocument();
    await fireEvent.click(screen.getByRole('button', { name: /BYOK/ }));
    expect(screen.getByTestId('byok-fields')).toBeInTheDocument();
    expect(screen.getByLabelText('API key')).toHaveAttribute('autocomplete', 'off');
  });

  it('filters and sorts research results', async () => {
    await ready();
    await fireEvent.click(screen.getByRole('button', { name: /Research/ }));
    const sort = screen.getByLabelText('Research sort');
    await fireEvent.change(sort, { target: { value: 'date' } });
    expect(screen.getAllByTestId(/research-r-/)[0]).toHaveAttribute('data-testid', 'research-r-2');
    const filter = screen.getByLabelText('Filter research');
    await fireEvent.input(filter, { target: { value: 'caption pacing' } });
    expect(screen.getByTestId('research-r-4')).toBeInTheDocument();
    expect(screen.queryByTestId('research-r-1')).not.toBeInTheDocument();
  });

  it('searches timestamped transcript segments and jumps back to full context', async () => {
    await ready();
    await fireEvent.click(screen.getByRole('button', { name: /Transcript/ }));
    const search = screen.getByLabelText('Search transcript');
    await fireEvent.input(search, { target: { value: 'النتيجة' } });
    expect(screen.getByText(/ابدأ بالنتيجة/)).toBeInTheDocument();
    expect(screen.queryByText(/أول ثلاث ثواني/)).not.toBeInTheDocument();
    await fireEvent.click(screen.getByRole('button', { name: 'Jump to 00:07' }));
    expect(screen.getByText(/أول ثلاث ثواني/)).toBeInTheDocument();
  });

  it('renders Arabic transcript content with RTL direction', async () => {
    await ready();
    await fireEvent.click(screen.getByRole('button', { name: /Transcript/ }));
    expect(screen.getByTestId('transcript-content')).toHaveAttribute('dir', 'rtl');
  });

  it('opens transcript entries from the library', async () => {
    await ready();
    await fireEvent.click(screen.getByRole('button', { name: /Library/ }));
    await fireEvent.click(screen.getByRole('button', { name: /Open Transcript: Hook breakdown/ }));
    expect(await screen.findByRole('heading', { name: 'Transcript workspace' })).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: 'Hook breakdown — Arabic sample' })).toBeInTheDocument();
  });

  it('uses global search to open a local transcript', async () => {
    await ready();
    const search = screen.getByLabelText('Search workspace');
    await fireEvent.input(search, { target: { value: 'caption pacing' } });
    const result = await screen.findByRole('button', { name: /Caption pacing breakdown.*Transcript/ });
    await fireEvent.click(result);
    expect(await screen.findByRole('heading', { name: 'Transcript workspace' })).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: 'Caption pacing breakdown' })).toBeInTheDocument();
  });

  it('selects and persists a Rust-backed output directory', async () => {
    const api = createMockClient();
    const chooseOutputDirectory = vi.spyOn(api, 'chooseOutputDirectory');
    const saveSettings = vi.spyOn(api, 'saveSettings');
    await ready(api);
    await fireEvent.click(screen.getByRole('button', { name: /Settings/ }));
    await fireEvent.click(screen.getByRole('button', { name: 'Choose output folder' }));
    await waitFor(() => expect(chooseOutputDirectory).toHaveBeenCalledTimes(1));
    expect(screen.getByText('/mock/scriptotar-output')).toBeInTheDocument();
    await fireEvent.click(screen.getByRole('button', { name: 'Save changes' }));
    await waitFor(() => expect(saveSettings).toHaveBeenCalledWith(expect.objectContaining({ outputDirectory: '/mock/scriptotar-output' })));
  });

  it('persists and reapplies the system appearance choice', async () => {
    await ready();
    await fireEvent.click(screen.getByRole('button', { name: /Settings/ }));
    await fireEvent.change(screen.getByLabelText('Theme'), { target: { value: 'system' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Save changes' }));
    await waitFor(() => expect(window.localStorage.getItem(APPEARANCE_STORAGE_KEY)).toBe('system'));
    expect(document.documentElement.dataset.theme).toBe('system');
  });

  it('shows an inspectable legacy migration result', async () => {
    const api = createMockClient();
    const importLegacyData = vi.spyOn(api, 'importLegacyData');
    await ready(api);
    await fireEvent.click(screen.getByRole('button', { name: /Settings/ }));
    await fireEvent.click(screen.getByRole('button', { name: 'Import legacy data' }));
    await waitFor(() => expect(importLegacyData).toHaveBeenCalledTimes(1));
    expect(screen.getByText(/Legacy import completed:/)).toHaveTextContent('1 transcripts');
    expect(screen.getByText(/Legacy import completed:/)).toHaveTextContent('Backup:');
  });

  it('shows a recoverable error state when bootstrap fails', async () => {
    render(App, { props: { api: createMockClient({ bootstrap: async () => { throw new Error('Database unavailable'); } }) } });
    const alert = await screen.findByRole('alert');
    expect(alert).toHaveTextContent('Database unavailable');
    expect(screen.getByRole('button', { name: 'Try again' })).toBeInTheDocument();
  });
});
