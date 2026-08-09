<script lang="ts">
  import type { Job } from '../types';
  import StateBadge from './StateBadge.svelte';
  export let job: Job;
  export let compact = false;
</script>
<article class:compact class="job-row" data-testid="job-{job.state}">
  <div class="job-main">
    <div class="job-title-row">
      <strong>{job.title}</strong>
      <StateBadge state={job.state} />
    </div>
    <div class="job-meta"><span>{job.source}</span><span>·</span><span>{job.updatedAt}</span>{#if job.detail}<span>·</span><span>{job.detail}</span>{/if}</div>
    {#if !compact && ['preparing','downloading','transcribing','processing'].includes(job.state)}
      {#if job.progress !== undefined}
        <div class="progress-line" aria-label={`${job.stageLabel} ${job.progress}%`}>
          <div class="progress-track"><div class="progress-fill" style={`width: ${job.progress}%`}></div></div>
          <span>{job.progress}%</span>
        </div>
      {:else}
        <div class="progress-unknown"><span class="pulse-dot"></span>{job.stageLabel} · progress not reported by worker</div>
      {/if}
    {/if}
  </div>
</article>
