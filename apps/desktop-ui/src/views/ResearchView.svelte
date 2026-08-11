<script lang="ts">
  import type { ResearchItem, WatchlistOperationalState, WatchlistStatus } from '../types';
  import EmptyState from '../components/EmptyState.svelte';
  export let items: ResearchItem[];
  export let watchlistStatuses: WatchlistStatus[] = [];
  export let onQueue: (ids: string[]) => Promise<void> | void;
  export let onScan: (url: string, limit: number) => Promise<void> | void;
  export let onSave: (url: string, limit: number) => Promise<void> | void;

  let query = '';
  let platform = 'All';
  let sort: 'views' | 'likes' | 'comments' | 'date' = 'views';
  let selected = new Set<string>();
  let profileUrl = '';
  let limit = 25;
  let scanning = false;
  let saving = false;
  let status = '';

  const stateLabel: Record<WatchlistOperationalState, string> = {
    healthy: 'Healthy',
    never_scanned: 'Never scanned',
    refreshing: 'Refreshing',
    retry_scheduled: 'Retry scheduled',
    failed: 'Failed'
  };

  $: filtered = items
    .filter((item) => platform === 'All' || item.platform === platform)
    .filter((item) => `${item.creator} ${item.title}`.toLowerCase().includes(query.toLowerCase()))
    .sort((a, b) => {
      if (sort === 'date') return (b.publishedAt || '').localeCompare(a.publishedAt || '');
      return (b[sort] || 0) - (a[sort] || 0);
    });

  function displayTime(value?: string) {
    if (!value) return '—';
    const parsed = new Date(value);
    return Number.isNaN(parsed.getTime()) ? value : parsed.toLocaleString();
  }

  function toggle(id: string) {
    selected = new Set(selected);
    selected.has(id) ? selected.delete(id) : selected.add(id);
  }
  async function scan() {
    if (!profileUrl.trim()) return;
    scanning = true;
    status = '';
    try { await onScan(profileUrl.trim(), limit); status = 'Research scan completed.'; }
    catch (error) { status = `Research scan unavailable: ${error instanceof Error ? error.message : String(error)}`; }
    finally { scanning = false; }
  }
  async function save() {
    if (!profileUrl.trim()) return;
    saving = true;
    status = '';
    try { await onSave(profileUrl.trim(), limit); status = 'Watchlist saved locally for this project.'; }
    catch (error) { status = `Watchlist save failed: ${error instanceof Error ? error.message : String(error)}`; }
    finally { saving = false; }
  }
  async function queueSelected() {
    status = '';
    try { await onQueue([...selected]); status = 'Selected media queued.'; }
    catch (error) { status = `Queue unavailable: ${error instanceof Error ? error.message : String(error)}`; }
  }
</script>
<section class="view-head"><div><span class="eyebrow">Creator intelligence</span><h1>Research</h1><p>Scan public creator profiles, compare performance signals, then queue only the media worth transcribing.</p></div></section>
<section class="research-capture panel">
  <label><span>Creator / profile URL</span><input bind:value={profileUrl} placeholder="https://…" aria-label="Creator profile URL" /></label>
  <label class="small-field"><span>Limit</span><select bind:value={limit}><option value={10}>10</option><option value={25}>25</option><option value={50}>50</option><option value={100}>100</option></select></label>
  <button class="button primary" disabled={scanning || !profileUrl.trim()} on:click={scan}>{scanning ? 'Scanning…' : 'Scan profile'}</button>
  <button class="button secondary" disabled={saving || !profileUrl.trim()} on:click={save}>{saving ? 'Saving…' : 'Save watchlist'}</button>
</section>
{#if status}<p class="status-copy" role="status">{status}</p>{/if}

<section class="panel watch-health" aria-labelledby="watch-health-title">
  <div class="watch-health-head">
    <div><span class="eyebrow">Background refresh</span><h2 id="watch-health-title">Watchlist health</h2></div>
    <small>Failures and retry timing are stored locally, including across restarts.</small>
  </div>
  {#if watchlistStatuses.length === 0}
    <p class="watch-empty">No saved watchlists in this project yet.</p>
  {:else}
    <div class="watch-health-grid">
      {#each watchlistStatuses as watchlist (watchlist.watchlistId)}
        <article class={`watch-health-card state-${watchlist.state}`} data-testid={`watchlist-status-${watchlist.watchlistId}`}>
          <div class="watch-health-title-row">
            <strong>{watchlist.label}</strong>
            <span class="watch-state">{stateLabel[watchlist.state]}</span>
          </div>
          <dl>
            <div><dt>Last attempt</dt><dd>{displayTime(watchlist.lastAttemptAt)}</dd></div>
            <div><dt>Last success</dt><dd>{displayTime(watchlist.lastSuccessfulScanAt)}</dd></div>
            {#if watchlist.nextRetryAt}<div><dt>Next retry</dt><dd>{displayTime(watchlist.nextRetryAt)}</dd></div>{/if}
          </dl>
          {#if watchlist.lastError}
            <p class="watch-error">{watchlist.lastError}</p>
          {:else if watchlist.state === 'never_scanned'}
            <p class="watch-note">This creator has not completed a watchlist scan yet.</p>
          {:else if watchlist.state === 'refreshing'}
            <p class="watch-note">A background creator scan is currently running.</p>
          {/if}
        </article>
      {/each}
    </div>
  {/if}
</section>

<section class="panel research-panel">
  <div class="filter-bar">
    <label class="search-field"><span>⌕</span><input aria-label="Filter research" bind:value={query} placeholder="Filter title or creator…" /></label>
    <select aria-label="Platform filter" bind:value={platform}><option>All</option><option>TikTok</option><option>YouTube</option><option>Instagram</option></select>
    <label class="sort-control">Sort <select aria-label="Research sort" bind:value={sort}><option value="views">Views</option><option value="likes">Likes</option><option value="comments">Comments</option><option value="date">Newest</option></select></label>
    <button class="button secondary" disabled={selected.size === 0} on:click={queueSelected}>Queue selected ({selected.size})</button>
  </div>
  {#if filtered.length === 0}
    <EmptyState title="No matching research" message="Change the filters or scan another creator profile." />
  {:else}
    <div class="research-table" role="table" aria-label="Research results">
      <div class="research-row table-head" role="row"><span></span><span>Media</span><span>Views</span><span>Likes</span><span>Comments</span><span>Date</span><span>Duration</span></div>
      {#each filtered as item}
        <div class="research-row" role="row" data-testid={`research-${item.id}`}>
          <label class="check-only"><input type="checkbox" checked={selected.has(item.id)} on:change={() => toggle(item.id)} aria-label={`Select ${item.title}`} /></label>
          <div class="media-cell"><div class="thumbnail-placeholder">{item.platform.slice(0,1)}</div><div><strong>{item.title}</strong><small>{item.creator} · {item.platform}</small></div></div>
          <strong>{item.views ? Intl.NumberFormat('en',{notation:'compact'}).format(item.views) : '—'}</strong>
          <span>{item.likes ? Intl.NumberFormat('en',{notation:'compact'}).format(item.likes) : '—'}</span>
          <span>{item.comments ? Intl.NumberFormat('en',{notation:'compact'}).format(item.comments) : '—'}</span>
          <span>{item.publishedAt || '—'}</span><span>{item.durationSeconds ? `${item.durationSeconds}s` : '—'}</span>
        </div>
      {/each}
    </div>
  {/if}
</section>

<style>
  .watch-health { display: grid; gap: 14px; }
  .watch-health-head { display: flex; gap: 18px; align-items: end; justify-content: space-between; }
  .watch-health-head h2 { margin: 3px 0 0; }
  .watch-health-head small, .watch-empty { color: var(--muted); }
  .watch-health-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(240px, 1fr)); gap: 10px; }
  .watch-health-card { display: grid; gap: 10px; padding: 13px; border: 1px solid var(--border); border-radius: 10px; background: rgba(255,255,255,.018); }
  .watch-health-title-row { display: flex; gap: 10px; justify-content: space-between; align-items: center; }
  .watch-state { font-size: 11px; font-weight: 650; padding: 4px 7px; border: 1px solid var(--border); border-radius: 999px; }
  .state-healthy .watch-state { color: var(--text); }
  .state-refreshing .watch-state, .state-retry_scheduled .watch-state { color: var(--accent); }
  .state-failed .watch-state { color: var(--danger, #ff8c8c); }
  dl { display: grid; gap: 5px; margin: 0; }
  dl div { display: flex; justify-content: space-between; gap: 12px; font-size: 11px; }
  dt { color: var(--muted); }
  dd { margin: 0; text-align: right; }
  .watch-error { margin: 0; padding: 9px 10px; border-radius: 8px; background: rgba(255, 92, 92, .08); font-size: 12px; line-height: 1.45; }
  .watch-note { margin: 0; color: var(--muted); font-size: 12px; line-height: 1.45; }
  @media (max-width: 720px) { .watch-health-head { align-items: start; flex-direction: column; } }
</style>
