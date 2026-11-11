<script lang="ts">
  import { locale } from '../i18n';
  import Select from '../components/ui/Select.svelte';
  import type { ResearchItem, WatchlistOperationalState, WatchlistStatus } from '../types';
  import { observedInteractionRate } from '../utils/researchMetrics';

  export let items: ResearchItem[];
  export let watchlistStatuses: WatchlistStatus[] = [];
  export let onQueue: (ids: string[]) => Promise<void> | void;
  export let onScan: (url: string, limit: number) => Promise<void> | void;
  export let onSave: (url: string, limit: number) => Promise<void> | void;

  const PAGE_SIZE = 50;
  // Matches both the Tauri IPC guard and backend queue-research admission limit.
  const QUEUE_BATCH_SIZE = 200;
  const stateLabel: Record<WatchlistOperationalState, string> = {
    healthy: 'Healthy',
    never_scanned: 'Never scanned',
    refreshing: 'Refreshing',
    retry_scheduled: 'Retry scheduled',
    failed: 'Failed'
  };

  let query = '';
  let platform = 'All';
  let sort: 'views' | 'likes' | 'comments' | 'date' | 'rate' = 'views';
  let page = 1;
  let selected = new Set<string>();
  let failedThumbnails = new Set<string>();
  let profileUrl = '';
  let limit = 25;
  let scanning = false;
  let saving = false;
  let queuing = false;
  let status = '';
  let statusError = false;

  $: isArabic = $locale === 'ar';
  $: numberLocale = isArabic ? 'ar' : 'en';
  $: compactNumber = new Intl.NumberFormat(numberLocale, { notation: 'compact', maximumFractionDigits: 1 });
  $: percentNumber = new Intl.NumberFormat(numberLocale, { style: 'percent', maximumFractionDigits: 1 });
  $: platforms = [...new Set(items.map((item) => item.platform).filter(Boolean))].sort();
  $: normalizedQuery = query.trim().toLocaleLowerCase();
  $: filtered = items
    .filter((item) => platform === 'All' || item.platform === platform)
    .filter((item) => (item.creator + ' ' + item.title + ' ' + item.platform).toLocaleLowerCase().includes(normalizedQuery))
    .sort((a, b) => {
      if (sort === 'date') return (b.publishedAt || '').localeCompare(a.publishedAt || '');
      if (sort === 'rate') return (observedInteractionRate(b) ?? -1) - (observedInteractionRate(a) ?? -1);
      return (b[sort] ?? -1) - (a[sort] ?? -1);
    });
  $: totalPages = Math.max(1, Math.ceil(filtered.length / PAGE_SIZE));
  $: currentPage = Math.min(page, totalPages);
  $: pageItems = filtered.slice((currentPage - 1) * PAGE_SIZE, currentPage * PAGE_SIZE);
  $: selectablePageItems = pageItems.filter((item) => !item.queued);
  $: selectedCount = items.filter((item) => selected.has(item.id) && !item.queued).length;
  $: pageSelected = selectablePageItems.length > 0 && selectablePageItems.every((item) => selected.has(item.id));
  $: attentionCount = watchlistStatuses.filter((item) => item.state === 'failed' || item.state === 'retry_scheduled').length;

  function metric(value?: number) {
    return typeof value === 'number' && Number.isFinite(value) ? compactNumber.format(value) : '—';
  }

  function rateLabel(item: ResearchItem) {
    const rate = observedInteractionRate(item);
    return rate === null ? '—' : percentNumber.format(rate);
  }

  function displayTime(value?: string) {
    if (!value) return '—';
    const parsed = new Date(value);
    return Number.isNaN(parsed.getTime()) ? value : parsed.toLocaleString(numberLocale);
  }

  function duration(value?: number) {
    if (typeof value !== 'number' || !Number.isFinite(value) || value < 0) return '—';
    const minutes = Math.floor(value / 60);
    const seconds = Math.floor(value % 60);
    return minutes + ':' + String(seconds).padStart(2, '0');
  }

  function toggle(id: string) {
    selected = new Set(selected);
    if (selected.has(id)) selected.delete(id);
    else selected.add(id);
  }

  function togglePageSelection() {
    const next = new Set(selected);
    for (const item of selectablePageItems) {
      if (pageSelected) next.delete(item.id);
      else next.add(item.id);
    }
    selected = next;
  }

  function updateQuery(value: string) {
    query = value;
    page = 1;
  }

  function updatePlatform(value: string) {
    platform = value;
    page = 1;
  }

  function updateSort(value: typeof sort) {
    sort = value;
    page = 1;
  }

  async function scan() {
    if (!profileUrl.trim() || scanning || saving) return;
    scanning = true;
    status = '';
    try {
      await onScan(profileUrl.trim(), limit);
      statusError = false;
      status = 'Research scan completed.';
    } catch (error) {
      statusError = true;
      status = 'Research scan unavailable: ' + (error instanceof Error ? error.message : String(error));
    } finally {
      scanning = false;
    }
  }

  async function save() {
    if (!profileUrl.trim() || scanning || saving) return;
    saving = true;
    status = '';
    try {
      await onSave(profileUrl.trim(), limit);
      statusError = false;
      status = 'Watchlist saved locally for this project.';
    } catch (error) {
      statusError = true;
      status = 'Watchlist save failed: ' + (error instanceof Error ? error.message : String(error));
    } finally {
      saving = false;
    }
  }

  async function queueSelected() {
    if (queuing) return;
    const ids = items.filter((item) => selected.has(item.id) && !item.queued).map((item) => item.id);
    if (!ids.length) return;
    queuing = true;
    status = '';
    let completed = 0;
    try {
      for (let index = 0; index < ids.length; index += QUEUE_BATCH_SIZE) {
        const batch = ids.slice(index, index + QUEUE_BATCH_SIZE);
        await onQueue(batch);
        const completedIds = new Set(batch);
        selected = new Set([...selected].filter((id) => !completedIds.has(id)));
        completed += batch.length;
      }
      statusError = false;
      status = 'Selected media queued.';
    } catch (error) {
      statusError = true;
      const detail = error instanceof Error ? error.message : String(error);
      status = 'Queue unavailable: ' + detail
        + (completed ? ' (' + completed + ' submitted; remaining selection preserved)' : '');
    } finally {
      queuing = false;
    }
  }
</script>

<section class="view-head research-head">
  <h1>Research</h1>
  <div class="research-summary">
    <span class="result-count" aria-live="polite">{filtered.length} / {items.length}</span>
    {#if attentionCount > 0}
      <a class="attention-link" href="#watch-health-title">{attentionCount} {isArabic ? (attentionCount === 1 ? 'قائمة متابعة تحتاج انتباهاً' : 'قوائم متابعة تحتاج انتباهاً') : (attentionCount === 1 ? 'watchlist needs attention' : 'watchlists need attention')}</a>
    {/if}
  </div>
</section>

<form class="panel research-capture" on:submit|preventDefault={scan}>
  <label class="url-field"><span>Creator / profile URL</span><input type="text" inputmode="url" bind:value={profileUrl} placeholder="https://…" aria-label="Creator profile URL" spellcheck="false" /></label>
  <div class="small-field">
    <Select label={isArabic ? 'حد الفحص' : 'Scan limit'} value={String(limit)}
      options={[10, 25, 50, 100].map((value) => ({ value: String(value), label: String(value) }))}
      onChange={(next) => { limit = Number(next); }} />
  </div>
  <button type="submit" class="button primary" disabled={scanning || saving || !profileUrl.trim()}>{scanning ? 'Scanning…' : 'Scan profile'}</button>
  <button type="button" class="button secondary" disabled={scanning || saving || !profileUrl.trim()} on:click={save}>{saving ? 'Saving…' : 'Save watchlist'}</button>
</form>

{#if status}<p class="status-copy action-status" role={statusError ? 'alert' : 'status'}>{status}</p>{/if}

<section class="panel research-panel" aria-label="Research results">
  <div class="filter-bar">
    <label class="search-field research-search"><span class="sr-only">Filter research</span><input aria-label="Filter research" value={query} on:input={(event) => updateQuery(event.currentTarget.value)} placeholder="Filter title or creator…" /></label>
    <Select label={isArabic ? 'مرشح المنصة' : 'Platform filter'} value={platform}
      options={[{ value: 'All', label: isArabic ? 'الكل' : 'All' }, ...platforms.map((option) => ({ value: option, label: option }))]}
      onChange={updatePlatform} />
    <div class="sort-control">
      <Select label={isArabic ? 'ترتيب البحث' : 'Research sort'} value={sort}
        options={[
          { value: 'views', label: isArabic ? 'المشاهدات' : 'Views' },
          { value: 'likes', label: isArabic ? 'الإعجابات' : 'Likes' },
          { value: 'comments', label: isArabic ? 'التعليقات' : 'Comments' },
          { value: 'date', label: isArabic ? 'الأحدث' : 'Newest' },
          { value: 'rate', label: isArabic ? 'التفاعلات / المشاهدات' : 'Interactions / views' }
        ]} onChange={(next) => updateSort(next as typeof sort)} />
    </div>
  </div>

  <div class="selection-bar">
    <label class="select-page">
      <input type="checkbox" checked={pageSelected} disabled={selectablePageItems.length === 0} on:change={togglePageSelection} aria-label={isArabic ? 'تحديد عناصر الصفحة' : 'Select page'} />
      <span>{isArabic ? 'تحديد الصفحة' : 'Select page'}</span>
    </label>
    <span class="selection-count" aria-live="polite">{isArabic ? 'محدد: ' : 'Selected: '}{selectedCount}</span>
    {#if selectedCount > 0}
      <button type="button" class="text-action" on:click={() => selected = new Set()}>{isArabic ? 'مسح التحديد' : 'Clear selection'}</button>
    {/if}
    <button type="button" class="button primary queue-action" disabled={queuing || !items.some((item) => selected.has(item.id) && !item.queued)} on:click={queueSelected}>
      {queuing ? (isArabic ? 'جارٍ الإضافة…' : 'Queueing…') : 'Queue selected (' + selectedCount + ')'}
    </button>
  </div>

  {#if filtered.length === 0}
    <div class="no-results"><strong>No matching research</strong><p>Change the filters or scan another creator profile.</p></div>
  {:else}
    <div class="research-table" role="table" aria-label="Research results">
      <div class="research-row table-head" role="row">
        <span aria-hidden="true"></span><span role="columnheader">Media</span><span role="columnheader">Views</span><span role="columnheader">Likes</span><span role="columnheader">Comments</span><span role="columnheader">Date</span><span role="columnheader">Duration</span>
      </div>
      {#each pageItems as item (item.id)}
        <div class="research-row" role="row" data-testid={'research-' + item.id}>
          <div class="check-only" role="cell"><label><input type="checkbox" checked={selected.has(item.id) && !item.queued} disabled={Boolean(item.queued)} on:change={() => toggle(item.id)} aria-label={'Select ' + item.title} /></label></div>
          <div class="media-cell" role="cell">
            {#if item.thumbnail && !failedThumbnails.has(item.id + ":" + item.thumbnail)}
              <img class="research-thumbnail" src={item.thumbnail} alt="" loading="lazy" on:error={() => failedThumbnails = new Set([...failedThumbnails, item.id + ":" + item.thumbnail])} />
            {:else}
              <div class="thumbnail-placeholder" aria-hidden="true">{item.platform.slice(0, 1)}</div>
            {/if}
            <div class="media-copy">
              <strong title={item.title}>{item.title}</strong>
              <small><span class="creator-name">{item.creator}</span><span class="source-platform">{item.platform}</span>{#if item.queued}<span class="queued-flag">{isArabic ? 'في الطابور' : 'Queued'}</span>{/if}</small>
              {#if observedInteractionRate(item) !== null}<small class="derived-metric" title="Likes plus comments divided by observed views; not unique people or causal performance">{isArabic ? 'التفاعلات / المشاهدات' : 'Interactions / views'}: {rateLabel(item)}</small>{/if}
            </div>
          </div>
          <div class="metric metric-views" role="cell"><span class="mobile-metric-label">Views</span><strong>{metric(item.views)}</strong></div>
          <span class="metric metric-likes" role="cell"><span class="mobile-metric-label">Likes</span>{metric(item.likes)}</span>
          <span class="metric metric-comments" role="cell"><span class="mobile-metric-label">Comments</span>{metric(item.comments)}</span>
          <span class="metric metric-date" role="cell"><span class="mobile-metric-label">Date</span>{item.publishedAt || '—'}</span>
          <span class="metric metric-duration" role="cell"><span class="mobile-metric-label">Duration</span>{duration(item.durationSeconds)}</span>
        </div>
      {/each}
    </div>
    {#if totalPages > 1}
      <nav class="page-controls" aria-label={isArabic ? 'صفحات البحث' : 'Research pages'}>
        <span>{isArabic ? 'الصفحة' : 'Page'} {currentPage} / {totalPages}</span>
        <div>
          <button type="button" disabled={currentPage === 1} on:click={() => page = currentPage - 1}>{isArabic ? 'السابق' : 'Previous'}</button>
          <button type="button" disabled={currentPage === totalPages} on:click={() => page = currentPage + 1}>{isArabic ? 'التالي' : 'Next'}</button>
        </div>
      </nav>
    {/if}
  {/if}
</section>

<section class="watch-health" aria-labelledby="watch-health-title">
  <header class="watch-health-head">
    <h2 id="watch-health-title">Watchlist health</h2>
    <span class:attention={attentionCount > 0} class="health-summary">{watchlistStatuses.length} {isArabic ? 'قوائم متابعة' : 'watchlists'}{#if attentionCount > 0} · {attentionCount} {isArabic ? 'تحتاج انتباهاً' : 'need attention'}{/if}</span>
  </header>
  {#if watchlistStatuses.length === 0}
    <p class="watch-empty">No saved watchlists in this project yet.</p>
  {:else}
    <div class="watch-health-list">
      {#each watchlistStatuses as watchlist (watchlist.watchlistId)}
        <article class={'watch-health-card state-' + watchlist.state} data-testid={'watchlist-status-' + watchlist.watchlistId}>
          <div class="watch-health-title-row">
            <strong>{watchlist.label}</strong>
            <span class="watch-state">{stateLabel[watchlist.state]}</span>
          </div>
          {#if watchlist.lastError}<p class="watch-error" role="status">{watchlist.lastError}</p>{/if}
          {#if watchlist.nextRetryAt}
            <p class="retry-time"><span>Next retry</span> <time>{displayTime(watchlist.nextRetryAt)}</time></p>
          {/if}
          <details class="watch-diagnostics">
            <summary>{isArabic ? 'تفاصيل التحديث' : 'Refresh details'}</summary>
            <dl>
              <div><dt>Last attempt</dt><dd>{displayTime(watchlist.lastAttemptAt)}</dd></div>
              <div><dt>Last success</dt><dd>{displayTime(watchlist.lastSuccessfulScanAt)}</dd></div>
            </dl>
          </details>
        </article>
      {/each}
    </div>
  {/if}
</section>

<style>
  .research-head { display: flex; flex-direction: row; justify-content: space-between; align-items: baseline; gap: 12px; margin-bottom: 12px; }
  .research-head h1 { margin: 0; }
  .research-summary { display: flex; flex-wrap: wrap; align-items: baseline; justify-content: flex-end; gap: 12px; }
  .result-count { color: var(--color-text-muted); font-size: var(--text-sm); font-variant-numeric: tabular-nums; }
  .attention-link { color: var(--color-warning); text-decoration: underline; text-underline-offset: 3px; font-size: var(--text-sm); }
  .research-capture { display: grid; grid-template-columns: minmax(0, 1fr) 96px auto auto; gap: 10px; align-items: end; padding: 14px; margin-bottom: 12px; }
  .research-capture label { min-width: 0; }
  .research-capture label span { font-size: var(--text-sm); }
  .research-capture input { width: 100%; direction: ltr; text-align: start; }
  .research-capture button { white-space: nowrap; }
  .action-status { margin: 0 0 12px; font-size: var(--text-sm); overflow-wrap: anywhere; }
  .research-panel { overflow: hidden; }
  .filter-bar { display: flex; align-items: center; gap: 9px; padding: 12px 14px; border-bottom: 1px solid var(--color-border); }
  .research-search { flex: 1; min-width: 180px; }
  .research-search input { width: 100%; }
  .sort-control { display: flex; align-items: center; gap: 8px; font-size: var(--text-sm); color: var(--color-text-muted); }
  .filter-bar select { min-width: 120px; }
  .selection-bar { display: flex; gap: 14px; align-items: center; padding: 9px 14px; background: var(--color-surface-raised); border-bottom: 1px solid var(--color-border); font-size: var(--text-sm); }
  .select-page { display: flex; align-items: center; gap: 8px; cursor: pointer; }
  .select-page input, .check-only input { appearance: none; -webkit-appearance: none; position: relative; flex-shrink: 0; width: 18px; height: 18px; margin: 0; border: 1.5px solid var(--color-border-strong); border-radius: 5px; background: var(--color-surface-raised); cursor: pointer; }
  .select-page input:checked, .check-only input:checked { background: var(--color-signal); border-color: var(--color-signal-strong); }
  .select-page input:checked::after, .check-only input:checked::after { content: ''; position: absolute; width: 5px; height: 9px; inset-inline-start: 5px; top: 2px; border: solid var(--color-signal-ink); border-width: 0 2px 2px 0; transform: rotate(45deg); }
  .select-page input:focus-visible, .check-only input:focus-visible { outline: 2px solid var(--color-signal-strong); outline-offset: 3px; }
  .select-page input:disabled, .check-only input:disabled { opacity: .5; cursor: not-allowed; }
  .selection-count { color: var(--color-text-muted); white-space: nowrap; }
  .text-action { border: 0; color: var(--color-text-muted); background: transparent; text-decoration: underline; text-underline-offset: 2px; }
  .queue-action { margin-inline-start: auto; white-space: nowrap; }
  .research-table { overflow-x: auto; }
  .research-row { display: grid; grid-template-columns: 30px minmax(260px, 2fr) repeat(3, minmax(62px, .5fr)) minmax(94px, .75fr) 62px; gap: 12px; min-width: 810px; align-items: center; min-height: 62px; padding: 9px 14px; border-block-start: 1px solid var(--color-border); font-size: var(--text-sm); }
  .research-row.table-head { min-height: 36px; border-block-start: 0; color: var(--color-text-muted); background: var(--color-surface-raised); font-size: var(--text-xs); font-weight: 600; }
  .media-cell { display: flex; gap: 11px; align-items: center; min-width: 0; }
  .research-thumbnail, .thumbnail-placeholder { width: 64px; height: 40px; flex-shrink: 0; border-radius: var(--radius-sm); background: var(--color-surface-strong); }
  .research-thumbnail { object-fit: cover; }
  .thumbnail-placeholder { display: grid; place-content: center; font-size: var(--text-lg); color: var(--color-text-muted); }
  .media-copy { min-width: 0; }
  .media-copy strong { display: block; color: var(--color-text); font-size: 13px; line-height: 1.4; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .media-copy small { display: flex; flex-wrap: wrap; align-items: center; gap: 7px; margin-top: 3px; font-size: 11px; }
  .creator-name { color: var(--color-text-muted); }
  .source-platform, .queued-flag { padding-inline-start: 7px; border-inline-start: 1px solid var(--color-border-strong); }
  .queued-flag { color: var(--color-signal); }
  .derived-metric { color: var(--color-text-muted); font-variant-numeric: tabular-nums; }
  .metric { font-variant-numeric: tabular-nums; color: var(--color-text-muted); }
  .metric-views { color: var(--color-text); }
  .mobile-metric-label { display: none; }
  .no-results { padding: 34px 22px; }
  .no-results strong { font-size: 14px; }
  .no-results p { color: var(--color-text-muted); margin: 6px 0 0; }
  .page-controls { display: flex; justify-content: space-between; align-items: center; gap: 12px; padding: 10px 14px; border-block-start: 1px solid var(--color-border); color: var(--color-text-muted); font-size: var(--text-sm); }
  .page-controls > div { display: flex; gap: 8px; }
  .page-controls button { min-height: 34px; padding: 0 12px; border: 1px solid var(--color-border); border-radius: var(--radius-sm); background: var(--color-surface-raised); }
  .page-controls button:disabled { opacity: .45; }
  .watch-health { margin-block-start: 20px; }
  .watch-health-head { display: flex; justify-content: space-between; align-items: baseline; gap: 14px; margin-block-end: 9px; }
  .watch-health-head h2 { margin: 0; font-size: var(--text-lg); }
  .health-summary, .watch-empty { color: var(--color-text-muted); font-size: var(--text-sm); }
  .health-summary.attention { color: var(--color-warning); }
  .watch-health-list { border: 1px solid var(--color-border); border-radius: var(--radius-md); overflow: hidden; }
  .watch-health-card { padding: 10px 14px; border-block-start: 1px solid var(--color-border); }
  .watch-health-card:first-child { border-block-start: 0; }
  .watch-health-title-row { display: flex; justify-content: space-between; align-items: center; gap: 12px; }
  .watch-health-title-row strong { font-size: 13px; min-width: 0; overflow-wrap: anywhere; }
  .watch-state { flex-shrink: 0; color: var(--color-text-muted); font-size: var(--text-sm); }
  .state-failed .watch-state { color: var(--color-danger); }
  .state-retry_scheduled .watch-state { color: var(--color-warning); }
  .state-refreshing .watch-state { color: var(--color-info); }
  .watch-error { margin: 5px 0 0; color: var(--color-danger); font-size: var(--text-sm); overflow-wrap: anywhere; }
  .retry-time { display: flex; flex-wrap: wrap; gap: 8px; margin: 5px 0 0; color: var(--color-text-muted); font-size: var(--text-sm); }
  .watch-diagnostics { margin-top: 6px; font-size: var(--text-sm); color: var(--color-text-muted); }
  .watch-diagnostics summary { width: fit-content; cursor: pointer; }
  .watch-diagnostics dl { display: grid; gap: 6px; margin: 8px 0 0; max-width: 450px; }
  .watch-diagnostics dl div { display: flex; justify-content: space-between; gap: 12px; }
  .watch-diagnostics dd { margin: 0; text-align: end; color: var(--color-text); }
  @media (max-width: 980px) {
    .research-capture { grid-template-columns: minmax(0, 1fr) 96px; }
    .research-capture button { width: 100%; }
    .filter-bar { flex-wrap: wrap; }
    .research-search { flex-basis: 100%; }
  }
  @media (max-width: 790px) {
    .research-row.table-head { display: none; }
    .research-row:not(.table-head) { min-width: 0; grid-template-columns: 26px minmax(0, 1fr) repeat(3, auto); grid-template-areas: 'check media media media media' '. views likes comments comments' '. date date duration duration'; gap: 9px 13px; margin: 8px; border: 1px solid var(--color-border); border-radius: var(--radius-sm); }
    .check-only { grid-area: check; }
    .media-cell { grid-area: media; }
    .metric-views { grid-area: views; }
    .metric-likes { grid-area: likes; }
    .metric-comments { grid-area: comments; }
    .metric-date { grid-area: date; }
    .metric-duration { grid-area: duration; }
    .mobile-metric-label { display: block; font-size: var(--text-xs); color: var(--color-text-muted); }
    .metric { display: grid; gap: 2px; }
    .selection-bar { flex-wrap: wrap; }
  }
  @media (max-width: 480px) {
    .research-head { flex-wrap: wrap; }
    .research-summary { justify-content: flex-start; }
    .research-capture { grid-template-columns: minmax(0, 1fr) 90px; }
    .research-capture button { grid-column: span 2; }
    .filter-bar select { min-width: 0; flex: 1; }
    .sort-control { flex: 1; }
    .sort-control select { min-width: 0; flex: 1; }
    .queue-action { margin-inline-start: 0; width: 100%; }
    .research-thumbnail, .thumbnail-placeholder { width: 52px; height: 38px; }
  }
</style>
