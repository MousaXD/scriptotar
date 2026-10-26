<script lang="ts">
  import { onMount } from 'svelte';
  import type { ScriptotarApi } from '../api/client';
  import type { McpConnectionStatus, McpPermissions } from '../types';

  export let api: ScriptotarApi;

  type Method = 'cloudflare' | 'openai' | 'manual';
  let step: 1 | 2 | 3 = 1;
  let method: Method = 'cloudflare';
  let permissions: McpPermissions | null = null;
  let connection: McpConnectionStatus | null = null;
  let busy = false;
  let error = '';
  let notice = '';
  let acknowledge = false;
  let approvedPaths = '';
  let approvedPrograms = '';
  let tunnelId = '';
  let apiKey = '';
  let rememberKey = true;
  let customHostname = '';
  let advanced = false;
  let copyStatus = '';
  let timer: ReturnType<typeof setInterval> | undefined;

  const powerful = (p: McpPermissions) => p.writeFiles || p.overwriteFiles || p.executeCommands;

  async function refresh() {
    try {
      connection = await api.mcpConnectionStatus();
    } catch {
      // The UI keeps its last meaningful state when background status polling fails.
    }
  }

  onMount(() => {
    void Promise.all([api.getMcpPermissions(), api.mcpConnectionStatus()]).then(([policy, status]) => {
      permissions = policy;
      approvedPaths = policy.allowedPaths.join('\n');
      approvedPrograms = policy.allowedPrograms.join('\n');
      connection = status;
      if (status.state === 'ready' || status.state === 'connecting') {
        step = 3;
        if (status.mode !== 'none') method = status.mode;
      }
    }).catch((err) => { error = err instanceof Error ? err.message : String(err); });
    timer = setInterval(() => { void refresh(); }, 3000);
    return () => { if (timer) clearInterval(timer); };
  });

  async function savePermissions() {
    if (!permissions || busy) return;
    busy = true;
    error = '';
    notice = '';
    try {
      const next: McpPermissions = {
        ...permissions,
        allowedPaths: approvedPaths.split('\n').map((x) => x.trim()).filter(Boolean),
        allowedPrograms: approvedPrograms.split('\n').map((x) => x.trim()).filter(Boolean)
      };
      permissions = await api.saveMcpPermissions(next, next.enabled && powerful(next) && acknowledge);
      approvedPaths = permissions.allowedPaths.join('\n');
      approvedPrograms = permissions.allowedPrograms.join('\n');
      acknowledge = false;
      await refresh();
      notice = 'Saved on this computer. ChatGPT cannot change these permissions.';
      step = 2;
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
    } finally {
      busy = false;
    }
  }

  async function connect() {
    if (!permissions?.enabled || busy) return;
    busy = true;
    error = '';
    notice = '';
    try {
      if (method === 'cloudflare') connection = await api.mcpConnectCloudflare();
      else if (method === 'openai') connection = await api.mcpConnectOpenai(tunnelId.trim(), apiKey.trim(), rememberKey);
      else connection = await api.mcpConnectManual(customHostname.trim());
      apiKey = ''; // Never keep a key in the DOM after IPC completes.
      step = 3;
      notice = 'Connection started. Complete the connector setup in ChatGPT, then test a read-only tool call.';
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
      notice = 'Disconnected. Any old public URL secret was revoked.';
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
    } finally {
      busy = false;
    }
  }

  async function copyUrl(value: string) {
    try {
      await navigator.clipboard.writeText(value);
      copyStatus = 'URL copied. Keep this secret link private.';
    } catch {
      copyStatus = 'Select the URL and copy it manually.';
    }
  }
</script>

<section id="settings-mcp" class="panel wizard" data-testid="mcp-permissions">
  <header class="wizard-head">
    <div>
      <span class="eyebrow">Connect Scriptotar to ChatGPT</span>
      <h2>MCP setup wizard</h2>
      <p>Review permissions, choose a connection, and copy the ChatGPT setup details. No terminal commands required.</p>
    </div>
    {#if connection?.state === 'ready'}<span class="wizard-status connected">Local connection ready</span>
    {:else if connection?.state === 'connecting'}<span class="wizard-status">Connecting…</span>
    {:else}<span class="wizard-status">Not connected</span>{/if}
  </header>

  <div class="steps" aria-label="MCP setup steps">
    <button class:current={step === 1} on:click={() => step = 1}><span>1</span> Permissions</button>
    <button class:current={step === 2} disabled={!permissions} on:click={() => step = 2}><span>2</span> Connection</button>
    <button class:current={step === 3} disabled={!connection || connection.mode === 'none'} on:click={() => step = 3}><span>3</span> Connect in ChatGPT</button>
  </div>

  {#if !permissions || !connection}
    <p class="hint">Loading Scriptotar's MCP configuration…</p>
  {:else if step === 1}
    <div class="wizard-pane" data-testid="mcp-step-permissions">
      <h3>Step 1 · Set your local permissions</h3>
      <p>These permissions are enforced by Scriptotar for every MCP request, even if ChatGPT approves it.</p>
      <label class="switch"><input type="checkbox" bind:checked={permissions.enabled}/><strong>Enable Scriptotar MCP</strong></label>
      <div class="permission-grid">
        <label><input type="checkbox" bind:checked={permissions.readData}/> Read projects, jobs and transcripts</label>
        <label><input type="checkbox" bind:checked={permissions.searchCreators}/> Search public creators</label>
        <label><input type="checkbox" bind:checked={permissions.downloadVideos}/> Download videos and transcribe media</label>
        <label><input type="checkbox" bind:checked={permissions.writeData}/> Create projects</label>
      </div>
      <button class="button secondary" on:click={() => advanced = !advanced}>{advanced ? 'Hide' : 'Show'} advanced file and command permissions</button>
      {#if advanced}
        <div class="advanced-permissions">
          <div class="permission-grid">
            <label><input type="checkbox" bind:checked={permissions.readFiles}/> Read approved text files</label>
            <label><input type="checkbox" bind:checked={permissions.writeFiles}/> Create approved text files</label>
            <label><input type="checkbox" bind:checked={permissions.overwriteFiles}/> Overwrite existing files</label>
            <label><input type="checkbox" bind:checked={permissions.executeCommands}/> Execute approved programs</label>
          </div>
          <label class="field"><span>Approved directories (one absolute path per line)</span><textarea rows="2" bind:value={approvedPaths} placeholder="/home/user/Documents/Scriptotar"></textarea></label>
          <label class="field"><span>Approved executables (one absolute path per line)</span><textarea rows="2" bind:value={approvedPrograms} placeholder="/usr/bin/git"></textarea></label>
          <p class="warning"><strong>Powerful access:</strong> approved executables use your operating-system account's privileges, so permitting a shell or interpreter is not a security sandbox.</p>
        </div>
      {/if}
      {#if permissions.enabled && powerful(permissions)}
        <label class="switch"><input type="checkbox" bind:checked={acknowledge}/> I approve powerful file and command access on this computer</label>
      {/if}
      <button class="button primary" disabled={busy || (permissions.enabled && powerful(permissions) && !acknowledge)} on:click={savePermissions}>{busy ? 'Saving…' : 'Save permissions & continue'}</button>
    </div>
  {:else if step === 2}
    <div class="wizard-pane" data-testid="mcp-step-connection">
      <h3>Step 2 · Choose how to connect</h3>
      <div class="method-grid">
        <label class:chosen={method === 'cloudflare'}><input type="radio" name="mcp-method" value="cloudflare" bind:group={method}/><strong>Cloudflare Quick Tunnel</strong><span>One-click temporary HTTPS URL. No Cloudflare account or API key needed.</span></label>
        <label class:chosen={method === 'openai'}><input type="radio" name="mcp-method" value="openai" bind:group={method}/><strong>OpenAI Secure Tunnel</strong><span>Private connection through OpenAI, with no public-facing endpoint.</span></label>
        <label class:chosen={method === 'manual'}><input type="radio" name="mcp-method" value="manual" bind:group={method}/><strong>My own HTTPS hostname</strong><span>Use a named Cloudflare tunnel or an existing HTTPS reverse proxy.</span></label>
      </div>
      {#if method === 'cloudflare'}
        <p class="hint">Scriptotar starts the bundled cloudflared tool and gives you a secret URL to paste into ChatGPT. The hostname changes every time you reconnect. <strong>Anyone with the entire URL can request permitted tools.</strong></p>
        {#if !connection.cloudflareAvailable}<p class="warning">cloudflared is missing from this installation. Install the MCP-enabled release with bundled tunnel tools.</p>{/if}
      {:else if method === 'openai'}
        <p class="hint">Create a tunnel and restricted key in <a href="https://platform.openai.com/settings/organization/tunnels" target="_blank" rel="noopener noreferrer">OpenAI Platform</a>. Choose only Tunnels Read + Use for the runtime key.</p>
        <label class="field"><span>OpenAI tunnel ID</span><input autocomplete="off" placeholder="tunnel_0123456789abcdef0123456789abcdef" bind:value={tunnelId}/></label>
        <label class="field"><span>Restricted API key (leave blank to reuse saved key)</span><input type="password" autocomplete="off" spellcheck="false" placeholder="sk-..." bind:value={apiKey}/></label>
        <label class="switch"><input type="checkbox" bind:checked={rememberKey}/> Remember key in the OS keyring (never plain-text settings)</label>
        {#if !connection.openaiAvailable}<p class="warning">OpenAI tunnel-client is missing from this installation.</p>{/if}
      {:else}
        <label class="field"><span>Your public HTTPS hostname</span><input placeholder="https://mcp.example.com" spellcheck="false" bind:value={customHostname}/></label>
        <p class="hint">Configure your Cloudflare named tunnel's origin to <code>http://127.0.0.1:39761</code>. Scriptotar adds a new 64-character secret path to your hostname automatically.</p>
      {/if}
      {#if !permissions.enabled}<p class="warning">Enable MCP in step 1 before connecting.</p>{/if}
      <button class="button primary" disabled={busy || !permissions.enabled || (method === 'cloudflare' && !connection.cloudflareAvailable) || (method === 'openai' && !connection.openaiAvailable)} on:click={connect}>{busy ? 'Connecting…' : 'Connect'}</button>
    </div>
  {:else}
    <div class="wizard-pane" data-testid="mcp-step-finish">
      <h3>Step 3 · Add Scriptotar to ChatGPT</h3>
      <p class="connection-state" data-state={connection.state}>{connection.state === 'ready' ? 'Gateway ready' : connection.state === 'connecting' ? 'Establishing connection…' : 'Connection needs attention'} · {connection.detail}</p>
      {#if connection.mode === 'openai'}
        <p>In ChatGPT, add a <strong>custom MCP server</strong>, choose <strong>Tunnel</strong>, and select this tunnel ID:</p>
        <div class="url-copy"><code>{connection.tunnelId}</code>{#if connection.tunnelId}<button class="button secondary" on:click={() => copyUrl(connection?.tunnelId || '')}>Copy ID</button>{/if}</div>
      {:else if connection.publicUrl}
        <p>In ChatGPT, add a <strong>custom MCP server</strong> and choose <strong>Server URL</strong>. Paste this exact URL:</p>
        <div class="url-copy"><code data-testid="mcp-public-url">{connection.publicUrl}</code><button class="button secondary" on:click={() => copyUrl(connection?.publicUrl || '')}>Copy URL</button></div>
        <p class="warning">Treat this URL like a password. Do not share it, publish it, or paste it into support chats. Stop disconnects the tunnel and revokes the secret.</p>
      {/if}
      {#if connection.mode === 'manual'}<p class="hint">A local gateway is running, but Scriptotar cannot verify your separately managed HTTPS tunnel. Check it before sharing the URL.</p>{/if}
      <p class="hint">After adding Scriptotar, test <code>list_projects</code> first. Keep Scriptotar running for connections and tool calls.</p>
      <a class="button secondary external" href="https://chatgpt.com/plugins" target="_blank" rel="noopener noreferrer">Open ChatGPT Plugins</a>
      {#if connection.state === 'error'}<button class="button secondary" on:click={() => step = 2}>Edit connection</button>{/if}
      <button class="button secondary danger" disabled={busy} on:click={disconnect}>{busy ? 'Stopping…' : 'Disconnect & revoke URL'}</button>
      {#if copyStatus}<p role="status">{copyStatus}</p>{/if}
    </div>
  {/if}
  {#if notice}<p class="message" role="status">{notice}</p>{/if}
  {#if error}<p class="warning" role="alert">{error}</p>{/if}
</section>

<style>
  .wizard { display: grid; gap: 16px; }
  .wizard-head { display: flex; justify-content: space-between; align-items: start; gap: 12px; }
  .wizard-head h2 { font-size: 20px; margin: 4px 0 6px; }
  .wizard-head p, .wizard-pane > p { font-size: 12px; line-height: 1.6; color: var(--muted); }
  .wizard-status { border: 1px solid var(--border); padding: 6px 9px; border-radius: 20px; white-space: nowrap; font-size: 10px; color: var(--muted); }
  .wizard-status.connected { border-color: var(--accent); color: var(--accent); }
  .steps { display: flex; gap: 6px; flex-wrap: wrap; padding-bottom: 12px; border-bottom: 1px solid var(--border); }
  .steps button { border: 0; border-radius: 7px; background: transparent; color: var(--muted); font-size: 11px; padding: 8px 10px; cursor: pointer; }
  .steps button.current { background: var(--surface-raised, var(--surface)); color: var(--text); }
  .steps button:disabled { cursor: not-allowed; opacity: .45; }
  .steps span { display: inline-flex; justify-content: center; align-items: center; border-radius: 50%; border: 1px solid var(--border); width: 19px; height: 19px; margin-right: 5px; }
  .wizard-pane { display: grid; align-content: start; gap: 12px; }
  .wizard-pane h3 { margin: 0; font-size: 14px; }
  .wizard-pane > button, .wizard-pane > a.button { justify-self: start; }
  .switch { display: flex; gap: 9px; align-items: center; font-size: 12px; }
  .permission-grid { display: grid; grid-template-columns: repeat(2, minmax(0,1fr)); gap: 12px; }
  .permission-grid label { display: flex; align-items: center; gap: 6px; font-size: 11px; }
  .advanced-permissions { display: grid; gap: 12px; border: 1px solid var(--border); border-radius: 8px; padding: 12px; }
  .method-grid { display: grid; grid-template-columns: repeat(3,minmax(0,1fr)); gap: 9px; }
  .method-grid label { display: grid; align-content: start; gap: 9px; padding: 13px; border: 1px solid var(--border); border-radius: 9px; cursor: pointer; min-width: 0; }
  .method-grid label.chosen { border-color: var(--accent); background: var(--surface-raised, var(--surface)); }
  .method-grid label strong { font-size: 12px; }
  .method-grid label span { font-size: 11px; line-height: 1.5; color: var(--muted); }
  .field { display: grid; gap: 6px; font-size: 11px; }
  .field input, .field textarea { min-width: 0; width: 100%; background: var(--surface); border: 1px solid var(--border); border-radius: 7px; color: var(--text); padding: 10px; font: inherit; }
  .field textarea { font-family: monospace; }
  .hint { line-height: 1.6; }
  .hint code, .url-copy code { overflow-wrap: anywhere; }
  .url-copy { display: flex; align-items: center; gap: 8px; padding: 10px; border: 1px solid var(--border); border-radius: 8px; min-width: 0; }
  .url-copy code { font-size: 11px; flex: 1; user-select: text; }
  .url-copy button { flex-shrink: 0; }
  .warning { color: var(--warn); line-height: 1.6; font-size: 11px; }
  .message { color: var(--accent); font-size: 11px; }
  .connection-state { border-left: 3px solid var(--accent); padding: 8px 12px; }
  .danger { color: var(--warn); }
  .external { text-decoration: none; }
  @media(max-width: 1000px) { .method-grid { grid-template-columns: 1fr; } .permission-grid { grid-template-columns: 1fr; } }
</style>
