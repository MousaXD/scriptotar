<script lang="ts">
  import { translator } from '../i18n/translate';
  import type { LibraryItem, LibraryKind } from '../types';
  import EmptyState from '../components/EmptyState.svelte';

  export let items: LibraryItem[];
  export let onOpen: (item: LibraryItem) => Promise<void> | void;

  let query = '';
  let kind: 'All' | LibraryKind = 'All';
  let sort: 'newest' | 'title' | 'type' = 'newest';
  let actionError = '';

  const kinds: Array<'All' | LibraryKind> = ['All', 'Transcript', 'Research', 'AI run', 'Project', 'Creator'];

  $: filtered = items
    .filter((item) => kind === 'All' || item.kind === kind)
    .filter((item) => `${item.title} ${item.subtitle} ${item.platform || ''} ${item.metric || ''}`.toLowerCase().includes(query.trim().toLowerCase()));
  $: visible = [...filtered].sort((a, b) => {
    if (sort === 'title') return a.title.localeCompare(b.title);
    if (sort === 'type') return a.kind.localeCompare(b.kind) || a.title.localeCompare(b.title);
    return items.indexOf(a) - items.indexOf(b);
  });
  $: counts = kinds.reduce((result, current) => {
    result[current] = current === 'All' ? items.length : items.filter((item) => item.kind === current).length;
    return result;
  }, {} as Record<'All' | LibraryKind, number>);

  function kindLabel(value: LibraryKind) {
    if (value === 'Transcript') return $translator('library.kind.transcript');
    if (value === 'Research') return $translator('library.kind.research');
    if (value === 'AI run') return $translator('library.kind.ai');
    if (value === 'Project') return $translator('library.kind.project');
    return $translator('library.kind.creator');
  }

  async function open(item: LibraryItem) {
    actionError = '';
    try { await onOpen(item); }
    catch (cause) { actionError = cause instanceof Error ? cause.message : 'Could not open this library item.'; }
  }
</script>

<section class="view-head library-head">
  <div><span class="eyebrow">Unified local index</span><h1>Library</h1><p>Browse the active project's transcripts, creator research, AI runs, projects, and creators, then open the related workspace directly.</p></div>
  <span class="library-total" data-i18n-ignore><strong>{items.length}</strong> {$translator('library.localItems', { count: items.length }).replace(String(items.length), '').trim()}</span>
</section>

<section class="panel library-panel">
  <div class="library-toolbar">
    <label class="search-field library-search"><span aria-hidden="true">⌕</span><input aria-label="Search library" bind:value={query} placeholder="Search your local library…" /></label>
    <label class="sort-select"><span>Sort</span><select bind:value={sort} aria-label="Library sort"><option value="newest">Newest first</option><option value="title">Title</option><option value="type">Type</option></select></label>
  </div>

  <div class="kind-tabs" role="group" aria-label="Library kind">
    {#each kinds as itemKind}
      <button class:active={kind === itemKind} aria-pressed={kind === itemKind} on:click={() => kind = itemKind}>
        <span>{itemKind}</span><b>{counts[itemKind]}</b>
      </button>
    {/each}
  </div>

  <div class="library-result-bar"><span aria-live="polite">{visible.length} {visible.length === 1 ? 'item' : 'items'}</span>{#if query.trim()}<span data-i18n-ignore>{$translator('library.matching', { query: query.trim() })}</span>{/if}</div>

  {#if visible.length === 0}
    <EmptyState title="Nothing matches" message="Try a broader search or another library type." />
  {:else}
    <div class="library-list">
      <div class="library-columns" aria-hidden="true" data-i18n-ignore><span>{$translator('library.column.item')}</span><span>{$translator('library.column.source')}</span><span>{$translator('library.column.signal')}</span><span>{$translator('library.column.date')}</span></div>
      {#each visible as item}
        <button aria-label={`Open ${item.kind}: ${item.title}`} on:click={() => open(item)}>
          <span class={`kind-icon kind-${item.kind.toLowerCase().replaceAll(' ', '-')}`} aria-hidden="true">{item.kind.slice(0,1)}</span>
          <span class="library-copy"><strong>{item.title}</strong><small><span class="kind-label" data-i18n-ignore>{kindLabel(item.kind)}</span>{item.subtitle}</small></span>
          <span class="library-meta">{item.platform || 'Local'}</span>
          <span class="library-meta metric-copy">{item.metric || '—'}</span>
          <span class="library-date">{item.date}</span>
          <span class="open-arrow" aria-hidden="true">›</span>
        </button>
      {/each}
    </div>
  {/if}
</section>
{#if actionError}<p class="status-copy" role="alert">{actionError}</p>{/if}

<style>
  .library-head { align-items: center; }
  .library-total { display: inline-flex; align-items: baseline; gap: 6px; padding: 7px 10px; border: 1px solid var(--border); border-radius: 999px; color: var(--muted); font-size: 11px; white-space: nowrap; }
  .library-total strong { color: var(--text); font-family: var(--font-technical); font-size: 13px; }
  .library-panel { overflow: hidden; }
  .library-toolbar { display: grid; grid-template-columns: minmax(0, 1fr) auto; gap: 9px; padding: 13px; border-bottom: 1px solid var(--border); }
  .library-search { min-width: 0; }
  .sort-select { display: flex; align-items: center; gap: 7px; color: var(--muted); font-size: 10px; }
  .sort-select select { min-width: 130px; }
  .kind-tabs { display: flex; gap: 5px; overflow-x: auto; padding: 10px 13px 9px; border-bottom: 1px solid var(--border); }
  .kind-tabs button { display: inline-flex; align-items: center; gap: 7px; min-height: 32px; padding: 0 10px; border: 1px solid transparent; border-radius: 7px; background: transparent; color: var(--muted); white-space: nowrap; font-size: 11px; }
  .kind-tabs button:hover { color: var(--text); background: var(--surface-2); }
  .kind-tabs button.active { border-color: var(--border); background: var(--surface-2); color: var(--text); }
  .kind-tabs b { min-width: 18px; padding: 1px 5px; border-radius: 999px; background: var(--surface-3); color: var(--faint); font-family: var(--font-technical); font-size: 9px; text-align: center; }
  .kind-tabs button.active b { color: var(--accent); }
  .library-result-bar { display: flex; align-items: center; justify-content: space-between; gap: 12px; min-height: 32px; padding: 0 14px; color: var(--faint); font-size: 10px; }
  .library-columns { display: grid; grid-template-columns: minmax(260px, 1fr) 100px 110px 100px; gap: 10px; min-height: 30px; padding: 0 48px 0 58px; align-items: center; border-top: 1px solid var(--border); color: var(--faint); background: color-mix(in srgb, var(--surface-2) 45%, transparent); font-size: 9px; font-weight: 650; letter-spacing: .05em; text-transform: uppercase; }
  .library-list > button { grid-template-columns: 34px minmax(260px, 1fr) 100px 110px 100px 20px; min-height: 66px; }
  .library-list > button:hover { background: color-mix(in srgb, var(--surface-2) 65%, transparent); }
  .kind-icon { font-family: var(--font-technical); }
  .library-copy { min-width: 0; }
  .library-copy strong { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .library-copy small { display: flex; align-items: center; gap: 7px; min-width: 0; }
  .kind-label { display: inline-flex; flex: 0 0 auto; padding: 2px 5px; border: 1px solid var(--border); border-radius: 5px; color: var(--faint); font-size: 8px; text-transform: uppercase; letter-spacing: .04em; }
  .metric-copy { color: var(--text); font-family: var(--font-technical); }
  .open-arrow { color: var(--faint); font-size: 18px; }

  @media (max-width: 900px) {
    .library-toolbar { grid-template-columns: 1fr; }
    .sort-select { justify-content: flex-end; }
    .library-columns { display: none; }
    .library-list > button { grid-template-columns: 34px minmax(0,1fr) 90px 20px; }
    .library-list .metric-copy, .library-date { display: none; }
  }
  @media (max-width: 620px) {
    .library-total { display: none; }
    .sort-select { justify-content: stretch; }
    .sort-select select { flex: 1; }
    .library-list > button { grid-template-columns: 30px minmax(0,1fr) 20px; }
    .library-list .library-meta { display: none; }
  }
</style>
