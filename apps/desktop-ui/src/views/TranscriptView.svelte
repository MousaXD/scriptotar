<script lang="ts">
  import { tick } from 'svelte';
  import type { Transcript, TranscriptSegment } from '../types';
  import EmptyState from '../components/EmptyState.svelte';

  export let transcripts: Transcript[];
  export let selectedId = '';

  let query = '';
  let status = '';
  let actionError = '';
  let searchInput: HTMLInputElement;

  $: if (transcripts.length > 0 && !transcripts.some((item) => item.id === selectedId)) selectedId = transcripts[0].id;
  $: selected = transcripts.find((item) => item.id === selectedId) || transcripts[0];
  $: matchingSegments = selected ? selected.segments.filter((segment) => segment.text.toLowerCase().includes(query.trim().toLowerCase())) : [];

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

  async function jumpToSegment(segmentId: string) {
    query = '';
    await tick();
    const target = document.getElementById(`segment-${segmentId}`);
    target?.scrollIntoView?.({ block: 'center', behavior: 'smooth' });
    (target as HTMLElement | null)?.focus?.();
  }

  function selectTranscript(id: string) {
    selectedId = id;
    query = '';
    status = '';
    actionError = '';
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
{#if !selected}
  <EmptyState title="No transcripts yet" message="Complete a transcription job and it will appear here." />
{:else}
  <div class="transcript-layout">
    <aside class="panel transcript-list" aria-label="Transcript list">{#each transcripts as transcript}<button class:active={transcript.id === selected.id} aria-current={transcript.id === selected.id ? 'true' : undefined} on:click={() => selectTranscript(transcript.id)}><strong>{transcript.title}</strong><small>{transcript.language} · {time(transcript.durationSeconds)}</small></button>{/each}</aside>
    <section class="panel transcript-reader">
      <div class="reader-head"><div><span class="eyebrow">{selected.platform}</span><h2>{selected.title}</h2><p>{selected.language} · {time(selected.durationSeconds)} · {selected.source}</p></div><div class="action-cluster"><button class="button secondary" on:click={copyText}>Copy text</button><button class="button secondary" on:click={() => downloadFile('txt', selected.text)}>Export TXT</button></div></div>
      <label class="search-field transcript-search"><span aria-hidden="true">⌕</span><input bind:this={searchInput} aria-label="Search transcript" bind:value={query} placeholder="Search transcript…" /><kbd>Ctrl F</kbd></label>
      {#if query.trim()}<p class="search-summary" aria-live="polite">{matchingSegments.length} matching {matchingSegments.length === 1 ? 'segment' : 'segments'}</p>{/if}
      <div class="transcript-content" dir={selected.direction} data-testid="transcript-content" class:rtl={selected.direction === 'rtl'}>
        {#if query && matchingSegments.length === 0}<p class="no-match">No timestamped segment contains “{query}”.</p>{:else}
          {#each (query ? matchingSegments : selected.segments) as segment}
            <article id={`segment-${segment.id}`} tabindex="-1" class="segment">
              <button class="timestamp" aria-label={`Jump to ${time(segment.startSeconds)}`} on:click={() => jumpToSegment(segment.id)}>{time(segment.startSeconds)}</button><p>{segment.text}</p>
            </article>
          {/each}
        {/if}
      </div>
    </section>
    <aside class="panel details-panel"><span class="eyebrow">Details</span><h3>Source metadata</h3><dl><div><dt>Language</dt><dd>{selected.language}</dd></div><div><dt>Direction</dt><dd>{selected.direction.toUpperCase()}</dd></div><div><dt>Duration</dt><dd>{time(selected.durationSeconds)}</dd></div><div><dt>Platform</dt><dd>{selected.platform}</dd></div></dl><div class="export-stack"><span>Exports</span><button on:click={() => downloadFile('txt', selected.text)}>TXT</button><button disabled={selected.segments.length === 0} on:click={() => downloadFile('timestamps.txt', timestampedText(selected.segments))}>Timestamp TXT</button><button disabled={selected.segments.length === 0} on:click={() => downloadFile('srt', srt(selected.segments))}>SRT</button><button disabled={selected.segments.length === 0} on:click={() => downloadFile('vtt', vtt(selected.segments), 'text/vtt;charset=utf-8')}>VTT</button><button on:click={() => downloadFile('json', JSON.stringify(selected, null, 2), 'application/json;charset=utf-8')}>JSON</button></div><button class="button secondary folder-unavailable" disabled title="The backend does not expose a persisted artifact directory for this transcript yet.">Open output folder unavailable</button></aside>
  </div>
  {#if status}<p class="status-copy" aria-live="polite">{status}</p>{/if}
  {#if actionError}<p class="status-copy" role="alert">{actionError}</p>{/if}
{/if}

<style>
  .search-summary { margin: 8px 0 0; font-size: 11px; }
  .segment:focus { outline: 2px solid var(--accent); outline-offset: 4px; border-radius: 5px; }
  .folder-unavailable { width: 100%; margin-top: 14px; white-space: normal; }
</style>
