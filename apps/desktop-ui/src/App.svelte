<script lang="ts">
  import { onMount } from 'svelte';
  import { translator } from './i18n/translate';
  import { getApi, type ScriptotarApi } from './api';
  import { applyAppearance, loadAppearance } from './appearance';
  import type { AppSettings, BootstrapData, Job, LibraryItem, MigrationStatus, ViewId, WorkspaceSearchResult } from './types';
  import AppShell from './components/AppShell.svelte';
  import ErrorState from './components/ErrorState.svelte';
  import DashboardView from './views/DashboardView.svelte';
  import WorkspaceSkeleton from './components/WorkspaceSkeleton.svelte';

  export let api: ScriptotarApi = getApi();
  let data: BootstrapData | null = null;
  let loading = true;
  let error = '';
  let switchError = '';
  let operationalError = '';
  let activeView: ViewId = 'dashboard';
  let globalSearch = '';
  let selectedTranscriptId = '';
  let jobRefreshInFlight = false;
  let jobRefreshTimer: number | undefined;
  let operationalRefreshInFlight = false;
  let transcriptSearchIds: string[] = [];
  let transcriptSearchTimer: number | undefined;
  let transcriptSearchGeneration = 0;
  let transcriptSearchKey = '';
  let workspaceGeneration = 0;
  let switchingProject = false;
  const pendingViews = new Set<ViewId>();
  let viewLoadErrors: Partial<Record<ViewId, string>> = {};
  let ResearchView: typeof import('./views/ResearchView.svelte')['default'] | undefined;
  let JobsView: typeof import('./views/JobsView.svelte')['default'] | undefined;
  let TranscriptView: typeof import('./views/TranscriptView.svelte')['default'] | undefined;
  let AiStudioView: typeof import('./views/AiStudioView.svelte')['default'] | undefined;
  let LibraryView: typeof import('./views/LibraryView.svelte')['default'] | undefined;
  let SettingsView: typeof import('./views/SettingsView.svelte')['default'] | undefined;

  const activeStates = new Set(['queued','preparing','downloading','transcribing','processing']);
  $: activeProject = data?.projects.find((project) => project.id === data?.activeProjectId) || data?.projects[0];
  $: activeJobs = data?.jobs.filter((job) => activeStates.has(job.state)).length || 0;
  $: searchResults = data && globalSearch.trim() ? buildSearchResults(data, globalSearch, new Set(transcriptSearchIds)) : [];
  $: scheduleTranscriptSearch(globalSearch, data?.activeProjectId || '');
  $: if (data) void ensureView(activeView);

  // The initial dashboard stays in the entry bundle. Inactive workspaces are loaded on demand.
  // Keep promises single-flight, including when a user navigates rapidly between views.
  async function ensureView(view: ViewId): Promise<void> {
    if (view === 'dashboard' || pendingViews.has(view)) return;
    if ((view === 'research' && ResearchView) || (view === 'jobs' && JobsView) ||
        (view === 'transcript' && TranscriptView) || (view === 'ai' && AiStudioView) ||
        (view === 'library' && LibraryView) || (view === 'settings' && SettingsView)) return;
    pendingViews.add(view);
    viewLoadErrors = { ...viewLoadErrors, [view]: '' };
    try {
      switch (view) {
        case 'research': ResearchView = (await import('./views/ResearchView.svelte')).default; break;
        case 'jobs': JobsView = (await import('./views/JobsView.svelte')).default; break;
        case 'transcript': TranscriptView = (await import('./views/TranscriptView.svelte')).default; break;
        case 'ai': AiStudioView = (await import('./views/AiStudioView.svelte')).default; break;
        case 'library': LibraryView = (await import('./views/LibraryView.svelte')).default; break;
        case 'settings': SettingsView = (await import('./views/SettingsView.svelte')).default; break;
      }
    } catch (cause) {
      viewLoadErrors = {
        ...viewLoadErrors,
        [view]: cause instanceof Error ? cause.message : 'Unable to load this workspace.'
      };
    } finally {
      pendingViews.delete(view);
    }
  }

  function invalidateTranscriptSearch(): void {
    transcriptSearchGeneration += 1;
    transcriptSearchKey = '';
    transcriptSearchIds = [];
    if (transcriptSearchTimer !== undefined) {
      window.clearTimeout(transcriptSearchTimer);
      transcriptSearchTimer = undefined;
    }
  }

  function prepareData(next: BootstrapData): BootstrapData {
    const appearance = loadAppearance(next.settings.appearance);
    applyAppearance(appearance);
    return { ...next, settings: { ...next.settings, appearance } };
  }

  function buildSearchResults(
    snapshot: BootstrapData,
    rawQuery: string,
    transcriptMatches: Set<string>
  ): WorkspaceSearchResult[] {
    const query = rawQuery.trim().toLocaleLowerCase();
    if (!query) return [];
    const match = (...values: Array<string | undefined>) => values.some((value) => value?.toLocaleLowerCase().includes(query));
    const results: WorkspaceSearchResult[] = [];

    for (const project of snapshot.projects) {
      if (match(project.name, project.description)) results.push({ id: `project:${project.id}`, kind: 'Project', title: project.name, subtitle: project.description || `${project.itemCount} items`, view: 'dashboard', projectId: project.id });
    }
    for (const transcript of snapshot.transcripts) {
      if (transcriptMatches.has(transcript.id) || match(transcript.title, transcript.language, transcript.platform)) results.push({ id: `transcript:${transcript.id}`, kind: 'Transcript', title: transcript.title, subtitle: `${transcript.language} · ${transcript.platform}`, view: 'transcript', projectId: transcript.projectId, targetId: transcript.id });
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

  function scheduleTranscriptSearch(rawQuery: string, projectId: string) {
    const query = rawQuery.trim();
    const key = projectId && query ? `${projectId}\u0000${query}` : '';
    if (key === transcriptSearchKey) return;
    transcriptSearchKey = key;
    transcriptSearchGeneration += 1;
    const generation = transcriptSearchGeneration;
    if (transcriptSearchTimer !== undefined) {
      window.clearTimeout(transcriptSearchTimer);
      transcriptSearchTimer = undefined;
    }
    if (!key) {
      transcriptSearchIds = [];
      return;
    }
    transcriptSearchTimer = window.setTimeout(async () => {
      transcriptSearchTimer = undefined;
      try {
        const ids = await api.searchTranscripts(query, 8);
        if (generation === transcriptSearchGeneration && transcriptSearchKey === key) {
          transcriptSearchIds = ids;
        }
      } catch {
        if (generation === transcriptSearchGeneration && transcriptSearchKey === key) {
          transcriptSearchIds = [];
        }
      }
    }, 160);
  }

  async function load(showLoading = true) {
    const generation = ++workspaceGeneration;
    if (showLoading && !data) loading = true;
    if (!data) error = '';
    try {
      const next = await api.bootstrap();
      if (generation !== workspaceGeneration) return;
      data = prepareData(next);
      operationalError = '';
    } catch (cause) {
      if (generation !== workspaceGeneration) return;
      const message = cause instanceof Error ? cause.message : 'Unable to load the desktop workspace.';
      if (data) operationalError = message;
      else error = message;
    } finally {
      if (generation === workspaceGeneration) loading = false;
    }
  }

  async function refresh() { if (!switchingProject) await load(false); }

  async function refreshJobs() {
    if (!data || jobRefreshInFlight || switchingProject) return;
    const generation = workspaceGeneration;
    const snapshot = data;
    jobRefreshInFlight = true;
    try {
      const previousStates = new Map(snapshot.jobs.map((job) => [job.id, job.state]));
      const jobs = await api.listJobs();
      if (generation !== workspaceGeneration || !data || data.activeProjectId !== snapshot.activeProjectId) return;
      const crossedTerminalBoundary = jobs.some((job) => {
        const previous = previousStates.get(job.id);
        return previous !== undefined && activeStates.has(previous) && !activeStates.has(job.state);
      });
      if (crossedTerminalBoundary) await refresh();
      else data = { ...data, jobs };
    } catch (cause) {
      if (generation === workspaceGeneration) {
        operationalError = cause instanceof Error ? cause.message : 'Unable to refresh jobs.';
      }
    } finally {
      jobRefreshInFlight = false;
    }
  }

  function scheduleJobRefresh() {
    if (jobRefreshTimer !== undefined) return;
    jobRefreshTimer = window.setTimeout(() => {
      jobRefreshTimer = undefined;
      void refreshJobs();
    }, 120);
  }

  async function refreshOperationalStatus() {
    if (!data || operationalRefreshInFlight || switchingProject) return;
    const generation = workspaceGeneration;
    const projectId = data.activeProjectId;
    operationalRefreshInFlight = true;
    try {
      const [watchlistStatuses, migrationStatus] = await Promise.all([
        api.getWatchlistStatuses(),
        api.getMigrationStatus()
      ]);
      if (generation !== workspaceGeneration || !data || data.activeProjectId !== projectId) return;
      data = { ...data, watchlistStatuses, migrationStatus };
      operationalError = '';
    } catch (cause) {
      if (generation === workspaceGeneration) {
        operationalError = cause instanceof Error
          ? cause.message
          : 'Could not refresh background operation status.';
      }
    } finally {
      operationalRefreshInFlight = false;
    }
  }

  async function selectProject(id: string): Promise<boolean> {
    if (switchingProject || !data) return false;
    if (id === data.activeProjectId) return true;
    const generation = ++workspaceGeneration;
    switchingProject = true;
    switchError = '';
    invalidateTranscriptSearch();
    globalSearch = '';
    try {
      const next = await api.selectProject(id);
      if (generation !== workspaceGeneration) return false;
      if (next.activeProjectId !== id) throw new Error('The selected project did not match the backend response.');
      data = prepareData(next);
      selectedTranscriptId = data.transcripts[0]?.id || '';
      return true;
    } catch (cause) {
      if (generation === workspaceGeneration) {
        switchError = cause instanceof Error ? cause.message : 'Unable to switch projects.';
      }
      return false;
    } finally {
      if (generation === workspaceGeneration) switchingProject = false;
    }
  }

  async function createProject(name: string) {
    if (switchingProject) return;
    const generation = ++workspaceGeneration;
    switchingProject = true;
    switchError = '';
    invalidateTranscriptSearch();
    globalSearch = '';
    try {
      const next = await api.createProject(name);
      if (generation !== workspaceGeneration) return;
      data = prepareData(next);
      selectedTranscriptId = data.transcripts[0]?.id || '';
      activeView = 'dashboard';
    } catch (cause) {
      if (generation === workspaceGeneration) {
        switchError = cause instanceof Error ? cause.message : 'Unable to create project.';
      }
      throw cause;
    } finally {
      if (generation === workspaceGeneration) switchingProject = false;
    }
  }

  async function saveWatchlist(profileUrl: string, limit: number) {
    if (!data || switchingProject) return;
    const generation = workspaceGeneration;
    const projectId = data.activeProjectId;
    const next = await api.saveWatchlist({ profileUrl, limit });
    if (generation === workspaceGeneration && !switchingProject &&
        data?.activeProjectId === projectId && next.activeProjectId === projectId) {
      data = prepareData(next);
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

  function openTranscript(id: string) {
    if (!data?.transcripts.some((item) => item.id === id)) {
      throw new Error('The selected transcript is not available in the active project.');
    }
    selectedTranscriptId = id;
    activeView = 'transcript';
  }

  async function openCompletedJob(job: Job) {
    if (!job.completedTranscriptId) {
      throw new Error('This completed job has no safe persisted transcript link. Open the transcript from Library; Scriptotar will not guess by title or source.');
    }
    openTranscript(job.completedTranscriptId);
  }

  async function openLibraryItem(item: LibraryItem) {
    if (!data) return;
    if (item.kind === 'Project') {
      if (item.projectId !== data.activeProjectId && !(await selectProject(item.projectId))) return;
      activeView = 'dashboard';
      return;
    }
    if (item.kind === 'Transcript') {
      const embeddedId = item.id.startsWith('transcript:') ? item.id.slice('transcript:'.length) : '';
      if (!embeddedId) throw new Error('The library entry does not contain a transcript identifier.');
      openTranscript(embeddedId);
      return;
    }
    if (item.kind === 'Research' || item.kind === 'Creator') activeView = 'research';
    else if (item.kind === 'AI run') activeView = 'ai';
  }

  async function openSearchResult(result: WorkspaceSearchResult) {
    if (!data) return;
    if (result.projectId && result.projectId !== data.activeProjectId && !(await selectProject(result.projectId))) return;
    if (result.kind === 'Transcript' && result.targetId) selectedTranscriptId = result.targetId;
    activeView = result.view;
    globalSearch = '';
  }

  async function settingsSaved(settings: AppSettings) {
    if (data) data = { ...data, settings: structuredClone(settings) };
    applyAppearance(settings.appearance);
    await refreshOperationalStatus();
  }

  async function migrationStatusChanged(status: MigrationStatus) {
    if (status.state === 'completed') {
      await refresh();
      return;
    }
    if (data) data = { ...data, migrationStatus: status };
  }

  function keyNav(event: KeyboardEvent) {
    // Modal dialogs must own keyboard input; workspace shortcuts must not navigate behind them.
    if (document.querySelector('[role="dialog"][aria-modal="true"]')) return;
    if (event.metaKey || event.ctrlKey || event.altKey) return;
    const target = event.target;
    // Window-dispatched shortcuts have no Element target; nested editor content
    // must not accidentally trigger workspace navigation either.
    if (target instanceof Element && target.closest('input, textarea, select, [contenteditable="true"]')) return;
    const shortcuts: Record<string, ViewId> = { d: 'dashboard', r: 'research', j: 'jobs', t: 'transcript', a: 'ai', l: 'library', ',': 'settings' };
    if (shortcuts[event.key.toLowerCase()]) activeView = shortcuts[event.key.toLowerCase()];
  }

  onMount(() => {
    let disposed = false;
    let unsubscribeJobs: (() => void) | undefined;
    void load();
    void api
      .subscribeJobChanges(() => scheduleJobRefresh())
      .then((unsubscribe) => {
        if (disposed) unsubscribe();
        else unsubscribeJobs = unsubscribe;
      })
      .catch(() => {
        // Periodic reconciliation below remains the safe fallback.
      });
    const jobReconcile = window.setInterval(() => {
      if (data?.jobs.some((job) => activeStates.has(job.state))) void refreshJobs();
    }, 15000);
    const operationalPoll = window.setInterval(() => void refreshOperationalStatus(), 15000);
    return () => {
      disposed = true;
      unsubscribeJobs?.();
      if (jobRefreshTimer !== undefined) window.clearTimeout(jobRefreshTimer);
      if (transcriptSearchTimer !== undefined) window.clearTimeout(transcriptSearchTimer);
      window.clearInterval(jobReconcile);
      window.clearInterval(operationalPoll);
    };
  });
</script>
<svelte:window onkeydown={keyNav} />

<AppShell {activeView} activeProjectId={data?.activeProjectId || ''} projects={data?.projects || []} {activeJobs} busy={loading || switchingProject} bind:globalSearch {searchResults} onNavigate={(view) => activeView = view} onProjectChange={selectProject} onSearchSelect={openSearchResult}>
  {#if loading || switchingProject}
    <WorkspaceSkeleton />
  {:else if error}
    <ErrorState title={$translator('error.bootTitle')} message={error} onRetry={() => load()} />
  {:else if data && activeProject}
    {#if switchError}
      <ErrorState title={$translator('error.projectTitle')} message={switchError} />
    {/if}
    {#if operationalError}
      <ErrorState title={$translator('error.backgroundTitle')} message={operationalError} onRetry={refreshOperationalStatus} />
    {/if}
    {#if viewLoadErrors[activeView]}
      <ErrorState title={$translator('error.bootTitle')} message={viewLoadErrors[activeView] || ''} onRetry={() => ensureView(activeView)} />
    {:else if activeView === 'dashboard'}
      <DashboardView project={activeProject} creators={data.creators} jobs={data.jobs} transcripts={data.transcripts} aiRuns={data.aiRuns} onNavigate={(view) => activeView = view} onCreateProject={createProject} onOpenTranscript={openTranscript} />
    {:else if activeView === 'research'}
      {#if ResearchView}
      <ResearchView items={data.research} watchlistStatuses={data.watchlistStatuses.filter((item) => item.projectId === data?.activeProjectId)} onQueue={async (ids) => { await api.queueResearch(ids); await refresh(); }} onScan={async (profileUrl, limit) => { await api.scanCreator({ profileUrl, limit }); await refresh(); }} onSave={saveWatchlist} />
      {:else}
        <WorkspaceSkeleton />
      {/if}
    {:else if activeView === 'jobs'}
      {#if JobsView}
      <JobsView jobs={data.jobs} onCancel={cancelJob} onRetry={retryJob} onChooseLocal={() => api.chooseLocalMedia()} onEnqueueLocal={enqueueLocal} onEnqueueUrl={enqueueUrl} onOpenCompleted={openCompletedJob} />
      {:else}
        <WorkspaceSkeleton />
      {/if}
    {:else if activeView === 'transcript'}
      {#if TranscriptView}
      <TranscriptView {api} transcripts={data.transcripts} bind:selectedId={selectedTranscriptId} />
      {:else}
        <WorkspaceSkeleton />
      {/if}
    {:else if activeView === 'ai'}
      {#if AiStudioView}
      {#key data.activeProjectId}
        <AiStudioView {api} transcripts={data.transcripts} />
      {/key}
      {:else}
        <WorkspaceSkeleton />
      {/if}
    {:else if activeView === 'library'}
      {#if LibraryView}
      <LibraryView items={data.library.filter((item) => item.projectId === data?.activeProjectId)} onOpen={openLibraryItem} />
      {:else}
        <WorkspaceSkeleton />
      {/if}
    {:else if activeView === 'settings'}
      {#if SettingsView}
      <SettingsView settings={data.settings} migrationStatus={data.migrationStatus} {api} onSaved={settingsSaved} onMigrationStatus={migrationStatusChanged} />
      {:else}
        <WorkspaceSkeleton />
      {/if}
    {/if}
  {:else}
    <ErrorState title={$translator('error.bootTitle')} message="The workspace has no available project." onRetry={() => load()} />
  {/if}
</AppShell>
