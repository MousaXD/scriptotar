<script lang="ts">
  import { translator } from '../i18n/translate';
  import Select from '../components/ui/Select.svelte';
  import { locale } from '../i18n';
  import type { LibraryItem, LibraryKind } from '../types';

  export let items: LibraryItem[];
  export let onOpen: (item: LibraryItem) => Promise<void> | void;

  const PAGE_SIZE = 75;
  const kinds: Array<'All' | LibraryKind> = ['All', 'Transcript', 'Research', 'AI run', 'Project', 'Creator'];

  let query = '';
  let kind: 'All' | LibraryKind = 'All';
  let sort: 'newest' | 'title' | 'type' = 'newest';
  let page = 1;
  let actionError = '';

  $: isArabic = $locale === 'ar';
  $: normalizedQuery = query.trim().toLocaleLowerCase();
  $: counts = countKinds(items);
  $: newestOrder = new Map(items.map((item, index) => [item, index]));
  $: filtered = items.filter((item) =>
    (kind === 'All' || item.kind === kind) &&
    (item.title + ' ' + item.subtitle + ' ' + item.kind + ' ' + (item.platform || '') + ' ' + (item.metric || ''))
      .toLocaleLowerCase().includes(normalizedQuery)
  );
  $: visible = [...filtered].sort((a, b) => {
    if (sort === 'title') return a.title.localeCompare(b.title, $locale);
    if (sort === 'type') return a.kind.localeCompare(b.kind) || a.title.localeCompare(b.title, $locale);
    return (newestOrder.get(a) ?? 0) - (newestOrder.get(b) ?? 0);
  });
  $: totalPages = Math.max(1, Math.ceil(visible.length / PAGE_SIZE));
  $: currentPage = Math.min(page, totalPages);
  $: pageItems = visible.slice((currentPage - 1) * PAGE_SIZE, currentPage * PAGE_SIZE);

  function countKinds(source: LibraryItem[]): Record<'All' | LibraryKind, number> {
    const result: Record<'All' | LibraryKind, number> = {
      All: source.length, Transcript: 0, Research: 0, 'AI run': 0, Project: 0, Creator: 0
    };
    for (const item of source) result[item.kind] += 1;
    return result;
  }

  function kindLabel(value: 'All' | LibraryKind) {
    if (value === 'All') return $translator('library.kindAll');
    if (value === 'Transcript') return $translator('library.kind.transcript');
    if (value === 'Research') return $translator('library.kind.research');
    if (value === 'AI run') return $translator('library.kind.ai');
    if (value === 'Project') return $translator('library.kind.project');
    return $translator('library.kind.creator');
  }

  function platformLabel(value?: string) {
    return !value || value === 'Local' ? $translator('library.local') : value;
  }

  function chooseKind(value: typeof kind) {
    kind = value;
    page = 1;
  }

  function updateQuery(value: string) {
    query = value;
    page = 1;
  }

  function updateSort(value: typeof sort) {
    sort = value;
    page = 1;
  }

  async function open(item: LibraryItem) {
    actionError = '';
    try {
      await onOpen(item);
    } catch (cause) {
      actionError = cause instanceof Error ? cause.message : $translator('library.openFailed');
    }
  }
</script>

<section class="view-head library-head" data-i18n-ignore>
  <h1>{$translator('library.heading')}</h1>
  <span class="library-total">{$translator('library.localItems', { count: items.length })}</span>
</section>

<section class="panel library-panel" data-i18n-ignore>
  <div class="library-toolbar">
    <label class="search-field library-search">
      <span class="sr-only">{$translator('library.search')}</span>
      <input aria-label={$translator('library.search')} value={query} on:input={(event) => updateQuery(event.currentTarget.value)} placeholder={$translator('library.searchPlaceholder')} />
    </label>
    <div class="sort-select">
      <Select label={$translator('library.sortControl')} value={sort}
        options={[
          { value: 'newest', label: $translator('library.sortNewest') },
          { value: 'title', label: $translator('library.sortTitle') },
          { value: 'type', label: $translator('library.sortType') }
        ]} onChange={(next) => updateSort(next as typeof sort)} />
    </div>
  </div>

  <div class="kind-tabs" role="group" aria-label={$translator('library.kindGroup')}>
    {#each kinds as itemKind (itemKind)}
      <button type="button" class:active={kind === itemKind} aria-pressed={kind === itemKind} on:click={() => chooseKind(itemKind)}>
        <span>{kindLabel(itemKind)}</span><b>{counts[itemKind]}</b>
      </button>
    {/each}
  </div>

  <div class="library-result-bar">
    <span aria-live="polite">{$translator(visible.length === 1 ? 'library.oneResult' : 'library.manyResults', { count: visible.length })}</span>
    {#if query.trim()}<span class="query-context">{$translator('library.matching', { query: query.trim() })}</span>{/if}
  </div>

  {#if visible.length === 0}
    <div class="library-empty">
      <strong>{$translator('library.emptyTitle')}</strong>
      <p>{$translator('library.emptyMessage')}</p>
    </div>
  {:else}
    <div class="library-list">
      <div class="library-columns" aria-hidden="true">
        <span>{$translator('library.column.item')}</span>
        <span>{$translator('library.column.source')}</span>
        <span>{$translator('library.column.signal')}</span>
        <span>{$translator('library.column.date')}</span>
      </div>
      {#each pageItems as item (item.id)}
        <button class="library-item" type="button" aria-label={$translator('library.openItem', { kind: kindLabel(item.kind), title: item.title })} on:click={() => open(item)}>
          <span class={'kind-icon kind-' + item.kind.toLowerCase().replaceAll(' ', '-')} aria-hidden="true">{item.kind.slice(0, 1)}</span>
          <span class="library-copy">
            <strong title={item.title}>{item.title}</strong>
            <small><span class="kind-label">{kindLabel(item.kind)}</span><span class="library-subtitle" title={item.subtitle}>{item.subtitle}</span></small>
          </span>
          <span class="library-meta">{platformLabel(item.platform)}</span>
          <span class="library-meta metric-copy">{item.metric || '—'}</span>
          <span class="library-date">{item.date}</span>
          <span class="open-arrow" aria-hidden="true">›</span>
        </button>
      {/each}
    </div>
    {#if totalPages > 1}
      <nav class="page-controls" aria-label={isArabic ? 'صفحات المكتبة' : 'Library pages'}>
        <span>{isArabic ? 'الصفحة' : 'Page'} {currentPage} / {totalPages}</span>
        <div>
          <button type="button" disabled={currentPage === 1} on:click={() => page = currentPage - 1}>{isArabic ? 'السابق' : 'Previous'}</button>
          <button type="button" disabled={currentPage === totalPages} on:click={() => page = currentPage + 1}>{isArabic ? 'التالي' : 'Next'}</button>
        </div>
      </nav>
    {/if}
  {/if}
</section>

{#if actionError}<p class="status-copy library-error" role="alert" data-i18n-ignore>{actionError}</p>{/if}

<style>
  .library-head { display: flex; flex-direction: row; justify-content: space-between; align-items: baseline; gap: 12px; margin-bottom: 12px; }
  .library-head h1 { margin: 0; }
  .library-total { color: var(--color-text-muted); font-size: var(--text-sm); font-variant-numeric: tabular-nums; }
  .library-panel { overflow: hidden; }
  .library-toolbar { display: grid; grid-template-columns: minmax(0, 1fr) auto; gap: 12px; padding: 12px 14px; align-items: center; border-bottom: 1px solid var(--color-border); }
  .library-search { min-width: 0; }
  .library-search input { width: 100%; }
  .sort-select { display: flex; align-items: center; gap: 9px; color: var(--color-text-muted); font-size: var(--text-sm); }
  .sort-select select { min-width: 155px; }
  .kind-tabs { display: flex; align-items: center; gap: 3px; overflow-x: auto; padding: 8px 12px; border-bottom: 1px solid var(--color-border); }
  .kind-tabs button { display: inline-flex; gap: 9px; align-items: center; justify-content: center; min-height: 38px; padding: 0 11px; white-space: nowrap; border: 1px solid transparent; border-radius: var(--radius-sm); background: transparent; color: var(--color-text-muted); font-size: var(--text-sm); cursor: pointer; }
  .kind-tabs button:hover { color: var(--color-text); background: var(--color-surface-raised); }
  .kind-tabs button.active { color: var(--color-text); background: var(--color-surface-raised); border-color: var(--color-border); }
  .kind-tabs b { font-weight: 500; font-variant-numeric: tabular-nums; color: var(--color-text-muted); }
  .library-result-bar { display: flex; justify-content: space-between; gap: 12px; align-items: center; min-height: 34px; padding: 0 14px; font-size: var(--text-sm); color: var(--color-text-muted); }
  .query-context { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .library-columns { display: grid; grid-template-columns: minmax(200px, 1fr) 112px 116px 112px; gap: 12px; min-height: 33px; align-items: center; padding-inline-start: 60px; padding-inline-end: 45px; border-top: 1px solid var(--color-border); background: var(--color-surface-raised); color: var(--color-text-muted); font-size: var(--text-xs); font-weight: 600; text-transform: none; }
  .library-list .library-item { display: grid; grid-template-columns: 32px minmax(200px, 1fr) 112px 116px 112px 14px; gap: 12px; align-items: center; width: 100%; min-height: 66px; padding: 8px 14px; border: 0; border-top: 1px solid var(--color-border); border-radius: 0; background: transparent; text-align: start; }
  .library-list .library-item:hover { background: var(--color-surface-raised); }
  .kind-icon { display: grid; place-items: center; width: 32px; height: 32px; border: 1px solid var(--color-border); border-radius: var(--radius-sm); color: var(--color-text-muted); font-size: 14px; font-weight: 650; }
  .kind-transcript { border-color: var(--color-border-strong); color: var(--color-info); }
  .kind-research { color: var(--color-signal); }
  .kind-ai-run { color: var(--color-warning); }
  .library-copy { display: block; min-width: 0; }
  .library-copy strong { display: block; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; color: var(--color-text); font-size: 13px; font-weight: 650; line-height: 1.4; }
  .library-copy small { display: flex; align-items: center; gap: 8px; min-width: 0; margin-top: 4px; color: var(--color-text-muted); font-size: var(--text-xs); }
  .kind-label { flex-shrink: 0; color: var(--color-text-muted); }
  .library-subtitle { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .library-meta, .library-date { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; color: var(--color-text-muted); font-size: var(--text-sm); }
  .metric-copy { color: var(--color-text); }
  .open-arrow { font-size: 20px; color: var(--color-text-muted); }
  .library-empty { padding: 34px 22px; }
  .library-empty strong { font-size: 14px; }
  .library-empty p { margin: 6px 0 0; color: var(--color-text-muted); }
  .library-error { margin-top: 12px; font-size: var(--text-sm); overflow-wrap: anywhere; }
  .page-controls { display: flex; align-items: center; justify-content: space-between; gap: 12px; padding: 10px 14px; border-top: 1px solid var(--color-border); color: var(--color-text-muted); font-size: var(--text-sm); }
  .page-controls > div { display: flex; gap: 8px; }
  .page-controls button { min-height: 34px; padding: 0 12px; border: 1px solid var(--color-border); border-radius: var(--radius-sm); background: var(--color-surface-raised); }
  .page-controls button:disabled { opacity: .45; }
  @media (max-width: 1020px) {
    .library-columns { display: none; }
    .library-list .library-item { grid-template-columns: 32px minmax(0, 1fr) 112px 14px; }
    .library-list .metric-copy, .library-date { display: none; }
  }
  @media (max-width: 640px) {
    .library-toolbar { grid-template-columns: 1fr; }
    .sort-select select { flex: 1; min-width: 0; }
    .library-list .library-item { grid-template-columns: 32px minmax(0, 1fr) 14px; gap: 9px; }
    .library-list .library-meta { display: none; }
    .library-head { flex-wrap: wrap; }
    .library-copy small { flex-wrap: wrap; }
  }
</style>
