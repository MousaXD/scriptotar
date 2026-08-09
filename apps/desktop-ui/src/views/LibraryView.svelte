<script lang="ts">
  import type { LibraryItem } from '../types';
  import EmptyState from '../components/EmptyState.svelte';
  export let items: LibraryItem[];
  let query = '';
  let kind = 'All';
  $: visible = items.filter((item) => kind === 'All' || item.kind === kind).filter((item) => `${item.title} ${item.subtitle} ${item.platform || ''}`.toLowerCase().includes(query.toLowerCase()));
</script>
<section class="view-head"><div><span class="eyebrow">Unified local index</span><h1>Library</h1><p>Browse transcripts, creator research, AI runs, projects, and creators without switching mental contexts.</p></div></section>
<section class="panel library-panel">
  <div class="filter-bar"><label class="search-field"><span>⌕</span><input aria-label="Search library" bind:value={query} placeholder="Search your local library…" /></label><select bind:value={kind} aria-label="Library kind"><option>All</option><option>Transcript</option><option>Research</option><option>AI run</option><option>Project</option><option>Creator</option></select></div>
  {#if visible.length === 0}<EmptyState title="Nothing matches" message="Try a broader search or another library type." />{:else}
    <div class="library-list">{#each visible as item}<button><span class="kind-icon">{item.kind.slice(0,1)}</span><span class="library-copy"><strong>{item.title}</strong><small>{item.subtitle}</small></span><span class="library-meta">{item.platform || 'Local'}</span><span class="library-meta">{item.metric || '—'}</span><span class="library-date">{item.date}</span><span>›</span></button>{/each}</div>
  {/if}
</section>
