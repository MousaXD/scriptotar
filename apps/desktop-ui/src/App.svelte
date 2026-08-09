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

  async function load(showLoading = true) {
    if (showLoading) loading = true;
    error = '';
    try { data = await api.bootstrap(); }
    catch (cause) { error = cause instanceof Error ? cause.message : 'Unable to load the desktop workspace.'; }
    finally { if (showLoading) loading = false; }
  }

  async function refresh() { await load(false); }

  async function selectProject(id: string) {
    await api.selectProject(id);
    await refresh();
  }

  async function enqueueLocal(path: string) {
    if (!data) return;
    await api.enqueueLocalMedia(data.activeProjectId, path);
    await refresh();
  }

  async function enqueueUrl(url: string) {
    if (!data) return;
    await api.enqueueUrl(data.activeProjectId, url);
    await refresh();
  }

  async function cancelJob(id: string) {
    await api.cancelJob(id);
    await refresh();
  }

  async function retryJob(id: string) {
    await api.retryJob(id);
    await refresh();
  }

  function keyNav(event: KeyboardEvent) {
    if (event.metaKey || event.ctrlKey || event.altKey) return;
    const target = event.target as HTMLElement | null;
    if (target?.matches('input, textarea, select, [contenteditable="true"]')) return;
    const shortcuts: Record<string, ViewId> = { d: 'dashboard', r: 'research', j: 'jobs', t: 'transcript', a: 'ai', l: 'library', ',': 'settings' };
    if (shortcuts[event.key.toLowerCase()]) activeView = shortcuts[event.key.toLowerCase()];
  }

  onMount(() => {
    void load();
    const poll = window.setInterval(() => {
      if (data?.jobs.some((job) => activeStates.has(job.state))) void refresh();
    }, 750);
    return () => window.clearInterval(poll);
  });
</script>
<svelte:window on:keydown={keyNav} />

{#if loading}
  <div class="boot-screen" aria-busy="true"><div class="boot-mark">S</div><div><strong>Opening Scriptotar</strong><span>Loading your local workspace…</span></div></div>
{:else if error}
  <div class="boot-screen error-boot"><ErrorState title="Could not open Scriptotar" message={error} onRetry={() => load()} /></div>
{:else if data && activeProject}
  <AppShell {activeView} activeProjectId={data.activeProjectId} projects={data.projects} {activeJobs} bind:globalSearch onNavigate={(view) => activeView = view} onProjectChange={selectProject}>
    {#if activeView === 'dashboard'}
      <DashboardView project={activeProject} creators={data.creators} jobs={data.jobs} transcripts={data.transcripts} aiRuns={data.aiRuns} onNavigate={(view) => activeView = view} />
    {:else if activeView === 'research'}
      <ResearchView items={data.research} onQueue={async (ids) => { await api.queueResearch(ids); await refresh(); }} onScan={async (profileUrl, limit) => { await api.scanCreator({ profileUrl, limit }); await refresh(); }} />
    {:else if activeView === 'jobs'}
      <JobsView jobs={data.jobs} onCancel={cancelJob} onRetry={retryJob} onEnqueueLocal={enqueueLocal} onEnqueueUrl={enqueueUrl} />
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
