<script lang="ts">
  import { onMount } from 'svelte';
  import { getApi, type ScriptotarApi } from './api';
  import type { BootstrapData, ViewId } from './types';
  import AppShell from './components/AppShell.svelte';
  import ErrorState from './components/ErrorState.svelte';
  import DashboardView from './views/DashboardView.svelte';
  import ResearchView from './views/ResearchView.svelte';
  import JobsView from './views/JobsView.svelte';
  import TranscriptView from './views/TranscriptView.svelte';
  import AiStudioView from './views/AiStudioView.svelte';
  import LibraryView from './views/LibraryView.svelte';
  import SettingsView from './views/SettingsView.svelte';

  export let api: ScriptotarApi = getApi();
  let data: BootstrapData | null = null;
  let loading = true;
  let error = '';
  let activeView: ViewId = 'dashboard';
  let globalSearch = '';

  const activeStates = new Set(['queued','preparing','downloading','transcribing','processing']);
  $: activeProject = data?.projects.find((project) => project.id === data?.activeProjectId) || data?.projects[0];
  $: activeJobs = data?.jobs.filter((job) => activeStates.has(job.state)).length || 0;

  async function load() {
    loading = true;
    error = '';
    try { data = await api.bootstrap(); }
    catch (cause) { error = cause instanceof Error ? cause.message : 'Unable to load the desktop workspace.'; }
    finally { loading = false; }
  }

  async function selectProject(id: string) {
    if (!data) return;
    await api.selectProject(id);
    data = { ...data, activeProjectId: id };
  }

  function keyNav(event: KeyboardEvent) {
    if (event.metaKey || event.ctrlKey || event.altKey) return;
    const target = event.target as HTMLElement | null;
    if (target?.matches('input, textarea, select, [contenteditable="true"]')) return;
    const shortcuts: Record<string, ViewId> = { d: 'dashboard', r: 'research', j: 'jobs', t: 'transcript', a: 'ai', l: 'library', ',': 'settings' };
    if (shortcuts[event.key.toLowerCase()]) activeView = shortcuts[event.key.toLowerCase()];
  }

  onMount(load);
</script>
<svelte:window on:keydown={keyNav} />

{#if loading}
  <div class="boot-screen" aria-busy="true"><div class="boot-mark">S</div><div><strong>Opening Scriptotar</strong><span>Loading your local workspace…</span></div></div>
{:else if error}
  <div class="boot-screen error-boot"><ErrorState title="Could not open Scriptotar" message={error} onRetry={load} /></div>
{:else if data && activeProject}
  <AppShell {activeView} activeProjectId={data.activeProjectId} projects={data.projects} {activeJobs} bind:globalSearch onNavigate={(view) => activeView = view} onProjectChange={selectProject}>
    {#if activeView === 'dashboard'}
      <DashboardView project={activeProject} creators={data.creators} jobs={data.jobs} transcripts={data.transcripts} aiRuns={data.aiRuns} onNavigate={(view) => activeView = view} />
    {:else if activeView === 'research'}
      <ResearchView items={data.research} onQueue={(ids) => api.queueResearch(ids)} onScan={(profileUrl, limit) => api.scanCreator({ profileUrl, limit })} />
    {:else if activeView === 'jobs'}
      <JobsView jobs={data.jobs} onCancel={(jobId) => api.cancelJob(jobId)} />
    {:else if activeView === 'transcript'}
      <TranscriptView transcripts={data.transcripts} />
    {:else if activeView === 'ai'}
      <AiStudioView {api} initialSource={data.transcripts[0]?.text || ''} />
    {:else if activeView === 'library'}
      <LibraryView items={data.library.filter((item) => item.projectId === data?.activeProjectId)} />
    {:else if activeView === 'settings'}
      <SettingsView settings={data.settings} {api} />
    {/if}
  </AppShell>
{/if}
