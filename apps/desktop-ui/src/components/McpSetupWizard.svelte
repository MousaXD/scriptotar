<script lang="ts">
  import { onMount } from 'svelte';
  import { translator } from '../i18n/translate';
  import type { ScriptotarApi } from '../api/client';
  import type { McpConnectionStatus, McpPermissions } from '../types';

  export let api: ScriptotarApi;

  type Method = 'cloudflare' | 'openai' | 'manual';
  let step: 1 | 2 | 3 = 1;
  let method: Method = 'cloudflare';
  let permissions: McpPermissions | null = null;
  let savedPermissions: McpPermissions | null = null;
  let connection: McpConnectionStatus | null = null;
  let busy = false;
  let error = '';
  let notice: '' | 'saved' | 'connected' | 'disconnected' = '';
  let acknowledge = false;
  let approvedPaths = '';
  let approvedPrograms = '';
  let tunnelId = '';
  let apiKey = '';
  let rememberKey = false;
  let customHostname = '';
  let advanced = false;
  let copyStatus: '' | 'copied' | 'failed' = '';
  let pollError = false;
  let loading = true;
  let timer: ReturnType<typeof setInterval> | undefined;


  const powerful = (p: McpPermissions) => p.writeFiles || p.overwriteFiles || p.executeCommands;
  const readLines = (text: string) => text.split('\n').map((x) => x.trim()).filter(Boolean);
  $: permissionDraft = permissions && {
    ...permissions, allowedPaths: readLines(approvedPaths), allowedPrograms: readLines(approvedPrograms)
  };
  $: permissionDirty = Boolean(savedPermissions && permissionDraft &&
    JSON.stringify(savedPermissions) !== JSON.stringify(permissionDraft));

  function discardPermissions() {
    if (!savedPermissions || busy) return;
    permissions = structuredClone(savedPermissions);
    approvedPaths = savedPermissions.allowedPaths.join('\n');
    approvedPrograms = savedPermissions.allowedPrograms.join('\n');
    acknowledge = false;
    error = '';
    notice = '';
  }

  async function load() {
    loading = true;
    error = '';
    try {
      const [policy, status] = await Promise.all([api.getMcpPermissions(), api.mcpConnectionStatus()]);
      savedPermissions = structuredClone(policy);
      permissions = structuredClone(policy);
      advanced = powerful(policy) || policy.readFiles || policy.allowedPaths.length > 0 || policy.allowedPrograms.length > 0;
      approvedPaths = policy.allowedPaths.join('\n');
      approvedPrograms = policy.allowedPrograms.join('\n');
      connection = status;
      if (status.state === 'ready' || status.state === 'connecting') {
        step = 3;
        if (status.mode !== 'none') method = status.mode;
      }
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
    } finally {
      loading = false;
    }
  }

  async function refresh() {
    try {
      connection = await api.mcpConnectionStatus();
      pollError = false;
    } catch {
      pollError = true;
    }
  }

  onMount(() => {
    void load();
    timer = setInterval(() => { void refresh(); }, 3000);
    return () => { if (timer) clearInterval(timer); };
  });

  async function savePermissions() {
    if (!permissionDraft || busy) return;
    busy = true;
    error = '';
    notice = '';
    try {
      const next: McpPermissions = structuredClone(permissionDraft);
      const persisted = await api.saveMcpPermissions(next, next.enabled && powerful(next) && acknowledge);
      // A user can edit the draft while an IPC save is pending. Never replace those
      // newer changes with the older request when the backend responds.
      const currentDraft = permissions && {
        ...permissions, allowedPaths: readLines(approvedPaths), allowedPrograms: readLines(approvedPrograms)
      };
      const changedDuringSave = JSON.stringify(currentDraft) !== JSON.stringify(next);
      savedPermissions = structuredClone(persisted);
      if (!changedDuringSave) {
        permissions = structuredClone(persisted);
        approvedPaths = persisted.allowedPaths.join('\n');
        approvedPrograms = persisted.allowedPrograms.join('\n');
        step = 2;
      } else {
        step = 1; // Leave the new, unsaved scope visible and block connection.
      }
      acknowledge = false;
      await refresh();
      notice = 'saved';
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
    } finally {
      busy = false;
    }
  }

  async function connect() {
    if (!permissions?.enabled || permissionDirty || busy) return;
    busy = true;
    error = '';
    notice = '';
    try {
      if (method === 'cloudflare') connection = await api.mcpConnectCloudflare();
      else if (method === 'openai') {
        const suppliedKey = apiKey.trim();
        apiKey = ''; // Clear the visible password field before network/IPC awaits.
        connection = await api.mcpConnectOpenai(tunnelId.trim(), suppliedKey, rememberKey);
      }
      else connection = await api.mcpConnectManual(customHostname.trim());
      apiKey = ''; // Never keep a key in the DOM after IPC completes.
      step = 3;
      notice = 'connected';
    } catch (err) {
      apiKey = '';
      error = err instanceof Error ? err.message : String(err);
    } finally {
      busy = false;
    }
  }

  async function disconnect() {
    if (busy) return;
    busy = true;
    error = '';
    try {
      connection = await api.mcpDisconnect();
      step = 2;
      notice = 'disconnected';
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
    } finally {
      busy = false;
    }
  }

  async function copyUrl(value: string) {
    try {
      await navigator.clipboard.writeText(value);
      copyStatus = 'copied';
    } catch {
      copyStatus = 'failed';
    }
  }
</script>

<section id="settings-mcp" class="panel wizard" data-testid="mcp-permissions" data-i18n-ignore>
  <header class="wizard-head">
    <div>
      <h2>{$translator('mcp.heading')}</h2>
    </div>
    {#if connection?.state === 'ready'}<span class="wizard-status connected">{$translator('mcp.ready')}</span>
    {:else if connection?.state === 'connecting'}<span class="wizard-status">{$translator('mcp.connecting')}</span>
    {:else}<span class="wizard-status">{$translator('mcp.notConnected')}</span>{/if}
  </header>

  <div class="steps" role="group" aria-label={$translator('mcp.steps')}>
    <button class:current={step === 1} aria-current={step === 1 ? 'step' : undefined} on:click={() => step = 1}><span>1</span> {$translator('mcp.permissions')}</button>
    <button class:current={step === 2} aria-current={step === 2 ? 'step' : undefined} disabled={!permissions || permissionDirty} on:click={() => step = 2}><span>2</span> {$translator('mcp.connection')}</button>
    <button class:current={step === 3} aria-current={step === 3 ? 'step' : undefined} disabled={!connection || connection.mode === 'none' || permissionDirty} on:click={() => step = 3}><span>3</span> {$translator('mcp.finish')}</button>
  </div>

  {#if !permissions || !connection}
    <p class="hint">{$translator('mcp.loading')}</p>
    {#if !loading && error}<button class="button secondary" on:click={load}>{$translator('mcp.retryLoad')}</button>{/if}
  {:else if step === 1}
    <div class="wizard-pane" data-testid="mcp-step-permissions">
      <h3>{$translator('mcp.permissionsHeading')}</h3>
      <p>{$translator('mcp.permissionsHelp')}</p>
      <label class="switch"><input type="checkbox" bind:checked={permissions.enabled} on:change={() => acknowledge = false}/><strong>{$translator('mcp.enable')}</strong></label>
      <div class="permission-grid">
        <label><input type="checkbox" bind:checked={permissions.readData} on:change={() => acknowledge = false}/> {$translator('mcp.readData')}</label>
        <label><input type="checkbox" bind:checked={permissions.searchCreators} on:change={() => acknowledge = false}/> {$translator('mcp.searchCreators')}</label>
        <label><input type="checkbox" bind:checked={permissions.downloadVideos} on:change={() => acknowledge = false}/> {$translator('mcp.downloadVideos')}</label>
        <label><input type="checkbox" bind:checked={permissions.writeData} on:change={() => acknowledge = false}/> {$translator('mcp.writeData')}</label>
      </div>
      <button class="button secondary" on:click={() => advanced = !advanced}>{advanced ? $translator('mcp.hide') : $translator('mcp.show')} {$translator('mcp.advanced')}</button>
      {#if advanced}
        <div class="advanced-permissions">
          <div class="permission-grid">
            <label><input type="checkbox" bind:checked={permissions.readFiles} on:change={() => acknowledge = false}/> {$translator('mcp.readFiles')}</label>
            <label><input type="checkbox" bind:checked={permissions.writeFiles} on:change={() => acknowledge = false}/> {$translator('mcp.writeFiles')}</label>
            <label><input type="checkbox" bind:checked={permissions.overwriteFiles} on:change={() => acknowledge = false}/> {$translator('mcp.overwriteFiles')}</label>
            <label><input type="checkbox" bind:checked={permissions.executeCommands} on:change={() => acknowledge = false}/> {$translator('mcp.executeCommands')}</label>
          </div>
          <label class="field"><span>{$translator('mcp.allowedPaths')}</span><textarea rows="2" dir="ltr" bind:value={approvedPaths} on:input={() => acknowledge = false} placeholder="/home/user/Documents/Scriptotar"></textarea></label>
          <label class="field"><span>{$translator('mcp.allowedPrograms')}</span><textarea rows="2" dir="ltr" bind:value={approvedPrograms} on:input={() => acknowledge = false} placeholder="/usr/bin/git"></textarea></label>
          <p class="warning"><strong>{$translator('mcp.powerfulTitle')}</strong> {$translator('mcp.powerfulWarning')}</p>
        </div>
      {/if}
      {#if permissions.enabled && powerful(permissions)}
        <label class="switch"><input type="checkbox" bind:checked={acknowledge}/> {$translator('mcp.approve')}</label>
      {/if}
      {#if permissionDirty}<p class="permission-unsaved" role="status">{$translator('mcp.unsaved')}</p>{/if}
      {#if permissionDirty}<button class="button secondary" disabled={busy} on:click={discardPermissions}>{$translator('mcp.discard')}</button>{/if}
      <button class="button primary" disabled={busy || !permissionDirty || (permissions.enabled && powerful(permissions) && !acknowledge)} on:click={savePermissions}>{busy ? $translator('mcp.saving') : $translator('mcp.savePermissions')}</button>
    </div>
  {:else if step === 2}
    <div class="wizard-pane" data-testid="mcp-step-connection">
      <h3>{$translator('mcp.connectionHeading')}</h3>
      <div class="method-grid">
        <label class:chosen={method === 'cloudflare'}><input type="radio" name="mcp-method" value="cloudflare" bind:group={method}/><strong>{$translator('mcp.cloudflare')}</strong><span>{$translator('mcp.cloudflareDesc')}</span></label>
        <label class:chosen={method === 'openai'}><input type="radio" name="mcp-method" value="openai" bind:group={method}/><strong>{$translator('mcp.openai')}</strong><span>{$translator('mcp.openaiDesc')}</span></label>
        <label class:chosen={method === 'manual'}><input type="radio" name="mcp-method" value="manual" bind:group={method}/><strong>{$translator('mcp.manual')}</strong><span>{$translator('mcp.manualDesc')}</span></label>
      </div>
      {#if method === 'cloudflare'}
        <p class="hint">{$translator('mcp.cloudflareHint')} <strong>{$translator('mcp.cloudflareWarning')}</strong></p>
        {#if !connection.cloudflareAvailable}<p class="warning">{$translator('mcp.cloudflareMissing')}</p>{/if}
      {:else if method === 'openai'}
        <p class="hint">{$translator('mcp.openaiPrefix')} <a href="https://platform.openai.com/settings/organization/tunnels" target="_blank" rel="noopener noreferrer">{$translator('mcp.openaiPlatform')}</a>{$translator('mcp.openaiSuffix')}</p>
        <label class="field"><span>{$translator('mcp.tunnelId')}</span><input dir="ltr" autocomplete="off" placeholder="tunnel_0123456789abcdef0123456789abcdef" bind:value={tunnelId}/></label>
        <label class="field"><span>{$translator('mcp.restrictedKey')}</span><input type="password" dir="ltr" autocomplete="off" spellcheck="false" placeholder="sk-..." bind:value={apiKey}/></label>
        <label class="switch"><input type="checkbox" bind:checked={rememberKey}/> {$translator('mcp.rememberKey')}</label>
        {#if !connection.openaiAvailable}<p class="warning">{$translator('mcp.openaiMissing')}</p>{/if}
      {:else}
        <label class="field"><span>{$translator('mcp.hostname')}</span><input dir="ltr" placeholder="https://mcp.example.com" spellcheck="false" bind:value={customHostname}/></label>
        <p class="hint">{$translator('mcp.manualPrefix')} <code>http://127.0.0.1:39761</code>{$translator('mcp.manualSuffix')}</p>
      {/if}
      {#if !permissions.enabled}<p class="warning">{$translator('mcp.disabledWarning')}</p>{/if}
      {#if permissionDirty}<p class="warning">{$translator('mcp.saveFirst')}</p>{/if}
      <button class="button primary" disabled={busy || permissionDirty || !permissions.enabled || (method === 'cloudflare' && !connection.cloudflareAvailable) || (method === 'openai' && !connection.openaiAvailable)} on:click={connect}>{busy ? $translator('mcp.connecting') : $translator('mcp.connect')}</button>
    </div>
  {:else}
    <div class="wizard-pane" data-testid="mcp-step-finish">
      <h3>{$translator('mcp.finishHeading')}</h3>
      <p class="connection-state" data-state={connection.state} dir="auto">{connection.state === 'ready' ? $translator('mcp.gatewayReady') : connection.state === 'connecting' ? $translator('mcp.establishing') : $translator('mcp.needsAttention')} · {connection.detail}</p>
      {#if connection.mode === 'openai'}
        <p>{$translator('mcp.finishPrefix')} <strong>{$translator('mcp.customServer')}</strong>, {$translator('mcp.choose')} <strong>{$translator('mcp.tunnel')}</strong> {$translator('mcp.tunnelSuffix')}</p>
        <div class="url-copy"><code dir="ltr">{connection.tunnelId}</code>{#if connection.tunnelId}<button class="button secondary" on:click={() => copyUrl(connection?.tunnelId || '')}>{$translator('mcp.copyId')}</button>{/if}</div>
      {:else if connection.publicUrl}
        <p>{$translator('mcp.finishPrefix')} <strong>{$translator('mcp.customServer')}</strong> {$translator('mcp.choose')} <strong>{$translator('mcp.serverUrl')}</strong>{$translator('mcp.urlSuffix')}</p>
        <div class="url-copy"><code dir="ltr" data-testid="mcp-public-url">{connection.publicUrl}</code><button class="button secondary" on:click={() => copyUrl(connection?.publicUrl || '')}>{$translator('mcp.copyUrl')}</button></div>
        <p class="warning">{$translator('mcp.secretWarning')}</p>
      {/if}
      {#if connection.mode === 'manual'}<p class="hint">{$translator('mcp.manualWarning')}</p>{/if}
      <p class="hint">{$translator('mcp.testPrefix')} <code>list_projects</code> {$translator('mcp.testSuffix')}</p>
      <a class="button secondary external" href="https://chatgpt.com/plugins" target="_blank" rel="noopener noreferrer">{$translator('mcp.openPlugins')}</a>
      {#if connection.state === 'error'}<button class="button secondary" on:click={() => step = 2}>{$translator('mcp.editConnection')}</button>{/if}
      <button class="button secondary danger" disabled={busy} on:click={disconnect}>{busy ? $translator('mcp.stopping') : $translator('mcp.disconnect')}</button>
      {#if copyStatus}<p role="status">{copyStatus === 'copied' ? $translator('mcp.copiedNotice') : $translator('mcp.copyFailed')}</p>{/if}
    </div>
  {/if}
  {#if notice}<p class="message" role="status">{notice === 'saved' ? $translator('mcp.savedNotice') : notice === 'connected' ? $translator('mcp.connectedNotice') : $translator('mcp.disconnectedNotice')}</p>{/if}
  {#if pollError}<p class="warning" role="status">{$translator('mcp.statusUnavailable')}</p>{/if}
  {#if error}<p class="warning" role="alert">{error}</p>{/if}
</section>

<style>
  .wizard { display: grid; gap: 16px; }
  .wizard-head { display: flex; justify-content: space-between; align-items: start; gap: 12px; }
  .wizard-head h2 { font-size: 20px; margin: 0; }
  .wizard-pane > p { font-size: 12px; line-height: 1.5; color: var(--muted); }
  .wizard-status { border: 1px solid var(--border); padding: 6px 9px; border-radius: 20px; white-space: nowrap; font-size: 12px; color: var(--muted); }
  .wizard-status.connected { border-color: var(--accent); color: var(--accent); }
  .steps { display: flex; gap: 6px; flex-wrap: wrap; padding-bottom: 12px; border-bottom: 1px solid var(--border); }
  .steps button { border: 0; border-radius: 7px; background: transparent; color: var(--muted); font-size: 12px; padding: 8px 10px; cursor: pointer; }
  .steps button.current { background: var(--surface-raised, var(--surface)); color: var(--text); }
  .steps button:disabled { cursor: not-allowed; opacity: .45; }
  .steps span { display: inline-flex; justify-content: center; align-items: center; border-radius: 50%; border: 1px solid var(--border); width: 19px; height: 19px; margin-right: 5px; }
  .wizard-pane { display: grid; align-content: start; gap: 12px; }
  .wizard-pane h3 { margin: 0; font-size: 14px; }
  .permission-unsaved { color: var(--warn); font-weight: 600; font-size: 12px; }
  .wizard-pane > button, .wizard-pane > a.button { justify-self: start; }
  .switch { display: flex; gap: 9px; align-items: center; font-size: 12px; }
  .permission-grid { display: grid; grid-template-columns: repeat(2, minmax(0,1fr)); gap: 12px; }
  .permission-grid label { display: flex; align-items: center; gap: 6px; font-size: 12px; }
  .advanced-permissions { display: grid; gap: 12px; border: 1px solid var(--border); border-radius: 8px; padding: 12px; }
  .method-grid { display: grid; grid-template-columns: repeat(3,minmax(0,1fr)); gap: 9px; }
  .method-grid label { display: grid; align-content: start; gap: 9px; padding: 13px; border: 1px solid var(--border); border-radius: 9px; cursor: pointer; min-width: 0; }
  .method-grid label.chosen { border-color: var(--accent); background: var(--surface-raised, var(--surface)); }
  .method-grid label strong { font-size: 12px; }
  .method-grid label span { font-size: 12px; line-height: 1.5; color: var(--muted); }
  .field { display: grid; gap: 6px; font-size: 12px; }
  .field input, .field textarea { min-width: 0; width: 100%; background: var(--surface); border: 1px solid var(--border); border-radius: 7px; color: var(--text); padding: 10px; font: inherit; }
  .field textarea { font-family: monospace; }
  .hint { line-height: 1.6; }
  .hint code, .url-copy code { overflow-wrap: anywhere; }
  .url-copy { display: flex; align-items: center; gap: 8px; padding: 10px; border: 1px solid var(--border); border-radius: 8px; min-width: 0; }
  .url-copy code { font-size: 11px; flex: 1; user-select: text; }
  .url-copy button { flex-shrink: 0; }
  .warning { color: var(--warn); line-height: 1.6; font-size: 12px; }
  .message { color: var(--accent); font-size: 11px; }
  .connection-state { border-inline-start: 3px solid var(--accent); padding: 8px 12px; }
  .danger { color: var(--warn); }
  .external { text-decoration: none; }
  @media(max-width: 1000px) { .method-grid { grid-template-columns: 1fr; } .permission-grid { grid-template-columns: 1fr; } }
  @media(max-width: 640px) { .wizard-head { flex-wrap: wrap; } .steps button { flex: 1 1 auto; } .url-copy { flex-wrap: wrap; } }
</style>
