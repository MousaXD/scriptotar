<script lang="ts">
  import type { AiRun, Creator, Job, Project, Transcript } from '../types';
  import JobRow from '../components/JobRow.svelte';

  export let project: Project;
  export let creators: Creator[];
  export let jobs: Job[];
  export let transcripts: Transcript[];
  export let aiRuns: AiRun[];
  export let onNavigate: (view: 'research' | 'jobs' | 'transcript' | 'ai') => void;

  const activeStates = new Set(['queued','preparing','downloading','transcribing','processing']);
  $: active = jobs.filter((job) => activeStates.has(job.state));
  $: attention = jobs.filter((job) => ['failed', 'interrupted'].includes(job.state));
  $: currentJobs = [...attention, ...active.filter((job) => !attention.some((item) => item.id === job.id))].slice(0, 5);
</script>

<section class="view-head home-head">
  <div>
    <span class="eyebrow">Workspace overview</span>
    <h1>{project.name}</h1>
    <p>{project.description || 'Creator research, transcripts, and AI work in one local workspace.'}</p>
  </div>
  <div class="home-actions">
    <button class="button secondary" on:click={() => onNavigate('jobs')}>Jobs</button>
    <button class="button primary" on:click={() => onNavigate('research')}>New research scan</button>
  </div>
</section>

<div class="home-grid">
  <section class="panel work-panel">
    <div class="panel-head">
      <div><span class="eyebrow">Activity</span><h2>Current jobs</h2></div>
      <button class="text-button" on:click={() => onNavigate('jobs')}>Open queue →</button>
    </div>
    {#if currentJobs.length === 0}
      <button class="quiet-empty" on:click={() => onNavigate('jobs')}>
        <strong>Jobs</strong>
        <span>Open queue →</span>
      </button>
    {:else}
      <div class="job-stack">
        {#each currentJobs as job}<JobRow {job} compact />{/each}
      </div>
    {/if}
  </section>

  <section class="panel output-panel">
    <div class="panel-head">
      <div><span class="eyebrow">Transcript library</span><h2>Recent transcripts</h2></div>
      <button class="text-button" on:click={() => onNavigate('transcript')}>Open →</button>
    </div>
    <div class="recent-stack">
      {#each transcripts.slice(0, 4) as transcript}
        <button on:click={() => onNavigate('transcript')}>
          <span><strong>{transcript.title}</strong><small>{transcript.language} · {Math.round(transcript.durationSeconds)} sec</small></span>
          <span aria-hidden="true">›</span>
        </button>
      {/each}
      {#if transcripts.length === 0}
        <button class="quiet-empty" on:click={() => onNavigate('jobs')}><strong>Transcript</strong><span>Jobs →</span></button>
      {/if}
    </div>
  </section>

  <section class="panel creator-panel">
    <div class="panel-head"><div><span class="eyebrow">Signals</span><h2>Recent creators</h2></div></div>
    <div class="creator-stack">
      {#each creators.slice(0, 4) as creator}
        <button class="creator-row" on:click={() => onNavigate('research')}>
          {#if creator.avatar}<img class="creator-avatar" src={creator.avatar} alt="" />{:else}<span class="avatar-fallback">{creator.name.slice(0,1)}</span>{/if}
          <span><strong>{creator.name}</strong><small>{creator.handle} · {creator.platform}</small></span>
          {#if creator.watchlisted}<span class="watch-chip">Watching</span>{/if}
        </button>
      {/each}
    </div>
  </section>

  <section class="panel ai-panel">
    <div class="panel-head"><div><span class="eyebrow">AI Studio</span><h2>Recent work</h2></div><button class="text-button" on:click={() => onNavigate('ai')}>Create →</button></div>
    <div class="recent-stack">
      {#each aiRuns.slice(0, 4) as run}
        <button on:click={() => onNavigate('ai')}><span><strong>{run.title}</strong><small>{run.task} · {run.mode === 'copy' ? 'Copy Prompt' : run.provider}</small></span><span aria-hidden="true">›</span></button>
      {/each}
    </div>
  </section>
</div>

<section class="metric-grid compact-metrics" aria-label="Project summary">
  <article><span>Active jobs</span><strong>{active.length}</strong><small>{attention.length} needs attention</small></article>
  <article><span>Recent transcripts</span><strong>{transcripts.length}</strong><small>Transcript library</small></article>
  <article><span>Recent creators</span><strong>{creators.length}</strong><small>{creators.filter((creator) => creator.watchlisted).length} watchlisted</small></article>
  <article><span>AI work</span><strong>{aiRuns.length}</strong><small>prompt and BYOK runs</small></article>
</section>

<style>
  .home-head { align-items: center; }
  .home-actions { display: flex; flex-wrap: wrap; gap: 8px; }
  .home-grid { display: grid; grid-template-columns: minmax(0, 1.35fr) minmax(300px, .85fr); gap: 12px; }
  .home-grid .panel { padding: 17px; }
  .work-panel { grid-row: span 2; }
  .creator-panel, .ai-panel { min-height: 220px; }
  .recent-stack { display: grid; margin-top: 10px; }
  .recent-stack > button { display: grid; grid-template-columns: minmax(0, 1fr) auto; align-items: center; gap: 12px; width: 100%; min-height: 54px; padding: 10px 2px; border: 0; border-top: 1px solid var(--border); background: transparent; text-align: left; }
  .recent-stack strong, .recent-stack small { display: block; }
  .recent-stack strong { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .recent-stack small { margin-top: 4px; color: var(--muted); font-size: 11px; }
  .quiet-empty { display: flex !important; align-items: center; justify-content: space-between; width: 100%; min-height: 64px; margin-top: 10px; padding: 0 12px !important; border: 1px dashed var(--border-strong) !important; border-radius: 9px; color: var(--muted); background: transparent !important; text-align: left; }
  .quiet-empty strong { color: var(--text); }
  .creator-avatar { width: 32px; height: 32px; border-radius: 50%; object-fit: cover; background: var(--surface-2); }
  .compact-metrics { margin-top: 12px; margin-bottom: 0; }
  .compact-metrics article { padding: 13px 15px; }
  .compact-metrics strong { margin: 7px 0 3px; font-size: 22px; }

  @media (max-width: 980px) {
    .home-grid { grid-template-columns: 1fr; }
    .work-panel { grid-row: auto; }
  }
  @media (max-width: 700px) {
    .home-actions { width: 100%; }
    .home-actions .button { flex: 1; }
  }
</style>
