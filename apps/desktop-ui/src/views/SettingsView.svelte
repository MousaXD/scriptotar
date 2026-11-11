<script lang="ts">
  import { locale, setLocale, type AppLocale } from '../i18n';
  import McpSetupWizard from '../components/McpSetupWizard.svelte';
  import Select from '../components/ui/Select.svelte';
  import Switch from '../components/ui/Switch.svelte';
  import { translator } from '../i18n/translate';
  import type { AppSettings, MigrationStatus } from '../types';
  import type { ScriptotarApi } from '../api/client';
  import { persistAppearance } from '../appearance';

  export let settings: AppSettings;
  export let migrationStatus: MigrationStatus;
  export let api: ScriptotarApi;
  export let onSaved: (settings: AppSettings) => Promise<void> | void = () => {};
  export let onMigrationStatus: (status: MigrationStatus) => Promise<void> | void = () => {};

  let savedSnapshot = structuredClone(settings);
  let draft = structuredClone(savedSnapshot);
  let status: '' | 'saved' | 'outputSelected' | 'outputReset' = '';
  let saveError = '';
  let migrationBusy = false;
  let busy = false;
  $: dirty = JSON.stringify(draft) !== JSON.stringify(savedSnapshot);

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

  function changeLocale(next: string) {
    setLocale(next as AppLocale);
  }

  function discardChanges() {
    if (busy) return;
    draft = structuredClone(savedSnapshot);
    status = '';
    saveError = '';
  }

  async function save() {
    if (busy || !dirty) return;
    busy = true;
    status = '';
    saveError = '';
    try {
      const next = structuredClone(draft);
      await api.saveSettings(next);
      savedSnapshot = structuredClone(next);
      persistAppearance(next.appearance);
      await onSaved(structuredClone(next));
      status = 'saved';
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
        status = 'outputSelected';
      }
    } catch (cause) {
      saveError = cause instanceof Error ? cause.message : $translator('settings.outputFailed');
    } finally {
      busy = false;
    }
  }

  function restoreDefaultOutput() {
    draft.outputDirectory = null;
    status = 'outputReset';
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
  <div><h1>{$translator('settings.heading')}</h1></div>
  <div class="save-cluster">{#if dirty}<span class="dirty-indicator" role="status"><span></span>{$translator('settings.unsaved')}</span><button class="button secondary" disabled={busy} on:click={discardChanges}>{$translator('settings.discard')}</button>{/if}<button class="button primary settings-save" disabled={busy || !dirty} on:click={save}>{busy ? $translator('settings.saving') : $translator('settings.save')}</button></div>
</section>

<div class="settings-layout" aria-busy={busy || migrationBusy}>
  <aside class="settings-nav panel" aria-label={$translator('settings.sections')} data-i18n-ignore>
    <span class="settings-nav-title">{$translator('settings.preferences')}</span>
    <a href="#settings-interface">{$translator('settings.nav.interface')}</a>
    <a href="#settings-transcription">{$translator('settings.nav.transcription')}</a>
    <a href="#settings-media">{$translator('settings.nav.media')}</a>
    <a href="#settings-storage">{$translator('settings.nav.storage')}</a>
    <a href="#settings-local">{$translator('settings.nav.local')}</a>
    <a href="#settings-watchlists">{$translator('settings.nav.watchlists')}</a>
    <span class="settings-nav-title secondary-group">{$translator('settings.advanced')}</span>
    <a href="#settings-mcp">{$translator('settings.mcpWizard')}</a>
    <a href="#settings-migration">{$translator('settings.nav.migration')}</a>
  </aside>

  <div class="settings-stack">
    <section id="settings-interface" data-i18n-ignore class="panel settings-section">
      <div><span class="eyebrow">{$translator('settings.interfaceEyebrow')}</span><h2>{$translator('settings.interfaceHeading')}</h2></div>
      <div class="settings-grid">
        <Select label={$translator('settings.theme')} value={draft.appearance}
          options={[{ value: 'dark', label: $translator('settings.themeDark') }, { value: 'system', label: $translator('settings.themeSystem') }]}
          onChange={(next) => { draft.appearance = next as typeof draft.appearance; }} />
        <Select label={$translator('settings.languageControl')} value={$locale}
          options={[{ value: 'en', label: $translator('language.english') }, { value: 'ar', label: $translator('language.arabic') }]}
          onChange={changeLocale} />
      </div>
    </section>

    <section id="settings-transcription" data-i18n-ignore class="panel settings-section">
      <div><span class="eyebrow">{$translator('settings.nav.transcription')}</span><h2>{$translator('settings.transcriptionHeading')}</h2></div>
      <div class="settings-grid">
        <Select label={$translator('settings.whisperModel')} value={draft.whisperModel}
          options={['small', 'medium', 'turbo', 'large-v3'].map((value) => ({ value, label: value }))}
          onChange={(next) => { draft.whisperModel = next as typeof draft.whisperModel; }} />
        <Select label={$translator('settings.device')} value={draft.device}
          options={[{ value: 'auto', label: $translator('settings.autoOption') }, { value: 'cpu', label: 'cpu' }, { value: 'cuda', label: 'cuda' }]}
          onChange={(next) => { draft.device = next as typeof draft.device; }} />
        <Select label={$translator('settings.language')} value={draft.language}
          options={[
            { value: 'auto', label: $translator('settings.autoOption') },
            { value: 'Arabic', label: $translator('settings.arabicOption') },
            { value: 'English', label: $translator('settings.englishOption') }
          ]} onChange={(next) => { draft.language = next as typeof draft.language; }} />
      </div>
    </section>

    <section id="settings-media" data-i18n-ignore class="panel settings-section">
      <div><span class="eyebrow">{$translator('settings.mediaEyebrow')}</span><h2>{$translator('settings.mediaHeading')}</h2></div>
      <div class="settings-grid">
        <Select label={$translator('settings.quality')} value={draft.quality}
          options={[
            { value: '720p', label: '720p' }, { value: '1080p', label: '1080p' },
            { value: 'Best', label: $translator('settings.bestQuality') },
            { value: 'Audio only', label: $translator('settings.audioOnly') }
          ]} onChange={(next) => { draft.quality = next as typeof draft.quality; }} />
        <Select label={$translator('settings.browserCookies')} value={draft.cookies}
          options={[
            { value: 'none', label: $translator('settings.noneOption') },
            ...['firefox', 'chrome', 'chromium', 'brave'].map((value) => ({ value, label: value }))
          ]} onChange={(next) => { draft.cookies = next as typeof draft.cookies; }} />
        <Select label={$translator('settings.durationLimit')} value={draft.maxDuration}
          options={[
            { value: '30 min', label: $translator('settings.thirtyMinutes') },
            { value: '60 min', label: $translator('settings.sixtyMinutes') },
            { value: '2 hours', label: $translator('settings.twoHours') },
            { value: 'Unlimited', label: $translator('settings.unlimited') }
          ]} onChange={(next) => { draft.maxDuration = next as typeof draft.maxDuration; }} />
      </div>
    </section>

    <section id="settings-storage" data-i18n-ignore class="panel settings-section"><div><span class="eyebrow">{$translator('settings.nav.storage')}</span><h2>{$translator('settings.storageHeading')}</h2></div><div class="output-picker"><span class="field-label">{$translator('settings.outputDirectory')}</span><code dir={draft.outputDirectory ? 'ltr' : 'auto'}>{draft.outputDirectory || $translator('settings.applicationDefault')}</code><div class="output-actions"><button class="button secondary" disabled={busy} on:click={chooseOutputDirectory}>{$translator('settings.chooseOutput')}</button><button class="button secondary" disabled={busy || draft.outputDirectory === null} on:click={restoreDefaultOutput}>{$translator('settings.restoreDefault')}</button></div></div></section>

    <section id="settings-local" data-i18n-ignore class="panel settings-section">
      <div><span class="eyebrow">{$translator('settings.localEyebrow')}</span><h2>{$translator('settings.localHeading')}</h2></div>
      <div class="toggle-grid">
        <Switch label={$translator('settings.copySource')} description={$translator('settings.copySourceHelp')} bind:checked={draft.copyLocalSource} />
        <Switch label={$translator('settings.translateSpeech')} description={$translator('settings.translateHelp')} bind:checked={draft.translate} />
        <Switch label={$translator('settings.batched')} description={$translator('settings.batchedHelp')} bind:checked={draft.batched} />
        <Switch label={$translator('settings.keepFailed')} description={$translator('settings.keepFailedHelp')} bind:checked={draft.keepFailed} />
      </div>
    </section>

    <section id="settings-watchlists" data-i18n-ignore class="panel settings-section">
      <div><span class="eyebrow">{$translator('settings.watchEyebrow')}</span><h2>{$translator('settings.watchHeading')}</h2></div>
      <div class="watch-settings">
        <div class="watch-toggle"><Switch label={$translator('settings.watchAutomatic')}
          description={$translator('settings.watchAutomaticHelp')} bind:checked={draft.autoWatch} /></div>
        <Select label={$translator('settings.watchControl')} value={draft.watchInterval} disabled={!draft.autoWatch}
          options={[
            { value: '30 min', label: $translator('settings.thirtyMinutes') },
            { value: '60 min', label: $translator('settings.sixtyMinutes') },
            { value: '2 hours', label: $translator('settings.twoHours') },
            { value: '6 hours', label: $translator('settings.sixHours') }
          ]} onChange={(next) => { draft.watchInterval = next as typeof draft.watchInterval; }} />
      </div>
    </section>

    <McpSetupWizard {api} />

    <section id="settings-migration" data-i18n-ignore class="panel settings-section migration-section">
      <div><span class="eyebrow">{$translator('settings.nav.migration')}</span><h2>{$translator('settings.migrationHeading')}</h2><p>{$translator('settings.migrationHelp')}</p></div>
      <div class="migration-status" data-testid="migration-status" data-state={migrationStatus.state} aria-live="polite">
        <div class="migration-heading"><span class={`migration-pill migration-${migrationStatus.state}`}>{migrationLabel[migrationStatus.state]}</span></div>
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

  </div>
</div>

{#if status}<p class="status-copy settings-status" aria-live="polite" data-i18n-ignore>{status === 'saved' ? $translator('settings.saved') : status === 'outputSelected' ? $translator('settings.outputSelected') : $translator('settings.outputReset')}</p>{/if}
{#if saveError}<p class="status-copy settings-status" role="alert" data-i18n-ignore>{saveError}</p>{/if}

<style>
  .settings-head { align-items: center; gap: 10px; }
  .save-cluster { display: flex; align-items: center; gap: 10px; flex-wrap: wrap; }
  .dirty-indicator { display: inline-flex; align-items: center; gap: 6px; color: var(--warn); font-size: 12px; white-space: nowrap; }
  .dirty-indicator > span { width: 6px; height: 6px; border-radius: 50%; background: var(--warn); }
  .settings-layout { display: grid; grid-template-columns: 174px minmax(0, 1fr); gap: 14px; align-items: start; }
  .settings-nav { position: sticky; top: 82px; display: grid; gap: 2px; padding: 8px; }
  .settings-nav-title { padding: 8px 9px 5px; color: var(--faint); font-size: 11px; font-weight: 700; letter-spacing: .07em; text-transform: uppercase; }
  .settings-nav-title.secondary-group { margin-top: 7px; padding-top: 11px; border-top: 1px solid var(--border); }
  .settings-nav a { min-height: 36px; display: flex; align-items: center; padding: 0 9px; border-radius: 7px; color: var(--muted); text-decoration: none; font-size: 12px; }
  .settings-nav a:hover, .settings-nav a:focus-visible { color: var(--text); background: var(--surface-2); }
  .settings-stack { display: grid; gap: 12px; min-width: 0; }
  .settings-section { scroll-margin-top: 82px; grid-template-columns: minmax(174px, 210px) minmax(0, 1fr); }
  .settings-section .eyebrow { display: none; }
  .settings-section h2 { margin: 0; }
  .settings-section > div:first-child { max-width: 310px; }
  .output-picker { display: grid; gap: 9px; min-width: 0; }
  .field-label { color: var(--muted); font-size: 12px; }
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
  .settings-status { margin-inline-start: 188px; }

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
