<script lang="ts">
  import { tick } from 'svelte';
  import { locale, setLocale, type AppLocale } from '../i18n';
  import { translator } from '../i18n/translate';
  import type { Project, ViewId, WorkspaceSearchResult } from '../types';
  import Icon, { type IconName } from './Icon.svelte';

  export let activeView: ViewId;
  export let activeProjectId: string;
  export let projects: Project[];
  export let activeJobs = 0;
  export let globalSearch = '';
  export let searchResults: WorkspaceSearchResult[] = [];
  export let onNavigate: (view: ViewId) => void;
  export let onProjectChange: (id: string) => Promise<void> | void;
  export let onSearchSelect: (result: WorkspaceSearchResult) => Promise<void> | void;

  type NavItem = { id: ViewId; label: string; icon: IconName; shortcut: string };
  type PaletteItem = { id: string; label: string; meta: string; keywords: string; run: () => Promise<void> | void };

  const SIDEBAR_KEY = 'scriptotar.sidebarCollapsed';
  let searchInput: HTMLInputElement;
  let paletteOpen = false;
  let paletteIndex = 0;
  let sidebarCollapsed = readSidebarState();

  $: workNav = [
    { id: 'dashboard', label: $translator('nav.home'), icon: 'home', shortcut: 'D' },
    { id: 'research', label: $translator('nav.research'), icon: 'research', shortcut: 'R' },
    { id: 'jobs', label: $translator('nav.queue'), icon: 'queue', shortcut: 'J' },
    { id: 'transcript', label: $translator('nav.transcripts'), icon: 'transcript', shortcut: 'T' }
  ] satisfies NavItem[];

  $: createNav = [
    { id: 'ai', label: $translator('nav.ai'), icon: 'sparkles', shortcut: 'A' },
    { id: 'library', label: $translator('nav.library'), icon: 'library', shortcut: 'L' }
  ] satisfies NavItem[];

  $: baseCommands = [
    command('new-transcription', $translator('command.newTranscription'), $translator('search.commands'), 'new transcription import media queue', () => navigate('jobs')),
    command('new-research', $translator('command.newResearch'), $translator('search.commands'), 'new research creator scan', () => navigate('research')),
    command('home', $translator('command.openHome'), $translator('search.navigate'), 'home dashboard', () => navigate('dashboard')),
    command('queue', $translator('command.openQueue'), $translator('search.navigate'), 'queue jobs activity', () => navigate('jobs')),
    command('transcripts', $translator('command.openTranscripts'), $translator('search.navigate'), 'transcript reader', () => navigate('transcript')),
    command('ai', $translator('command.openAi'), $translator('search.navigate'), 'ai studio prompt', () => navigate('ai')),
    command('library', $translator('command.openLibrary'), $translator('search.navigate'), 'library local index', () => navigate('library')),
    command('settings', $translator('command.openSettings'), $translator('search.navigate'), 'settings preferences', () => navigate('settings'))
  ] satisfies PaletteItem[];

  $: projectCommands = globalSearch.trim()
    ? projects.map((project) => command(
        `project:${project.id}`,
        project.name,
        $translator('search.project'),
        `${project.name} ${project.description || ''} project`,
        async () => {
          await onProjectChange(project.id);
          closePalette();
        }
      ))
    : [];

  $: commandQuery = globalSearch.trim().toLocaleLowerCase();
  $: filteredCommands = [...baseCommands, ...projectCommands].filter((item) => !commandQuery || `${item.label} ${item.keywords}`.toLocaleLowerCase().includes(commandQuery));
  $: resultItems = searchResults.map((result) => command(
    `search:${result.id}`,
    result.title,
    result.kind,
    `${result.title} ${result.subtitle} ${result.kind}`,
    async () => {
      await onSearchSelect(result);
      closePalette();
    }
  ));
  $: paletteItems = [...filteredCommands.slice(0, 8), ...resultItems.slice(0, 8)] as PaletteItem[];
  $: if (paletteIndex >= paletteItems.length) paletteIndex = Math.max(0, paletteItems.length - 1);

  function readSidebarState() {
    if (typeof window === 'undefined') return false;
    try { return window.localStorage.getItem(SIDEBAR_KEY) === '1'; }
    catch { return false; }
  }

  function command(id: string, label: string, meta: string, keywords: string, run: () => Promise<void> | void): PaletteItem {
    return { id, label, meta, keywords, run };
  }

  function navigate(view: ViewId) {
    onNavigate(view);
    closePalette();
  }

  function toggleSidebar() {
    sidebarCollapsed = !sidebarCollapsed;
    try { window.localStorage.setItem(SIDEBAR_KEY, sidebarCollapsed ? '1' : '0'); }
    catch { /* A hardened webview may deny local persistence. */ }
  }

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

  async function openPalette() {
    paletteOpen = true;
    paletteIndex = 0;
    await tick();
    searchInput?.focus();
    searchInput?.select();
  }

  function topbarSearch(event: Event) {
    globalSearch = (event.currentTarget as HTMLInputElement).value;
    void openPalette();
  }

  function closePalette() {
    paletteOpen = false;
    globalSearch = '';
    paletteIndex = 0;
  }

  function paletteKeys(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      event.preventDefault();
      closePalette();
      return;
    }
    if (event.key === 'ArrowDown') {
      event.preventDefault();
      paletteIndex = paletteItems.length ? (paletteIndex + 1) % paletteItems.length : 0;
      return;
    }
    if (event.key === 'ArrowUp') {
      event.preventDefault();
      paletteIndex = paletteItems.length ? (paletteIndex - 1 + paletteItems.length) % paletteItems.length : 0;
      return;
    }
    if (event.key === 'Enter' && paletteItems[paletteIndex]) {
      event.preventDefault();
      void paletteItems[paletteIndex].run();
    }
  }

  function globalShortcut(event: KeyboardEvent) {
    if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'k') {
      event.preventDefault();
      void openPalette();
      return;
    }
    if (paletteOpen && event.key === 'Escape') closePalette();
  }
</script>
<svelte:window onkeydown={globalShortcut} />

<div class:collapsed={sidebarCollapsed} class="app-frame shell-v2">
  <aside class="sidebar" aria-label={$translator('nav.primary')} data-i18n-ignore>
    <div class="brand-row">
      <div class="brand-lockup">
        <div class="brand-mark">S</div>
        <div class="brand-copy"><strong>Scriptotar</strong><span>{$translator('brand.subtitle')}</span></div>
      </div>
      <button class="rail-toggle" type="button" on:click={toggleSidebar} aria-label={sidebarCollapsed ? $translator('sidebar.expand') : $translator('sidebar.collapse')} title={sidebarCollapsed ? $translator('sidebar.expand') : $translator('sidebar.collapse')}>
        <Icon name="panel" size={17} />
      </button>
    </div>

    <nav class="primary-nav">
      <section class="nav-group" aria-label={$translator('nav.work')}>
        <span class="nav-group-label">{$translator('nav.work')}</span>
        {#each workNav as item}
          <button class:active={activeView === item.id} aria-current={activeView === item.id ? 'page' : undefined} title={`${item.label} · ${item.shortcut}`} on:click={() => navigate(item.id)}>
            <Icon name={item.icon} />
            <span class="nav-label">{item.label}</span>
            {#if item.id === 'jobs' && activeJobs > 0}<b class="activity-count" aria-label={$translator('jobs.active', { count: activeJobs })}>{activeJobs}</b>{/if}
          </button>
        {/each}
      </section>

      <section class="nav-group" aria-label={$translator('nav.create')}>
        <span class="nav-group-label">{$translator('nav.create')}</span>
        {#each createNav as item}
          <button class:active={activeView === item.id} aria-current={activeView === item.id ? 'page' : undefined} title={`${item.label} · ${item.shortcut}`} on:click={() => navigate(item.id)}>
            <Icon name={item.icon} />
            <span class="nav-label">{item.label}</span>
          </button>
        {/each}
      </section>
    </nav>

    <div class="sidebar-utility">
      <button class:active={activeView === 'settings'} class="settings-link" aria-current={activeView === 'settings' ? 'page' : undefined} title={`${$translator('nav.settings')} · ,`} on:click={() => navigate('settings')}>
        <Icon name="settings" />
        <span class="nav-label">{$translator('nav.settings')}</span>
      </button>
      <div class="local-status" title={$translator('status.workspace')}>
        <span class="local-dot"></span>
        <div class="local-copy"><strong>{$translator('status.local')}</strong><small>{$translator('status.workspace')}</small></div>
      </div>
    </div>
  </aside>

  <div class="workspace-shell">
    <header class="topbar" data-i18n-ignore>
      <div class="project-control">
        <label for="project-select">{$translator('project.label')}</label>
        <select id="project-select" value={activeProjectId} on:change={changeProject}>
          {#each projects as project}<option value={project.id}>{project.name}</option>{/each}
        </select>
      </div>

      <label class="command-trigger">
        <Icon name="search" size={17} />
        <input aria-label={$translator('search.field')} value={globalSearch} on:focus={openPalette} on:input={topbarSearch} placeholder={$translator('search.open')} />
        <kbd>{$translator('palette.shortcut')}</kbd>
      </label>

      <div class="topbar-actions">
        <label class="language-control" title={$translator('language.label')}>
          <Icon name="globe" size={17} />
          <span class="sr-only">{$translator('language.label')}</span>
          <select value={$locale} on:change={changeLocale} aria-label={$translator('language.label')}>
            <option value="en">EN</option>
            <option value="ar">ع</option>
          </select>
        </label>
        <button class:busy={activeJobs > 0} class="activity-button" type="button" on:click={() => navigate('jobs')} aria-label={$translator('jobs.open')}>
          <span class="activity-orb" class:busy={activeJobs > 0}></span>
          <span>{activeJobs > 0 ? $translator('jobs.active', { count: activeJobs }) : $translator('jobs.idle')}</span>
        </button>
      </div>
    </header>
    <main class="workspace"><slot /></main>
  </div>
</div>

{#if paletteOpen}
  <div class="palette-backdrop" role="presentation" on:mousedown={(event) => { if (event.currentTarget === event.target) closePalette(); }} data-i18n-ignore>
    <section class="command-palette" role="dialog" aria-modal="true" aria-label={$translator('search.dialog')}>
      <label class="palette-search">
        <Icon name="search" size={19} />
        <span class="sr-only">{$translator('search.open')}</span>
        <input bind:this={searchInput} bind:value={globalSearch} on:keydown={paletteKeys} placeholder={$translator('search.placeholder')} aria-label={$translator('search.open')} aria-controls="command-results" aria-activedescendant={paletteItems[paletteIndex] ? `palette-${paletteItems[paletteIndex].id}` : undefined} />
        <kbd>Esc</kbd>
      </label>
      <div id="command-results" class="palette-results" role="listbox" aria-label={$translator('search.results')}>
        {#if paletteItems.length === 0}
          <div class="palette-empty">{$translator('search.empty')}</div>
        {:else}
          {#each paletteItems as item, index (item.id)}
            <button id={`palette-${item.id}`} class:selected={index === paletteIndex} role="option" aria-selected={index === paletteIndex} on:mouseenter={() => paletteIndex = index} on:click={() => item.run()}>
              <span class="palette-item-copy"><strong>{item.label}</strong><small>{item.meta}</small></span>
              <span class="palette-arrow" aria-hidden="true">↵</span>
            </button>
          {/each}
        {/if}
      </div>
      <footer class="palette-footer"><span>{$translator('palette.hint')}</span><span class="privacy-hint"><span class="local-dot"></span>{$translator('status.local')}</span></footer>
    </section>
  </div>
{/if}

<style>
  .app-frame { --shell-sidebar-width: var(--sidebar-expanded); grid-template-columns: var(--shell-sidebar-width) minmax(0, 1fr); background: var(--color-canvas); }
  .app-frame.collapsed { --shell-sidebar-width: var(--sidebar-collapsed); }

  .sidebar { z-index: 20; width: var(--shell-sidebar-width); padding: var(--space-3); border: 0; border-inline-end: 1px solid var(--color-border); background: color-mix(in srgb, var(--color-sidebar) 94%, transparent); transition: width var(--motion-normal) var(--ease-standard); }
  .brand-row { display: flex; align-items: center; gap: var(--space-2); min-height: 48px; padding: 0 2px var(--space-4); border-bottom: 1px solid var(--color-border); }
  .brand-lockup { flex: 1; min-width: 0; padding: 0; border: 0; }
  .brand-copy { min-width: 0; overflow: hidden; }
  .brand-copy strong { font-size: var(--text-md); letter-spacing: -.01em; }
  .brand-copy span { margin-top: 2px; font-size: 9px; white-space: nowrap; }
  .brand-mark { flex: 0 0 auto; width: 34px; height: 34px; border-radius: var(--radius-md); }
  .rail-toggle { display: grid; place-items: center; width: 32px; height: 32px; padding: 0; border: 1px solid transparent; border-radius: var(--radius-sm); background: transparent; color: var(--color-text-faint); }
  .rail-toggle:hover { border-color: var(--color-border); color: var(--color-text); background: var(--color-surface-raised); }

  .primary-nav { display: grid; gap: var(--space-5); padding: var(--space-4) 0; }
  .nav-group { display: grid; gap: 3px; }
  .nav-group-label { padding: 0 10px 6px; color: var(--color-text-faint); font-size: 9px; font-weight: 700; letter-spacing: .09em; text-transform: uppercase; white-space: nowrap; }
  .nav-group button,
  .settings-link { position: relative; display: grid; grid-template-columns: 22px minmax(0, 1fr) auto; align-items: center; gap: 10px; width: 100%; min-height: 42px; padding: 0 10px; border: 1px solid transparent; border-radius: var(--radius-md); background: transparent; color: var(--color-text-muted); text-align: start; }
  .nav-group button:hover,
  .settings-link:hover { color: var(--color-text); background: var(--color-surface-raised); }
  .nav-group button.active,
  .settings-link.active { color: var(--color-text); border-color: color-mix(in srgb, var(--color-signal-strong) 36%, var(--color-border)); background: color-mix(in srgb, var(--color-signal-strong) 10%, var(--color-surface)); }
  .nav-group button.active::before,
  .settings-link.active::before { content: ''; position: absolute; inset-inline-start: -13px; width: 3px; height: 22px; border-radius: var(--radius-pill); background: var(--color-signal-strong); }
  .nav-label { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: var(--text-sm); font-weight: 620; }
  .activity-count { position: static; min-width: 20px; padding: 2px 6px; border-radius: var(--radius-pill); background: var(--color-signal); color: var(--color-signal-ink); font-size: 10px; text-align: center; }

  .sidebar-utility { margin-top: auto; display: grid; gap: var(--space-2); }
  .local-status { display: flex; align-items: center; gap: 9px; min-height: 42px; padding: 8px 10px; color: var(--color-text-muted); }
  .local-copy { min-width: 0; overflow: hidden; }
  .local-copy strong, .local-copy small { display: block; white-space: nowrap; }
  .local-copy strong { color: var(--color-text); font-size: var(--text-xs); }
  .local-copy small { margin-top: 2px; color: var(--color-text-faint); font-size: 9px; }

  .collapsed .brand-copy,
  .collapsed .nav-group-label,
  .collapsed .nav-label,
  .collapsed .local-copy { display: none; }
  .collapsed .brand-row { justify-content: center; flex-direction: column; padding-bottom: var(--space-3); }
  .collapsed .brand-lockup { flex: 0 0 auto; }
  .collapsed .rail-toggle { width: 34px; }
  .collapsed .nav-group button,
  .collapsed .settings-link { grid-template-columns: 1fr; place-items: center; padding: 0; }
  .collapsed .activity-count { position: absolute; inset-inline-end: 2px; top: 2px; min-width: 17px; padding: 1px 4px; }
  .collapsed .local-status { justify-content: center; padding-inline: 0; }

  .workspace-shell { min-width: 0; }
  .topbar { min-height: var(--topbar-height); grid-template-columns: minmax(170px, 230px) minmax(240px, 620px) auto; gap: var(--space-3); padding: 10px var(--space-5); border-color: var(--color-border); background: color-mix(in srgb, var(--color-canvas) 86%, transparent); }
  .project-control { min-width: 0; }
  .project-control label { color: var(--color-text-faint); font-size: 9px; }
  .project-control select { min-width: 0; border-color: transparent; background: transparent; font-weight: 650; }
  .project-control select:hover, .project-control select:focus-visible { border-color: var(--color-border); background: var(--color-surface); }

  .command-trigger { display: grid; grid-template-columns: auto minmax(0, 1fr) auto; align-items: center; gap: 9px; min-height: var(--control-height); padding: 0 10px; border: 1px solid var(--color-border); border-radius: var(--radius-md); background: color-mix(in srgb, var(--color-surface) 86%, transparent); color: var(--color-text-faint); text-align: start; cursor: text; }
  .command-trigger:hover, .command-trigger:focus-within { border-color: var(--color-border-strong); background: var(--color-surface-raised); color: var(--color-text-muted); }
  .command-trigger input { min-width: 0; min-height: 34px; padding: 0; border: 0; background: transparent; color: var(--color-text); outline: 0; box-shadow: none; font-size: var(--text-sm); }
  .command-trigger input:focus-visible { box-shadow: none; }
  .command-trigger input::placeholder { color: var(--color-text-faint); }
  .command-trigger kbd { justify-self: end; }

  .topbar-actions { justify-self: end; display: flex; align-items: center; gap: var(--space-2); }
  .language-control { display: flex; align-items: center; gap: 4px; min-height: var(--control-height); padding-inline: 8px 4px; border: 1px solid var(--color-border); border-radius: var(--radius-md); background: var(--color-surface); color: var(--color-text-muted); }
  .language-control select { min-width: 45px; min-height: 32px; padding: 0 4px; border: 0; background: transparent; color: var(--color-text); font-size: var(--text-xs); font-weight: 700; }
  .activity-button { min-height: var(--control-height); border-radius: var(--radius-md); font-size: var(--text-xs); }
  .activity-button.busy { border-color: color-mix(in srgb, var(--color-info) 44%, var(--color-border)); color: var(--color-text); }

  .workspace { width: min(var(--workspace-max), 100%); padding: 26px 28px 48px; }

  .palette-backdrop { position: fixed; inset: 0; z-index: 100; display: grid; place-items: start center; padding: min(15vh, 130px) var(--space-4) var(--space-4); background: rgb(0 0 0 / 48%); backdrop-filter: blur(6px); }
  .command-palette { width: min(680px, 100%); overflow: hidden; border: 1px solid var(--color-border-strong); border-radius: var(--radius-lg); background: var(--color-surface); box-shadow: var(--shadow-overlay); }
  .palette-search { display: grid; grid-template-columns: auto minmax(0, 1fr) auto; align-items: center; gap: 10px; min-height: 58px; padding: 0 var(--space-4); border-bottom: 1px solid var(--color-border); color: var(--color-text-faint); }
  .palette-search input { width: 100%; min-height: 48px; padding: 0; border: 0; background: transparent; color: var(--color-text); outline: 0; box-shadow: none; font-size: var(--text-md); }
  .palette-search input:focus-visible { box-shadow: none; }
  .palette-results { max-height: min(54vh, 480px); overflow-y: auto; padding: 6px; }
  .palette-results button { display: grid; grid-template-columns: minmax(0, 1fr) auto; align-items: center; gap: var(--space-3); width: 100%; min-height: 54px; padding: 8px 10px; border: 1px solid transparent; border-radius: var(--radius-md); background: transparent; text-align: start; }
  .palette-results button:hover, .palette-results button.selected { border-color: var(--color-border); background: var(--color-surface-raised); }
  .palette-item-copy strong, .palette-item-copy small { display: block; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .palette-item-copy strong { color: var(--color-text); font-size: var(--text-sm); }
  .palette-item-copy small { margin-top: 3px; color: var(--color-text-faint); font-size: var(--text-xs); }
  .palette-arrow { color: var(--color-text-faint); font-family: var(--font-technical); }
  .palette-empty { padding: 30px 20px; color: var(--color-text-muted); text-align: center; font-size: var(--text-sm); }
  .palette-footer { display: flex; align-items: center; justify-content: space-between; gap: var(--space-3); min-height: 38px; padding: 0 var(--space-4); border-top: 1px solid var(--color-border); color: var(--color-text-faint); font-size: 10px; }
  .privacy-hint { display: inline-flex; align-items: center; gap: 7px; white-space: nowrap; }
  .privacy-hint .local-dot { width: 6px; height: 6px; box-shadow: none; }

  @media (max-width: 1040px) {
    .app-frame { --shell-sidebar-width: var(--sidebar-collapsed); }
    .brand-copy, .nav-group-label, .nav-label, .local-copy { display: none; }
    .brand-row { justify-content: center; flex-direction: column; padding-bottom: var(--space-3); }
    .brand-lockup { flex: 0 0 auto; }
    .nav-group button, .settings-link { grid-template-columns: 1fr; place-items: center; padding: 0; }
    .activity-count { position: absolute; inset-inline-end: 2px; top: 2px; min-width: 17px; padding: 1px 4px; }
    .local-status { justify-content: center; padding-inline: 0; }
    .topbar { grid-template-columns: minmax(150px, 210px) minmax(180px, 1fr) auto; padding-inline: var(--space-3); }
    .activity-button > span:last-child { display: none; }
  }

  @media (max-width: 720px) {
    .app-frame { display: block; }
    .sidebar { position: static; width: 100%; height: auto; padding: 8px; border-inline-end: 0; border-bottom: 1px solid var(--color-border); }
    .brand-row, .sidebar-utility, .nav-group-label { display: none; }
    .primary-nav { display: flex; gap: 4px; padding: 0; overflow-x: auto; }
    .nav-group { display: flex; gap: 4px; }
    .nav-group button { display: flex; min-width: 42px; width: 42px; justify-content: center; padding: 0; }
    .nav-group button.active::before { inset-inline-start: 9px; inset-block-start: auto; bottom: -5px; width: 22px; height: 3px; }
    .topbar { position: static; grid-template-columns: minmax(0, 1fr) auto; gap: 8px; padding: 8px 10px; }
    .project-control { grid-template-columns: 1fr; }
    .project-control label { display: none; }
    .command-trigger { grid-column: 1 / -1; grid-row: 2; }
    .command-trigger kbd { display: none; }
    .workspace { padding: 20px 12px 36px; }
    .palette-backdrop { padding-top: 8vh; }
    .palette-footer > span:first-child { display: none; }
  }
</style>
