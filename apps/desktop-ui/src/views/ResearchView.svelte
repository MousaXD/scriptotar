<script lang="ts">
  import type { ResearchItem } from '../types';
  import EmptyState from '../components/EmptyState.svelte';
  export let items: ResearchItem[];
  export let onQueue: (ids: string[]) => Promise<void> | void;
  export let onScan: (url: string, limit: number) => Promise<void> | void;

  let query = '';
  let platform = 'All';
  let sort: 'views' | 'likes' | 'comments' | 'date' = 'views';
  let selected = new Set<string>();
  let profileUrl = '';
  let limit = 25;
  let scanning = false;

  $: filtered = items
    .filter((item) => platform === 'All' || item.platform === platform)
    .filter((item) => `${item.creator} ${item.title}`.toLowerCase().includes(query.toLowerCase()))
    .sort((a, b) => {
      if (sort === 'date') return (b.publishedAt || '').localeCompare(a.publishedAt || '');
      return (b[sort] || 0) - (a[sort] || 0);
    });

  function toggle(id: string) {
    selected = new Set(selected);
    selected.has(id) ? selected.delete(id) : selected.add(id);
  }
  async function scan() {
    if (!profileUrl.trim()) return;
    scanning = true;
    try { await onScan(profileUrl.trim(), limit); } finally { scanning = false; }
  }
</script>
<section class="view-head"><div><span class="eyebrow">Creator intelligence</span><h1>Research</h1><p>Scan public creator profiles, compare performance signals, then queue only the media worth transcribing.</p></div></section>
<section class="research-capture panel">
  <label><span>Creator / profile URL</span><input bind:value={profileUrl} placeholder="https://…" aria-label="Creator profile URL" /></label>
  <label class="small-field"><span>Limit</span><select bind:value={limit}><option value={10}>10</option><option value={25}>25</option><option value={50}>50</option><option value={100}>100</option></select></label>
  <button class="button primary" disabled={scanning || !profileUrl.trim()} on:click={scan}>{scanning ? 'Scanning…' : 'Scan profile'}</button>
  <button class="button secondary">Save watchlist</button>
</section>
<section class="panel research-panel">
  <div class="filter-bar">
    <label class="search-field"><span>⌕</span><input aria-label="Filter research" bind:value={query} placeholder="Filter title or creator…" /></label>
    <select aria-label="Platform filter" bind:value={platform}><option>All</option><option>TikTok</option><option>YouTube</option><option>Instagram</option></select>
    <label class="sort-control">Sort <select aria-label="Research sort" bind:value={sort}><option value="views">Views</option><option value="likes">Likes</option><option value="comments">Comments</option><option value="date">Newest</option></select></label>
    <button class="button secondary" disabled={selected.size === 0} on:click={() => onQueue([...selected])}>Queue selected ({selected.size})</button>
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
