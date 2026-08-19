import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import App from './App.svelte';
import { APPEARANCE_STORAGE_KEY } from './appearance';
import { createMockClient, mockBootstrap } from './api/mockClient';
import type { MigrationStatus, WatchlistStatus } from './types';

beforeEach(() => {
  window.localStorage.clear();
  delete document.documentElement.dataset.theme;
});

async function ready(api = createMockClient()) {
  render(App, { props: { api } });
  await screen.findByRole('heading', { name: 'Creator Lab' });
  return api;
}

function bootstrapWithMigration(migrationStatus: MigrationStatus) {
  return {
    ...structuredClone(mockBootstrap),
    migrationStatus: structuredClone(migrationStatus)
  };
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

  it('refreshes active jobs from backend events without repeatedly bootstrapping the full workspace', async () => {
    let notify: ((jobId: string) => void) | undefined;
    const api = createMockClient({
      subscribeJobChanges: async (listener) => {
        notify = listener;
        return () => {};
      }
    });
    const bootstrap = vi.spyOn(api, 'bootstrap');
    const listJobs = vi.spyOn(api, 'listJobs');
    await ready(api);
    notify?.('j-1');
    await waitFor(() => expect(listJobs).toHaveBeenCalled());
    expect(bootstrap).toHaveBeenCalledTimes(1);
  });

  it('opens the persisted transcript from a completed job', async () => {
    const api = createMockClient();
    const getTranscript = vi.spyOn(api, 'getTranscript');
    await ready(api);
    await fireEvent.click(screen.getByRole('button', { name: /Jobs/ }));
    await fireEvent.click(screen.getByRole('button', { name: 'Open transcript' }));
    expect(await screen.findByRole('heading', { name: 'Transcript workspace' })).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: 'Caption pacing breakdown' })).toBeInTheDocument();
    await waitFor(() => expect(getTranscript).toHaveBeenCalledWith('t-en'));
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

  it('surfaces persisted watchlist retry state and safe failure detail', async () => {
    await ready();
    await fireEvent.click(screen.getByRole('button', { name: /Research/ }));
    const card = screen.getByTestId('watchlist-status-w-2');
    expect(card).toHaveTextContent('Retry scheduled');
    expect(card).toHaveTextContent('Creator refresh needs valid browser authentication or provider access.');
    expect(card).toHaveTextContent('Next retry');
  });

  it('renders never-scanned, refreshing, healthy, failed, retry, and recovered watchlists', async () => {
    const watchlistStatuses: WatchlistStatus[] = [
      {
        watchlistId: 'never', projectId: 'p-creator-lab', label: 'Never creator', state: 'never_scanned'
      },
      {
        watchlistId: 'refreshing', projectId: 'p-creator-lab', label: 'Refreshing creator', state: 'refreshing',
        lastAttemptAt: '2026-08-10T10:00:00Z'
      },
      {
        watchlistId: 'healthy', projectId: 'p-creator-lab', label: 'Healthy creator', state: 'healthy',
        lastAttemptAt: '2026-08-10T10:00:00Z', lastSuccessfulScanAt: '2026-08-10T10:00:01Z'
      },
      {
        watchlistId: 'failed', projectId: 'p-creator-lab', label: 'Failed creator', state: 'failed',
        lastAttemptAt: '2026-08-10T10:00:00Z', lastError: 'Creator refresh failed safely.'
      },
      {
        watchlistId: 'retry', projectId: 'p-creator-lab', label: 'Retry creator', state: 'retry_scheduled',
        lastAttemptAt: '2026-08-10T10:00:00Z', lastError: 'Creator refresh could not reach the provider.',
        nextRetryAt: '2026-08-10T10:30:00Z'
      },
      {
        watchlistId: 'recovered', projectId: 'p-creator-lab', label: 'Recovered creator', state: 'healthy',
        lastAttemptAt: '2026-08-10T10:30:00Z', lastSuccessfulScanAt: '2026-08-10T10:30:01Z'
      }
    ];
    const api = createMockClient({
      bootstrap: async () => ({ ...structuredClone(mockBootstrap), watchlistStatuses: structuredClone(watchlistStatuses) }),
      getWatchlistStatuses: async () => structuredClone(watchlistStatuses)
    });
    await ready(api);
    await fireEvent.click(screen.getByRole('button', { name: /Research/ }));

    expect(screen.getByTestId('watchlist-status-never')).toHaveTextContent('Never scanned');
    expect(screen.getByTestId('watchlist-status-refreshing')).toHaveTextContent('Refreshing');
    expect(screen.getByTestId('watchlist-status-healthy')).toHaveTextContent('Healthy');
    expect(screen.getByTestId('watchlist-status-failed')).toHaveTextContent('Failed');
    expect(screen.getByTestId('watchlist-status-failed')).toHaveTextContent('Creator refresh failed safely.');
    expect(screen.getByTestId('watchlist-status-retry')).toHaveTextContent('Retry scheduled');
    expect(screen.getByTestId('watchlist-status-retry')).toHaveTextContent('Next retry');
    expect(screen.getByTestId('watchlist-status-recovered')).toHaveTextContent('Healthy');
    expect(screen.getByTestId('watchlist-status-recovered')).not.toHaveTextContent('failed');
  });

  it('disables manual scan action while a refresh is in flight', async () => {
    const api = createMockClient();
    let finishScan: (() => void) | undefined;
    const scanPending = new Promise<void>((resolve) => { finishScan = resolve; });
    vi.spyOn(api, 'scanCreator').mockReturnValue(scanPending);
    await ready(api);
    await fireEvent.click(screen.getByRole('button', { name: /Research/ }));
    await fireEvent.input(screen.getByLabelText('Creator profile URL'), {
      target: { value: 'https://www.youtube.com/@creator' }
    });
    const scan = screen.getByRole('button', { name: 'Scan profile' });
    await fireEvent.click(scan);
    await waitFor(() => expect(screen.getByRole('button', { name: 'Scanning…' })).toBeDisabled());
    finishScan?.();
    await waitFor(() => expect(screen.getByRole('button', { name: 'Scan profile' })).toBeEnabled());
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

  it('delegates transcript-body workspace search to the backend index', async () => {
    const api = createMockClient();
    const searchTranscripts = vi.spyOn(api, 'searchTranscripts');
    await ready(api);
    const search = screen.getByLabelText('Search workspace');
    await fireEvent.input(search, { target: { value: 'visual payoff' } });
    await waitFor(() => expect(searchTranscripts).toHaveBeenCalledWith('visual payoff', 8));
    expect(await screen.findByRole('button', { name: /Caption pacing breakdown.*Transcript/ })).toBeInTheDocument();
  });

  it('supports Arabic transcript-body workspace search through the backend index', async () => {
    const api = createMockClient();
    const searchTranscripts = vi.spyOn(api, 'searchTranscripts');
    await ready(api);
    const search = screen.getByLabelText('Search workspace');
    await fireEvent.input(search, { target: { value: 'النتيجة' } });
    await waitFor(() => expect(searchTranscripts).toHaveBeenCalledWith('النتيجة', 8));
    expect(await screen.findByRole('button', { name: /Hook breakdown — Arabic sample.*Transcript/ })).toBeInTheDocument();
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

  it('shows structured migration discovery status and a recovery action', async () => {
    const api = createMockClient();
    const retryLegacyMigration = vi.spyOn(api, 'retryLegacyMigration');
    await ready(api);
    await fireEvent.click(screen.getByRole('button', { name: /Settings/ }));
    const migration = screen.getByTestId('migration-status');
    expect(migration).toHaveTextContent('No legacy database found');
    expect(migration).toHaveTextContent('No Scriptotar Classic database was found');
    await fireEvent.click(screen.getByRole('button', { name: 'Retry migration discovery' }));
    await waitFor(() => expect(retryLegacyMigration).toHaveBeenCalledTimes(1));
  });

  it('requires an explicit opaque choice when multiple legacy databases are found', async () => {
    const api = createMockClient();
    vi.spyOn(api, 'bootstrap').mockResolvedValue({
      ...structuredClone(mockBootstrap),
      migrationStatus: {
        state: 'requires_choice',
        message: 'Multiple legacy databases were found. Choose one safely.',
        candidates: [
          { id: 'candidate-11111111-1111-5111-8111-111111111111', label: 'Scriptotar Classic database (option 1)' },
          { id: 'candidate-22222222-2222-5222-8222-222222222222', label: 'WeSamBoss database (option 2)' }
        ]
      }
    });
    const selectCandidate = vi.spyOn(api, 'selectLegacyMigrationCandidate');
    await ready(api);
    await fireEvent.click(screen.getByRole('button', { name: /Settings/ }));
    const migration = screen.getByTestId('migration-status');
    expect(migration).toHaveTextContent('Choice required');
    expect(migration).toHaveTextContent('Multiple legacy databases were found');
    await fireEvent.click(screen.getByRole('button', { name: 'WeSamBoss database (option 2)' }));
    await waitFor(() => expect(selectCandidate).toHaveBeenCalledWith('candidate-22222222-2222-5222-8222-222222222222'));
  });

  it('shows invalid legacy candidate state with a safe retry path', async () => {
    const migrationStatus: MigrationStatus = {
      state: 'invalid_db',
      message: 'A discovered legacy database is not a safe, readable SQLite file.',
      candidates: []
    };
    const api = createMockClient({ bootstrap: async () => bootstrapWithMigration(migrationStatus) });
    await ready(api);
    await fireEvent.click(screen.getByRole('button', { name: /Settings/ }));
    const migration = screen.getByTestId('migration-status');
    expect(migration).toHaveTextContent('Invalid legacy database');
    expect(screen.getByRole('button', { name: 'Retry migration discovery' })).toBeEnabled();
  });

  it('shows failed migration state and can retry discovery', async () => {
    let migrationStatus: MigrationStatus = {
      state: 'failed',
      message: 'Migration failed safely.',
      candidates: []
    };
    const api = createMockClient({
      bootstrap: async () => bootstrapWithMigration(migrationStatus),
      retryLegacyMigration: async () => {
        migrationStatus = {
          state: 'no_legacy_db',
          message: 'No Scriptotar Classic database was found in the standard legacy locations.',
          candidates: []
        };
        return structuredClone(migrationStatus);
      }
    });
    await ready(api);
    await fireEvent.click(screen.getByRole('button', { name: /Settings/ }));
    expect(screen.getByTestId('migration-status')).toHaveTextContent('Migration failed');
    await fireEvent.click(screen.getByRole('button', { name: 'Retry migration discovery' }));
    await waitFor(() => expect(screen.getByTestId('migration-status')).toHaveTextContent('No legacy database found'));
  });

  it('shows migration in progress without offering another migration action', async () => {
    const migrationStatus: MigrationStatus = {
      state: 'in_progress',
      message: 'Scriptotar is importing the prepared legacy snapshot.',
      candidates: []
    };
    const api = createMockClient({ bootstrap: async () => bootstrapWithMigration(migrationStatus) });
    await ready(api);
    await fireEvent.click(screen.getByRole('button', { name: /Settings/ }));
    const migration = screen.getByTestId('migration-status');
    expect(migration).toHaveAttribute('data-state', 'in_progress');
    expect(migration).toHaveTextContent('Importing');
    expect(screen.queryByRole('button', { name: /migration|import prepared|retry migration/i })).not.toBeInTheDocument();
  });

  it('imports a prepared snapshot and renders successful migration counts', async () => {
    const completed: MigrationStatus = {
      state: 'completed',
      message: 'Legacy migration completed.',
      candidates: [],
      report: {
        skipped: false,
        backup_path: '/mock/history.sqlite3.scriptotar-next.bak',
        projects: 1,
        jobs: 2,
        transcripts: 3,
        research_items: 4,
        watchlists: 5,
        ai_runs: 6
      }
    };
    let migrationStatus: MigrationStatus = {
      state: 'ready',
      message: 'A safe legacy database snapshot is prepared for import.',
      candidates: []
    };
    const api = createMockClient({
      bootstrap: async () => bootstrapWithMigration(migrationStatus),
      retryLegacyMigration: async () => {
        migrationStatus = structuredClone(completed);
        return structuredClone(completed);
      }
    });
    await ready(api);
    await fireEvent.click(screen.getByRole('button', { name: /Settings/ }));
    expect(screen.getByRole('button', { name: 'Import prepared snapshot' })).toBeEnabled();
    await fireEvent.click(screen.getByRole('button', { name: 'Import prepared snapshot' }));
    await waitFor(() => expect(screen.getByTestId('migration-status')).toHaveTextContent('Completed'));
    expect(screen.getByTestId('migration-status')).toHaveTextContent('Imported 1 projects, 2 jobs, 3 transcripts, 4 research items, 5 watchlists, and 6 AI runs.');
    expect(screen.queryByRole('button', { name: 'Import prepared snapshot' })).not.toBeInTheDocument();
  });

  it('shows a recoverable error state when bootstrap fails', async () => {
    render(App, { props: { api: createMockClient({ bootstrap: async () => { throw new Error('Database unavailable'); } }) } });
    const alert = await screen.findByRole('alert');
    expect(alert).toHaveTextContent('Database unavailable');
    expect(screen.getByRole('button', { name: 'Try again' })).toBeInTheDocument();
  });
});