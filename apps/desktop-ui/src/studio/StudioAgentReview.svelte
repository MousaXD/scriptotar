<script lang="ts">
  import { onDestroy } from 'svelte';
  import { locale } from '../i18n';
  import Select from '../components/ui/Select.svelte';
  import Checkbox from '../components/ui/Checkbox.svelte';
  import type { ContentStudy, StudioGateway, StudySourceRef } from './contract';
  import type { StudioAgentRunGateway, StudioRun, StudioRunDecision, StudioRunRequest } from './agenticRuntime';
  import { isSameSourceSet, limitsValid, validatesStudioRun } from './agenticRuntime';
  import { workflowStrings } from './workflowStrings';

  export let projectId: string;
  export let study: ContentStudy | null = null;
  export let gateway: StudioGateway | undefined = undefined;
  export let prompt = '';
  export let dirty = false;

  let provider = 'OpenAI';
  let model = '';
  let tokens = '6000';
  let dollars = '1.00';
  let consent = false;
  let run: StudioRun | null = null;
  let error = '';
  let pending = false;
  let loadedStudyKey = '';
  let observedContext = '';
  let requestEpoch = 0;

  $: t = workflowStrings[$locale];
  $: bridge = gateway as (StudioGateway & StudioAgentRunGateway) | undefined;
  $: context = JSON.stringify([projectId, study?.id, study?.revision, study?.sources, prompt,
    provider, model, tokens, dollars]);
  $: if (context !== observedContext) {
    observedContext = context;
    requestEpoch++;
    consent = false;
    error = '';
    // The previous run may still exist durably in the backend. Do not
    // display it as an authorization for a newly edited Study or payload.
    if (run && (run.studyRevision !== study?.revision || run.projectId !== projectId ||
      run.studyId !== study?.id || !isSameSourceSet(run.selectedSources, study?.sources ?? []) ||
      (prompt && run.exactPrompt !== prompt) || run.provider !== provider || run.model !== model.trim() ||
      run.maxTokens !== Number(tokens) || run.maxUsd !== Number(dollars))) {
      run = null;
    }
  }
  $: studyKey = projectId + ':' + (study?.id ?? '');
  $: if (study?.id && bridge?.listStudioRuns && studyKey !== loadedStudyKey) {
    loadedStudyKey = studyKey;
    void refresh();
  }
  $: exactPrompt = run?.exactPrompt || prompt;
  $: sources = study?.sources ?? [];
  $: isCurrentRun = !!run && run.projectId === projectId && run.studyId === study?.id &&
    run.studyRevision === study?.revision &&
    isSameSourceSet(run.selectedSources, sources) && !dirty &&
    run.provider === provider && run.model === model.trim() &&
    run.maxTokens === Number(tokens) && run.maxUsd === Number(dollars) &&
    (!prompt || run.exactPrompt === prompt);
  $: canPropose = !!study && !!prompt.trim() && !!model.trim() && !dirty && !pending &&
    limitsValid(tokens, dollars) && !!bridge?.proposeStudioRun;
  $: canDecide = isCurrentRun && !!bridge?.decideStudioRun && !pending;

  onDestroy(() => { requestEpoch++; });

  function fail(cause: unknown, fallback: string) {
    error = cause instanceof Error ? cause.message : fallback;
  }

  function assertRun(next: StudioRun, id: string, expectedRevision: number) {
    if (!validatesStudioRun(next, projectId, id) || next.studyRevision !== expectedRevision) {
      throw new Error(t.stale);
    }
  }

  async function refresh() {
    if (!bridge?.listStudioRuns || !study) return;
    const id = study.id, rev = study.revision, epoch = ++requestEpoch;
    pending = true;
    error = '';
    try {
      const existing = await bridge.listStudioRuns(projectId, id);
      if (epoch !== requestEpoch || study?.id !== id || study.revision !== rev) return;
      if (!Array.isArray(existing) || existing.some(item => !validatesStudioRun(item, projectId, id))) {
        throw new Error(t.stale);
      }
      run = existing[0] ?? null;
      if (run) {
        provider = run.provider;
        model = run.model;
        tokens = String(run.maxTokens);
        dollars = String(run.maxUsd);
      }
      consent = false;
    } catch (cause) {
      if (epoch === requestEpoch) fail(cause, t.decisionError);
    } finally {
      if (epoch === requestEpoch) pending = false;
    }
  }

  async function propose() {
    const current = study;
    if (!current || !canPropose || !bridge?.proposeStudioRun) return;
    const request: StudioRunRequest = {
      provider, model: model.trim(), exactPrompt: prompt,
      selectedSources: current.sources.map(source => ({ ...source })),
      maxTokens: Number(tokens), maxUsd: Number(dollars)
    };
    const id = current.id, rev = current.revision, epoch = ++requestEpoch, snapshot = context;
    pending = true;
    error = '';
    consent = false;
    try {
      const next = await bridge.proposeStudioRun(projectId, id, rev, request);
      if (epoch !== requestEpoch || snapshot !== context || study?.id !== id) return;
      assertRun(next, id, rev);
      if (next.provider !== request.provider || next.model !== request.model ||
        next.exactPrompt !== request.exactPrompt ||
        next.maxTokens !== request.maxTokens || next.maxUsd !== request.maxUsd ||
        !isSameSourceSet(next.selectedSources, request.selectedSources) ||
        next.state !== 'awaiting_approval') throw new Error(t.stale);
      run = next;
    } catch (cause) {
      if (epoch === requestEpoch) fail(cause, t.requestError);
    } finally {
      if (epoch === requestEpoch) pending = false;
    }
  }

  async function decide(action: StudioRunDecision) {
    const current = run, savedStudy = study;
    if (!current || !savedStudy || !canDecide || !bridge?.decideStudioRun) return;
    if (action === 'approve' && !consent) return;
    const id = savedStudy.id, rev = savedStudy.revision, epoch = ++requestEpoch;
    pending = true;
    error = '';
    try {
      const next = await bridge.decideStudioRun(projectId, id, current.runId, action);
      if (epoch !== requestEpoch || study?.id !== id || study?.revision !== rev) return;
      assertRun(next, id, rev);
      if (next.runId !== current.runId || next.exactPrompt !== current.exactPrompt ||
        !isSameSourceSet(next.selectedSources, current.selectedSources)) throw new Error(t.stale);
      run = next;
      consent = false;
    } catch (cause) {
      if (epoch === requestEpoch) fail(cause, t.decisionError);
    } finally {
      if (epoch === requestEpoch) pending = false;
    }
  }

  function stateName(state: StudioRun['state']): string {
    switch (state) {
      case 'planned': return t.statePlanned;
      case 'awaiting_approval': return t.stateAwaitingApproval;
      case 'running': return t.stateRunning;
      case 'needs_input': return t.stateNeedsInput;
      case 'blocked': return t.stateBlocked;
      case 'failed': return t.stateFailed;
      case 'cancelled': return t.stateCancelled;
      case 'completed': return t.stateCompleted;
    }
  }
</script>

<section class="studio-run-review" aria-labelledby="studio-run-heading" dir={$locale === 'ar' ? 'rtl' : 'ltr'}>
  <div class="run-heading">
    <h2 id="studio-run-heading">{t.runHeading}</h2>
    {#if run && isCurrentRun}<span role="status">{stateName(run.state)}</span>{/if}
  </div>
  {#if !study}
    <p class="run-muted">{t.noStudy}</p>
  {:else}
    {#if !prompt && !run}<p class="run-muted">{t.noPrompt}</p>{/if}
    {#if !bridge?.proposeStudioRun || !bridge?.decideStudioRun}
      <p class="run-muted" role="status">{t.disconnected}</p>
    {/if}
    <div class="run-fields">
      <Select label={t.provider} value={provider}
        options={[{ value: 'OpenAI', label: 'OpenAI' }, { value: 'Anthropic', label: 'Anthropic' },
          { value: 'Google', label: 'Google' }, { value: 'Local', label: 'Local' }]}
        onChange={(value) => { provider = value; }} />
      <label>{t.model}
        <input type="text" bind:value={model} maxlength="100" dir="ltr" autocomplete="off" />
      </label>
      <label>{t.maxTokens}
        <input type="text" inputmode="numeric" bind:value={tokens} maxlength="6" dir="ltr" />
      </label>
      <label>{t.maxUsd}
        <input type="text" inputmode="decimal" bind:value={dollars} maxlength="6" dir="ltr" />
      </label>
    </div>
    {#if !limitsValid(tokens, dollars)}
      <p class="run-error" role="alert">{t.budgetError}</p>
    {/if}
    {#if exactPrompt}
      <details class="request-preview">
        <summary>{t.reviewScope} · {sources.length} {t.scopeCount}</summary>
        <ul>
          {#each (run?.selectedSources ?? sources) as ref (ref.kind + ':' + ref.id)}
            <li dir="auto">{ref.kind} · {ref.id}</li>
          {/each}
        </ul>
        <pre dir="auto">{exactPrompt}</pre>
      </details>
    {/if}
    {#if bridge?.proposeStudioRun && bridge?.decideStudioRun}
      <div class="run-actions">
        <button type="button" class="button secondary" disabled={!canPropose}
          on:click={() => void propose()}>{t.propose}</button>
        {#if bridge.listStudioRuns}
          <button type="button" class="button secondary" disabled={pending}
            on:click={() => void refresh()}>{t.refresh}</button>
        {/if}
        {#if run && isCurrentRun}
          {#if run.state === 'awaiting_approval'}
            <Checkbox label={t.consent} checked={consent} onChange={(value) => { consent = value; }} />
            <button type="button" class="button primary" disabled={!canDecide || !consent}
              on:click={() => void decide('approve')}>{t.approve}</button>
            <button type="button" class="button secondary" disabled={!canDecide}
              on:click={() => void decide('deny')}>{t.deny}</button>
          {:else if run.state === 'running' || run.state === 'needs_input'}
            <button type="button" class="button secondary" disabled={!canDecide}
              on:click={() => void decide('cancel')}>{t.cancel}</button>
          {:else if run.state === 'blocked'}
            <button type="button" class="button secondary" disabled={!canDecide}
              on:click={() => void decide('recover')}>{t.recover}</button>
          {/if}
        {/if}
      </div>
    {/if}
    {#if run && isCurrentRun && run.totalSteps != null}
      <p class="run-muted">{t.runProgress}: {run.completedSteps ?? 0}/{run.totalSteps}</p>
    {/if}
    {#if run?.lastError && isCurrentRun}<p class="run-error" role="alert" dir="auto">{run.lastError}</p>{/if}
    {#if run && !isCurrentRun}<p class="run-error" role="alert">{t.stale}</p>{/if}
    {#if error}<p class="run-error" role="alert">{error}</p>{/if}
    <p class="run-muted">{t.notApproved}</p>
  {/if}
</section>

<style>
  .studio-run-review { display: grid; gap: 10px; }
  .run-heading { display: flex; flex-wrap: wrap; align-items: center; justify-content: space-between; gap: 8px; }
  .run-heading h2 { margin: 0; }
  .run-heading span { font-size: var(--text-xs); font-weight: 600; }
  .run-muted { font-size: var(--text-xs); color: var(--color-text-muted); margin: 0; line-height: 1.5; }
  .run-error { color: var(--color-danger, #b44); margin: 0; overflow-wrap: anywhere; }
  .run-fields { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 9px; }
  .run-fields label { display: grid; gap: 5px; min-width: 0; font-size: var(--text-sm); font-weight: 600; }
  .run-fields input { min-width: 0; width: 100%; }
  .run-actions { display: flex; flex-wrap: wrap; align-items: center; gap: 8px; }
  .request-preview { min-width: 0; font-size: var(--text-sm); }
  .request-preview summary { cursor: pointer; font-weight: 600; }
  .request-preview ul { margin: 8px 0; padding-inline-start: 20px; font-size: var(--text-xs); overflow-wrap: anywhere; }
  .request-preview pre { max-height: 260px; overflow: auto; white-space: pre-wrap; overflow-wrap: anywhere; font: inherit; font-size: var(--text-xs); }
  @media (max-width: 560px) { .run-fields { grid-template-columns: 1fr; } }
</style>