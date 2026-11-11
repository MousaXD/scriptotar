<script lang="ts">
  import type { Job, JobState } from '../types';
  import { translator } from '../i18n/translate';
  import StateBadge from './StateBadge.svelte';

  export let job: Job;
  export let compact = false;

  const runningStates = new Set<JobState>(['preparing', 'downloading', 'transcribing', 'processing']);
  $: running = runningStates.has(job.state);
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
      <strong dir="auto" title={job.title}>{job.title}</strong>
      <StateBadge state={job.state} />
    </div>
    <div class="job-meta">
      <span class="job-source" dir="auto" title={job.source}>{job.source}</span>
      <span aria-hidden="true">·</span>
      <span>{job.updatedAt}</span>
    </div>
    {#if job.detail && (!compact || job.state === 'failed' || job.state === 'interrupted')}
      <p class="job-detail" dir="auto">{job.detail}</p>
    {/if}
    {#if recovery && !compact}<p class="job-guidance" data-i18n-ignore>{recovery}</p>{/if}
    {#if running}
      {#if safeProgress !== undefined}
        <div class="progress-line" role="progressbar" aria-label={job.stageLabel || $translator('queue.filterActive')} aria-valuemin="0" aria-valuemax="100" aria-valuenow={safeProgress}>
          <div class="progress-caption"><span>{job.stageLabel}</span><span>{Math.round(safeProgress)}%</span></div>
          <div class="progress-track"><div class="progress-fill" style={`width: ${safeProgress}%`}></div></div>
        </div>
      {:else}
        <div class="progress-unknown" role="status"><span class="pulse-dot" aria-hidden="true"></span>{job.stageLabel} · {$translator('job.unknownProgress')}</div>
      {/if}
    {:else if job.state === 'queued' && job.stageLabel}
      <p class="queue-stage">{job.stageLabel}</p>
    {/if}
  </div>
</article>

<style>
  .job-row { min-width: 0; padding: var(--space-3) var(--space-1); border-block-start: 1px solid var(--color-border); }
  .job-row:first-child { border-block-start: 0; }
  .job-row.compact { padding-block: 11px; }
  .job-main { min-width: 0; }
  .job-title-row { display: flex; justify-content: space-between; align-items: flex-start; gap: var(--space-3); }
  .job-title-row strong { min-width: 0; overflow-wrap: anywhere; color: var(--color-text); font-size: var(--text-md); line-height: 1.45; }
  .job-title-row :global(.state-badge) { flex: none; font-size: var(--text-xs); }
  .job-meta { display: flex; align-items: center; flex-wrap: wrap; gap: var(--space-2); margin-top: var(--space-1); color: var(--color-text-muted); font-size: var(--text-xs); }
  .job-source { display: inline-block; max-width: min(100%, 520px); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .job-detail { margin: var(--space-2) 0 0; color: var(--color-text-muted); font-size: var(--text-sm); line-height: 1.5; white-space: pre-wrap; overflow-wrap: anywhere; }
  .job-guidance { margin: var(--space-1) 0 0; color: var(--color-text-faint); font-size: var(--text-xs); line-height: 1.5; }
  .progress-line { display: grid; gap: var(--space-1); margin-top: var(--space-2); }
  .progress-caption { display: flex; justify-content: space-between; gap: var(--space-2); color: var(--color-text-muted); font-size: var(--text-xs); }
  .progress-track { height: 5px; overflow: hidden; border-radius: var(--radius-pill); background: var(--color-surface-strong); }
  .progress-fill { height: 100%; border-radius: inherit; background: var(--color-info); }
  .progress-unknown, .queue-stage { display: flex; align-items: center; gap: var(--space-1); margin: var(--space-2) 0 0; color: var(--color-text-muted); font-size: var(--text-xs); }
  .pulse-dot { flex: none; width: 6px; height: 6px; border-radius: 50%; background: var(--color-info); }
  .compact .job-title-row strong { font-size: var(--text-sm); }
  .compact .job-detail { font-size: var(--text-xs); }
</style>
