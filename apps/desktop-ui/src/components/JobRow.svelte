<script lang="ts">
  import type { Job } from '../types';
  import { translator } from '../i18n/translate';
  import StateBadge from './StateBadge.svelte';
  export let job: Job;
  export let compact = false;

  $: safeProgress = typeof job.progress === 'number' && Number.isFinite(job.progress)
    ? Math.max(0, Math.min(100, job.progress))
    : undefined;

  $: recovery = job.state === 'interrupted'
    ? $translator('job.interruptedHelp')
    : job.state === 'failed'
      ? $translator('job.failedHelp')
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
    {#if recovery && !compact}<p class="job-guidance" data-i18n-ignore>{recovery}</p>{/if}
    {#if !compact && ['preparing','downloading','transcribing','processing'].includes(job.state)}
      {#if safeProgress !== undefined}
        <div class="progress-line" role="progressbar" aria-label={job.stageLabel} aria-valuemin="0" aria-valuemax="100" aria-valuenow={safeProgress}>
          <div class="progress-track"><div class="progress-fill" style={`width: ${safeProgress}%`}></div></div>
          <span>{safeProgress}%</span>
        </div>
      {:else}
        <div class="progress-unknown" role="status"><span class="pulse-dot"></span>{job.stageLabel} · {$translator('job.unknownProgress')}</div>
      {/if}
    {/if}
  </div>
</article>

<style>
  .job-detail { margin: 7px 0 0; color: var(--muted); font-size: 10px; line-height: 1.45; overflow-wrap: anywhere; }
  .job-guidance { margin: 5px 0 0; color: var(--faint); font-size: 10px; line-height: 1.45; }
</style>
