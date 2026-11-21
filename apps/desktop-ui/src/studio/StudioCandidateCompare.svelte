<script lang="ts">
  import { onDestroy } from 'svelte';
  import { locale } from '../i18n';
  import type { ContentStudy, DraftCandidate, StudioGateway } from './contract';
  import { workflowStrings } from './workflowStrings';

  export let study: ContentStudy | null = null;
  export let gateway: StudioGateway | undefined = undefined;
  export let projectId = '';
  export let pendingEdits = false;
  export let onChoose: (candidateId: string) => void | Promise<void> = () => {};

  let expanded = false;
  let loading = false;
  let error = '';
  let previews: { candidate: DraftCandidate; content: string; revision: number }[] = [];
  let observed = '';
  let epoch = 0;
  $: t = workflowStrings[$locale];
  $: candidates = (study?.drafts ?? []).slice(0, 3);
  // A receipt may refer to candidates beyond the first three visible
  // summaries (for example a manual candidate precedes the generated batch).
  // Track ALL saved draft revisions so an unseen candidate edit invalidates
  // an already-rendered three-script comparison.
  $: signature = JSON.stringify([projectId, study?.id, study?.revision,
    (study?.drafts ?? []).map(item => [item.id, item.revision]),
    (study?.archivedDrafts ?? []).map(item => [item.id, item.revision])]);
  $: if (signature !== observed) {
    observed = signature;
    epoch++;
    expanded = false;
    previews = [];
    error = '';
    loading = false;
  }

  onDestroy(() => { epoch++; });

  async function openComparison() {
    if (expanded) { expanded = false; return; }
    if (!study || !gateway || candidates.length < 2 || loading) return;
    expanded = true;
    loading = true;
    error = '';
    const current = study;
    const expected = signature;
    const request = ++epoch;
    try {
      // Prefer the *exact* three draft IDs in the latest immutable v11
      // receipt, not the three newest summaries (a Study may already have a
      // manual candidate, and edits/renames can reorder the draft list).
      // This is read-only; no automatic AI request or approval occurs here.
      let selected = candidates;
      if (gateway.listStudyWritingBatches) {
        const batches = await gateway.listStudyWritingBatches(projectId, current.id);
        if (request !== epoch || signature !== expected) return;
        if (!Array.isArray(batches)) throw new Error(t.comparisonError);
        if (batches.length > 0) {
          const batch = batches[0];
          if (batch.projectId !== projectId || batch.studyId !== current.id ||
              !Array.isArray(batch.draftIds) || batch.draftIds.length !== 3 ||
              new Set(batch.draftIds).size !== 3) {
            throw new Error(t.comparisonError);
          }
          const exact = batch.draftIds.map(id => current.drafts?.find(item => item.id === id));
          if (exact.some(item => !item)) throw new Error(t.comparisonError);
          selected = exact as DraftCandidate[];
        }
      }
      const result = await Promise.all(selected.map(async candidate => {
        const item = await gateway.getStudy(projectId, current.id, candidate.id);
        if (item.id !== current.id || item.projectId !== projectId ||
            item.revision !== current.revision ||
            item.draftId !== candidate.id || item.draftRevision !== candidate.revision ||
            !item.drafts?.some(d => d.id === candidate.id)) {
          throw new Error(t.comparisonError);
        }
        return { candidate, content: item.draft, revision: item.draftRevision };
      }));
      if (request !== epoch || signature !== expected) return;
      previews = result;
    } catch (cause) {
      if (request === epoch) error = cause instanceof Error ? cause.message : t.comparisonError;
    } finally {
      if (request === epoch) loading = false;
    }
  }
</script>

{#if study && gateway && candidates.length > 1}
  <section class="candidate-compare" aria-label={t.compare} dir={$locale === 'ar' ? 'rtl' : 'ltr'}>
    <button type="button" class="button secondary" aria-expanded={expanded}
      disabled={loading} on:click={() => void openComparison()}>{t.compare} ({candidates.length})</button>
    {#if expanded}
      {#if pendingEdits}<p class="compare-muted" role="status">{t.unsaved}</p>{/if}
      {#if loading}<p class="compare-muted" role="status">{t.comparisonLoading}</p>{/if}
      {#if error}<p role="alert" class="compare-error">{error}</p>{/if}
      {#if previews.length}
        <div class="comparison-grid">
          {#each previews as item (item.candidate.id)}
            <article class:current={study.draftId === item.candidate.id}>
              <h3 dir="auto">{item.candidate.title}</h3>
              <span class="compare-muted">{t.savedRevision} {item.revision}</span>
              <pre dir="auto">{item.content || t.noCandidateText}</pre>
              <button type="button" class="button secondary"
                on:click={() => onChoose(item.candidate.id)}>{t.editCandidate}</button>
            </article>
          {/each}
        </div>
      {/if}
    {/if}
  </section>
{/if}

<style>
  .candidate-compare { margin-block: 12px; display: grid; gap: 10px; }
  .candidate-compare > button { justify-self: start; }
  .comparison-grid { display: grid; gap: 10px; grid-template-columns: repeat(3, minmax(0, 1fr)); }
  .comparison-grid article { border: 1px solid var(--color-border); border-radius: var(--radius-md); padding: 10px; min-width: 0; display: grid; gap: 7px; align-content: start; }
  .comparison-grid article.current { border-color: var(--color-border-strong); }
  .comparison-grid h3 { font-size: var(--text-sm); margin: 0; overflow-wrap: anywhere; }
  .comparison-grid pre { font: inherit; font-size: var(--text-xs); line-height: 1.6; white-space: pre-wrap; overflow-wrap: anywhere; min-height: 110px; max-height: 270px; overflow: auto; }
  .comparison-grid button { justify-self: start; }
  .compare-muted { margin: 0; font-size: var(--text-xs); color: var(--color-text-muted); }
  .compare-error { margin: 0; color: var(--color-danger, #b44); }
  @media (max-width: 980px) { .comparison-grid { grid-template-columns: 1fr; } }
</style>