<script lang="ts">
  import type { LibraryItem, LibraryKind } from '../types';
  import EmptyState from '../components/EmptyState.svelte';

  export let items: LibraryItem[];
  export let onOpen: (item: LibraryItem) => Promise<void> | void;

  let query = '';
  let kind: 'All' | LibraryKind = 'All';
  let sort: 'newest' | 'title' | 'type' = 'newest';
  let actionError = '';

  $: filtered = items
    .filter((item) => kind === 'All' || item.kind === kind)
    .filter((item) => `${item.title} ${item.subtitle} ${item.platform || ''} ${item.metric || ''}`.toLowerCase().includes(query.trim().toLowerCase()));
  $: visible = [...filtered].sort((a, b) => {
    if (sort === 'title') return a.title.localeCompare(b.title);
    if (sort === 'type') return a.kind.localeCompare(b.kind) || a.title.localeCompare(b.title);
    return items.indexOf(a) - items.indexOf(b);
  });

  async function open(item: LibraryItem) {
    actionError = '';
    try { await onOpen(item); }
    catch (cause) { actionError = cause instanceof Error ? cause.message : 'Could not open this library item.'; }
  }
</script>

<section class="view-head"><div><span class="eyebrow">Unified local index</span><h1>Library</h1><p>Browse the active project's transcripts, creator research, AI runs, projects, and creators, then open the related workspace directly.</p></div></section>
<section class="panel library-panel">
  <div class="filter-bar"><label class="search-field"><span aria-hidden="true">⌕</span><input aria-label="Search library" bind:value={query} placeholder="Search your local library…" /></label><select bind:value={kind} aria-label="Library kind"><option>All</option><option>Transcript</option><option>Research</option><option>AI run</option><option>Project</option><option>Creator</option></select><select bind:value={sort} aria-label="Library sort"><option value="newest">Newest first</option><option value="title">Title</option><option value="type">Type</option></select></div>
  <p class="library-count" aria-live="polite">{visible.length} {visible.length === 1 ? 'item' : 'items'}</p>
  {#if visible.length === 0}<EmptyState title="Nothing matches" message="Try a broader search or another library type." />{:else}
    <div class="library-list">{#each visible as item}<button aria-label={`Open ${item.kind}: ${item.title}`} on:click={() => open(item)}><span class="kind-icon" aria-hidden="true">{item.kind.slice(0,1)}</span><span class="library-copy"><strong>{item.title}</strong><small>{item.subtitle}</small></span><span class="library-meta">{item.platform || 'Local'}</span><span class="library-meta">{item.metric || '—'}</span><span class="library-date">{item.date}</span><span aria-hidden="true">›</span></button>{/each}</div>
  {/if}
</section>
{#if actionError}<p class="status-copy" role="alert">{actionError}</p>{/if}

<style>
  .library-count { margin: 9px 13px; color: var(--faint); font-size: 10px; }
  @media (max-width: 900px) {
    .filter-bar { flex-wrap: wrap; }
    .filter-bar .search-field { flex-basis: 100%; }
  }
</style>
