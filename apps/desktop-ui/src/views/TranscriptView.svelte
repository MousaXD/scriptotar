<script lang="ts">
  import { tick } from 'svelte';
  import { translator } from '../i18n/translate';
  import type { Transcript, TranscriptSegment } from '../types';
  import type { ScriptotarApi } from '../api/client';
  import EmptyState from '../components/EmptyState.svelte';
  import Icon from '../components/Icon.svelte';

  export let api: ScriptotarApi;
  export let transcripts: Transcript[];
  export let selectedId = '';

  const LIST_KEY = 'scriptotar.transcriptListOpen';
  const DETAILS_KEY = 'scriptotar.transcriptDetailsOpen';

  let query = '';
  let status = '';
  let actionError = '';
  let searchInput: HTMLInputElement;
  let listOpen = readPanel(LIST_KEY, true);
  let detailsOpen = readPanel(DETAILS_KEY, true);
  let searchCursor = 0;
  let previousQuery = '';
  let selected: Transcript | undefined;
  let selectedLoadId = '';
  let selectedLoading = false;
  let selectedLoadError = '';
  let selectedGeneration = 0;

  $: if (transcripts.length > 0 && !transcripts.some((item) => item.id === selectedId)) selectedId = transcripts[0].id;
  $: if (selectedId && selectedId !== selectedLoadId) void loadSelected(selectedId);
  $: matchingSegments = selected ? selected.segments.filter((segment) => segment.text.toLowerCase().includes(query.trim().toLowerCase())) : [];
  $: if (query !== previousQuery) { previousQuery = query; searchCursor = 0; }
  $: if (searchCursor >= matchingSegments.length) searchCursor = Math.max(0, matchingSegments.length - 1);

  async function loadSelected(id: string) {
    selectedLoadId = id;
    selectedLoading = true;
    selectedLoadError = '';
    const generation = ++selectedGeneration;
    try {
      const transcript = await api.getTranscript(id);
      if (generation === selectedGeneration && selectedId === id) selected = transcript;
    } catch (cause) {
      if (generation === selectedGeneration && selectedId === id) {
        selected = undefined;
        selectedLoadError = cause instanceof Error ? cause.message : 'Could not load the transcript.';
      }
    } finally {
      if (generation === selectedGeneration) selectedLoading = false;
    }
  }

  function readPanel(key: string, fallback: boolean) {
    if (typeof window === 'undefined') return fallback;
    try {
      const value = window.localStorage.getItem(key);
      return value === null ? fallback : value === '1';
    } catch { return fallback; }
  }

  function setPanel(key: string, value: boolean) {
    try { window.localStorage.setItem(key, value ? '1' : '0'); }
    catch { /* local UI state can remain in-memory */ }
  }

  function toggleList() {
    listOpen = !listOpen;
    setPanel(LIST_KEY, listOpen);
  }

  function toggleDetails() {
    detailsOpen = !detailsOpen;
    setPanel(DETAILS_KEY, detailsOpen);
  }

  function time(seconds: number) {
    const mins = Math.floor(seconds / 60).toString().padStart(2, '0');
    const secs = Math.floor(seconds % 60).toString().padStart(2, '0');
    return `${mins}:${secs}`;
  }

  function subtitleTimestamp(seconds: number, separator: ',' | '.') {
    const hours = Math.floor(seconds / 3600).toString().padStart(2, '0');
    const minutes = Math.floor((seconds % 3600) / 60).toString().padStart(2, '0');
    const wholeSeconds = Math.floor(seconds % 60).toString().padStart(2, '0');
    const millis = Math.floor((seconds % 1) * 1000).toString().padStart(3, '0');
    return `${hours}:${minutes}:${wholeSeconds}${separator}${millis}`;
  }

  function safeName(value: string) {
    return value.trim().replace(/[\\/:*?"<>|]+/g, '-').replace(/\s+/g, ' ').slice(0, 100) || 'transcript';
  }

  function srt(segments: TranscriptSegment[]) {
    return segments.map((segment, index) => `${index + 1}\n${subtitleTimestamp(segment.startSeconds, ',')} --> ${subtitleTimestamp(segment.endSeconds, ',')}\n${segment.text}\n`).join('\n');
  }

  function vtt(segments: TranscriptSegment[]) {
    return `WEBVTT\n\n${segments.map((segment) => `${subtitleTimestamp(segment.startSeconds, '.')} --> ${subtitleTimestamp(segment.endSeconds, '.')}\n${segment.text}\n`).join('\n')}`;
  }

  function timestampedText(segments: TranscriptSegment[]) {
    return segments.map((segment) => `[${time(segment.startSeconds)}] ${segment.text}`).join('\n');
  }

  function downloadFile(extension: string, content: string, mime = 'text/plain;charset=utf-8') {
    if (!selected) return;
    actionError = '';
    const blob = new Blob([content], { type: mime });
    const href = URL.createObjectURL(blob);
    const anchor = document.createElement('a');
    anchor.href = href;
    anchor.download = `${safeName(selected.title)}.${extension}`;
    anchor.click();
    URL.revokeObjectURL(href);
    status = `${extension.toUpperCase()} export prepared.`;
  }

  async function copyText() {
    if (!selected) return;
    actionError = '';
    try {
      if (navigator.clipboard?.writeText) {
        await navigator.clipboard.writeText(selected.text);
      } else {
        const textarea = document.createElement('textarea');
        textarea.value = selected.text;
        textarea.style.position = 'fixed';
        textarea.style.opacity = '0';
        document.body.appendChild(textarea);
        textarea.select();
        const copied = document.execCommand?.('copy');
        textarea.remove();
        if (!copied) throw new Error('Clipboard access is unavailable in this desktop runtime.');
      }
      status = 'Transcript copied to the clipboard.';
    } catch (cause) {
      actionError = cause instanceof Error ? cause.message : 'Could not copy the transcript.';
    }
  }

  async function scrollToSegment(segmentId: string) {
    await tick();
    const target = document.getElementById(`segment-${segmentId}`);
    target?.scrollIntoView?.({ block: 'center', behavior: 'smooth' });
    (target as HTMLElement | null)?.focus?.({ preventScroll: true });
  }

  async function jumpToSegment(segmentId: string) {
    query = '';
    searchCursor = 0;
    await scrollToSegment(segmentId);
  }

  async function jumpSearch(delta: number) {
    if (matchingSegments.length === 0) return;
    searchCursor = (searchCursor + delta + matchingSegments.length) % matchingSegments.length;
    await scrollToSegment(matchingSegments[searchCursor].id);
  }

  function selectTranscript(id: string) {
    if (selectedId !== id) selected = undefined;
    selectedId = id;
    query = '';
    status = '';
    actionError = '';
    searchCursor = 0;
  }

  function searchShortcut(event: KeyboardEvent) {
    if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'f') {
      event.preventDefault();
      searchInput?.focus();
      searchInput?.select();
    }
  }
</script>
<svelte:window onkeydown={searchShortcut} />

<section class="view-head"><div><span class="eyebrow">Review + reuse</span><h1>Transcript workspace</h1><p>Search timestamped text, jump between matching segments, copy clean text, and export local formats without inventing artifact paths.</p></div></section>
{#if transcripts.length === 0}
  <EmptyState title="No transcripts yet" message="Complete a transcription job and it will appear here." />
{:else if !selected}
  <section class="panel transcript-loading" aria-busy={selectedLoading}>
    <strong>{selectedLoading ? 'Loading transcript…' : 'Transcript unavailable'}</strong>
    {#if selectedLoadError}<p role="alert">{selectedLoadError}</p>{/if}
  </section>
{:else}
  <div class:list-hidden={!listOpen} class:details-hidden={!detailsOpen} class="transcript-shell-v2">
    {#if listOpen}
      <aside class="panel transcript-list" aria-label="Transcript list">
        <div class="rail-head"><span class="eyebrow">Transcript</span><button class="rail-button" on:click={toggleList} aria-label={$translator('transcript.hideList')} title={$translator('transcript.hideList')}>‹</button></div>
        <div class="transcript-list-scroll">
          {#each transcripts as transcript}
            <button class:active={transcript.id === selected.id} aria-current={transcript.id === selected.id ? 'true' : undefined} on:click={() => selectTranscript(transcript.id)}>
              <strong>{transcript.title}</strong><small>{transcript.language} · {time(transcript.durationSeconds)}</small>
            </button>
          {/each}
        </div>
      </aside>
    {/if}

    <section class="panel transcript-reader">
      <div class="reader-head">
        <div class="reader-title"><span class="eyebrow">{selected.platform}</span><h2>{selected.title}</h2><p>{selected.language} · {time(selected.durationSeconds)} · {selected.source}</p></div>
        <div class="action-cluster">
          {#if !listOpen}<button class="icon-button" on:click={toggleList} aria-label={$translator('transcript.showList')} title={$translator('transcript.showList')}><Icon name="panel" size={17} /></button>{/if}
          <button class="button secondary" on:click={copyText}>Copy text</button>
          <details class="export-menu">
            <summary class="button secondary export-summary">{$translator('transcript.export')}</summary>
            <div class="export-popover">
              <button on:click={() => downloadFile('txt', selected?.text || '')}>TXT</button>
              <button disabled={selected.segments.length === 0} on:click={() => downloadFile('timestamps.txt', timestampedText(selected?.segments || []))}>Timestamp TXT</button>
              <button disabled={selected.segments.length === 0} on:click={() => downloadFile('srt', srt(selected?.segments || []))}>SRT</button>
              <button disabled={selected.segments.length === 0} on:click={() => downloadFile('vtt', vtt(selected?.segments || []), 'text/vtt;charset=utf-8')}>VTT</button>
              <button on:click={() => downloadFile('json', JSON.stringify(selected, null, 2), 'application/json;charset=utf-8')}>JSON</button>
            </div>
          </details>
          {#if !detailsOpen}<button class="icon-button" on:click={toggleDetails} aria-label={$translator('transcript.showDetails')} title={$translator('transcript.showDetails')}><Icon name="settings" size={17} /></button>{/if}
        </div>
      </div>

      <div class="transcript-search-row">
        <label class="search-field transcript-search"><span aria-hidden="true">⌕</span><input bind:this={searchInput} aria-label="Search transcript" bind:value={query} placeholder="Search transcript…" /><kbd>Ctrl F</kbd></label>
        {#if query.trim()}
          <div class="search-navigation" aria-live="polite">
            <span>{matchingSegments.length > 0 ? $translator('transcript.matches', { current: searchCursor + 1, count: matchingSegments.length }) : $translator('transcript.noMatches')}</span>
            <button disabled={matchingSegments.length === 0} on:click={() => jumpSearch(-1)} aria-label={$translator('transcript.previousMatch')} title={$translator('transcript.previousMatch')}>↑</button>
            <button disabled={matchingSegments.length === 0} on:click={() => jumpSearch(1)} aria-label={$translator('transcript.nextMatch')} title={$translator('transcript.nextMatch')}>↓</button>
          </div>
        {/if}
      </div>

      <div class="transcript-content" dir={selected.direction} data-testid="transcript-content" data-i18n-ignore class:rtl={selected.direction === 'rtl'}>
        {#if query && matchingSegments.length === 0}
          <p class="no-match">{$translator('transcript.noMatches')}</p>
        {:else}
          {#each (query ? matchingSegments : selected.segments) as segment}
            <article id={`segment-${segment.id}`} tabindex="-1" class:search-current={Boolean(query) && matchingSegments[searchCursor]?.id === segment.id} class="segment">
              <button class="timestamp" aria-label={`Jump to ${time(segment.startSeconds)}`} on:click={() => jumpToSegment(segment.id)}>{time(segment.startSeconds)}</button>
              <p>{segment.text}</p>
            </article>
          {/each}
        {/if}
      </div>
    </section>

    {#if detailsOpen}
      <aside class="panel details-panel">
        <div class="rail-head"><div><span class="eyebrow">Details</span><h3>Source metadata</h3></div><button class="rail-button" on:click={toggleDetails} aria-label={$translator('transcript.hideDetails')} title={$translator('transcript.hideDetails')}>›</button></div>
        <dl><div><dt>Language</dt><dd>{selected.language}</dd></div><div><dt>Direction</dt><dd>{selected.direction.toUpperCase()}</dd></div><div><dt>Duration</dt><dd>{time(selected.durationSeconds)}</dd></div><div><dt>Platform</dt><dd>{selected.platform}</dd></div></dl>
        <button class="button secondary folder-unavailable" disabled title="The backend does not expose a persisted artifact directory for this transcript yet.">Open output folder unavailable</button>
      </aside>
    {/if}
  </div>
  {#if status}<p class="status-copy" aria-live="polite">{status}</p>{/if}
  {#if actionError}<p class="status-copy" role="alert">{actionError}</p>{/if}
{/if}

<style>
  .transcript-shell-v2 { display: grid; grid-template-columns: 220px minmax(0, 1fr) 210px; gap: 12px; align-items: start; }
  .transcript-shell-v2.list-hidden { grid-template-columns: minmax(0, 1fr) 210px; }
  .transcript-shell-v2.details-hidden { grid-template-columns: 220px minmax(0, 1fr); }
  .transcript-shell-v2.list-hidden.details-hidden { grid-template-columns: minmax(0, 1fr); }
  .rail-head { display: flex; align-items: flex-start; justify-content: space-between; gap: 8px; }
  .rail-head h3 { margin: 0; }
  .rail-button, .icon-button, .search-navigation button { display: grid; place-items: center; width: 32px; height: 32px; padding: 0; border: 1px solid var(--border); border-radius: 7px; background: var(--surface-2); color: var(--muted); }
  .rail-button:hover, .icon-button:hover, .search-navigation button:hover:not(:disabled) { color: var(--text); border-color: var(--border-strong); }
  .transcript-list { position: sticky; top: 82px; max-height: calc(100vh - 108px); padding: 10px; }
  .transcript-list-scroll { max-height: calc(100vh - 160px); overflow-y: auto; }
  .transcript-list-scroll > button { display: block; width: 100%; padding: 11px; border: 0; border-radius: 8px; background: transparent; text-align: left; }
  .transcript-list-scroll > button:hover { background: var(--surface-2); }
  .transcript-list-scroll > button.active { background: color-mix(in srgb, var(--accent-strong) 10%, var(--surface)); }
  .transcript-list-scroll strong, .transcript-list-scroll small { display: block; }
  .transcript-list-scroll strong { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .transcript-list-scroll small { margin-top: 5px; color: var(--muted); font-size: 10px; }
  .transcript-reader { min-height: 660px; padding: 18px; }
  .reader-head { display: flex; justify-content: space-between; gap: 16px; }
  .reader-title { min-width: 0; }
  .reader-title h2 { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .reader-head p { margin: 5px 0 0; font-size: 11px; }
  .action-cluster { display: flex; align-items: flex-start; flex-wrap: wrap; gap: 7px; }
  .export-menu { position: relative; }
  .export-summary { display: grid; place-items: center; list-style: none; cursor: pointer; }
  .export-summary::-webkit-details-marker { display: none; }
  .export-popover { position: absolute; inset-inline-end: 0; top: calc(100% + 6px); z-index: 15; display: grid; min-width: 160px; padding: 5px; border: 1px solid var(--border-strong); border-radius: 9px; background: var(--surface); box-shadow: var(--shadow-2); }
  .export-popover button { min-height: 34px; padding: 0 9px; border: 0; border-radius: 6px; background: transparent; color: var(--text); text-align: left; }
  .export-popover button:hover:not(:disabled) { background: var(--surface-2); }
  .transcript-search-row { display: grid; grid-template-columns: minmax(0, 1fr) auto; gap: 8px; align-items: center; margin: 15px 0 8px; }
  .transcript-search { margin: 0; }
  .search-navigation { display: flex; align-items: center; gap: 5px; color: var(--muted); font-size: 10px; }
  .search-navigation > span { min-width: 62px; text-align: right; }
  .search-navigation button { width: 30px; height: 30px; font-family: var(--font-technical); }
  .transcript-content { max-height: calc(100vh - 250px); min-height: 500px; overflow: auto; padding: 5px 3px 42px; scroll-padding-block: 30%; }
  .segment { display: grid; grid-template-columns: 54px minmax(0, 1fr); gap: 14px; padding: 14px 8px; border-top: 1px solid var(--border); border-radius: 7px; transition: background var(--motion-fast) var(--ease-standard), border-color var(--motion-fast) var(--ease-standard); }
  .segment.search-current { border-color: color-mix(in srgb, var(--accent-strong) 45%, var(--border)); background: color-mix(in srgb, var(--accent-strong) 7%, transparent); }
  .segment:focus { outline: none; box-shadow: inset 3px 0 0 var(--accent-strong); }
  .segment p { margin: 0; color: var(--text); font-size: 14px; line-height: 1.85; }
  .timestamp { align-self: start; padding: 4px 5px; border: 0; border-radius: 5px; background: var(--surface-2); color: var(--info); font-size: 10px; }
  .details-panel { position: sticky; top: 82px; padding: 14px; }
  .details-panel dl { margin: 15px 0; }
  .details-panel dl div { display: flex; justify-content: space-between; gap: 10px; padding: 8px 0; border-top: 1px solid var(--border); font-size: 10px; }
  .details-panel dt { color: var(--faint); }
  .details-panel dd { margin: 0; }
  .folder-unavailable { width: 100%; margin-top: 14px; white-space: normal; }
  .no-match { padding: 20px 0; }

  :global(html[dir='rtl']) .transcript-list-scroll > button,
  :global(html[dir='rtl']) .export-popover button { text-align: right; }

  @media (max-width: 1150px) {
    .transcript-shell-v2 { grid-template-columns: 190px minmax(0, 1fr); }
    .transcript-shell-v2.list-hidden { grid-template-columns: minmax(0, 1fr); }
    .details-panel { grid-column: 2; position: static; }
    .list-hidden .details-panel { grid-column: 1; }
  }
  @media (max-width: 760px) {
    .transcript-shell-v2, .transcript-shell-v2.list-hidden, .transcript-shell-v2.details-hidden, .transcript-shell-v2.list-hidden.details-hidden { grid-template-columns: 1fr; }
    .transcript-list { position: static; max-height: none; }
    .transcript-list-scroll { display: flex; max-height: none; overflow-x: auto; }
    .transcript-list-scroll > button { min-width: 190px; }
    .reader-head { flex-direction: column; }
    .transcript-search-row { grid-template-columns: 1fr; }
    .search-navigation { justify-content: flex-end; }
    .transcript-content { min-height: 430px; max-height: 65vh; }
    .details-panel { position: static; grid-column: 1; }
  }
</style>
