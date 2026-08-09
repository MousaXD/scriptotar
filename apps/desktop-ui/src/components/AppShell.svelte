<script lang="ts">
  import { tick } from 'svelte';
  import type { Project, ViewId } from '../types';
  export let activeView: ViewId;
  export let activeProjectId: string;
  export let projects: Project[];
  export let activeJobs = 0;
  export let globalSearch = '';
  export let onNavigate: (view: ViewId) => void;
  export let onProjectChange: (id: string) => Promise<void> | void;

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
</script>
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
      <label class="global-search" aria-label="Global search">
        <span>⌕</span><input bind:value={globalSearch} placeholder="Search this workspace…" />
        <kbd>Ctrl K</kbd>
      </label>
      <button class="activity-button" on:click={() => onNavigate('jobs')} aria-label="Open jobs">
        <span class="activity-orb" class:busy={activeJobs > 0}></span>
        <span>{activeJobs > 0 ? `${activeJobs} active` : 'Idle'}</span>
      </button>
    </header>
    <main class="workspace"><slot /></main>
  </div>
</div>
