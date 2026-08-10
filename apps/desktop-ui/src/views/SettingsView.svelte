<script lang="ts">
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

  const migrationLabel: Record<MigrationStatus['state'], string> = {
    completed: 'Completed',
    no_legacy_db: 'No legacy database found',
    ready: 'Ready to import',
    in_progress: 'Importing',
    requires_choice: 'Choice required',
    invalid_db: 'Invalid legacy database',
    failed: 'Migration failed'
  };

  const progressStatus = (): MigrationStatus => ({
    state: 'in_progress',
    message: 'Scriptotar is importing the prepared legacy snapshot. The source database remains untouched.',
    candidates: []
  });

  const failedStatus = (): MigrationStatus => ({
    state: 'failed',
    message: 'The migration request could not be completed. The legacy source database was not modified; retry when the local error is resolved.',
    candidates: []
  });

  async function save() {
    if (busy) return;
    busy = true;
    status = '';
    saveError = '';
    try {
      await api.saveSettings(draft);
      persistAppearance(draft.appearance);
      await onSaved(structuredClone(draft));
      status = 'Settings saved. New transcription jobs and watchlist refreshes will use these preferences.';
    } catch (cause) {
      saveError = cause instanceof Error ? cause.message : 'Could not save settings.';
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
        status = 'Output folder selected. Save changes to make it the default.';
      }
    } catch (cause) {
      saveError = cause instanceof Error ? cause.message : 'Could not choose an output folder.';
    } finally {
      busy = false;
    }
  }

  function restoreDefaultOutput() {
    draft.outputDirectory = null;
    status = 'Default output location selected. Save changes to apply it.';
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

<section class="view-head"><div><span class="eyebrow">Local preferences</span><h1>Settings</h1><p>Transcription, downloads, storage, privacy, and appearance stay explicit. Settings are validated below the UI before they are persisted.</p></div><button class="button primary" disabled={busy} on:click={save}>Save changes</button></section>
<div class="settings-stack" aria-busy={busy || migrationBusy}>
  <section class="panel settings-section"><div><span class="eyebrow">Transcription</span><h2>Speech engine</h2><p>Choose quality and compute defaults. Model installation remains a host responsibility.</p></div><div class="settings-grid"><label><span>Whisper model</span><select bind:value={draft.whisperModel}><option>small</option><option>medium</option><option>turbo</option><option>large-v3</option></select></label><label><span>Device</span><select bind:value={draft.device}><option>auto</option><option>cpu</option><option>cuda</option></select></label><label><span>Language</span><select bind:value={draft.language}><option>auto</option><option>Arabic</option><option>English</option></select></label></div></section>
  <section class="panel settings-section"><div><span class="eyebrow">Downloads + cookies</span><h2>Media acquisition</h2><p>Browser cookies are selected by browser name only. Cookie secrets never belong in the frontend.</p></div><div class="settings-grid"><label><span>Quality</span><select bind:value={draft.quality}><option>720p</option><option>1080p</option><option>Best</option><option>Audio only</option></select></label><label><span>Browser cookies</span><select bind:value={draft.cookies}><option>none</option><option>firefox</option><option>chrome</option><option>chromium</option><option>brave</option></select></label><label><span>Duration safety limit</span><select bind:value={draft.maxDuration}><option>30 min</option><option>60 min</option><option>2 hours</option><option>Unlimited</option></select></label></div></section>
  <section class="panel settings-section"><div><span class="eyebrow">Storage</span><h2>Transcript output</h2><p>Choose where new transcription result folders are created. Rust verifies that a selected directory exists and is writable before saving it.</p></div><div class="output-picker"><span class="field-label">Current output directory</span><code>{draft.outputDirectory || 'Application default'}</code><div class="output-actions"><button class="button secondary" disabled={busy} on:click={chooseOutputDirectory}>Choose output folder</button><button class="button secondary" disabled={busy || draft.outputDirectory === null} on:click={restoreDefaultOutput}>Restore default</button></div></div></section>
  <section class="panel settings-section"><div><span class="eyebrow">Privacy + processing</span><h2>Local behavior</h2><p>These values are persisted by the Rust settings layer and applied to future jobs.</p></div><div class="toggle-grid"><label><input type="checkbox" bind:checked={draft.copyLocalSource}/><span><strong>Copy local source media</strong><small>Keep a copy beside generated transcript artifacts.</small></span></label><label><input type="checkbox" bind:checked={draft.translate}/><span><strong>Translate speech to English</strong><small>Ask the transcription worker to translate.</small></span></label><label><input type="checkbox" bind:checked={draft.batched}/><span><strong>Batched inference</strong><small>Faster on suitable hardware, with higher memory use.</small></span></label><label><input type="checkbox" bind:checked={draft.keepFailed}/><span><strong>Keep failed partial artifacts</strong><small>Useful for debugging interrupted media stages.</small></span></label></div></section>

  <section class="panel settings-section">
    <div><span class="eyebrow">Creator watchlists</span><h2>Background refresh</h2><p>Automatic refresh uses the configured local research provider. Failures, retry timing, and recovery are visible in Research.</p></div>
    <div class="watch-settings">
      <label class="watch-toggle"><input type="checkbox" bind:checked={draft.autoWatch}/><span><strong>Refresh saved watchlists automatically</strong><small>Scriptotar scans due watchlists while the app is running and records failures instead of hiding them.</small></span></label>
      <label><span>Refresh interval</span><select aria-label="Creator watch refresh" bind:value={draft.watchInterval} disabled={!draft.autoWatch}><option>30 min</option><option>60 min</option><option>2 hours</option><option>6 hours</option></select></label>
    </div>
  </section>

  <section class="panel settings-section migration-section">
    <div><span class="eyebrow">Legacy migration</span><h2>Import Scriptotar Classic data</h2><p>Discovery uses a read-only, WAL-aware SQLite snapshot. Source databases are never selected by raw frontend paths and are not overwritten.</p></div>
    <div class="migration-status" data-testid="migration-status" data-state={migrationStatus.state} aria-live="polite">
      <div class="migration-heading"><strong>{migrationLabel[migrationStatus.state]}</strong><span class={`migration-pill migration-${migrationStatus.state}`}>{migrationStatus.state.replaceAll('_', ' ')}</span></div>
      <p>{migrationStatus.message}</p>
      {#if migrationStatus.state === 'completed' && migrationStatus.report}
        <p class="migration-counts">Imported {migrationStatus.report.projects} projects, {migrationStatus.report.jobs} jobs, {migrationStatus.report.transcripts} transcripts, {migrationStatus.report.research_items} research items, {migrationStatus.report.watchlists} watchlists, and {migrationStatus.report.ai_runs} AI runs.</p>
      {/if}
      {#if migrationStatus.state === 'requires_choice'}
        <div class="migration-candidates" aria-label="Legacy database choices">
          {#each migrationStatus.candidates as candidate (candidate.id)}
            <button class="button secondary" disabled={migrationBusy} on:click={() => chooseMigrationCandidate(candidate.id)}>{candidate.label}</button>
          {/each}
        </div>
      {:else if migrationStatus.state !== 'completed' && migrationStatus.state !== 'in_progress'}
        <button class="button secondary" disabled={migrationBusy} on:click={retryMigration}>{migrationBusy ? 'Checking…' : migrationStatus.state === 'ready' ? 'Import prepared snapshot' : 'Retry migration discovery'}</button>
      {/if}
    </div>
  </section>

  <section class="panel settings-section"><div><span class="eyebrow">Appearance</span><h2>Interface</h2><p>Dark stays fixed; System follows the operating-system light/dark preference. Appearance is a local UI preference and contains no sensitive data.</p></div><div class="settings-grid"><label><span>Theme</span><select aria-label="Theme" bind:value={draft.appearance}><option value="dark">Dark</option><option value="system">System</option></select></label></div></section>
</div>
{#if status}<p class="status-copy" aria-live="polite">{status}</p>{/if}
{#if saveError}<p class="status-copy" role="alert">{saveError}</p>{/if}

<style>
  .output-picker { display: grid; gap: 9px; min-width: 0; }
  .field-label { color: var(--muted); font-size: 10px; }
  code { display: block; max-width: 560px; overflow-wrap: anywhere; padding: 10px 12px; border: 1px solid var(--border); border-radius: 9px; background: #0b1218; color: var(--text); }
  .output-actions { display: flex; flex-wrap: wrap; gap: 8px; }
  .watch-settings { display: grid; gap: 14px; min-width: min(560px, 100%); }
  .watch-toggle { display: flex; gap: 10px; align-items: start; }
  .watch-toggle span { display: grid; gap: 3px; }
  .watch-toggle small { color: var(--muted); line-height: 1.4; }
  .migration-status { display: grid; gap: 10px; min-width: min(600px, 100%); padding: 14px; border: 1px solid var(--border); border-radius: 10px; }
  .migration-heading { display: flex; align-items: center; justify-content: space-between; gap: 12px; }
  .migration-pill { padding: 4px 7px; border: 1px solid var(--border); border-radius: 999px; color: var(--muted); font-size: 10px; text-transform: uppercase; letter-spacing: .04em; }
  .migration-failed, .migration-invalid_db { color: var(--danger, #ff8c8c); }
  .migration-requires_choice, .migration-ready, .migration-in_progress { color: var(--accent); }
  .migration-status p { margin: 0; color: var(--muted); line-height: 1.5; }
  .migration-counts { color: var(--text) !important; }
  .migration-candidates { display: flex; flex-wrap: wrap; gap: 8px; }
</style>