import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import ResearchView from './ResearchView.svelte';
import LibraryView from './LibraryView.svelte';
import { setLocale } from '../i18n';
import type { LibraryItem, ResearchItem, WatchlistStatus } from '../types';

beforeEach(() => {
  setLocale('en');
});

function researchItems(count: number): ResearchItem[] {
  return Array.from({ length: count }, (_, index) => ({
    id: 'r-' + index,
    creatorId: 'c-' + index,
    creator: index % 2 === 0 ? 'Frame Studio' : 'مبدع عربي',
    title: index === 0 ? 'An exceptionally long research title '.repeat(12) : 'Video ' + index,
    platform: index % 2 === 0 ? 'YouTube' : 'TikTok',
    sourceUrl: 'https://example.org/watch/' + index,
    views: count - index,
    likes: index,
    comments: 12,
    publishedAt: '2026-10-' + String((index % 28) + 1).padStart(2, '0')
  }));
}

function libraryItems(count: number): LibraryItem[] {
  const kinds: LibraryItem['kind'][] = ['Transcript', 'Research', 'AI run', 'Project', 'Creator'];
  return Array.from({ length: count }, (_, index) => ({
    id: 'item:' + index,
    projectId: 'project-a',
    kind: kinds[index % kinds.length],
    title: index === 0 ? 'A very long title '.repeat(30) : 'Artifact ' + index,
    subtitle: 'Source creator ' + index,
    platform: index % 2 === 0 ? 'YouTube' : 'Local',
    metric: (index * 100).toString() + ' views',
    date: '2026-10-01'
  }));
}

function mountResearch(items: ResearchItem[], options: { onQueue?: (ids: string[]) => Promise<void> | void; onScan?: (url: string, limit: number) => Promise<void> | void; watchlistStatuses?: WatchlistStatus[] } = {}) {
  return render(ResearchView, {
    props: {
      items,
      watchlistStatuses: options.watchlistStatuses || [],
      onQueue: options.onQueue || vi.fn(),
      onScan: options.onScan || vi.fn(),
      onSave: vi.fn()
    }
  });
}

describe('Research workspace', () => {
  it('bounds large result lists to 50 rows while retaining filtering, sorting and pagination', async () => {
    mountResearch(researchItems(121));
    expect(screen.getAllByTestId(/^research-r-/)).toHaveLength(50);
    const pages = screen.getByRole('navigation', { name: 'Research pages' });
    await fireEvent.click(within(pages).getByRole('button', { name: 'Next' }));
    expect(screen.getAllByTestId(/^research-r-/)).toHaveLength(50);
    expect(screen.queryByTestId('research-r-0')).not.toBeInTheDocument();
    await fireEvent.click(within(pages).getByRole('button', { name: 'Next' }));
    expect(screen.getAllByTestId(/^research-r-/)).toHaveLength(21);
    await fireEvent.input(screen.getByRole('textbox', { name: 'Filter research' }), { target: { value: 'Video 17' } });
    expect(screen.getByTestId('research-r-17')).toBeInTheDocument();
    expect(screen.queryByRole('navigation', { name: 'Research pages' })).not.toBeInTheDocument();
    await fireEvent.change(screen.getByRole('combobox', { name: 'Platform filter' }), { target: { value: 'YouTube' } });
    expect(screen.queryByTestId('research-r-17')).not.toBeInTheDocument();
  });

  it('selects the current page without losing selections on other pages and queues only once', async () => {
    const onQueue = vi.fn().mockResolvedValue(undefined);
    mountResearch(researchItems(62), { onQueue });
    await fireEvent.click(screen.getByRole('checkbox', { name: 'Select page' }));
    expect(screen.getByText('Selected: 50')).toBeInTheDocument();
    await fireEvent.click(screen.getByRole('button', { name: 'Next' }));
    await fireEvent.click(screen.getByRole('checkbox', { name: 'Select Video 50' }));
    expect(screen.getByText('Selected: 51')).toBeInTheDocument();
    await fireEvent.click(screen.getByRole('button', { name: 'Queue selected (51)' }));
    await waitFor(() => expect(onQueue).toHaveBeenCalledTimes(1));
    expect(onQueue.mock.calls[0][0]).toHaveLength(51);
    await waitFor(() => expect(screen.getByText('Selected: 0')).toBeInTheDocument());
  });

  it('batches selection across five pages below the backend 200-ID limit', async () => {
    const onQueue = vi.fn().mockResolvedValue(undefined);
    mountResearch(researchItems(201), { onQueue });
    for (let page = 0; page < 5; page++) {
      await fireEvent.click(screen.getByRole('checkbox', { name: 'Select page' }));
      if (page < 4) await fireEvent.click(screen.getByRole('button', { name: 'Next' }));
    }
    expect(screen.getByText('Selected: 201')).toBeInTheDocument();
    await fireEvent.click(screen.getByRole('button', { name: 'Queue selected (201)' }));
    await waitFor(() => expect(onQueue).toHaveBeenCalledTimes(2));
    expect(onQueue.mock.calls[0][0]).toHaveLength(200);
    expect(onQueue.mock.calls[1][0]).toHaveLength(1);
    expect(new Set(onQueue.mock.calls.flatMap(([ids]) => ids)).size).toBe(201);
    await waitFor(() => expect(screen.getByText('Selected: 0')).toBeInTheDocument());
  });

  it('retains unsubmitted selections and exposes partial failure after the first batch', async () => {
    const onQueue = vi.fn()
      .mockResolvedValueOnce(undefined)
      .mockRejectedValueOnce(new Error('Queue disk unavailable'));
    mountResearch(researchItems(201), { onQueue });
    for (let page = 0; page < 5; page++) {
      await fireEvent.click(screen.getByRole('checkbox', { name: 'Select page' }));
      if (page < 4) await fireEvent.click(screen.getByRole('button', { name: 'Next' }));
    }
    await fireEvent.click(screen.getByRole('button', { name: 'Queue selected (201)' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('Queue disk unavailable');
    expect(screen.getByRole('alert')).toHaveTextContent('200 submitted; remaining selection preserved');
    expect(screen.getByText('Selected: 1')).toBeInTheDocument();
    expect(onQueue).toHaveBeenCalledTimes(2);
    expect(screen.getByRole('button', { name: 'Queue selected (1)' })).toBeEnabled();
  });

  it('submits profile text to backend validation instead of silently blocking non-scheme input', async () => {
    const onScan = vi.fn().mockResolvedValue(undefined);
    mountResearch([], { onScan });
    const field = screen.getByRole('textbox', { name: 'Creator profile URL' });
    await fireEvent.input(field, { target: { value: 'youtube.com/@creator' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Scan profile' }));
    await waitFor(() => expect(onScan).toHaveBeenCalledWith('youtube.com/@creator', 25));
  });

  it('keeps already queued media visible but excludes them from actionable selections', async () => {
    const source = researchItems(3);
    source[0].queued = true;
    const onQueue = vi.fn().mockResolvedValue(undefined);
    mountResearch(source, { onQueue });
    expect(screen.getByRole('checkbox', { name: /^Select An exceptionally long research title/ })).toBeDisabled();
    await fireEvent.click(screen.getByRole('checkbox', { name: 'Select page' }));
    expect(screen.getByText('Selected: 2')).toBeInTheDocument();
    await fireEvent.click(screen.getByRole('button', { name: 'Queue selected (2)' }));
    await waitFor(() => expect(onQueue).toHaveBeenCalledWith(['r-1', 'r-2']));
  });

  it('keeps watchlist failures and retry timing visible while grouping normal diagnostics', async () => {
    mountResearch([], {
      watchlistStatuses: [{
        watchlistId: 'watch-1', projectId: 'p1', label: 'Creator A',
        state: 'retry_scheduled', lastError: 'Provider authentication expired',
        nextRetryAt: '2026-10-02T12:00:00Z', lastAttemptAt: '2026-10-02T11:00:00Z'
      }]
    });
    const watchlist = screen.getByTestId('watchlist-status-watch-1');
    expect(watchlist).toHaveTextContent('Retry scheduled');
    expect(watchlist).toHaveTextContent('Provider authentication expired');
    expect(watchlist).toHaveTextContent('Next retry');
    expect(screen.getByRole('link', { name: '1 watchlist needs attention' })).toHaveAttribute('href', '#watch-health-title');
    const details = within(watchlist).getByText('Refresh details');
    expect(details.closest('details')).not.toHaveAttribute('open');
    await fireEvent.click(details);
    expect(details.closest('details')).toHaveAttribute('open');
  });

  it('replaces broken thumbnails with a visible local fallback', async () => {
    const source = researchItems(1);
    source[0].thumbnail = 'https://example.org/broken-image.png';
    mountResearch(source);
    const image = document.querySelector('.research-thumbnail');
    expect(image).not.toBeNull();
    await fireEvent.error(image!);
    expect(document.querySelector('.research-thumbnail')).toBeNull();
    expect(screen.getByTestId('research-r-0')).toHaveTextContent('YouTube');
  });

  it('handles scan failures without silently swallowing backend errors', async () => {
    const onScan = vi.fn().mockRejectedValue(new Error('Provider unavailable'));
    mountResearch([], { onScan });
    await fireEvent.input(screen.getByRole('textbox', { name: 'Creator profile URL' }), {
      target: { value: 'https://youtube.com/@creator' }
    });
    await fireEvent.click(screen.getByRole('button', { name: 'Scan profile' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('Provider unavailable');
    expect(onScan).toHaveBeenCalledWith('https://youtube.com/@creator', 25);
    expect(screen.getByRole('button', { name: 'Scan profile' })).toBeEnabled();
  });
});

describe('Library workspace', () => {
  it('renders bounded result pages for thousands of artifacts and opens the selected item', async () => {
    const onOpen = vi.fn();
    render(LibraryView, { props: { items: libraryItems(1010), onOpen } });
    const rows = screen.getAllByRole('button', { name: /Open (Transcript|Research|AI run|Project|Creator):/ });
    expect(rows).toHaveLength(75);
    const pages = screen.getByRole('navigation', { name: 'Library pages' });
    await fireEvent.click(within(pages).getByRole('button', { name: 'Next' }));
    const button = screen.getByRole('button', { name: 'Open Transcript: Artifact 75' });
    await fireEvent.click(button);
    expect(onOpen).toHaveBeenCalledWith(expect.objectContaining({ id: 'item:75' }));
    expect(screen.queryByRole('button', { name: /Open Transcript: A very long title/ })).not.toBeInTheDocument();
  });

  it('filters by type, supports text search and preserves the correct artifact identity', async () => {
    render(LibraryView, { props: { items: libraryItems(200), onOpen: vi.fn() } });
    const kindGroup = screen.getByRole('group', { name: 'Library kind' });
    await fireEvent.click(within(kindGroup).getByRole('button', { name: /Research/ }));
    expect(screen.queryByRole('button', { name: /^Open Transcript:/ })).not.toBeInTheDocument();
    expect(screen.getAllByRole('button', { name: /^Open Research:/ })).toHaveLength(40);
    await fireEvent.input(screen.getByRole('textbox', { name: 'Search library' }), { target: { value: 'Source creator 6' } });
    expect(screen.getByRole('button', { name: 'Open Research: Artifact 6' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Open Research: Artifact 61' })).toBeInTheDocument();
    await fireEvent.change(screen.getByRole('combobox', { name: 'Library sort' }), { target: { value: 'title' } });
    expect(screen.getByRole('button', { name: 'Open Research: Artifact 6' })).toBeInTheDocument();
  });

  it('translates navigation controls in Arabic while preserving content and opening failures', async () => {
    const onOpen = vi.fn().mockRejectedValue(new Error('Artifact offline'));
    render(LibraryView, { props: { items: [{
      id: 'transcript:1', kind: 'Transcript', title: 'Original creator title',
      subtitle: 'Author name', projectId: 'p1', date: 'Now'
    }], onOpen } });
    setLocale('ar');
    expect(await screen.findByRole('heading', { name: 'المكتبة' })).toBeInTheDocument();
    const button = screen.getByRole('button', { name: 'فتح نص مفرغ: Original creator title' });
    expect(button).toHaveTextContent('Original creator title');
    expect(screen.getByText('محلي')).toBeInTheDocument();
    await fireEvent.click(button);
    expect(await screen.findByRole('alert')).toHaveTextContent('Artifact offline');
  });
});
