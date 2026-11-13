<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { translator } from './i18n/translate';
  import { locale } from './i18n';
  import { getApi, type ScriptotarApi } from './api';
  import { applyAppearance, loadAppearance } from './appearance';
  import type { AppSettings, BootstrapSummaryData, Job, LibraryItem, MigrationStatus, ViewId, WorkspaceSearchResult } from './types';
  import AppShell from './components/AppShell.svelte';
  import ErrorState from './components/ErrorState.svelte';
  import DashboardView from './views/DashboardView.svelte';
  import WorkspaceSkeleton from './components/WorkspaceSkeleton.svelte';
  import ResearchView from './views/ResearchView.svelte';
  import JobsView from './views/JobsView.svelte';
  import TranscriptView from './views/TranscriptView.svelte';
  import AiStudioView from './views/AiStudioView.svelte';
  import LibraryView from './views/LibraryView.svelte';
  import SettingsView from './views/SettingsView.svelte';
  import StudioView from './views/StudioView.svelte';
  import type { StudySourceRef } from './studio/contract';

  export let api: ScriptotarApi = getApi();
  let data: BootstrapSummaryData | null = null;
  let loading = true;
  let error = '';
  let switchError = '';
  let operationalError = '';
  let activeView: ViewId = 'dashboard';
  let globalSearch = '';
  let selectedTranscriptId = '';
  let studioIncoming: (StudySourceRef & { projectId: string }) | null = null;
  let firstProjectName = '';
  let jobRefreshGeneration: number | undefined;
  let jobRefreshTimer: number | undefined;
  let operationalRefreshGeneration: number | undefined;
  let transcriptSearchIds: string[] = [];
  let transcriptSearchTimer: number | undefined;
  let transcriptSearchGeneration = 0;
  let transcriptSearchKey = '';
  let workspaceGeneration = 0;
  let switchingProject = false;
  let creatingProject = false;
  let savingWatchlist = false;
  const activeStates = new Set(['queued','preparing','downloading','transcribing','processing']);
  function startupMark(name: string): void {
    if (typeof performance !== 'undefined' && typeof performance.mark === 'function') {
      performance.mark(name);
    }
  }
  // Never silently show data from another project when the backend returns a stale ID.
  $: activeProject = data?.projects.find((project) => project.id === data?.activeProjectId);
  $: activeJobs = data?.jobs.filter((job) => activeStates.has(job.state)).length || 0;
  $: searchResults = data && globalSearch.trim() ? buildSearchResults(data, globalSearch, new Set(transcriptSearchIds)) : [];
  $: scheduleTranscriptSearch(globalSearch, data?.activeProjectId || '');

  function invalidateTranscriptSearch(): void {
    transcriptSearchGeneration += 1;
    transcriptSearchKey = '';
    transcriptSearchIds = [];
    if (transcriptSearchTimer !== undefined) {
      window.clearTimeout(transcriptSearchTimer);
      transcriptSearchTimer = undefined;
    }
  }

  function prepareData(next: BootstrapSummaryData): BootstrapSummaryData {
    const appearance = loadAppearance(next.settings.appearance);
    applyAppearance(appearance);
    return { ...next, settings: { ...next.settings, appearance } };
  }

  function buildSearchResults(
    snapshot: BootstrapSummaryData,
    rawQuery: string,
    transcriptMatches: Set<string>
  ): WorkspaceSearchResult[] {
    const query = rawQuery.trim().toLocaleLowerCase();
    if (!query) return [];
    const match = (...values: Array<string | undefined>) => values.some((value) => value?.toLocaleLowerCase().includes(query));
    const results: WorkspaceSearchResult[] = [];

    for (const project of snapshot.projects) {
      if (match(project.name, project.description)) results.push({ id: `project:${project.id}`, kind: 'Project', title: project.name, subtitle: project.description || `${project.itemCount} items`, view: 'dashboard', projectId: project.id });
      if (results.length >= 10) return results;
    }
    for (const transcript of snapshot.transcripts) {
      if (transcriptMatches.has(transcript.id) || match(transcript.title, transcript.language, transcript.platform)) results.push({ id: `transcript:${transcript.id}`, kind: 'Transcript', title: transcript.title, subtitle: `${transcript.language} · ${transcript.platform}`, view: 'transcript', projectId: transcript.projectId, targetId: transcript.id });
      if (results.length >= 10) return results;
    }
    for (const item of snapshot.research) {
      if (match(item.title, item.creator, item.platform)) results.push({ id: `research:${item.id}`, kind: 'Research', title: item.title, subtitle: `${item.creator} · ${item.platform}`, view: 'research', projectId: snapshot.activeProjectId, targetId: item.id });
      if (results.length >= 10) return results;
    }
    for (const creator of snapshot.creators) {
      if (match(creator.name, creator.handle, creator.platform)) results.push({ id: `creator:${creator.id}`, kind: 'Creator', title: creator.name, subtitle: `${creator.handle} · ${creator.platform}`, view: 'research', projectId: snapshot.activeProjectId, targetId: creator.id });
      if (results.length >= 10) return results;
    }
    for (const run of snapshot.aiRuns) {
      if (match(run.title, run.task, run.provider, run.model)) results.push({ id: `ai:${run.id}`, kind: 'AI run', title: run.title, subtitle: `${run.task} · ${run.provider || 'Local prompt'}`, view: 'ai', projectId: snapshot.activeProjectId, targetId: run.id });
      if (results.length >= 10) return results;
    }
    return results;
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
    const firstBootstrap = !data && showLoading;
    const generation = ++workspaceGeneration;
    let inconsistentResponse = false;
    if (firstBootstrap) startupMark('scriptotar.bootstrap-start');
    if (showLoading && !data) loading = true;
    if (!data) error = '';
    try {
      const next = await api.bootstrapSummary();
      if (generation !== workspaceGeneration) return;
      if ((next.projects.length === 0 && next.activeProjectId !== '') ||
          (next.projects.length > 0 && !next.projects.some((project) => project.id === next.activeProjectId))) {
        inconsistentResponse = true;
        throw new Error('The backend did not identify an active project.');
      }
      data = prepareData(next);
      switchError = '';
      if (firstBootstrap) {
        startupMark('scriptotar.bootstrap-ready');
        if (typeof performance !== 'undefined' && typeof performance.mark === 'function' &&
            typeof performance.measure === 'function') {
          performance.measure('scriptotar.bootstrap', 'scriptotar.bootstrap-start', 'scriptotar.bootstrap-ready');
        }
      }
      operationalError = '';
    } catch (cause) {
      if (generation !== workspaceGeneration) return;
      const message = cause instanceof Error ? cause.message : 'Unable to load the desktop workspace.';
      if (inconsistentResponse) {
        // The backend no longer identifies a trustworthy active workspace.
        // Unlike a transient refresh failure, stale project data is not safe to use.
        data = null;
        error = message;
      } else if (data) operationalError = message;
      else error = message;
    } finally {
      if (generation === workspaceGeneration) loading = false;
    }
  }

  async function refresh() { if (!switchingProject && !creatingProject && !savingWatchlist) await load(false); }

  async function refreshJobs() {
    if (!data || jobRefreshGeneration === workspaceGeneration || switchingProject || creatingProject) return;
    const generation = workspaceGeneration;
    const snapshot = data;
    jobRefreshGeneration = generation;
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
      if (jobRefreshGeneration === generation) jobRefreshGeneration = undefined;
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
    if (!data || operationalRefreshGeneration === workspaceGeneration || switchingProject || creatingProject) return;
    const generation = workspaceGeneration;
    const projectId = data.activeProjectId;
    operationalRefreshGeneration = generation;
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
      if (operationalRefreshGeneration === generation) operationalRefreshGeneration = undefined;
    }
  }

  async function reconcileFailedProjectMutation(previousProjectId: string, generation: number, message: string) {
    try {
      // A backend command can commit the active-project change before a later
      // bootstrap/transport error rejects its response. Never assume that a
      // rejected mutation left the backend on the previous project.
      const snapshot = await api.bootstrapSummary();
      if (generation !== workspaceGeneration) return;
      const hasValidActiveProject = snapshot.projects.length === 0
        ? snapshot.activeProjectId === ''
        : snapshot.projects.some((project) => project.id === snapshot.activeProjectId);
      if (!hasValidActiveProject || snapshot.activeProjectId !== previousProjectId) {
        throw new Error('The active project changed after the failed operation.');
      }
      data = prepareData(snapshot);
      return snapshot;
    } catch {
      if (generation !== workspaceGeneration) return;
      data = null;
      error = `${message} The active project could not be verified. Retry loading the workspace.`;
    }
  }

  async function selectProject(id: string): Promise<boolean> {
    if (switchingProject || creatingProject || savingWatchlist || !data) return false;
    if (id === data.activeProjectId) return true;
    const previousProjectId = data.activeProjectId;
    const generation = ++workspaceGeneration;
    let inconsistentResponse = false;
    switchingProject = true;
    switchError = '';
    invalidateTranscriptSearch();
    globalSearch = '';
    try {
      const next = await api.selectProject(id);
      if (generation !== workspaceGeneration) return false;
      if (next.activeProjectId !== id || !next.projects.some((project) => project.id === id)) {
        inconsistentResponse = true;
        throw new Error('The selected project did not match the backend response.');
      }
      data = prepareData(next);
      selectedTranscriptId = data.transcripts[0]?.id || '';
       studioIncoming = null;
       studioIncoming = null;
      return true;
    } catch (cause) {
      if (generation === workspaceGeneration) {
        switchError = cause instanceof Error ? cause.message : 'Unable to switch projects.';
        if (inconsistentResponse) {
          // An inconsistent backend snapshot leaves the active project unknowable.
          // Fail closed rather than letting stale data remain actionable.
          data = null;
          error = switchError;
        } else {
          await reconcileFailedProjectMutation(previousProjectId, generation, switchError);
        }
      }
      return false;
    } finally {
      if (generation === workspaceGeneration) switchingProject = false;
    }
  }

  async function createProject(name: string) {
    if (savingWatchlist) throw new Error('Finish saving the watchlist before creating a project.');
    if (switchingProject || creatingProject) return;
    const hadActiveProject = Boolean(activeProject);
    const previousProjectId = data?.activeProjectId || '';
    const priorProjectIds = new Set(data?.projects.map((project) => project.id) || []);
    const generation = ++workspaceGeneration;
    let inconsistentResponse = false;
    creatingProject = true;
    switchError = '';
    invalidateTranscriptSearch();
    globalSearch = '';
    try {
      const next = await api.createProject(name);
      if (generation !== workspaceGeneration) return;
      const createdProject = next.projects.find((project) => project.id === next.activeProjectId);
      if (!createdProject) {
        inconsistentResponse = true;
        throw new Error('The created project was not available in the backend response.');
      }
      if (priorProjectIds.has(createdProject.id) || createdProject.name !== name.trim()) {
        // A successful creation must identify a *new* project with the requested name;
        // do not mistake another existing project for the result of this mutation.
        inconsistentResponse = true;
        throw new Error('The created project did not match the backend response.');
      }
      data = prepareData(next);
      selectedTranscriptId = data.transcripts[0]?.id || '';
      activeView = 'dashboard';
    } catch (cause) {
      if (generation === workspaceGeneration) {
        const message = cause instanceof Error ? cause.message : 'Unable to create project.';
        if (inconsistentResponse) {
          // A malformed successful mutation can leave the backend's active project unknown.
          data = null;
          error = message;
        } else {
          if (!hadActiveProject) switchError = message;
          const recovered = await reconcileFailedProjectMutation(previousProjectId, generation, message);
          // Creating a project and activating it are separate backend writes.
          // A rejected activation can leave a newly created project available.
          if (recovered?.projects.some((project) =>
            !priorProjectIds.has(project.id) && project.name === name.trim()
          )) {
            const warning = 'A project with this name may already have been created. Check the project selector before retrying.';
            switchError = warning;
            throw new Error(warning);
          }
        }
      }
      throw cause;
    } finally {
      if (generation === workspaceGeneration) creatingProject = false;
    }
  }

  async function saveWatchlist(profileUrl: string, limit: number) {
    if (!data || switchingProject || creatingProject) return;
    if (savingWatchlist) throw new Error('A watchlist save is already in progress.');
    // The backend resolves the active project inside save_watchlist(), after URL
    // validation. Keep project selection stable until that write completes.
    savingWatchlist = true;
    const generation = workspaceGeneration;
    const projectId = data.activeProjectId;
    try {
      const next = await api.saveWatchlist({ profileUrl, limit });
      if (generation !== workspaceGeneration || switchingProject || creatingProject ||
          data?.activeProjectId !== projectId) return;
      if (next.activeProjectId !== projectId || !next.projects.some((project) => project.id === projectId)) {
        // The write may have changed the backend's active project unexpectedly.
        // Do not keep issuing operations against a workspace that may no longer be active.
        error = 'The watchlist response did not match the active project. Reload the workspace.';
        data = null;
        throw new Error(error);
      }
      data = prepareData(next);
    } finally {
      savingWatchlist = false;
    }
  }

  async function createFirstProject(event: SubmitEvent) {
    event.preventDefault();
    const name = firstProjectName.trim();
    if (!name) return;
    try {
      await createProject(name);
    } catch {
      // createProject supplies the visible, retryable error in the empty workspace.
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

  function useInStudio(kind: StudySourceRef['kind'], id: string) {
    if (!data) return;
    const available = kind === 'research'
      ? data.research.some((item) => item.id === id)
      : data.transcripts.some((item) => item.id === id && item.projectId === data.activeProjectId);
    if (!available) { switchError = 'The selected source is not available in this project.'; return; }
    studioIncoming = { kind, id, projectId: data.activeProjectId };
    activeView = 'studio';
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
    switchError = '';
    if (result.projectId && result.projectId !== data.activeProjectId && !(await selectProject(result.projectId))) return;
    if (result.kind === 'Transcript' && result.targetId) {
      if (!data?.transcripts.some((item) => item.id === result.targetId)) {
        switchError = 'The selected transcript is not available in the active project.';
        return;
      }
      selectedTranscriptId = result.targetId;
    }
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
    if (document.querySelector('[role="dialog"][aria-modal="true"], dialog[open]')) return;
    if (event.metaKey || event.ctrlKey || event.altKey) return;
    const target = event.target;
    // Window-dispatched shortcuts have no Element target; nested editor content
    // must not accidentally trigger workspace navigation either.
    // Editing and selector widgets own their letter keys, including plaintext-only
    // contenteditable fields and custom ARIA textboxes that are not native inputs.
    if (target instanceof Element &&
        target.closest('input, textarea, select, [contenteditable]:not([contenteditable="false"]), [role="textbox"], [role="searchbox"], [role="listbox"], [role="option"], [role="combobox"]')) return;
    const shortcuts: Record<string, ViewId> = { d: 'dashboard', r: 'research', s: 'studio', j: 'jobs', t: 'transcript', a: 'ai', l: 'library', ',': 'settings' };
    const destination = shortcuts[event.key.toLowerCase()];
    if (destination) {
      activeView = destination;
      // Shortcuts bypass AppShell's pointer navigation; restore the view heading below the sticky toolbar.
      void tick().then(() => window.scrollTo({ top: 0, behavior: 'instant' }));
    }
  }

  onMount(() => {
    startupMark('scriptotar.shell-mounted');
    let disposed = false;
    let unsubscribeJobs: (() => void) | undefined;
    void load();
    void api
      .subscribeJobChanges(() => { if (!disposed) scheduleJobRefresh(); })
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
      // All pending IPC responses are now obsolete, including an unresolved bootstrap.
      workspaceGeneration += 1;
      invalidateTranscriptSearch();
      unsubscribeJobs?.();
      if (jobRefreshTimer !== undefined) window.clearTimeout(jobRefreshTimer);
      window.clearInterval(jobReconcile);
      window.clearInterval(operationalPoll);
    };
  });
</script>
<svelte:window onkeydown={keyNav} />

<AppShell {activeView} activeProjectId={data?.activeProjectId || ''} projects={data?.projects || []} {activeJobs} busy={loading || switchingProject || creatingProject || savingWatchlist} bind:globalSearch {searchResults} onNavigate={(view) => activeView = view} onProjectChange={selectProject} onSearchSelect={openSearchResult}>
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
    {#if activeView === 'dashboard'}
      <DashboardView project={activeProject} creators={data.creators} jobs={data.jobs} transcripts={data.transcripts} aiRuns={data.aiRuns} onNavigate={(view) => activeView = view} onCreateProject={createProject} onOpenTranscript={openTranscript} />
    {:else if activeView === 'research'}
      <ResearchView items={data.research} onUseInStudio={(item) => useInStudio('research', item.id)} watchlistStatuses={data.watchlistStatuses.filter((item) => item.projectId === data?.activeProjectId)} onQueue={async (ids) => { await api.queueResearch(ids); await refresh(); }} onScan={async (profileUrl, limit) => { await api.scanCreator({ profileUrl, limit }); await refresh(); }} onSave={saveWatchlist} />
    {:else if activeView === 'jobs'}
      <JobsView jobs={data.jobs} onCancel={cancelJob} onRetry={retryJob} onChooseLocal={() => api.chooseLocalMedia()} onEnqueueLocal={enqueueLocal} onEnqueueUrl={enqueueUrl} onOpenCompleted={openCompletedJob} />
    {:else if activeView === 'transcript'}
      <TranscriptView {api} transcripts={data.transcripts} bind:selectedId={selectedTranscriptId} onUseInStudio={(id) => useInStudio('transcript', id)} />
    {:else if activeView === 'studio'}
      {#key data.activeProjectId}
        <StudioView {api} projectId={data.activeProjectId} research={data.research} transcripts={data.transcripts} jobs={data.jobs} incoming={studioIncoming} onNavigate={(view) => activeView = view} />
      {/key}
    {:else if activeView === 'ai'}
      {#key data.activeProjectId}
        <AiStudioView {api} transcripts={data.transcripts} />
      {/key}
    {:else if activeView === 'library'}
      <LibraryView items={data.library.filter((item) => item.projectId === data?.activeProjectId)} onOpen={openLibraryItem} />
    {:else if activeView === 'settings'}
      <SettingsView settings={data.settings} migrationStatus={data.migrationStatus} {api} onSaved={settingsSaved} onMigrationStatus={migrationStatusChanged} />
    {/if}
  {:else if data?.projects.length}
    <ErrorState
      title={$translator('error.bootTitle')}
      message={$locale === 'ar' ? 'تعذر العثور على المشروع النشط. حاول إعادة تحميل مساحة العمل.' : 'The active project could not be found. Retry loading the workspace.'}
      onRetry={() => load()}
    />
  {:else}
    <section class="first-project" aria-labelledby="first-project-title">
      <h1 id="first-project-title">{$locale === 'ar' ? 'أنشئ مشروعك الأول' : 'Create your first project'}</h1>
      <p>{$locale === 'ar' ? 'مساحة العمل فارغة. أضف مشروعاً للبدء.' : 'Your workspace is empty. Add a project to get started.'}</p>
      {#if switchError}
        <ErrorState title={$translator('error.projectTitle')} message={switchError} />
      {/if}
      <form on:submit={createFirstProject}>
        <label for="first-project-name">{$locale === 'ar' ? 'اسم المشروع' : 'Project name'}</label>
        <input id="first-project-name" bind:value={firstProjectName} required maxlength="100" autocomplete="off" disabled={creatingProject} />
        <button type="submit" class="button primary" disabled={creatingProject}>{creatingProject ? $translator('project.creating') : $locale === 'ar' ? 'إنشاء مشروع' : 'Create project'}</button>
      </form>
    </section>
  {/if}
</AppShell>

<style>
  .first-project { max-width: 560px; margin-block: clamp(32px, 8vh, 110px); }
  .first-project h1 { margin-block-end: 8px; }
  .first-project p { margin-block-end: 24px; }
  .first-project form { display: grid; gap: 12px; max-width: 380px; }
  .first-project label { font-weight: 600; }
  .first-project input { min-width: 0; width: 100%; }
  .first-project button { justify-self: start; }
</style>
