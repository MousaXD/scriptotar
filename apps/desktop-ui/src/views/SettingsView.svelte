<script lang="ts">
  import type { AppSettings } from '../types';
  import type { ScriptotarApi } from '../api/client';

  export let settings: AppSettings;
  export let api: ScriptotarApi;

  let draft = structuredClone(settings);
  let status = '';
  let migrationStatus = '';
  let migrationError = '';

  async function save() {
    await api.saveSettings(draft);
    status = 'Settings saved through the API contract.';
  }

  async function importLegacy() {
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
    }
  }
</script>
<section class="view-head"><div><span class="eyebrow">Local preferences</span><h1>Settings</h1><p>Transcription, downloads, cookies, AI policy, storage, privacy, and appearance stay explicit.</p></div><button class="button primary" on:click={save}>Save changes</button></section>
<div class="settings-stack">
  <section class="panel settings-section"><div><span class="eyebrow">Transcription</span><h2>Speech engine</h2><p>Choose quality and compute defaults. Model installation remains a host responsibility.</p></div><div class="settings-grid"><label><span>Whisper model</span><select bind:value={draft.whisperModel}><option>small</option><option>medium</option><option>turbo</option><option>large-v3</option></select></label><label><span>Device</span><select bind:value={draft.device}><option>auto</option><option>cpu</option><option>cuda</option></select></label><label><span>Language</span><select bind:value={draft.language}><option>auto</option><option>Arabic</option><option>English</option></select></label></div></section>
  <section class="panel settings-section"><div><span class="eyebrow">Downloads + cookies</span><h2>Media acquisition</h2><p>Browser cookies are selected by browser name only. Cookie secrets never belong in the frontend.</p></div><div class="settings-grid"><label><span>Quality</span><select bind:value={draft.quality}><option>720p</option><option>1080p</option><option>Best</option><option>Audio only</option></select></label><label><span>Browser cookies</span><select bind:value={draft.cookies}><option>none</option><option>firefox</option><option>chrome</option><option>chromium</option><option>brave</option></select></label><label><span>Duration safety limit</span><select bind:value={draft.maxDuration}><option>30 min</option><option>60 min</option><option>2 hours</option><option>Unlimited</option></select></label></div></section>
  <section class="panel settings-section"><div><span class="eyebrow">Privacy + storage</span><h2>Local behavior</h2><p>Frontend preferences only. Application data and secret permissions are owned by Rust.</p></div><div class="toggle-grid"><label><input type="checkbox" bind:checked={draft.copyLocalSource}/><span><strong>Copy local source media</strong><small>Keep a copy beside generated transcript artifacts.</small></span></label><label><input type="checkbox" bind:checked={draft.translate}/><span><strong>Translate speech to English</strong><small>Ask the transcription worker to translate.</small></span></label><label><input type="checkbox" bind:checked={draft.batched}/><span><strong>Batched inference</strong><small>Faster on suitable hardware, with higher memory use.</small></span></label><label><input type="checkbox" bind:checked={draft.keepFailed}/><span><strong>Keep failed partial artifacts</strong><small>Useful for debugging interrupted media stages.</small></span></label></div></section>
  <section class="panel settings-section"><div><span class="eyebrow">Legacy migration</span><h2>Import Tkinter data</h2><p>The Rust importer creates a backup first, leaves the legacy database untouched, and skips an already imported fingerprint.</p></div><div><button class="button" on:click={importLegacy}>Import legacy data</button>{#if migrationStatus}<p class="status-copy" aria-live="polite">{migrationStatus}</p>{/if}{#if migrationError}<p class="status-copy" role="alert">{migrationError}</p>{/if}</div></section>
  <section class="panel settings-section"><div><span class="eyebrow">Appearance</span><h2>Interface</h2><p>Dark is first-class; system mode is available for host integration.</p></div><div class="settings-grid"><label><span>Theme</span><select bind:value={draft.appearance}><option value="dark">Dark</option><option value="system">System</option></select></label><label><span>Creator watch refresh</span><select bind:value={draft.watchInterval}><option>30 min</option><option>60 min</option><option>2 hours</option><option>6 hours</option></select></label></div></section>
</div>
<p class="status-copy" aria-live="polite">{status}</p>
