<script lang="ts">
  import { locale, setLocale, type AppLocale } from '../i18n';
  import McpSetupWizard from '../components/McpSetupWizard.svelte';
  import { translator } from '../i18n/translate';
  import type { AppSettings, MigrationStatus } from '../types';
  import type { ScriptotarApi } from '../api/client';
  import { persistAppearance } from '../appearance';

  export let settings: AppSettings;
  export let migrationStatus: MigrationStatus;
  export let api: ScriptotarApi;
  export let onSaved: (settings: AppSettings) => Promise<void> | void = () => {};
  export let onMigrationStatus: (status: MigrationStatus) => Promise<void> | void = () => {};

  let draft = structuredClone(settings);
  let status = '';
  let saveError = '';
  let migrationBusy = false;
  let busy = false;
  $: dirty = JSON.stringify(draft) !== JSON.stringify(settings);

  $: migrationLabel = {
    completed: $translator('settings.migrationCompleted'),
    no_legacy_db: $translator('settings.migrationNone'),
    ready: $translator('settings.migrationReady'),
    in_progress: $translator('settings.migrationInProgress'),
    requires_choice: $translator('settings.migrationChoice'),
    invalid_db: $translator('settings.migrationInvalid'),
    failed: $translator('settings.migrationFailed')
  } satisfies Record<MigrationStatus['state'], string>;

  const progressStatus = (): MigrationStatus => ({
    state: 'in_progress',
    message: $translator('settings.migrationProgress'),
    candidates: []
  });

  const failedStatus = (): MigrationStatus => ({
    state: 'failed',
    message: $translator('settings.migrationFailureHelp'),
    candidates: []
  });

  function changeLocale(event: Event) {
    setLocale((event.currentTarget as HTMLSelectElement).value as AppLocale);
  }

  async function save() {
    if (busy) return;
    busy = true;
    status = '';
    saveError = '';
    try {
      await api.saveSettings(draft);
      persistAppearance(draft.appearance);
      await onSaved(structuredClone(draft));
      status = $translator('settings.saved');
    } catch (cause) {
      saveError = cause instanceof Error ? cause.message : $translator('settings.saveFailed');
    } finally {
      busy = false;
    }
  }

  async function chooseOutputDirectory() {
    if (busy) return;
    busy = true;
    status = '';
    saveError = '';
    try {
      const selected = await api.chooseOutputDirectory();
      if (selected) {
        draft.outputDirectory = selected;
        status = $translator('settings.outputSelected');
      }
    } catch (cause) {
      saveError = cause instanceof Error ? cause.message : $translator('settings.outputFailed');
    } finally {
      busy = false;
    }
  }

  function restoreDefaultOutput() {
    draft.outputDirectory = null;
    status = $translator('settings.outputReset');
    saveError = '';
  }

  async function retryMigration() {
    if (migrationBusy) return;
    migrationBusy = true;
    migrationStatus = progressStatus();
    try {
      const next = await api.retryLegacyMigration();
      migrationStatus = next;
      await onMigrationStatus(next);
    } catch {
      migrationStatus = failedStatus();
      await onMigrationStatus(migrationStatus);
    } finally {
      migrationBusy = false;
    }
  }

  async function chooseMigrationCandidate(candidateId: string) {
    if (migrationBusy) return;
    migrationBusy = true;
    migrationStatus = progressStatus();
    try {
      const next = await api.selectLegacyMigrationCandidate(candidateId);
      migrationStatus = next;
      await onMigrationStatus(next);
    } catch {
      migrationStatus = failedStatus();
      await onMigrationStatus(migrationStatus);
    } finally {
      migrationBusy = false;
    }
  }
</script>

<section class="view-head settings-head" data-i18n-ignore>
  <div><span class="eyebrow">{$translator('settings.headline')}</span><h1>{$translator('settings.heading')}</h1><p>{$translator('settings.description')}</p></div>
  <div class="save-cluster">{#if dirty}<span class="dirty-indicator" data-i18n-ignore><span></span>{$translator('settings.unsaved')}</span>{/if}<button class="button primary settings-save" disabled={busy} on:click={save}>{$translator('settings.save')}</button></div>
</section>

<div class="settings-layout" aria-busy={busy || migrationBusy}>
  <aside class="settings-nav panel" aria-label={$translator('settings.sections')} data-i18n-ignore>
    <span class="settings-nav-title">{$translator('settings.preferences')}</span>
    <a href="#settings-mcp">{$translator('settings.mcpWizard')}</a>
    <a href="#settings-transcription">{$translator('settings.nav.transcription')}</a>
    <a href="#settings-media">{$translator('settings.nav.media')}</a>
    <a href="#settings-storage">{$translator('settings.nav.storage')}</a>
    <a href="#settings-local">{$translator('settings.nav.local')}</a>
    <a href="#settings-watchlists">{$translator('settings.nav.watchlists')}</a>
    <span class="settings-nav-title secondary-group">{$translator('settings.advanced')}</span>
    <a href="#settings-migration">{$translator('settings.nav.migration')}</a>
    <a href="#settings-interface">{$translator('settings.nav.interface')}</a>
  </aside>

  <div class="settings-stack">
    <McpSetupWizard {api} />

    <section id="settings-transcription" data-i18n-ignore class="panel settings-section"><div><span class="eyebrow">{$translator('settings.nav.transcription')}</span><h2>{$translator('settings.transcriptionHeading')}</h2><p>{$translator('settings.transcriptionHelp')}</p></div><div class="settings-grid"><label><span>{$translator('settings.whisperModel')}</span><select bind:value={draft.whisperModel}><option>small</option><option>medium</option><option>turbo</option><option>large-v3</option></select></label><label><span>{$translator('settings.device')}</span><select bind:value={draft.device}><option>auto</option><option>cpu</option><option>cuda</option></select></label><label><span>{$translator('settings.language')}</span><select bind:value={draft.language}><option>auto</option><option>Arabic</option><option>English</option></select></label></div></section>

    <section id="settings-media" data-i18n-ignore class="panel settings-section"><div><span class="eyebrow">{$translator('settings.mediaEyebrow')}</span><h2>{$translator('settings.mediaHeading')}</h2><p>{$translator('settings.mediaHelp')}</p></div><div class="settings-grid"><label><span>{$translator('settings.quality')}</span><select bind:value={draft.quality}><option>720p</option><option>1080p</option><option value="Best">{$translator('settings.bestQuality')}</option><option value="Audio only">{$translator('settings.audioOnly')}</option></select></label><label><span>{$translator('settings.browserCookies')}</span><select bind:value={draft.cookies}><option>none</option><option>firefox</option><option>chrome</option><option>chromium</option><option>brave</option></select></label><label><span>{$translator('settings.durationLimit')}</span><select bind:value={draft.maxDuration}><option>30 min</option><option>60 min</option><option>2 hours</option><option value="Unlimited">{$translator('settings.unlimited')}</option></select></label></div></section>

    <section id="settings-storage" data-i18n-ignore class="panel settings-section"><div><span class="eyebrow">{$translator('settings.nav.storage')}</span><h2>{$translator('settings.storageHeading')}</h2><p>{$translator('settings.storageHelp')}</p></div><div class="output-picker"><span class="field-label">{$translator('settings.outputDirectory')}</span><code dir="auto">{draft.outputDirectory || $translator('settings.applicationDefault')}</code><div class="output-actions"><button class="button secondary" disabled={busy} on:click={chooseOutputDirectory}>{$translator('settings.chooseOutput')}</button><button class="button secondary" disabled={busy || draft.outputDirectory === null} on:click={restoreDefaultOutput}>{$translator('settings.restoreDefault')}</button></div></div></section>

    <section id="settings-local" data-i18n-ignore class="panel settings-section"><div><span class="eyebrow">{$translator('settings.localEyebrow')}</span><h2>{$translator('settings.localHeading')}</h2><p>{$translator('settings.localHelp')}</p></div><div class="toggle-grid"><label><input type="checkbox" bind:checked={draft.copyLocalSource}/><span><strong>{$translator('settings.copySource')}</strong><small>{$translator('settings.copySourceHelp')}</small></span></label><label><input type="checkbox" bind:checked={draft.translate}/><span><strong>{$translator('settings.translateSpeech')}</strong><small>{$translator('settings.translateHelp')}</small></span></label><label><input type="checkbox" bind:checked={draft.batched}/><span><strong>{$translator('settings.batched')}</strong><small>{$translator('settings.batchedHelp')}</small></span></label><label><input type="checkbox" bind:checked={draft.keepFailed}/><span><strong>{$translator('settings.keepFailed')}</strong><small>{$translator('settings.keepFailedHelp')}</small></span></label></div></section>

    <section id="settings-watchlists" data-i18n-ignore class="panel settings-section">
      <div><span class="eyebrow">{$translator('settings.watchEyebrow')}</span><h2>{$translator('settings.watchHeading')}</h2><p>{$translator('settings.watchHelp')}</p></div>
      <div class="watch-settings">
        <label class="watch-toggle"><input type="checkbox" bind:checked={draft.autoWatch}/><span><strong>{$translator('settings.watchAutomatic')}</strong><small>{$translator('settings.watchAutomaticHelp')}</small></span></label>
        <label><span>{$translator('settings.watchInterval')}</span><select aria-label={$translator('settings.watchControl')} bind:value={draft.watchInterval} disabled={!draft.autoWatch}><option>30 min</option><option>60 min</option><option>2 hours</option><option>6 hours</option></select></label>
      </div>
    </section>

    <section id="settings-migration" data-i18n-ignore class="panel settings-section migration-section">
      <div><span class="eyebrow">{$translator('settings.nav.migration')}</span><h2>{$translator('settings.migrationHeading')}</h2><p>{$translator('settings.migrationHelp')}</p></div>
      <div class="migration-status" data-testid="migration-status" data-state={migrationStatus.state} aria-live="polite">
        <div class="migration-heading"><strong>{migrationLabel[migrationStatus.state]}</strong><span class={`migration-pill migration-${migrationStatus.state}`}>{migrationLabel[migrationStatus.state]}</span></div>
        <p dir="auto">{migrationStatus.message}</p>
        {#if migrationStatus.state === 'completed' && migrationStatus.report}
          <p class="migration-counts">{$translator('settings.migrationImported', { projects: migrationStatus.report.projects, jobs: migrationStatus.report.jobs, transcripts: migrationStatus.report.transcripts, research: migrationStatus.report.research_items, watchlists: migrationStatus.report.watchlists, ai: migrationStatus.report.ai_runs })}</p>
        {/if}
        {#if migrationStatus.state === 'requires_choice'}
          <div class="migration-candidates" aria-label={$translator('settings.migrationChoices')}>
            {#each migrationStatus.candidates as candidate (candidate.id)}
              <button class="button secondary" disabled={migrationBusy} on:click={() => chooseMigrationCandidate(candidate.id)}>{candidate.label}</button>
            {/each}
          </div>
        {:else if migrationStatus.state !== 'completed' && migrationStatus.state !== 'in_progress'}
          <button class="button secondary" disabled={migrationBusy} on:click={retryMigration}>{migrationBusy ? $translator('settings.migrationChecking') : migrationStatus.state === 'ready' ? $translator('settings.migrationImport') : $translator('settings.migrationRetry')}</button>
        {/if}
      </div>
    </section>

    <section id="settings-interface" data-i18n-ignore class="panel settings-section"><div><span class="eyebrow">{$translator('settings.interfaceEyebrow')}</span><h2>{$translator('settings.interfaceHeading')}</h2><p>{$translator('settings.interfaceHelp')}</p></div><div class="settings-grid"><label><span>{$translator('settings.theme')}</span><select aria-label={$translator('settings.theme')} bind:value={draft.appearance}><option value="dark">{$translator('settings.themeDark')}</option><option value="system">{$translator('settings.themeSystem')}</option></select></label><label data-i18n-ignore><span>{$translator('language.label')}</span><select aria-label={$translator('settings.languageControl')} value={$locale} on:change={changeLocale}><option value="en">{$translator('language.english')}</option><option value="ar">{$translator('language.arabic')}</option></select></label></div></section>
  </div>
</div>

{#if status}<p class="status-copy settings-status" aria-live="polite" data-i18n-ignore>{status}</p>{/if}
{#if saveError}<p class="status-copy settings-status" role="alert" data-i18n-ignore>{saveError}</p>{/if}

<style>
  .settings-head { align-items: center; }
  .save-cluster { display: flex; align-items: center; gap: 10px; }
  .dirty-indicator { display: inline-flex; align-items: center; gap: 6px; color: var(--muted); font-size: 10px; white-space: nowrap; }
  .dirty-indicator > span { width: 6px; height: 6px; border-radius: 50%; background: var(--warn); }
  .settings-layout { display: grid; grid-template-columns: 190px minmax(0, 1fr); gap: 12px; align-items: start; }
  .settings-nav { position: sticky; top: 82px; display: grid; gap: 2px; padding: 8px; }
  .settings-nav-title { padding: 8px 9px 5px; color: var(--faint); font-size: 9px; font-weight: 700; letter-spacing: .07em; text-transform: uppercase; }
  .settings-nav-title.secondary-group { margin-top: 7px; padding-top: 11px; border-top: 1px solid var(--border); }
  .settings-nav a { min-height: 34px; display: flex; align-items: center; padding: 0 9px; border-radius: 7px; color: var(--muted); text-decoration: none; font-size: 11px; }
  .settings-nav a:hover, .settings-nav a:focus-visible { color: var(--text); background: var(--surface-2); }
  .settings-stack { display: grid; gap: 12px; min-width: 0; }
  .settings-section { scroll-margin-top: 82px; grid-template-columns: minmax(220px, 280px) minmax(0, 1fr); }
  .settings-section > div:first-child { max-width: 310px; }
  .output-picker { display: grid; gap: 9px; min-width: 0; }
  .field-label { color: var(--muted); font-size: 10px; }
  code { display: block; max-width: 560px; overflow-wrap: anywhere; padding: 10px 12px; border: 1px solid var(--border); border-radius: 9px; background: var(--surface-2); color: var(--text); }
  .output-actions { display: flex; flex-wrap: wrap; gap: 8px; }
  .watch-settings { display: grid; gap: 14px; min-width: min(560px, 100%); }
  .watch-toggle { display: flex; gap: 10px; align-items: start; }
  .watch-toggle span { display: grid; gap: 3px; }
  .watch-toggle small { color: var(--muted); line-height: 1.4; }
  .migration-section { border-color: color-mix(in srgb, var(--info) 18%, var(--border)); }
  .migration-status { display: grid; gap: 10px; min-width: min(600px, 100%); padding: 14px; border: 1px solid var(--border); border-radius: 10px; background: color-mix(in srgb, var(--surface-2) 42%, transparent); }
  .migration-heading { display: flex; align-items: center; justify-content: space-between; gap: 12px; }
  .migration-pill { padding: 4px 7px; border: 1px solid var(--border); border-radius: 999px; color: var(--muted); font-size: 10px; text-transform: uppercase; letter-spacing: .04em; }
  .migration-failed, .migration-invalid_db { color: var(--danger, #ff8c8c); }
  .migration-requires_choice, .migration-ready, .migration-in_progress { color: var(--accent); }
  .migration-status p { margin: 0; color: var(--muted); line-height: 1.5; }
  .migration-counts { color: var(--text) !important; }
  .migration-candidates { display: flex; flex-wrap: wrap; gap: 8px; }
  .settings-status { margin-inline-start: 202px; }

  @media (max-width: 1000px) {
    .settings-layout { grid-template-columns: 1fr; }
    .settings-nav { position: static; display: flex; gap: 4px; overflow-x: auto; }
    .settings-nav-title { display: none; }
    .settings-nav a { flex: 0 0 auto; border: 1px solid var(--border); background: var(--surface); }
    .settings-status { margin-inline-start: 0; }
  }
  @media (max-width: 760px) {
    .settings-head { align-items: flex-start; }
    .save-cluster { width: 100%; justify-content: space-between; }
    .settings-section { grid-template-columns: 1fr; }
  }
</style>
