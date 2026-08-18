<script lang="ts">
  import { onMount } from 'svelte';
  import type { Job, JobState } from '../types';
  import { hasNativeFileDrop, subscribeToNativeFileDrop } from '../tauriRuntime';
  import JobRow from '../components/JobRow.svelte';
  import EmptyState from '../components/EmptyState.svelte';

  export let jobs: Job[];
  export let onCancel: (jobId: string) => Promise<void> | void;
  export let onRetry: (jobId: string) => Promise<void> | void;
  export let onChooseLocal: () => Promise<string | null> | string | null;
  export let onEnqueueLocal: (path: string) => Promise<void> | void;
  export let onEnqueueUrl: (url: string) => Promise<void> | void;
  export let onOpenCompleted: (job: Job) => Promise<void> | void;

  let filter: 'all' | 'active' | 'attention' | 'done' = 'all';
  let localPath = '';
  let manualPath = '';
  let sourceUrl = '';
  let actionError = '';
  let actionStatus = '';
  let busy = false;
  let dragActive = false;
  let nativeDropAvailable = false;

  const filters: { id: 'all' | 'active' | 'attention' | 'done'; label: string }[] = [
    { id: 'all', label: 'All' }, { id: 'active', label: 'Active' }, { id: 'attention', label: 'Needs attention' }, { id: 'done', label: 'Finished' }
  ];
  const active = new Set<JobState>(['queued','preparing','downloading','transcribing','processing']);
  $: visible = jobs.filter((job) => filter === 'all' || (filter === 'active' && active.has(job.state)) || (filter === 'attention' && ['failed','interrupted'].includes(job.state)) || (filter === 'done' && ['completed','cancelled'].includes(job.state)));
  $: activeCount = jobs.filter((job) => active.has(job.state)).length;
  $: attentionCount = jobs.filter((job) => ['failed', 'interrupted'].includes(job.state)).length;

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
        actionStatus = 'Video selected. Queue it when ready.';
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
      ? `Selected ${displayName(selected)}. Queue it when ready; only the first of ${paths.length} dropped files is selected.`
      : `Dropped ${displayName(selected)}. Queue it when ready.`;
  }

  onMount(() => {
    nativeDropAvailable = hasNativeFileDrop();
    if (!nativeDropAvailable) return;

    let disposed = false;
    let unlisten: (() => void) | undefined;

    void subscribeToNativeFileDrop((event) => {
      if (event.type === 'enter' || event.type === 'over') {
        dragActive = true;
        return;
      }
      if (event.type === 'leave') {
        dragActive = false;
        return;
      }
      acceptDroppedPaths(event.paths);
    }).then((next) => {
      if (disposed) next?.();
      else unlisten = next;
    }).catch((error) => {
      dragActive = false;
      nativeDropAvailable = false;
      actionError = error instanceof Error ? error.message : 'Native file drop could not be enabled.';
    });

    return () => {
      disposed = true;
      unlisten?.();
    };
  });
</script>

<section class="view-head queue-head">
  <div><span class="eyebrow">Persistent activity</span><h1>Jobs</h1><p>Choose local media with the desktop picker, drop a file into the desktop app, or queue a supported URL. Rust still validates every path and owns queue state.</p></div>
  <div class="queue-summary" aria-label="Job filters">
    <span><strong>{activeCount}</strong> Active</span>
    {#if attentionCount > 0}<span class="attention-count"><strong>{attentionCount}</strong> Needs attention</span>{/if}
  </div>
</section>

<section class:drag-active={dragActive} class="panel jobs-capture" aria-label="Add transcription job" aria-busy={busy}>
  {#if dragActive}
    <div class="drop-overlay" aria-live="polite"><div><strong>Drop to select media</strong><span>The file will still be validated by Scriptotar before it can be queued.</span></div></div>
  {/if}

  <div class="capture-heading">
    <span class="eyebrow">Add transcription job</span>
    <h2>Local media</h2>
    <p>{nativeDropAvailable ? 'Choose a video or drop one anywhere in this window.' : 'Use the native desktop picker for normal operation.'}</p>
  </div>

  <div class="source-grid">
    <section class:ready={Boolean(localPath)} class:drop-ready={nativeDropAvailable} class="source-card local-source">
      <div class="source-label"><span class="source-icon" aria-hidden="true">＋</span><div><strong>Local media</strong><small>{localPath ? displayName(localPath) : nativeDropAvailable ? 'Drop a video anywhere in this window' : 'No video selected'}</small></div></div>
      {#if localPath}<code class="selected-path">{localPath}</code>{/if}
      <div class="capture-actions">
        <button class="button secondary" disabled={busy} on:click={chooseLocal}>Choose video</button>
        <button class="button primary" disabled={busy || !localPath} on:click={() => run(async () => { await onEnqueueLocal(localPath); actionStatus = `Queued ${displayName(localPath)}.`; localPath = ''; })}>Queue selected</button>
      </div>
    </section>

    <section class:ready={Boolean(sourceUrl.trim())} class="source-card url-source">
      <label class="url-capture"><span>Supported media URL</span><input aria-label="Media URL" bind:value={sourceUrl} placeholder="https://youtube.com/…" /></label>
      <div class="capture-actions single-action"><button class="button primary" disabled={busy || !sourceUrl.trim()} on:click={() => run(async () => { const queued = sourceUrl.trim(); await onEnqueueUrl(queued); sourceUrl = ''; actionStatus = 'URL queued.'; })}>Queue URL</button></div>
    </section>
  </div>

  <details class="advanced-path">
    <summary>Advanced: enter a local path manually</summary>
    <div><input aria-label="Manual local media path" bind:value={manualPath} placeholder="/home/me/video.mp4" /><button class="button secondary" disabled={busy || !manualPath.trim()} on:click={() => run(async () => { await onEnqueueLocal(manualPath.trim()); actionStatus = `Queued ${displayName(manualPath.trim())}.`; manualPath = ''; })}>Queue path</button></div>
  </details>
</section>

{#if actionError}<p class="status-copy queue-message error-message" role="alert">{actionError}</p>{/if}
{#if actionStatus}<p class="status-copy queue-message" aria-live="polite">{actionStatus}</p>{/if}

<div class="queue-toolbar">
  <div class="segmented" role="group" aria-label="Job filters">
    {#each filters as item}<button class:active={filter === item.id} aria-pressed={filter === item.id} on:click={() => filter = item.id}>{item.label}</button>{/each}
  </div>
  <span class="visible-count">{visible.length} / {jobs.length}</span>
</div>

<section class="panel jobs-panel" aria-busy={busy}>
  {#if visible.length === 0}
    <EmptyState title="No jobs in this view" message="Change the filter or queue media above." />
  {:else}
    {#each visible as job}
      <div class="job-with-actions">
        <JobRow {job} />
        <div class="job-actions">
          {#if active.has(job.state)}<button class="button danger subtle" disabled={busy} on:click={() => run(() => onCancel(job.id))}>Cancel</button>{/if}
          {#if ['failed','interrupted','cancelled'].includes(job.state)}<button class="button secondary subtle" disabled={busy} on:click={() => run(() => onRetry(job.id))}>Retry</button>{/if}
          {#if job.state === 'completed'}<button class="button secondary subtle" disabled={busy} on:click={() => run(() => onOpenCompleted(job))}>Open transcript</button>{/if}
        </div>
      </div>
    {/each}
  {/if}
</section>

<style>
  .queue-head { align-items: center; }
  .queue-summary { display: flex; flex-wrap: wrap; gap: 8px; }
  .queue-summary > span { display: inline-flex; align-items: center; gap: 6px; min-height: 34px; padding: 0 10px; border: 1px solid var(--border); border-radius: 999px; color: var(--muted); font-size: 11px; }
  .queue-summary strong { color: var(--text); }
  .queue-summary .attention-count { color: var(--warn); border-color: color-mix(in srgb, var(--warn) 35%, var(--border)); }

  .jobs-capture { position: relative; display: grid; gap: 14px; margin-bottom: 14px; padding: 16px; overflow: hidden; }
  .jobs-capture.drag-active { border-color: color-mix(in srgb, var(--accent-strong) 72%, var(--border)); box-shadow: 0 0 0 1px color-mix(in srgb, var(--accent-strong) 32%, transparent), var(--shadow-1); }
  .drop-overlay { position: absolute; inset: 0; z-index: 12; display: grid; place-items: center; padding: 20px; background: color-mix(in srgb, var(--surface) 90%, var(--accent-strong) 10%); backdrop-filter: blur(5px); pointer-events: none; }
  .drop-overlay > div { display: grid; gap: 7px; max-width: 420px; padding: 18px 22px; border: 1px dashed color-mix(in srgb, var(--accent-strong) 70%, var(--border)); border-radius: 12px; background: var(--surface); text-align: center; }
  .drop-overlay strong { color: var(--text); font-size: 14px; }
  .drop-overlay span { color: var(--muted); font-size: 11px; line-height: 1.5; }
  .capture-heading { display: grid; grid-template-columns: minmax(0, 1fr) minmax(260px, .8fr); column-gap: 24px; align-items: end; padding-bottom: 13px; border-bottom: 1px solid var(--border); }
  .capture-heading .eyebrow { grid-column: 1 / -1; }
  .capture-heading h2, .capture-heading p { margin: 0; }
  .capture-heading p { justify-self: end; max-width: 420px; font-size: 11px; text-align: right; }
  .source-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 10px; }
  .source-card { display: grid; gap: 12px; min-height: 150px; padding: 14px; border: 1px solid var(--border); border-radius: 11px; background: color-mix(in srgb, var(--surface-2) 45%, transparent); }
  .source-card.ready { border-color: color-mix(in srgb, var(--accent-strong) 38%, var(--border)); background: color-mix(in srgb, var(--accent-strong) 5%, var(--surface)); }
  .source-card.drop-ready:not(.ready) { border-style: dashed; }
  .source-label { display: flex; align-items: center; gap: 10px; }
  .source-label strong, .source-label small { display: block; }
  .source-label small { margin-top: 4px; color: var(--muted); font-size: 11px; }
  .source-icon { display: grid; place-items: center; width: 34px; height: 34px; border: 1px solid var(--border); border-radius: 9px; color: var(--accent); background: var(--surface); font-size: 18px; }
  .selected-path { max-width: 100%; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; padding: 8px 9px; border-radius: 7px; color: var(--muted); font-size: 10px; }
  .capture-actions, .job-actions { display: flex; flex-wrap: wrap; gap: 7px; align-self: end; }
  .single-action { justify-content: flex-end; }
  .url-capture { display: grid; gap: 7px; }
  .url-capture span { color: var(--muted); font-size: 11px; }
  .url-capture input { width: 100%; }
  .advanced-path { padding-top: 2px; color: var(--muted); font-size: 11px; }
  .advanced-path summary { width: fit-content; cursor: pointer; }
  .advanced-path div { display: grid; grid-template-columns: minmax(0, 1fr) auto; gap: 8px; margin-top: 10px; }
  .queue-message { margin: -4px 0 12px; padding: 8px 10px; border-inline-start: 2px solid var(--accent-strong); background: color-mix(in srgb, var(--accent-strong) 5%, transparent); }
  .error-message { border-color: var(--danger); }
  .queue-toolbar { display: flex; align-items: center; justify-content: space-between; gap: 12px; }
  .visible-count { color: var(--faint); font-family: var(--font-technical); font-size: 10px; }
  .job-actions { justify-content: flex-end; }

  @media (max-width: 900px) {
    .source-grid { grid-template-columns: 1fr; }
    .capture-heading { grid-template-columns: 1fr; }
    .capture-heading p { justify-self: start; text-align: left; }
  }
  @media (max-width: 700px) {
    .queue-summary { width: 100%; }
    .advanced-path div { grid-template-columns: 1fr; }
    .capture-actions, .job-actions { justify-content: flex-start; }
  }
</style>
