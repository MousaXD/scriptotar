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
</script>
<section class="view-head">
  <div><span class="eyebrow">Workspace overview</span><h1>{project.name}</h1><p>{project.description || 'Creator research, transcripts, and AI work in one local workspace.'}</p></div>
  <button class="button primary" on:click={() => onNavigate('research')}>New research scan</button>
</section>
<section class="metric-grid" aria-label="Project summary">
  <article><span>Library items</span><strong>{project.itemCount}</strong><small>across research + transcripts</small></article>
  <article><span>Active jobs</span><strong>{jobs.filter((job) => activeStates.has(job.state)).length}</strong><small>queue keeps moving after failures</small></article>
  <article><span>Recent creators</span><strong>{creators.length}</strong><small>{creators.filter((creator) => creator.watchlisted).length} watchlisted</small></article>
  <article><span>AI work</span><strong>{aiRuns.length}</strong><small>prompt and BYOK runs</small></article>
</section>
<div class="dashboard-grid">
  <section class="panel span-2">
    <div class="panel-head"><div><span class="eyebrow">Activity</span><h2>Current jobs</h2></div><button class="text-button" on:click={() => onNavigate('jobs')}>Open queue →</button></div>
    <div class="job-stack">{#each jobs.slice(0, 4) as job}<JobRow {job} compact />{/each}</div>
  </section>
  <section class="panel">
    <div class="panel-head"><div><span class="eyebrow">Signals</span><h2>Recent creators</h2></div></div>
    <div class="creator-stack">{#each creators as creator}<button class="creator-row" on:click={() => onNavigate('research')}><span class="avatar-fallback">{creator.name.slice(0,1)}</span><span><strong>{creator.name}</strong><small>{creator.handle} · {creator.platform}</small></span>{#if creator.watchlisted}<span class="watch-chip">Watching</span>{/if}</button>{/each}</div>
  </section>
  <section class="panel">
    <div class="panel-head"><div><span class="eyebrow">Transcript library</span><h2>Recent transcripts</h2></div><button class="text-button" on:click={() => onNavigate('transcript')}>Open →</button></div>
    <div class="simple-list">{#each transcripts as transcript}<button on:click={() => onNavigate('transcript')}><strong>{transcript.title}</strong><small>{transcript.language} · {Math.round(transcript.durationSeconds)} sec</small></button>{/each}</div>
  </section>
  <section class="panel">
    <div class="panel-head"><div><span class="eyebrow">AI Studio</span><h2>Recent work</h2></div><button class="text-button" on:click={() => onNavigate('ai')}>Create →</button></div>
    <div class="simple-list">{#each aiRuns as run}<button on:click={() => onNavigate('ai')}><strong>{run.title}</strong><small>{run.task} · {run.mode === 'copy' ? 'Copy Prompt' : run.provider}</small></button>{/each}</div>
  </section>
</div>
