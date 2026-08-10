<script lang="ts">
  import { onMount } from 'svelte';
  import { getApi, type ScriptotarApi } from './api';
  import { applyAppearance, loadAppearance } from './appearance';
  import type { AppSettings, BootstrapData, Job, LibraryItem, ViewId, WorkspaceSearchResult } from './types';
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
  let switchError = '';
  let activeView: ViewId = 'dashboard';
  let globalSearch = '';
  let selectedTranscriptId = '';
  let jobRefreshInFlight = false;

  const activeStates = new Set(['queued','preparing','downloading','transcribing','processing']);
  $: activeProject = data?.projects.find((project) => project.id === data?.activeProjectId) || data?.projects[0];
  $: activeJobs = data?.jobs.filter((job) => activeStates.has(job.state)).length || 0;
  $: searchResults = data ? buildSearchResults(data, globalSearch) : [];

  function prepareData(next: BootstrapData): BootstrapData {
    const appearance = loadAppearance(next.settings.appearance);
    applyAppearance(appearance);
    return { ...next, settings: { ...next.settings, appearance } };
  }

  function buildSearchResults(snapshot: BootstrapData, rawQuery: string): WorkspaceSearchResult[] {
    const query = rawQuery.trim().toLocaleLowerCase();
    if (!query) return [];
    const match = (...values: Array<string | undefined>) => values.some((value) => value?.toLocaleLowerCase().includes(query));
    const results: WorkspaceSearchResult[] = [];

    for (const project of snapshot.projects) {
      if (match(project.name, project.description)) results.push({ id: `project:${project.id}`, kind: 'Project', title: project.name, subtitle: project.description || `${project.itemCount} items`, view: 'dashboard', projectId: project.id });
    }
    for (const transcript of snapshot.transcripts) {
      if (match(transcript.title, transcript.text, transcript.language, transcript.platform)) results.push({ id: `transcript:${transcript.id}`, kind: 'Transcript', title: transcript.title, subtitle: `${transcript.language} · ${transcript.platform}`, view: 'transcript', projectId: transcript.projectId, targetId: transcript.id });
    }
    for (const item of snapshot.research) {
      if (match(item.title, item.creator, item.platform)) results.push({ id: `research:${item.id}`, kind: 'Research', title: item.title, subtitle: `${item.creator} · ${item.platform}`, view: 'research', projectId: snapshot.activeProjectId, targetId: item.id });
    }
    for (const creator of snapshot.creators) {
      if (match(creator.name, creator.handle, creator.platform)) results.push({ id: `creator:${creator.id}`, kind: 'Creator', title: creator.name, subtitle: `${creator.handle} · ${creator.platform}`, view: 'research', projectId: snapshot.activeProjectId, targetId: creator.id });
    }
    for (const run of snapshot.aiRuns) {
      if (match(run.title, run.task, run.provider, run.model)) results.push({ id: `ai:${run.id}`, kind: 'AI run', title: run.title, subtitle: `${run.task} · ${run.provider || 'Local prompt'}`, view: 'ai', projectId: snapshot.activeProjectId, targetId: run.id });
    }
    return results.slice(0, 10);
  }

  async function load(showLoading = true) {
    if (showLoading) loading = true;
    error = '';
    try { data = prepareData(await api.bootstrap()); }
    catch (cause) { error = cause instanceof Error ? cause.message : 'Unable to load the desktop workspace.'; }
    finally { if (showLoading) loading = false; }
  }

  async function refresh() { await load(false); }

  async function refreshJobs() {
    if (!data || jobRefreshInFlight) return;
    jobRefreshInFlight = true;
    try {
      const previousStates = new Map(data.jobs.map((job) => [job.id, job.state]));
      const jobs = await api.listJobs();
      const crossedTerminalBoundary = jobs.some((job) => {
        const previous = previousStates.get(job.id);
        return previous !== undefined && activeStates.has(previous) && !activeStates.has(job.state);
      });
      if (crossedTerminalBoundary) await refresh();
      else data = { ...data, jobs };
    } finally {
      jobRefreshInFlight = false;
    }
  }

  async function selectProject(id: string) {
    switchError = '';
    try {
      data = prepareData(await api.selectProject(id));
      selectedTranscriptId = data.transcripts[0]?.id || '';
    } catch (cause) {
      switchError = cause instanceof Error ? cause.message : 'Unable to switch projects.';
      if (data) data = { ...data };
    }
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

  async function openCompletedJob(job: Job) {
    const transcript = data?.transcripts.find((item) => item.source === job.source || item.title === job.title);
    if (!transcript) throw new Error('This job is complete, but no persisted transcript for it is available in the active project yet. Refresh the workspace or open the transcript from Library.');
    selectedTranscriptId = transcript.id;
    activeView = 'transcript';
  }

  async function openLibraryItem(item: LibraryItem) {
    if (!data) return;
    if (item.kind === 'Project') {
      if (item.projectId !== data.activeProjectId) await selectProject(item.projectId);
      activeView = 'dashboard';
      return;
    }
    if (item.kind === 'Transcript') {
      const embeddedId = item.id.startsWith('transcript:') ? item.id.slice('transcript:'.length) : '';
      const transcript = data.transcripts.find((candidate) => candidate.id === embeddedId || candidate.title === item.title);
      if (!transcript) throw new Error('The library entry exists, but its transcript is not available in the active project.');
      selectedTranscriptId = transcript.id;
      activeView = 'transcript';
      return;
    }
    if (item.kind === 'Research' || item.kind === 'Creator') activeView = 'research';
    else if (item.kind === 'AI run') activeView = 'ai';
  }

  async function openSearchResult(result: WorkspaceSearchResult) {
    if (!data) return;
    if (result.projectId && result.projectId !== data.activeProjectId) await selectProject(result.projectId);
    if (result.kind === 'Transcript' && result.targetId) selectedTranscriptId = result.targetId;
    activeView = result.view;
    globalSearch = '';
  }

  function settingsSaved(settings: AppSettings) {
    if (data) data = { ...data, settings: structuredClone(settings) };
    applyAppearance(settings.appearance);
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
      if (data?.jobs.some((job) => activeStates.has(job.state))) void refreshJobs();
    }, 1000);
    return () => window.clearInterval(poll);
  });
</script>
<svelte:window onkeydown={keyNav} />

{#if loading}
  <div class="boot-screen" aria-busy="true"><div class="boot-mark">S</div><div><strong>Opening Scriptotar</strong><span>Loading your local workspace…</span></div></div>
{:else if error}
  <div class="boot-screen error-boot"><ErrorState title="Could not open Scriptotar" message={error} onRetry={() => load()} /></div>
{:else if data && activeProject}
  <AppShell {activeView} activeProjectId={data.activeProjectId} projects={data.projects} {activeJobs} bind:globalSearch {searchResults} onNavigate={(view) => activeView = view} onProjectChange={selectProject} onSearchSelect={openSearchResult}>
    {#if switchError}
      <ErrorState title="Could not switch projects" message={switchError} />
    {/if}
    {#if activeView === 'dashboard'}
      <DashboardView project={activeProject} creators={data.creators} jobs={data.jobs} transcripts={data.transcripts} aiRuns={data.aiRuns} onNavigate={(view) => activeView = view} />
    {:else if activeView === 'research'}
      <ResearchView items={data.research} onQueue={async (ids) => { await api.queueResearch(ids); await refresh(); }} onScan={async (profileUrl, limit) => { await api.scanCreator({ profileUrl, limit }); await refresh(); }} onSave={async (profileUrl, limit) => { data = prepareData(await api.saveWatchlist({ profileUrl, limit })); }} />
    {:else if activeView === 'jobs'}
      <JobsView jobs={data.jobs} onCancel={cancelJob} onRetry={retryJob} onChooseLocal={() => api.chooseLocalMedia()} onEnqueueLocal={enqueueLocal} onEnqueueUrl={enqueueUrl} onOpenCompleted={openCompletedJob} />
    {:else if activeView === 'transcript'}
      <TranscriptView transcripts={data.transcripts} bind:selectedId={selectedTranscriptId} />
    {:else if activeView === 'ai'}
      <AiStudioView {api} initialSource={data.transcripts[0]?.text || ''} />
    {:else if activeView === 'library'}
      <LibraryView items={data.library.filter((item) => item.projectId === data?.activeProjectId)} onOpen={openLibraryItem} />
    {:else if activeView === 'settings'}
      <SettingsView settings={data.settings} {api} onSaved={settingsSaved} />
    {/if}
  </AppShell>
{/if}
