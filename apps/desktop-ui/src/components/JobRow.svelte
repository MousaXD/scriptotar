<script lang="ts">
  import type { Job } from '../types';
  import StateBadge from './StateBadge.svelte';
  export let job: Job;
  export let compact = false;

  $: recovery = job.state === 'interrupted'
    ? 'Scriptotar stopped before this job finished. Retry starts a new attempt; it does not pretend to resume the old process.'
    : job.state === 'failed'
      ? 'Review the failure detail, fix the likely cause, then retry the job.'
      : '';
</script>
<article class:compact class="job-row" data-testid="job-{job.state}">
  <div class="job-main">
    <div class="job-title-row">
      <strong>{job.title}</strong>
      <StateBadge state={job.state} />
    </div>
    <div class="job-meta"><span>{job.source}</span><span>·</span><span>{job.updatedAt}</span></div>
    {#if job.detail}<p class="job-detail">{job.detail}</p>{/if}
    {#if recovery && !compact}<p class="job-guidance">{recovery}</p>{/if}
    {#if !compact && ['preparing','downloading','transcribing','processing'].includes(job.state)}
      {#if job.progress !== undefined}
        <div class="progress-line" role="progressbar" aria-label={job.stageLabel} aria-valuemin="0" aria-valuemax="100" aria-valuenow={job.progress}>
          <div class="progress-track"><div class="progress-fill" style={`width: ${job.progress}%`}></div></div>
          <span>{job.progress}%</span>
        </div>
      {:else}
        <div class="progress-unknown" role="status"><span class="pulse-dot"></span>{job.stageLabel} · progress not reported by worker</div>
      {/if}
    {/if}
  </div>
</article>

<style>
  .job-detail { margin: 7px 0 0; color: var(--muted); font-size: 10px; line-height: 1.45; overflow-wrap: anywhere; }
  .job-guidance { margin: 5px 0 0; color: var(--faint); font-size: 10px; line-height: 1.45; }
</style>
