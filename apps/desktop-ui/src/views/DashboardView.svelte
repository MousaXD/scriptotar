<script lang="ts">
  import type { AiRun, Creator, Job, Project, Transcript } from '../types';
  import { translator } from '../i18n/translate';
  import JobRow from '../components/JobRow.svelte';

  export let project: Project;
  export let creators: Creator[];
  export let jobs: Job[];
  export let transcripts: Transcript[];
  export let aiRuns: AiRun[];
  export let onNavigate: (view: 'research' | 'jobs' | 'transcript' | 'ai') => void;
  export let onCreateProject: (name: string) => Promise<void>;
  export let onOpenTranscript: (id: string) => void;

  const activeStates = new Set(['queued','preparing','downloading','transcribing','processing']);
  let projectFormOpen = false;
  let projectName = '';
  let projectCreating = false;
  let projectError = '';

  $: active = jobs.filter((job) => activeStates.has(job.state));
  $: attention = jobs.filter((job) => ['failed', 'interrupted'].includes(job.state));
  $: currentJobs = [...attention, ...active.filter((job) => !attention.some((item) => item.id === job.id))].slice(0, 5);

  function openProjectForm() {
    projectError = '';
    projectFormOpen = true;
  }

  function closeProjectForm() {
    if (projectCreating) return;
    projectFormOpen = false;
    projectName = '';
    projectError = '';
  }

  async function createProject() {
    const name = projectName.trim();
    if (!name) {
      projectError = $translator('project.required');
      return;
    }
    projectCreating = true;
    projectError = '';
    try {
      await onCreateProject(name);
      projectFormOpen = false;
      projectName = '';
    } catch (cause) {
      projectError = cause instanceof Error ? cause.message : $translator('project.createFailed');
    } finally {
      projectCreating = false;
    }
  }
</script>

<section class="view-head home-head" data-i18n-ignore>
  <div>
    <span class="eyebrow">{$translator('home.overview')}</span>
    <h1>{project.name}</h1>
    <p>{project.description || $translator('home.description')}</p>
  </div>
  <div class="home-actions">
    <button class="button secondary" on:click={openProjectForm}>{$translator('project.new')}</button>
    <button class="button secondary" on:click={() => onNavigate('jobs')}>{$translator('home.jobs')}</button>
    <button class="button primary" on:click={() => onNavigate('research')}>{$translator('home.newResearch')}</button>
  </div>
</section>

{#if projectFormOpen}
  <section class="panel project-create-panel" data-i18n-ignore aria-label={$translator('project.createTitle')}>
    <div class="panel-head">
      <div><span class="eyebrow">{$translator('project.label')}</span><h2>{$translator('project.createTitle')}</h2></div>
    </div>
    <form class="project-create-form" on:submit|preventDefault={createProject}>
      <label>
        <span>{$translator('project.name')}</span>
        <input
          bind:value={projectName}
          maxlength="120"
          autocomplete="off"
          disabled={projectCreating}
          placeholder={$translator('project.namePlaceholder')}
          aria-label={$translator('project.name')}
        />
      </label>
      {#if projectError}<p class="project-error" role="alert">{projectError}</p>{/if}
      <div class="project-form-actions">
        <button class="button secondary" type="button" disabled={projectCreating} on:click={closeProjectForm}>{$translator('project.cancel')}</button>
        <button class="button primary" type="submit" disabled={projectCreating}>{projectCreating ? $translator('project.creating') : $translator('project.create')}</button>
      </div>
    </form>
  </section>
{/if}

<div class="home-grid">
  <section class="panel work-panel">
    <div class="panel-head">
      <div><span class="eyebrow">{$translator('home.activity')}</span><h2>{$translator('home.currentJobs')}</h2></div>
      <button class="text-button" on:click={() => onNavigate('jobs')}>{$translator('home.openQueue')}</button>
    </div>
    {#if currentJobs.length === 0}
      <button class="quiet-empty" data-i18n-ignore on:click={() => onNavigate('jobs')}>
        <strong>{$translator('home.jobs')}</strong>
        <span>{$translator('home.openQueue')}</span>
      </button>
    {:else}
      <div class="job-stack">
        {#each currentJobs as job}<JobRow {job} compact />{/each}
      </div>
    {/if}
  </section>

  <section class="panel output-panel" data-i18n-ignore>
    <div class="panel-head">
      <div><span class="eyebrow">{$translator('home.transcriptLibrary')}</span><h2>{$translator('home.recentTranscripts')}</h2></div>
      <button class="text-button" on:click={() => onNavigate('transcript')}>{$translator('home.open')}</button>
    </div>
    <div class="recent-stack">
      {#each transcripts.slice(0, 4) as transcript}
        <button on:click={() => onOpenTranscript(transcript.id)}>
          <span><strong>{transcript.title}</strong><small>{transcript.language} · {$translator('home.seconds', { count: Math.round(transcript.durationSeconds) })}</small></span>
          <span aria-hidden="true">›</span>
        </button>
      {/each}
      {#if transcripts.length === 0}
        <button class="quiet-empty" on:click={() => onNavigate('jobs')}><strong>{$translator('home.transcript')}</strong><span>{$translator('home.jobs')} →</span></button>
      {/if}
    </div>
  </section>

  <section class="panel creator-panel" data-i18n-ignore>
    <div class="panel-head"><div><span class="eyebrow">{$translator('home.signals')}</span><h2>{$translator('home.recentCreators')}</h2></div></div>
    <div class="creator-stack">
      {#each creators.slice(0, 4) as creator}
        <button class="creator-row" on:click={() => onNavigate('research')}>
          {#if creator.avatar}<img class="creator-avatar" src={creator.avatar} alt="" />{:else}<span class="avatar-fallback">{creator.name.slice(0,1)}</span>{/if}
          <span><strong>{creator.name}</strong><small>{creator.handle} · {creator.platform}</small></span>
          {#if creator.watchlisted}<span class="watch-chip">{$translator('home.watching')}</span>{/if}
        </button>
      {/each}
    </div>
  </section>

  <section class="panel ai-panel" data-i18n-ignore>
    <div class="panel-head"><div><span class="eyebrow">{$translator('nav.ai')}</span><h2>{$translator('home.recentWork')}</h2></div><button class="text-button" on:click={() => onNavigate('ai')}>{$translator('home.create')}</button></div>
    <div class="recent-stack">
      {#each aiRuns.slice(0, 4) as run}
        <button on:click={() => onNavigate('ai')}><span><strong>{run.title}</strong><small>{run.task} · {run.mode === 'copy' ? $translator('home.copyPrompt') : run.provider}</small></span><span aria-hidden="true">›</span></button>
      {/each}
    </div>
  </section>
</div>

<section class="metric-grid compact-metrics" data-i18n-ignore aria-label={$translator('home.projectSummary')}>
  <article><span>{$translator('home.activeJobs')}</span><strong>{active.length}</strong><small>{$translator('home.needsAttention', { count: attention.length })}</small></article>
  <article><span>{$translator('home.recentTranscripts')}</span><strong>{transcripts.length}</strong><small>{$translator('home.transcriptLibrary')}</small></article>
  <article><span>{$translator('home.recentCreators')}</span><strong>{creators.length}</strong><small>{$translator('home.watchlisted', { count: creators.filter((creator) => creator.watchlisted).length })}</small></article>
  <article><span>{$translator('home.aiWork')}</span><strong>{aiRuns.length}</strong><small>{$translator('home.promptRuns')}</small></article>
</section>

<style>
  .home-head { align-items: center; }
  .home-actions { display: flex; flex-wrap: wrap; gap: 8px; }
  .project-create-panel { margin-bottom: 12px; }
  .project-create-form { display: grid; gap: 12px; max-width: 620px; }
  .project-create-form label { display: grid; gap: 7px; color: var(--muted); font-size: 12px; font-weight: 650; }
  .project-create-form input { width: 100%; }
  .project-form-actions { display: flex; justify-content: flex-end; gap: 8px; }
  .project-error { margin: 0; color: var(--danger, #f87171); font-size: 12px; }
  .home-grid { display: grid; grid-template-columns: minmax(0, 1.35fr) minmax(300px, .85fr); gap: 12px; }
  .home-grid .panel { padding: 17px; }
  .work-panel { grid-row: span 2; }
  .creator-panel, .ai-panel { min-height: 220px; }
  .recent-stack { display: grid; margin-top: 10px; }
  .recent-stack > button { display: grid; grid-template-columns: minmax(0, 1fr) auto; align-items: center; gap: 12px; width: 100%; min-height: 54px; padding: 10px 2px; border: 0; border-top: 1px solid var(--border); background: transparent; text-align: start; }
  .recent-stack strong, .recent-stack small { display: block; }
  .recent-stack strong { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .recent-stack small { margin-top: 4px; color: var(--muted); font-size: 11px; }
  .quiet-empty { display: flex !important; align-items: center; justify-content: space-between; width: 100%; min-height: 64px; margin-top: 10px; padding: 0 12px !important; border: 1px dashed var(--border-strong) !important; border-radius: 9px; color: var(--muted); background: transparent !important; text-align: start; }
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
    .project-form-actions { flex-direction: column-reverse; }
    .project-form-actions .button { width: 100%; }
  }
</style>
