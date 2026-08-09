<script lang="ts">
  import type { Job, JobState } from '../types';
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
  const filters: { id: 'all' | 'active' | 'attention' | 'done'; label: string }[] = [
    { id: 'all', label: 'All' }, { id: 'active', label: 'Active' }, { id: 'attention', label: 'Needs attention' }, { id: 'done', label: 'Finished' }
  ];
  const active = new Set<JobState>(['queued','preparing','downloading','transcribing','processing']);
  $: visible = jobs.filter((job) => filter === 'all' || (filter === 'active' && active.has(job.state)) || (filter === 'attention' && ['failed','interrupted'].includes(job.state)) || (filter === 'done' && ['completed','cancelled'].includes(job.state)));

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
</script>

<section class="view-head"><div><span class="eyebrow">Persistent activity</span><h1>Jobs</h1><p>Choose local media with the desktop picker or queue a supported URL. Rust still validates every path and owns queue state.</p></div></section>
<section class="panel jobs-capture" aria-label="Add transcription job">
  <div class="local-file-choice">
    <span class="field-label">Local media</span>
    <strong>{localPath ? displayName(localPath) : 'No video selected'}</strong>
    <small>{localPath || 'Use the native desktop picker for normal operation.'}</small>
  </div>
  <div class="capture-actions">
    <button class="button secondary" disabled={busy} on:click={chooseLocal}>Choose video</button>
    <button class="button primary" disabled={busy || !localPath} on:click={() => run(async () => { await onEnqueueLocal(localPath); actionStatus = `Queued ${displayName(localPath)}.`; localPath = ''; })}>Queue selected</button>
  </div>
  <label class="url-capture"><span>Supported media URL</span><input aria-label="Media URL" bind:value={sourceUrl} placeholder="https://youtube.com/…" /></label>
  <button class="button secondary" disabled={busy || !sourceUrl.trim()} on:click={() => run(async () => { const queued = sourceUrl.trim(); await onEnqueueUrl(queued); sourceUrl = ''; actionStatus = 'URL queued.'; })}>Queue URL</button>
  <details class="advanced-path">
    <summary>Advanced: enter a local path manually</summary>
    <div><input aria-label="Manual local media path" bind:value={manualPath} placeholder="/home/me/video.mp4" /><button class="button secondary" disabled={busy || !manualPath.trim()} on:click={() => run(async () => { await onEnqueueLocal(manualPath.trim()); actionStatus = `Queued ${displayName(manualPath.trim())}.`; manualPath = ''; })}>Queue path</button></div>
  </details>
</section>
{#if actionError}<p class="status-copy" role="alert">{actionError}</p>{/if}
{#if actionStatus}<p class="status-copy" aria-live="polite">{actionStatus}</p>{/if}
<div class="segmented" role="group" aria-label="Job filters">
  {#each filters as item}<button class:active={filter === item.id} aria-pressed={filter === item.id} on:click={() => filter = item.id}>{item.label}</button>{/each}
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
