<script lang="ts">
  import type { Job, JobState } from '../types';
  import JobRow from '../components/JobRow.svelte';
  export let jobs: Job[];
  export let onCancel: (jobId: string) => Promise<void> | void;
  let filter: 'all' | 'active' | 'attention' | 'done' = 'all';
  const filters: { id: 'all' | 'active' | 'attention' | 'done'; label: string }[] = [
    { id: 'all', label: 'All' }, { id: 'active', label: 'Active' }, { id: 'attention', label: 'Needs attention' }, { id: 'done', label: 'Finished' }
  ];
  const active = new Set<JobState>(['queued','preparing','downloading','transcribing','processing']);
  $: visible = jobs.filter((job) => filter === 'all' || (filter === 'active' && active.has(job.state)) || (filter === 'attention' && ['failed','interrupted'].includes(job.state)) || (filter === 'done' && ['completed','cancelled'].includes(job.state)));
</script>
<section class="view-head"><div><span class="eyebrow">Persistent activity</span><h1>Jobs</h1><p>Real state from the queue, with unknown progress shown as unknown instead of invented percentages.</p></div></section>
<div class="segmented" role="group" aria-label="Job filters">
  {#each filters as item}<button class:active={filter === item.id} on:click={() => filter = item.id}>{item.label}</button>{/each}
</div>
<section class="panel jobs-panel">
  {#each visible as job}<div class="job-with-actions"><JobRow {job} />{#if ['preparing','downloading','transcribing','processing'].includes(job.state)}<button class="button danger subtle" on:click={() => onCancel(job.id)}>Cancel</button>{/if}</div>{/each}
</section>
