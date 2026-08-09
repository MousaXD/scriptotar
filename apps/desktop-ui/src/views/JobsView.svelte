<script lang="ts">
  import type { Job, JobState } from '../types';
  import JobRow from '../components/JobRow.svelte';
  export let jobs: Job[];
  export let onCancel: (jobId: string) => Promise<void> | void;
  export let onRetry: (jobId: string) => Promise<void> | void;
  export let onEnqueueLocal: (path: string) => Promise<void> | void;
  export let onEnqueueUrl: (url: string) => Promise<void> | void;
  let filter: 'all' | 'active' | 'attention' | 'done' = 'all';
  let localPath = '';
  let sourceUrl = '';
  let actionError = '';
  let busy = false;
  const filters: { id: 'all' | 'active' | 'attention' | 'done'; label: string }[] = [
    { id: 'all', label: 'All' }, { id: 'active', label: 'Active' }, { id: 'attention', label: 'Needs attention' }, { id: 'done', label: 'Finished' }
  ];
  const active = new Set<JobState>(['queued','preparing','downloading','transcribing','processing']);
  $: visible = jobs.filter((job) => filter === 'all' || (filter === 'active' && active.has(job.state)) || (filter === 'attention' && ['failed','interrupted'].includes(job.state)) || (filter === 'done' && ['completed','cancelled'].includes(job.state)));

  async function run(action: () => Promise<void> | void) {
    busy = true;
    actionError = '';
    try { await action(); }
    catch (error) { actionError = error instanceof Error ? error.message : String(error); }
    finally { busy = false; }
  }
</script>
<section class="view-head"><div><span class="eyebrow">Persistent activity</span><h1>Jobs</h1><p>Real state from the Rust-owned queue, with unknown progress shown as unknown instead of invented percentages.</p></div></section>
<section class="research-capture panel" aria-label="Add transcription job">
  <label><span>Local media path</span><input aria-label="Local media path" bind:value={localPath} placeholder="/home/me/video.mp4" /></label>
  <button class="button primary" disabled={busy || !localPath.trim()} on:click={() => run(async () => { await onEnqueueLocal(localPath.trim()); localPath = ''; })}>Queue local media</button>
  <label><span>Supported media URL</span><input aria-label="Media URL" bind:value={sourceUrl} placeholder="https://youtube.com/…" /></label>
  <button class="button secondary" disabled={busy || !sourceUrl.trim()} on:click={() => run(async () => { await onEnqueueUrl(sourceUrl.trim()); sourceUrl = ''; })}>Queue URL</button>
</section>
{#if actionError}<p class="status-copy" role="alert">{actionError}</p>{/if}
<div class="segmented" role="group" aria-label="Job filters">
  {#each filters as item}<button class:active={filter === item.id} on:click={() => filter = item.id}>{item.label}</button>{/each}
</div>
<section class="panel jobs-panel">
  {#each visible as job}
    <div class="job-with-actions">
      <JobRow {job} />
      {#if active.has(job.state)}<button class="button danger subtle" on:click={() => run(() => onCancel(job.id))}>Cancel</button>{/if}
      {#if ['failed','interrupted','cancelled'].includes(job.state)}<button class="button secondary subtle" on:click={() => run(() => onRetry(job.id))}>Retry</button>{/if}
    </div>
  {/each}
</section>
