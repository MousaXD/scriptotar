<script lang="ts">
  import type { AiMode, AiProvider } from '../types';
  import type { ScriptotarApi } from '../api/client';
  export let api: ScriptotarApi;
  export let initialSource = '';
  export let onCompleted: () => Promise<void> | void = () => {};

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
    if (mode === 'byok' && !apiKey.trim()) { status = 'Enter an API key for this session, or switch to Copy Prompt.'; return; }
    busy = true;
    try {
      result = await api.runAi(payload());
      if (mode === 'copy') {
        prompt = result;
        status = 'Copy Prompt prepared locally and added to local AI history.';
      } else {
        status = `Finished with ${provider}.`;
      }
      await onCompleted();
    }
    catch (error) { status = `AI request failed: ${error instanceof Error ? error.message : 'Unknown error'}`; }
    finally { busy = false; }
  }
</script>
<section class="view-head"><div><span class="eyebrow">Optional AI layer</span><h1>AI Studio</h1><p>Build portable prompts with no key, or use your own provider credentials for a direct run.</p></div></section>
<div class="ai-mode-card panel">
  <div class="mode-switch" role="group" aria-label="AI mode"><button class:active={mode === 'copy'} on:click={() => mode = 'copy'}><strong>Copy Prompt</strong><span>No API key · nothing sent</span></button><button class:active={mode === 'byok'} on:click={() => mode = 'byok'}><strong>BYOK</strong><span>Use your own provider key</span></button></div>
  <div class="ai-config-grid"><label><span>Task</span><select bind:value={task}><option>Viral breakdown</option><option>Hook ideas</option><option>New short-form script</option><option>Structure remix</option><option>Content ideas</option><option>Caption + CTA</option><option>Voice profile</option><option>B-roll shot list</option></select></label><label class:disabled-field={mode === 'copy'}><span>Provider</span><select aria-label="AI provider" bind:value={provider} disabled={mode === 'copy'}><option>OpenAI</option><option>Anthropic</option><option>Gemini</option><option>OpenAI-compatible</option><option>Local (coming later)</option></select></label><label class:disabled-field={mode === 'copy'}><span>Model</span><input bind:value={model} disabled={mode === 'copy'} /></label></div>
  {#if mode === 'byok'}<div class="byok-strip" data-testid="byok-fields"><label><span>API key · session only</span><input aria-label="API key" type="password" bind:value={apiKey} autocomplete="off" placeholder="Paste key for this run" /></label>{#if provider === 'OpenAI-compatible'}<label><span>Base URL</span><input bind:value={baseUrl} placeholder="https://…" /></label>{/if}<p>Keys are not persisted by this frontend or the Rust AI-run store.</p></div>{/if}
</div>
<div class="ai-grid"><section class="panel ai-source"><div class="panel-head"><div><span class="eyebrow">Input</span><h2>Source context</h2></div><span class="timer-chip">~{Math.floor(speakingSeconds/60)}:{String(speakingSeconds%60).padStart(2,'0')} spoken</span></div><textarea bind:value={sourceText} placeholder="Paste or load transcript/research text…"></textarea></section><section class="panel ai-prompt"><div class="panel-head"><div><span class="eyebrow">Portable artifact</span><h2>Generated prompt</h2></div></div><textarea bind:value={prompt} placeholder="Build a prompt to preview it here…"></textarea></section></div>
<section class="panel ai-brief-grid"><label><span>Topic / goal</span><input bind:value={topic} /></label><label><span>Audience</span><input bind:value={audience} /></label><label><span>Target duration</span><input bind:value={duration} /></label><label><span>CTA</span><input bind:value={cta} /></label><label class="wide"><span>Voice / style instructions</span><input bind:value={voice} /></label></section>
<div class="ai-actions"><button class="button primary" on:click={buildPrompt}>Build prompt</button><button class="button secondary" disabled={!prompt} on:click={() => copyText(prompt, 'Prompt')}>Copy prompt</button><button class="button secondary" disabled={busy || (mode === 'byok' && provider === 'Local (coming later)')} on:click={runAi}>{busy ? 'Running…' : mode === 'copy' ? 'Prepare for copy' : 'Run with API'}</button><span class="status-copy" aria-live="polite">{status}</span></div>
{#if result}<section class="panel result-card"><div class="panel-head"><h2>{mode === 'copy' ? 'Prepared prompt' : 'AI result'}</h2><button class="text-button" on:click={() => copyText(result, mode === 'copy' ? 'Prompt' : 'Result')}>Copy {mode === 'copy' ? 'prompt' : 'result'}</button></div><div class="result-copy">{result}</div></section>{/if}
