<script lang="ts">
  import type { AiMode, AiProvider } from '../types';
  import type { ScriptotarApi } from '../api/client';
  export let api: ScriptotarApi;
  export let initialSource = '';

  let mode: AiMode = 'copy';
  let provider: AiProvider = 'OpenAI';
  let model = 'gpt-5.2';
  let task = 'Viral breakdown';
  let sourceText = initialSource;
  let topic = '';
  let audience = '';
  let duration = '30–45 seconds';
  let cta = '';
  let voice = '';
  let baseUrl = '';
  let apiKey = '';
  let prompt = '';
  let result = '';
  let status = 'Prompt-only mode keeps the generated prompt local.';
  let busy = false;

  $: words = sourceText.trim() ? sourceText.trim().split(/\s+/).length : 0;
  $: speakingSeconds = Math.round(words / 2.5);
  $: briefFields = [topic, audience, cta, voice].filter((value) => value.trim()).length;

  function payload() {
    return { mode, provider, model, task, sourceText, topic, audience, duration, cta, voice, baseUrl, apiKey: mode === 'byok' ? apiKey : undefined };
  }
  async function buildPrompt() {
    prompt = await api.buildAiPrompt(payload());
    status = 'Prompt built locally. Nothing has been sent to an AI provider.';
  }
  async function copyText(value: string, label: string) {
    if (!value) return;
    if (!navigator.clipboard?.writeText) {
      status = 'Clipboard access is unavailable in this runtime. Select the text and copy it manually.';
      return;
    }
    try {
      await navigator.clipboard.writeText(value);
      status = `${label} copied to the clipboard.`;
    } catch (error) {
      status = `Copy failed: ${error instanceof Error ? error.message : 'clipboard access was denied'}`;
    }
  }
  async function runAi() {
    if (mode === 'copy') { await buildPrompt(); status = 'Copy Prompt mode is ready for manual use elsewhere.'; return; }
    if (!apiKey.trim()) { status = 'Enter an API key for this session, or switch to Copy Prompt.'; return; }
    busy = true;
    try { result = await api.runAi(payload()); status = `Finished with ${provider}.`; }
    catch (error) { status = `AI request failed: ${error instanceof Error ? error.message : 'Unknown error'}`; }
    finally { busy = false; }
  }
</script>

<section class="view-head ai-head">
  <div><span class="eyebrow">Optional AI layer</span><h1>AI Studio</h1><p>Build portable prompts with no key, or use your own provider credentials for a direct run.</p></div>
  <div class="mode-switch compact-mode" role="group" aria-label="AI mode">
    <button class:active={mode === 'copy'} on:click={() => mode = 'copy'}><strong>Copy Prompt</strong><span>No API key · nothing sent</span></button>
    <button class:active={mode === 'byok'} on:click={() => mode = 'byok'}><strong>BYOK</strong><span>Use your own provider key</span></button>
  </div>
</section>

<section class="panel ai-setup">
  <div class="setup-heading"><div><span class="eyebrow">1 · Configure</span><h2>Prompt setup</h2></div><p>{mode === 'copy' ? 'Everything stays local until you copy the prompt elsewhere.' : 'The API key exists for this session only.'}</p></div>
  <div class="ai-config-grid">
    <label><span>Task</span><select bind:value={task}><option>Viral breakdown</option><option>Hook ideas</option><option>New short-form script</option><option>Structure remix</option><option>Content ideas</option><option>Caption + CTA</option><option>Voice profile</option><option>B-roll shot list</option></select></label>
    <label class:disabled-field={mode === 'copy'}><span>Provider</span><select aria-label="AI provider" bind:value={provider} disabled={mode === 'copy'}><option>OpenAI</option><option>Anthropic</option><option>Gemini</option><option>OpenAI-compatible</option><option>Local (coming later)</option></select></label>
    <label class:disabled-field={mode === 'copy'}><span>Model</span><input bind:value={model} disabled={mode === 'copy'} /></label>
  </div>
  {#if mode === 'byok'}
    <div class="byok-strip" data-testid="byok-fields">
      <label><span>API key · session only</span><input aria-label="API key" type="password" bind:value={apiKey} autocomplete="off" placeholder="Paste key for this run" /></label>
      {#if provider === 'OpenAI-compatible'}<label><span>Base URL</span><input bind:value={baseUrl} placeholder="https://…" /></label>{/if}
      <p>Keys are not persisted by this frontend. The Rust host will own secure storage and endpoint policy.</p>
    </div>
  {/if}
</section>

<div class="ai-workspace">
  <section class="panel ai-source">
    <div class="panel-head"><div><span class="eyebrow">2 · Source</span><h2>Source context</h2></div><span class="timer-chip">~{Math.floor(speakingSeconds/60)}:{String(speakingSeconds%60).padStart(2,'0')} spoken</span></div>
    <textarea bind:value={sourceText} placeholder="Paste or load transcript/research text…"></textarea>
  </section>

  <section class="panel brief-panel">
    <div class="panel-head"><div><span class="eyebrow">3 · Direction</span><h2>Creative brief</h2></div><span class="brief-count">{briefFields}/4 filled</span></div>
    <div class="ai-brief-grid">
      <label><span>Topic / goal</span><input bind:value={topic} /></label>
      <label><span>Audience</span><input bind:value={audience} /></label>
      <label><span>Target duration</span><input bind:value={duration} /></label>
      <label><span>CTA</span><input bind:value={cta} /></label>
      <label class="wide"><span>Voice / style instructions</span><input bind:value={voice} /></label>
    </div>
  </section>

  <section class="panel ai-prompt">
    <div class="panel-head"><div><span class="eyebrow">4 · Artifact</span><h2>Generated prompt</h2></div>{#if prompt}<span class="ready-chip">Ready</span>{/if}</div>
    <textarea bind:value={prompt} placeholder="Build a prompt to preview it here…"></textarea>
  </section>
</div>

<div class="ai-action-bar">
  <span class="status-copy" aria-live="polite">{status}</span>
  <div class="ai-actions">
    <button class="button secondary" disabled={!prompt} on:click={() => copyText(prompt, 'Prompt')}>Copy prompt</button>
    <button class="button primary" on:click={buildPrompt}>Build prompt</button>
    <button class="button secondary run-button" disabled={busy || (mode === 'byok' && provider === 'Local (coming later)')} on:click={runAi}>{busy ? 'Running…' : mode === 'copy' ? 'Prepare for copy' : 'Run with API'}</button>
  </div>
</div>

{#if result}
  <section class="panel result-card"><div class="panel-head"><div><span class="eyebrow">Result</span><h2>AI result</h2></div><button class="text-button" on:click={() => copyText(result, 'Result')}>Copy result</button></div><div class="result-copy">{result}</div></section>
{/if}

<style>
  .ai-head { align-items: center; }
  .compact-mode { width: min(420px, 100%); margin: 0; }
  .mode-switch { display: grid; grid-template-columns: 1fr 1fr; gap: 7px; }
  .mode-switch button { display: grid; gap: 3px; padding: 10px 12px; border: 1px solid var(--border); border-radius: 9px; background: var(--surface); text-align: left; }
  .mode-switch button:hover { border-color: var(--border-strong); }
  .mode-switch button.active { border-color: color-mix(in srgb, var(--accent-strong) 45%, var(--border)); background: color-mix(in srgb, var(--accent-strong) 8%, var(--surface)); }
  .mode-switch strong { font-size: 12px; }
  .mode-switch span { color: var(--muted); font-size: 9px; }

  .ai-setup { display: grid; gap: 14px; margin-bottom: 12px; padding: 15px; }
  .setup-heading { display: flex; align-items: end; justify-content: space-between; gap: 20px; padding-bottom: 12px; border-bottom: 1px solid var(--border); }
  .setup-heading h2, .setup-heading p { margin: 0; }
  .setup-heading p { max-width: 440px; font-size: 11px; text-align: right; }
  .ai-config-grid { display: grid; grid-template-columns: minmax(180px, 1.3fr) 1fr 1fr; gap: 9px; }
  .ai-config-grid label, .ai-brief-grid label, .byok-strip label { display: grid; gap: 5px; }
  .ai-config-grid label span, .ai-brief-grid label span, .byok-strip label span { color: var(--muted); font-size: 10px; }
  .disabled-field { opacity: .55; }
  .byok-strip { display: grid; grid-template-columns: 1fr 1fr; gap: 10px; padding: 12px; border: 1px solid color-mix(in srgb, var(--accent-strong) 28%, var(--border)); border-radius: 10px; background: color-mix(in srgb, var(--accent-strong) 4%, transparent); }
  .byok-strip p { grid-column: 1 / -1; margin: 0; color: var(--muted); font-size: 10px; }

  .ai-workspace { display: grid; grid-template-columns: 1.15fr .85fr; grid-template-areas: 'source brief' 'prompt prompt'; gap: 12px; }
  .ai-source { grid-area: source; }
  .brief-panel { grid-area: brief; }
  .ai-prompt { grid-area: prompt; }
  .ai-source, .brief-panel, .ai-prompt { padding: 15px; }
  .ai-source textarea, .ai-prompt textarea { width: 100%; min-height: 270px; margin-top: 12px; line-height: 1.65; }
  .ai-prompt textarea { min-height: 220px; font-family: var(--font-technical); font-size: 12px; }
  .timer-chip, .ready-chip, .brief-count { padding: 4px 8px; border-radius: 999px; background: color-mix(in srgb, var(--accent-strong) 8%, var(--surface)); color: var(--accent); font-size: 9px; }
  .brief-count { color: var(--muted); background: var(--surface-2); }
  .ai-brief-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 10px; margin-top: 12px; }
  .ai-brief-grid .wide { grid-column: 1 / -1; }

  .ai-action-bar { position: sticky; bottom: 0; z-index: 8; display: flex; align-items: center; justify-content: space-between; gap: 16px; margin: 12px 0; padding: 10px 12px; border: 1px solid var(--border); border-radius: 11px; background: color-mix(in srgb, var(--surface) 94%, transparent); box-shadow: var(--shadow-1); backdrop-filter: blur(12px); }
  .ai-actions { display: flex; align-items: center; justify-content: flex-end; gap: 8px; flex: 0 0 auto; }
  .status-copy { margin: 0; min-width: 0; }
  .run-button { border-color: color-mix(in srgb, var(--info) 25%, var(--border)); }
  .result-card { padding: 15px; }
  .result-copy { margin-top: 12px; white-space: pre-wrap; color: var(--text); line-height: 1.65; }

  :global(html[dir='rtl']) .mode-switch button { text-align: right; }
  :global(html[dir='rtl']) .setup-heading p { text-align: left; }

  @media (max-width: 980px) {
    .ai-head { align-items: flex-start; flex-direction: column; }
    .compact-mode { width: 100%; }
    .ai-workspace { grid-template-columns: 1fr; grid-template-areas: 'source' 'brief' 'prompt'; }
  }
  @media (max-width: 760px) {
    .setup-heading { align-items: flex-start; flex-direction: column; }
    .setup-heading p { text-align: left; }
    .ai-config-grid, .ai-brief-grid, .byok-strip { grid-template-columns: 1fr; }
    .ai-brief-grid .wide, .byok-strip p { grid-column: auto; }
    .ai-action-bar { position: static; align-items: stretch; flex-direction: column; }
    .ai-actions { flex-wrap: wrap; justify-content: stretch; }
    .ai-actions .button { flex: 1; }
  }
</style>
