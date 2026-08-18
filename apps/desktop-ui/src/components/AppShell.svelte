<script lang="ts">
  import { tick } from 'svelte';
  import { locale, setLocale, type AppLocale } from '../i18n';
  import type { Project, ViewId, WorkspaceSearchResult } from '../types';

  export let activeView: ViewId;
  export let activeProjectId: string;
  export let projects: Project[];
  export let activeJobs = 0;
  export let globalSearch = '';
  export let searchResults: WorkspaceSearchResult[] = [];
  export let onNavigate: (view: ViewId) => void;
  export let onProjectChange: (id: string) => Promise<void> | void;
  export let onSearchSelect: (result: WorkspaceSearchResult) => Promise<void> | void;

  let searchInput: HTMLInputElement;

  const nav: { id: ViewId; label: string; key: string }[] = [
    { id: 'dashboard', label: 'Dashboard', key: 'D' },
    { id: 'research', label: 'Research', key: 'R' },
    { id: 'jobs', label: 'Jobs', key: 'J' },
    { id: 'transcript', label: 'Transcript', key: 'T' },
    { id: 'ai', label: 'AI Studio', key: 'A' },
    { id: 'library', label: 'Library', key: 'L' },
    { id: 'settings', label: 'Settings', key: ',' }
  ];

  async function changeProject(event: Event) {
    const select = event.currentTarget as HTMLSelectElement;
    const requestedProjectId = select.value;
    await onProjectChange(requestedProjectId);
    await tick();
    select.value = activeProjectId;
  }

  function changeLocale(event: Event) {
    setLocale((event.currentTarget as HTMLSelectElement).value as AppLocale);
  }

  function searchKeys(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      globalSearch = '';
      searchInput.blur();
      return;
    }
    if (event.key === 'Enter' && searchResults[0]) {
      event.preventDefault();
      void onSearchSelect(searchResults[0]);
    }
  }

  function globalShortcut(event: KeyboardEvent) {
    if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'k') {
      event.preventDefault();
      searchInput?.focus();
      searchInput?.select();
    }
  }
</script>
<svelte:window onkeydown={globalShortcut} />

<div class="app-frame">
  <aside class="sidebar" aria-label="Primary navigation">
    <div class="brand-lockup"><div class="brand-mark">S</div><div><strong>Scriptotar</strong><span>Creator workstation</span></div></div>
    <nav>
      {#each nav as item}
        <button class:active={activeView === item.id} aria-current={activeView === item.id ? 'page' : undefined} on:click={() => onNavigate(item.id)}>
          <span>{item.label}</span><kbd>{item.key}</kbd>
          {#if item.id === 'jobs' && activeJobs > 0}<b class="activity-count" aria-label={`${activeJobs} active jobs`}>{activeJobs}</b>{/if}
        </button>
      {/each}
    </nav>
    <div class="sidebar-foot"><span class="local-dot"></span><div><strong>Local-first</strong><small>Rust-owned workspace state</small></div></div>
  </aside>

  <div class="workspace-shell">
    <header class="topbar">
      <div class="project-control">
        <label for="project-select">Project</label>
        <select id="project-select" value={activeProjectId} on:change={changeProject}>
          {#each projects as project}<option value={project.id}>{project.name}</option>{/each}
        </select>
      </div>
      <div class="global-search-wrap">
        <label class="global-search" aria-label="Global search">
          <span aria-hidden="true">⌕</span><input bind:this={searchInput} bind:value={globalSearch} on:keydown={searchKeys} aria-label="Search workspace" aria-expanded={Boolean(globalSearch.trim())} aria-controls="workspace-search-results" placeholder="Search transcripts, projects, creators…" />
          <kbd>Ctrl K</kbd>
        </label>
        {#if globalSearch.trim()}
          <div id="workspace-search-results" class="search-results" aria-label="Workspace search results">
            {#if searchResults.length === 0}
              <p>No local matches.</p>
            {:else}
              {#each searchResults as result}
                <button on:click={() => onSearchSelect(result)}><span><strong>{result.title}</strong><small>{result.subtitle}</small></span><b>{result.kind}</b></button>
              {/each}
            {/if}
          </div>
        {/if}
      </div>
      <div class="topbar-actions">
        <label class="language-control">
          <span>Language</span>
          <select value={$locale} on:change={changeLocale} aria-label="Interface language">
            <option value="en">English</option>
            <option value="ar">Arabic</option>
          </select>
        </label>
        <button class="activity-button" on:click={() => onNavigate('jobs')} aria-label="Open jobs">
          <span class="activity-orb" class:busy={activeJobs > 0}></span>
          <span>{activeJobs > 0 ? `${activeJobs} active` : 'Idle'}</span>
        </button>
      </div>
    </header>
    <main class="workspace"><slot /></main>
  </div>
</div>

<style>
  .global-search-wrap { position: relative; min-width: 0; }
  .search-results { position: absolute; top: calc(100% + 7px); left: 0; right: 0; z-index: 30; overflow: hidden; border: 1px solid var(--border-strong); border-radius: 10px; background: var(--surface); box-shadow: 0 18px 45px rgb(0 0 0 / 35%); }
  .search-results p { margin: 0; padding: 13px; font-size: 11px; }
  .search-results button { display: grid; grid-template-columns: minmax(0, 1fr) auto; align-items: center; gap: 12px; width: 100%; padding: 10px 12px; border: 0; border-top: 1px solid var(--border); background: transparent; text-align: left; }
  .search-results button:first-child { border-top: 0; }
  .search-results button:hover, .search-results button:focus-visible { background: var(--surface-2); }
  .search-results strong, .search-results small { display: block; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .search-results small { margin-top: 3px; color: var(--muted); font-size: 10px; }
  .search-results b { color: var(--accent); font-size: 9px; text-transform: uppercase; letter-spacing: .06em; }
  .topbar-actions { justify-self: end; display: flex; align-items: center; gap: 8px; min-width: 0; }
  .language-control { display: flex; align-items: center; gap: 7px; color: var(--faint); font-size: 10px; text-transform: uppercase; letter-spacing: .06em; }
  .language-control select { min-width: 104px; background: var(--surface); text-transform: none; letter-spacing: 0; }

  @media (max-width: 820px) {
    .language-control > span { position: absolute; width: 1px; height: 1px; padding: 0; margin: -1px; overflow: hidden; clip: rect(0, 0, 0, 0); white-space: nowrap; border: 0; }
    .language-control select { min-width: 88px; }
  }
</style>
