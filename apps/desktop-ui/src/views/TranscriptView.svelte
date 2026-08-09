<script lang="ts">
  import type { Transcript } from '../types';
  import EmptyState from '../components/EmptyState.svelte';
  export let transcripts: Transcript[];
  let selectedId = transcripts[0]?.id || '';
  let query = '';
  $: selected = transcripts.find((item) => item.id === selectedId) || transcripts[0];
  $: matchingSegments = selected ? selected.segments.filter((segment) => segment.text.toLowerCase().includes(query.toLowerCase())) : [];

  function time(seconds: number) {
    const mins = Math.floor(seconds / 60).toString().padStart(2, '0');
    const secs = Math.floor(seconds % 60).toString().padStart(2, '0');
    return `${mins}:${secs}`;
  }
</script>
<section class="view-head"><div><span class="eyebrow">Review + reuse</span><h1>Transcript workspace</h1><p>Search timestamped text, copy clean language, and export without leaving the research context.</p></div></section>
{#if !selected}
  <EmptyState title="No transcripts yet" message="Complete a transcription job and it will appear here." />
{:else}
  <div class="transcript-layout">
    <aside class="panel transcript-list" aria-label="Transcript list">{#each transcripts as transcript}<button class:active={transcript.id === selected.id} on:click={() => selectedId = transcript.id}><strong>{transcript.title}</strong><small>{transcript.language} · {time(transcript.durationSeconds)}</small></button>{/each}</aside>
    <section class="panel transcript-reader">
      <div class="reader-head"><div><span class="eyebrow">{selected.platform}</span><h2>{selected.title}</h2><p>{selected.language} · {time(selected.durationSeconds)} · {selected.source}</p></div><div class="action-cluster"><button class="button secondary">Copy</button><button class="button secondary">Export</button></div></div>
      <label class="search-field transcript-search"><span>⌕</span><input aria-label="Search transcript" bind:value={query} placeholder="Search transcript…" /><kbd>⌘ F</kbd></label>
      <div class="transcript-content" dir={selected.direction} data-testid="transcript-content" class:rtl={selected.direction === 'rtl'}>
        {#if query && matchingSegments.length === 0}<p class="no-match">No timestamped segment contains “{query}”.</p>{:else}
          {#each (query ? matchingSegments : selected.segments) as segment}<article class="segment"><button class="timestamp" aria-label={`Jump to ${time(segment.startSeconds)}`}>{time(segment.startSeconds)}</button><p>{segment.text}</p></article>{/each}
        {/if}
      </div>
    </section>
    <aside class="panel details-panel"><span class="eyebrow">Details</span><h3>Source metadata</h3><dl><div><dt>Language</dt><dd>{selected.language}</dd></div><div><dt>Direction</dt><dd>{selected.direction.toUpperCase()}</dd></div><div><dt>Duration</dt><dd>{time(selected.durationSeconds)}</dd></div><div><dt>Platform</dt><dd>{selected.platform}</dd></div></dl><div class="export-stack"><span>Exports</span><button>TXT</button><button>Timestamp TXT</button><button>SRT</button><button>VTT</button><button>JSON</button></div></aside>
  </div>
{/if}
