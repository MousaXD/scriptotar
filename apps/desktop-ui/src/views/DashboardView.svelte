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

  const activeStates = new Set(['queued', 'preparing', 'downloading', 'transcribing', 'processing']);
  let projectFormOpen = false;
  let projectName = '';
  let projectCreating = false;
  let projectError = '';

  $: active = jobs.filter((job) => activeStates.has(job.state));
  $: attention = jobs.filter((job) => job.state === 'failed' || job.state === 'interrupted');
  $: currentJobs = [...attention, ...active].slice(0, 6);

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

<header class="dashboard-header" data-i18n-ignore>
  <div class="project-heading">
    <h1 dir="auto">{project.name}</h1>
    {#if project.description}<p dir="auto">{project.description}</p>{/if}
  </div>
  <div class="header-actions">
    <button type="button" class="button primary" on:click={() => onNavigate('jobs')}>
      {$translator('queue.addJob')}
    </button>
    <button type="button" class="button secondary" on:click={openProjectForm}>{$translator('project.new')}</button>
    <button type="button" class="text-button" on:click={() => onNavigate('research')}>{$translator('home.newResearch')}</button>
  </div>
</header>

{#if projectFormOpen}
  <section class="project-create" aria-label={$translator('project.createTitle')} data-i18n-ignore>
    <form on:submit|preventDefault={createProject}>
      <label for="dashboard-project-name">{$translator('project.name')}</label>
      <input id="dashboard-project-name" bind:value={projectName} maxlength="120" autocomplete="off" disabled={projectCreating} placeholder={$translator('project.namePlaceholder')} />
      {#if projectError}<p role="alert" class="project-error">{projectError}</p>{/if}
      <div class="project-form-actions">
        <button type="button" class="button secondary" on:click={closeProjectForm} disabled={projectCreating}>{$translator('project.cancel')}</button>
        <button type="submit" class="button primary" disabled={projectCreating}>{projectCreating ? $translator('project.creating') : $translator('project.create')}</button>
      </div>
    </form>
  </section>
{/if}

<div class="workspace-primary" data-i18n-ignore>
  <section class="workspace-section jobs-section" aria-label={$translator('home.currentJobs')}>
    <div class="section-heading">
      <div class="heading-with-count">
        <h2>{$translator('home.currentJobs')}</h2>
        {#if attention.length > 0}<span class="attention-indicator">{$translator('home.needsAttention', { count: attention.length })}</span>{/if}
        {#if active.length > 0}<span class="active-indicator">{active.length} {$translator('queue.filterActive')}</span>{/if}
      </div>
      <button type="button" class="text-button" on:click={() => onNavigate('jobs')}>{$translator('home.openQueue')}</button>
    </div>
    {#if currentJobs.length}
      <div class="dashboard-jobs">
        {#each currentJobs as job (job.id)}<JobRow {job} compact />{/each}
      </div>
    {:else}
      <button type="button" class="empty-action" on:click={() => onNavigate('jobs')}>
        <strong>{$translator('queue.emptyTitle')}</strong>
        <span>{$translator('queue.addJob')}</span>
      </button>
    {/if}
  </section>

  <section class="workspace-section transcript-section" aria-label={$translator('home.recentTranscripts')}>
    <div class="section-heading">
      <h2>{$translator('home.recentTranscripts')}</h2>
      <button type="button" class="text-button" on:click={() => onNavigate('transcript')}>{$translator('home.open')}</button>
    </div>
    {#if transcripts.length}
      <div class="transcript-list">
        {#each transcripts.slice(0, 6) as transcript (transcript.id)}
          <button type="button" class="transcript-item" on:click={() => onOpenTranscript(transcript.id)}>
            <span class="transcript-copy">
              <strong dir="auto" title={transcript.title}>{transcript.title}</strong>
              <small>{transcript.language} · {$translator('home.seconds', { count: Math.round(transcript.durationSeconds) })}</small>
            </span>
            <span class="row-chevron" aria-hidden="true">›</span>
          </button>
        {/each}
      </div>
    {:else}
      <button type="button" class="empty-action" on:click={() => onNavigate('jobs')}>
        <strong>{$translator('home.transcript')}</strong><span>{$translator('queue.addJob')}</span>
      </button>
    {/if}
  </section>
</div>

<div class="workspace-secondary" data-i18n-ignore>
  <section class="workspace-section" aria-label={$translator('home.recentCreators')}>
    <div class="section-heading"><h2>{$translator('home.recentCreators')}</h2><button type="button" class="text-button" on:click={() => onNavigate('research')}>{$translator('home.newResearch')}</button></div>
    {#if creators.length}
      <div class="secondary-list">
        {#each creators.slice(0, 3) as creator (creator.id)}
          <button type="button" class="creator-item" on:click={() => onNavigate('research')}>
            {#if creator.avatar}<img class="creator-avatar" alt="" src={creator.avatar} />{:else}<span class="avatar-fallback" aria-hidden="true">{creator.name.slice(0, 1)}</span>{/if}
            <span class="secondary-copy"><strong dir="auto">{creator.name}</strong><small dir="auto">{creator.handle} · {creator.platform}</small></span>
            {#if creator.watchlisted}<span class="watch-chip">{$translator('home.watching')}</span>{/if}
          </button>
        {/each}
      </div>
    {:else}
      <button type="button" class="empty-link" on:click={() => onNavigate('research')}>{$translator('home.newResearch')}</button>
    {/if}
  </section>
  <section class="workspace-section" aria-label={$translator('home.recentWork')}>
    <div class="section-heading"><h2>{$translator('home.recentWork')}</h2><button type="button" class="text-button" on:click={() => onNavigate('ai')}>{$translator('home.create')}</button></div>
    {#if aiRuns.length}
      <div class="secondary-list">
        {#each aiRuns.slice(0, 3) as run (run.id)}
          <button type="button" class="ai-item" on:click={() => onNavigate('ai')}>
            <span class="secondary-copy"><strong dir="auto">{run.title}</strong><small>{run.task} · {run.mode === 'copy' ? $translator('home.copyPrompt') : run.provider}</small></span>
            <span class="row-chevron" aria-hidden="true">›</span>
          </button>
        {/each}
      </div>
    {:else}
      <button type="button" class="empty-link" on:click={() => onNavigate('ai')}>{$translator('home.create')}</button>
    {/if}
  </section>
</div>

<style>
  .dashboard-header { display: flex; align-items: center; justify-content: space-between; gap: var(--space-4); margin-bottom: var(--space-5); }
  .project-heading { min-width: 0; }
  .project-heading h1 { margin: 0; overflow-wrap: anywhere; font-size: var(--text-xl); line-height: 1.25; }
  .project-heading p { margin: 5px 0 0; color: var(--color-text-muted); font-size: var(--text-sm); }
  .header-actions, .project-form-actions { display: flex; align-items: center; flex-wrap: wrap; gap: var(--space-2); }
  .header-actions { justify-content: flex-end; }
  .header-actions .text-button { min-height: var(--control-height); padding-inline: var(--space-2); }
  .project-create { margin-bottom: var(--space-4); padding: var(--space-4); border: 1px solid var(--color-border); border-radius: var(--radius-md); background: var(--color-surface); }
  .project-create form { display: grid; gap: var(--space-2); max-width: 600px; }
  .project-create label { color: var(--color-text-muted); font-size: var(--text-sm); }
  .project-create input { width: 100%; }
  .project-form-actions { justify-content: flex-end; margin-top: var(--space-2); }
  .project-error { margin: 0; color: var(--color-danger); font-size: var(--text-sm); }
  .workspace-primary { display: grid; grid-template-columns: minmax(0, 1.35fr) minmax(280px, 1fr); gap: var(--space-5); }
  .workspace-secondary { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: var(--space-5); margin-top: var(--space-5); }
  .workspace-section { min-width: 0; padding: var(--space-3) 0; border-block-start: 1px solid var(--color-border); }
  .section-heading, .heading-with-count { display: flex; align-items: center; flex-wrap: wrap; gap: var(--space-2); }
  .section-heading { justify-content: space-between; min-height: var(--control-height); margin-bottom: var(--space-2); }
  .section-heading h2 { margin: 0; font-size: var(--text-md); font-weight: 700; }
  .section-heading .text-button { min-height: 34px; padding-inline: var(--space-1); white-space: nowrap; }
  .attention-indicator, .active-indicator { padding: 3px 7px; border-radius: var(--radius-sm); font-size: var(--text-xs); font-weight: 650; }
  .attention-indicator { color: var(--color-warning); background: color-mix(in srgb, var(--color-warning) 10%, transparent); }
  .active-indicator { color: var(--color-info); background: color-mix(in srgb, var(--color-info) 10%, transparent); }
  .dashboard-jobs { min-width: 0; }
  .transcript-item, .creator-item, .ai-item { display: flex; align-items: center; gap: var(--space-3); width: 100%; min-width: 0; min-height: 58px; padding: var(--space-2) var(--space-1); border: 0; border-block-start: 1px solid var(--color-border); background: transparent; text-align: start; }
  .transcript-item:first-child, .creator-item:first-child, .ai-item:first-child { border-block-start: 0; }
  .transcript-item:hover, .creator-item:hover, .ai-item:hover { background: var(--color-surface-raised); }
  .transcript-copy, .secondary-copy { display: block; min-width: 0; flex: 1; }
  .transcript-copy strong, .secondary-copy strong { display: block; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; color: var(--color-text); font-size: var(--text-md); }
  .transcript-copy small, .secondary-copy small { display: block; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; margin-top: var(--space-1); color: var(--color-text-muted); font-size: var(--text-xs); }
  .row-chevron { color: var(--color-text-faint); font-size: var(--text-lg); }
  .creator-avatar, .avatar-fallback { flex: none; width: 30px; height: 30px; border-radius: var(--radius-sm); object-fit: cover; background: var(--color-surface-raised); }
  .avatar-fallback { display: grid; place-items: center; color: var(--color-signal); font-weight: 700; }
  .watch-chip { flex: none; color: var(--color-signal); font-size: var(--text-xs); }
  .empty-action, .empty-link { display: flex; justify-content: space-between; align-items: center; gap: var(--space-3); width: 100%; min-height: 68px; padding: var(--space-3); border: 1px dashed var(--color-border-strong); border-radius: var(--radius-md); background: transparent; text-align: start; }
  .empty-action strong { color: var(--color-text); font-size: var(--text-sm); }
  .empty-action span, .empty-link { color: var(--color-text-muted); font-size: var(--text-sm); }
  .empty-link { justify-content: flex-start; min-height: 48px; }
  .empty-action:hover, .empty-link:hover { border-color: var(--color-signal); color: var(--color-text); }
  @media (max-width: 1050px) { .workspace-primary { grid-template-columns: 1fr; } }
  @media (max-width: 760px) {
    .dashboard-header { align-items: stretch; flex-direction: column; }
    .header-actions { justify-content: flex-start; }
    .workspace-secondary { grid-template-columns: 1fr; }
  }
  @media (max-width: 480px) { .header-actions .button { flex: 1; } .project-form-actions .button { flex: 1; } }
</style>
