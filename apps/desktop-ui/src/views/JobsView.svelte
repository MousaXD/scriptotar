<script lang="ts">
  import { onMount } from 'svelte';
  import { locale } from '../i18n';
  import { translator } from '../i18n/translate';
  import type { Job, JobState } from '../types';
  import { hasNativeFileDrop, subscribeToNativeFileDrop } from '../tauriRuntime';
  import { extractUrlsFromText } from '../utils/textUtils';
  import JobRow from '../components/JobRow.svelte';
  import EmptyState from '../components/EmptyState.svelte';

  export let jobs: Job[];
  export let onCancel: (jobId: string) => Promise<void> | void;
  export let onRetry: (jobId: string) => Promise<void> | void;
  export let onChooseLocal: () => Promise<string | null> | string | null;
  export let onEnqueueLocal: (path: string) => Promise<void> | void;
  export let onEnqueueUrl: (url: string) => Promise<void> | void;
  export let onOpenCompleted: (job: Job) => Promise<void> | void;

  type QueueFilter = 'all' | 'active' | 'attention' | 'done';
  const activeStates = new Set<JobState>(['queued', 'preparing', 'downloading', 'transcribing', 'processing']);
  let filter: QueueFilter = 'all';
  let pageSize = 30;
  let localPath = '';
  let manualPath = '';
  let sourceUrl = '';
  let actionError = '';
  let actionStatus = '';
  let busy = false;
  let dragActive = false;
  let nativeDropAvailable = false;

  $: filters = [
    { id: 'all', label: $translator('queue.filterAll'), count: jobs.length },
    { id: 'active', label: $translator('queue.filterActive'), count: activeCount },
    { id: 'attention', label: $translator('queue.filterAttention'), count: attentionCount },
    { id: 'done', label: $translator('queue.filterDone'), count: doneCount }
  ] satisfies { id: QueueFilter; label: string; count: number }[];
  $: activeCount = jobs.filter((job) => activeStates.has(job.state)).length;
  $: attentionCount = jobs.filter((job) => job.state === 'failed' || job.state === 'interrupted').length;
  $: doneCount = jobs.filter((job) => job.state === 'completed' || job.state === 'cancelled').length;
  $: visible = jobs.filter((job) => filter === 'all'
    || (filter === 'active' && activeStates.has(job.state))
    || (filter === 'attention' && (job.state === 'failed' || job.state === 'interrupted'))
    || (filter === 'done' && (job.state === 'completed' || job.state === 'cancelled')));
  $: shown = visible.slice(0, pageSize);

  function changeFilter(next: QueueFilter) {
    filter = next;
    pageSize = 30;
  }

  async function run(action: () => Promise<void> | void) {
    if (busy) return;
    busy = true;
    actionError = '';
    actionStatus = '';
    try { await action(); }
    catch (error) { actionError = error instanceof Error ? error.message : String(error); }
    finally { busy = false; }
  }

  async function chooseLocal() {
    await run(async () => {
      const selected = await onChooseLocal();
      if (selected) {
        localPath = selected;
        actionStatus = $translator('queue.selectedStatus');
      }
    });
  }

  function displayName(path: string) {
    return path.split(/[\\/]/).filter(Boolean).at(-1) || path;
  }

  function acceptDroppedPaths(paths: string[]) {
    dragActive = false;
    const selected = paths[0];
    if (!selected) return;
    localPath = selected;
    actionError = '';
    actionStatus = paths.length > 1
      ? $translator('queue.dropMultiple', { name: displayName(selected), count: paths.length })
      : $translator('queue.dropSelected', { name: displayName(selected) });
  }

  function enqueueSelectedLocal() {
    return run(async () => {
      const selected = localPath;
      if (!selected) return;
      await onEnqueueLocal(selected);
      localPath = '';
      actionStatus = $translator('queue.queuedFile', { name: displayName(selected) });
    });
  }

  function enqueueManualPath() {
    return run(async () => {
      const selected = manualPath.trim();
      if (!selected) return;
      await onEnqueueLocal(selected);
      manualPath = '';
      actionStatus = $translator('queue.queuedFile', { name: displayName(selected) });
    });
  }

  function enqueueUrl() {
    return run(async () => {
      const url = sourceUrl.trim();
      if (!url) return;
      await onEnqueueUrl(url);
      sourceUrl = '';
      actionStatus = $translator('queue.queuedUrl');
    });
  }

  async function enqueueFromClipboard() {
    await run(async () => {
      if (!navigator.clipboard?.readText) {
        actionError = $translator('queue.clipboardDenied');
        return;
      }
      let clipText = '';
      try { clipText = await navigator.clipboard.readText(); }
      catch { actionError = $translator('queue.clipboardDenied'); return; }
      const urls = extractUrlsFromText(clipText);
      if (!urls.length) { actionError = $translator('queue.clipboardEmpty'); return; }
      for (const url of urls) await onEnqueueUrl(url);
      actionStatus = urls.length === 1
        ? $translator('queue.clipboardQueuedSingle')
        : $translator('queue.clipboardQueuedMultiple', { count: urls.length });
    });
  }

  function handleDragOver(event: DragEvent) {
    event.preventDefault();
    dragActive = true;
  }

  function handleDragLeave(event: DragEvent) {
    if (event.currentTarget === event.target) dragActive = false;
  }

  function handleDrop(event: DragEvent) {
    event.preventDefault();
    dragActive = false;
    if (!event.dataTransfer) return;
    const text = event.dataTransfer.getData('text/uri-list') || event.dataTransfer.getData('text/plain');
    const urls = extractUrlsFromText(text);
    if (urls.length) {
      sourceUrl = urls[0];
      actionStatus = $translator('queue.dropLink', { url: urls[0] });
      actionError = '';
      return;
    }
    // Web File.name is not a filesystem path. Native Tauri drop events provide paths;
    // browser fallback may only use an explicitly supplied path, never a bare name.
    if (nativeDropAvailable) return;
    const dropped = Array.from(event.dataTransfer.files ?? []);
    const paths = dropped.map((file) => (file as File & { path?: string }).path).filter((path): path is string => Boolean(path));
    if (paths.length) acceptDroppedPaths(paths);
    else if (dropped.length) actionError = $translator('queue.nativeDropFailed');
  }

  onMount(() => {
    nativeDropAvailable = hasNativeFileDrop();
    if (!nativeDropAvailable) return;
    let disposed = false;
    let unlisten: (() => void) | undefined;
    void subscribeToNativeFileDrop((event) => {
      if (event.type === 'enter' || event.type === 'over') { dragActive = true; return; }
      if (event.type === 'leave') { dragActive = false; return; }
      acceptDroppedPaths(event.paths);
    }).then((next) => {
      if (disposed) next?.();
      else unlisten = next;
    }).catch((error) => {
      dragActive = false;
      nativeDropAvailable = false;
      actionError = error instanceof Error ? error.message : $translator('queue.nativeDropFailed');
    });
    return () => { disposed = true; unlisten?.(); };
  });
</script>

<header class="queue-header" data-i18n-ignore>
  <h1>{$translator('queue.heading')}</h1>
  <div class="queue-indicators" aria-label={$translator('queue.filters')}>
    <span>{activeCount} {$translator('queue.filterActive')}</span>
    {#if attentionCount}<span class="attention-indicator">{attentionCount} {$translator('queue.filterAttention')}</span>{/if}
  </div>
</header>

<section
  class:drag-active={dragActive}
  class="ingestion-workspace"
  data-i18n-ignore
  aria-label={$translator('queue.addJob')}
  aria-busy={busy}
  on:dragover={handleDragOver}
  on:dragenter={handleDragOver}
  on:dragleave={handleDragLeave}
  on:drop={handleDrop}
>
  {#if dragActive}
    <div class="drop-overlay" aria-live="polite"><strong>{$translator('queue.dropTitle')}</strong><span>{$translator('queue.dropSafety')}</span></div>
  {/if}
  <div class="ingestion-heading"><h2>{$translator('queue.addJob')}</h2></div>
  <div class="ingestion-columns">
    <section class="source-block" aria-label={$translator('queue.localMedia')}>
      <h3>{$translator('queue.localMedia')}</h3>
      <div class="selected-source">
        {#if localPath}
          <strong dir="auto" title={localPath}>{displayName(localPath)}</strong>
        {:else}
          <span>{nativeDropAvailable ? $translator('queue.dropEmpty') : $translator('queue.noVideo')}</span>
        {/if}
      </div>
      <div class="capture-actions">
        <button type="button" class="button secondary" disabled={busy} on:click={chooseLocal}>{$translator('queue.chooseVideo')}</button>
        <button type="button" class="button primary" disabled={busy || !localPath} on:click={enqueueSelectedLocal}>{$translator('queue.queueSelected')}</button>
      </div>
    </section>
    <section class="source-block" aria-label={$translator('queue.supportedUrl')}>
      <form class="url-form" on:submit|preventDefault={enqueueUrl}>
        <label for="queue-url">{$translator('queue.supportedUrl')}</label>
        <input id="queue-url" aria-label={$translator('queue.mediaUrl')} bind:value={sourceUrl} placeholder="https://youtube.com/…" inputmode="url" autocomplete="off" disabled={busy} />
        <div class="capture-actions">
          <button type="button" class="button secondary" disabled={busy} on:click={enqueueFromClipboard} title={$translator('queue.clipboardUrls')}>{$translator('queue.clipboardUrls')}</button>
          <button type="submit" class="button primary" disabled={busy || !sourceUrl.trim()}>{$translator('queue.queueUrl')}</button>
        </div>
      </form>
    </section>
  </div>
  <details class="advanced-path">
    <summary>{$translator('queue.advancedPath')}</summary>
    <form on:submit|preventDefault={enqueueManualPath}>
      <input aria-label={$translator('queue.manualPath')} bind:value={manualPath} placeholder="/home/me/video.mp4" disabled={busy} />
      <button class="button secondary" type="submit" disabled={busy || !manualPath.trim()}>{$translator('queue.queuePath')}</button>
    </form>
  </details>
</section>

{#if actionError}<p class="queue-feedback error-message" role="alert" data-i18n-ignore>{actionError}</p>{/if}
{#if actionStatus}<p class="queue-feedback" role="status" data-i18n-ignore>{actionStatus}</p>{/if}

<section class="queue-workspace" aria-label={$translator('queue.heading')}>
  <div class="queue-toolbar" data-i18n-ignore>
    <div class="queue-filters" role="group" aria-label={$translator('queue.filters')}>
      {#each filters as item (item.id)}
        <button type="button" class:active={filter === item.id} aria-pressed={filter === item.id} on:click={() => changeFilter(item.id)}>
          <span>{item.label}</span><span class="filter-count">{item.count}</span>
        </button>
      {/each}
    </div>
    <span class="visible-count" aria-live="polite">{shown.length} / {visible.length}</span>
  </div>
  <div class="jobs-list" aria-busy={busy}>
    {#if visible.length === 0}
      <EmptyState title={$translator('queue.emptyTitle')} message={$translator('queue.emptyMessage')} />
    {:else}
      {#each shown as job (job.id)}
        <div class="job-with-actions">
          <JobRow {job} />
          <div class="job-actions" data-i18n-ignore>
            {#if activeStates.has(job.state)}<button type="button" class="button danger subtle" disabled={busy} on:click={() => run(() => onCancel(job.id))}>{$translator('queue.cancel')}</button>{/if}
            {#if job.state === 'failed' || job.state === 'interrupted' || job.state === 'cancelled'}<button type="button" class="button secondary subtle" disabled={busy} on:click={() => run(() => onRetry(job.id))}>{$translator('queue.retry')}</button>{/if}
            {#if job.state === 'completed'}<button type="button" class="button secondary subtle" disabled={busy} on:click={() => run(() => onOpenCompleted(job))}>{$translator('queue.openTranscript')}</button>{/if}
          </div>
        </div>
      {/each}
      {#if shown.length < visible.length}
        <button type="button" class="show-more" on:click={() => pageSize += 30}>
          {$locale === 'ar' ? 'عرض المزيد من المهام' : 'Show more jobs'} ({visible.length - shown.length})
        </button>
      {/if}
    {/if}
  </div>
</section>

<style>
  .queue-header { display: flex; align-items: center; justify-content: space-between; flex-wrap: wrap; gap: var(--space-3); margin-bottom: var(--space-4); }
  .queue-header h1 { margin: 0; font-size: var(--text-xl); }
  .queue-indicators { display: flex; align-items: center; flex-wrap: wrap; gap: var(--space-3); color: var(--color-text-muted); font-size: var(--text-sm); }
  .attention-indicator { color: var(--color-warning); }
  .ingestion-workspace { position: relative; margin-bottom: var(--space-5); padding: var(--space-4); border: 1px solid var(--color-border); border-radius: var(--radius-md); background: var(--color-surface); }
  .ingestion-workspace.drag-active { border-color: var(--color-signal); }
  .ingestion-heading h2 { margin: 0 0 var(--space-3); font-size: var(--text-md); }
  .ingestion-columns { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: var(--space-4); }
  .source-block { display: flex; flex-direction: column; min-width: 0; gap: var(--space-2); }
  .source-block + .source-block { border-inline-start: 1px solid var(--color-border); padding-inline-start: var(--space-4); }
  .source-block h3, .url-form label { display: block; margin: 0; color: var(--color-text-muted); font-size: var(--text-sm); font-weight: 650; }
  .selected-source { display: flex; align-items: center; min-width: 0; flex: 1; min-height: var(--control-height); }
  .selected-source strong, .selected-source span { display: block; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: var(--text-sm); }
  .selected-source span { color: var(--color-text-muted); }
  .capture-actions { display: flex; flex-wrap: wrap; gap: var(--space-2); margin-top: auto; }
  .url-form { display: flex; flex-direction: column; flex: 1; gap: var(--space-2); }
  .url-form input { min-width: 0; width: 100%; }
  .advanced-path { margin-top: var(--space-3); padding-top: var(--space-3); border-block-start: 1px solid var(--color-border); color: var(--color-text-muted); font-size: var(--text-xs); }
  .advanced-path summary { width: fit-content; min-height: 28px; cursor: pointer; }
  .advanced-path form { display: flex; gap: var(--space-2); margin-top: var(--space-2); }
  .advanced-path input { flex: 1; min-width: 0; }
  .drop-overlay { position: absolute; z-index: 2; inset: 0; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: var(--space-2); padding: var(--space-4); border-radius: inherit; background: color-mix(in srgb, var(--color-surface) 92%, var(--color-signal)); text-align: center; pointer-events: none; }
  .drop-overlay strong { font-size: var(--text-md); }
  .drop-overlay span { color: var(--color-text-muted); font-size: var(--text-sm); }
  .queue-feedback { margin: calc(-1 * var(--space-3)) 0 var(--space-4); padding: var(--space-2) var(--space-3); border-inline-start: 2px solid var(--color-signal); background: var(--color-surface); color: var(--color-text-muted); font-size: var(--text-sm); overflow-wrap: anywhere; }
  .error-message { border-color: var(--color-danger); color: var(--color-danger); }
  .queue-toolbar { display: flex; align-items: center; justify-content: space-between; gap: var(--space-3); border-block-end: 1px solid var(--color-border); }
  .queue-filters { display: flex; flex-wrap: wrap; gap: var(--space-1); }
  .queue-filters button { display: flex; align-items: center; gap: var(--space-2); min-height: var(--control-height); padding: 0 var(--space-3); border: 0; border-block-end: 2px solid transparent; background: transparent; color: var(--color-text-muted); font-size: var(--text-sm); }
  .queue-filters button.active { border-color: var(--color-signal); color: var(--color-text); }
  .queue-filters button:hover { color: var(--color-text); background: var(--color-surface-raised); }
  .filter-count { color: var(--color-text-faint); font-family: var(--font-technical); font-size: var(--text-xs); }
  .visible-count { color: var(--color-text-faint); font-family: var(--font-technical); font-size: var(--text-xs); white-space: nowrap; }
  .jobs-list { min-width: 0; }
  .job-with-actions { display: grid; grid-template-columns: minmax(0, 1fr) auto; align-items: center; gap: var(--space-3); border-block-start: 1px solid var(--color-border); }
  .job-with-actions:first-child { border-block-start: 0; }
  .job-with-actions :global(.job-row) { border: 0; }
  .job-actions { display: flex; flex-wrap: wrap; justify-content: flex-end; gap: var(--space-2); }
  .job-actions .button { white-space: nowrap; }
  .show-more { display: block; width: 100%; min-height: var(--control-height); padding: var(--space-3); border: 0; border-block-start: 1px solid var(--color-border); background: transparent; color: var(--color-signal); font-size: var(--text-sm); text-align: center; }
  .show-more:hover { background: var(--color-surface-raised); }
  @media (max-width: 900px) { .ingestion-columns { grid-template-columns: 1fr; } .source-block + .source-block { padding: var(--space-3) 0 0; border-inline-start: 0; border-block-start: 1px solid var(--color-border); } }
  @media (max-width: 760px) { .queue-toolbar { align-items: flex-start; flex-direction: column; } .queue-filters button { padding-inline: var(--space-2); } .visible-count { margin-bottom: var(--space-2); } .job-with-actions { grid-template-columns: 1fr; gap: 0; padding-bottom: var(--space-3); } .job-actions { justify-content: flex-start; } }
  @media (max-width: 480px) { .capture-actions .button { flex: 1; } .advanced-path form { flex-direction: column; } }
</style>
