<script lang="ts">
  import type { AppSettings } from '../types';
  import type { ScriptotarApi } from '../api/client';
  import { persistAppearance } from '../appearance';

  export let settings: AppSettings;
  export let api: ScriptotarApi;

  let draft = structuredClone(settings);
  let status = '';
  let saveError = '';
  let migrationStatus = '';
  let migrationError = '';
  let busy = false;

  async function save() {
    if (busy) return;
    busy = true;
    status = '';
    saveError = '';
    try {
      await api.saveSettings(draft);
      persistAppearance(draft.appearance);
      status = 'Settings saved. New transcription jobs will use these preferences.';
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

  async function importLegacy() {
    if (busy) return;
    busy = true;
    migrationError = '';
    migrationStatus = '';
    try {
      const report = await api.importLegacyData();
      const counts = `${report.projects} projects, ${report.jobs} jobs, ${report.transcripts} transcripts, ${report.research_items} research items, ${report.watchlists} watchlists, ${report.ai_runs} AI runs`;
      const backup = report.backup_path ? ` Backup: ${report.backup_path}` : '';
      migrationStatus = report.skipped
        ? `Legacy data was already imported; no duplicate rows were created.${backup}`
        : `Legacy import completed: ${counts}.${backup}`;
    } catch (cause) {
      migrationError = cause instanceof Error ? cause.message : 'Legacy import failed.';
    } finally {
      busy = false;
    }
  }
</script>

<section class="view-head"><div><span class="eyebrow">Local preferences</span><h1>Settings</h1><p>Transcription, downloads, storage, privacy, and appearance stay explicit. Settings are validated below the UI before they are persisted.</p></div><button class="button primary" disabled={busy} on:click={save}>Save changes</button></section>
<div class="settings-stack" aria-busy={busy}>
  <section class="panel settings-section"><div><span class="eyebrow">Transcription</span><h2>Speech engine</h2><p>Choose quality and compute defaults. Model installation remains a host responsibility.</p></div><div class="settings-grid"><label><span>Whisper model</span><select bind:value={draft.whisperModel}><option>small</option><option>medium</option><option>turbo</option><option>large-v3</option></select></label><label><span>Device</span><select bind:value={draft.device}><option>auto</option><option>cpu</option><option>cuda</option></select></label><label><span>Language</span><select bind:value={draft.language}><option>auto</option><option>Arabic</option><option>English</option></select></label></div></section>
  <section class="panel settings-section"><div><span class="eyebrow">Downloads + cookies</span><h2>Media acquisition</h2><p>Browser cookies are selected by browser name only. Cookie secrets never belong in the frontend.</p></div><div class="settings-grid"><label><span>Quality</span><select bind:value={draft.quality}><option>720p</option><option>1080p</option><option>Best</option><option>Audio only</option></select></label><label><span>Browser cookies</span><select bind:value={draft.cookies}><option>none</option><option>firefox</option><option>chrome</option><option>chromium</option><option>brave</option></select></label><label><span>Duration safety limit</span><select bind:value={draft.maxDuration}><option>30 min</option><option>60 min</option><option>2 hours</option><option>Unlimited</option></select></label></div></section>
  <section class="panel settings-section"><div><span class="eyebrow">Storage</span><h2>Transcript output</h2><p>Choose where new transcription result folders are created. Rust verifies that a selected directory exists and is writable before saving it.</p></div><div class="output-picker"><span class="field-label">Current output directory</span><code>{draft.outputDirectory || 'Application default'}</code><div class="output-actions"><button class="button secondary" disabled={busy} on:click={chooseOutputDirectory}>Choose output folder</button><button class="button secondary" disabled={busy || draft.outputDirectory === null} on:click={restoreDefaultOutput}>Restore default</button></div></div></section>
  <section class="panel settings-section"><div><span class="eyebrow">Privacy + processing</span><h2>Local behavior</h2><p>These values are persisted by the Rust settings layer and applied to future jobs.</p></div><div class="toggle-grid"><label><input type="checkbox" bind:checked={draft.copyLocalSource}/><span><strong>Copy local source media</strong><small>Keep a copy beside generated transcript artifacts.</small></span></label><label><input type="checkbox" bind:checked={draft.translate}/><span><strong>Translate speech to English</strong><small>Ask the transcription worker to translate.</small></span></label><label><input type="checkbox" bind:checked={draft.batched}/><span><strong>Batched inference</strong><small>Faster on suitable hardware, with higher memory use.</small></span></label><label><input type="checkbox" bind:checked={draft.keepFailed}/><span><strong>Keep failed partial artifacts</strong><small>Useful for debugging interrupted media stages.</small></span></label></div></section>
  <section class="panel settings-section"><div><span class="eyebrow">Legacy migration</span><h2>Import Tkinter data</h2><p>The Rust importer creates a backup first, leaves the legacy database untouched, and skips an already imported fingerprint.</p></div><div><button class="button" disabled={busy} on:click={importLegacy}>Import legacy data</button>{#if migrationStatus}<p class="status-copy" aria-live="polite">{migrationStatus}</p>{/if}{#if migrationError}<p class="status-copy" role="alert">{migrationError}</p>{/if}</div></section>
  <section class="panel settings-section"><div><span class="eyebrow">Appearance</span><h2>Interface</h2><p>Dark stays fixed; System follows the operating-system light/dark preference. Appearance is a local UI preference and contains no sensitive data.</p></div><div class="settings-grid"><label><span>Theme</span><select aria-label="Theme" bind:value={draft.appearance}><option value="dark">Dark</option><option value="system">System</option></select></label><label><span>Creator watch refresh</span><select aria-label="Creator watch refresh" bind:value={draft.watchInterval} disabled title="Automatic watchlist refresh depends on the research provider integration."><option>30 min</option><option>60 min</option><option>2 hours</option><option>6 hours</option></select><small class="unavailable-note">Saved for compatibility, but automatic refresh is unavailable until the live research provider is integrated.</small></label></div></section>
</div>
{#if status}<p class="status-copy" aria-live="polite">{status}</p>{/if}
{#if saveError}<p class="status-copy" role="alert">{saveError}</p>{/if}

<style>
  .output-picker { display: grid; gap: 9px; min-width: 0; }
  .field-label { color: var(--muted); font-size: 10px; }
  code { display: block; max-width: 560px; overflow-wrap: anywhere; padding: 10px 12px; border: 1px solid var(--border); border-radius: 9px; background: #0b1218; color: var(--text); }
  .output-actions { display: flex; flex-wrap: wrap; gap: 8px; }
  .unavailable-note { display: block; max-width: 360px; color: var(--faint); line-height: 1.4; }
</style>
