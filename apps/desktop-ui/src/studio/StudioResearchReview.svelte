<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { locale } from '../i18n';
  import type { ScriptotarApi } from '../api/client';
  import type { StudyResearchReport, StudyResearchRun, StudyResearchSource } from './agent';
  import { matchesResearchReportScope } from './researchReportGuard';

  export let api: ScriptotarApi;
  export let projectId: string;
  export let studyId: string;
  /** The displayed editor has changes not yet authenticated by the Study host. */
  export let pendingEdits = false;

  const labels = {
    en: {
      title: 'Study research', reportLabel: 'Study research report', privacy: 'Local analysis of saved Study sources. No AI provider or paid request.',
      refresh: 'Refresh', prepare: 'Prepare research', approve: 'Approve local analysis',
      deny: 'Deny', cancel: 'Cancel', recover: 'Reconcile saved run',
      noSources: 'Attach and save a research item or transcript to analyze it.',
      selected: 'selected', empty: 'No saved research report yet.',
      observations: 'creative observations · not independently fact-checked',
      limits: 'Limitations', unavailable: 'The saved report cannot be reopened because its source or authorization changed.',
      state: 'Saved run state', error: 'Research operation unavailable', choose: 'Choose at least one saved source.',
      limit: 'At most eight sources per research run.',
      loading: 'Loading saved research…', transcript: 'Transcript', research: 'Research',
      approvalScope: 'Approve these exact sources', sourceCount: 'sources',
      missingScope: 'A selected source is no longer available. Deny this run and prepare a new one.',
      saveBeforeApproval: 'Save Study changes before preparing or approving local research.',
      provenance: 'Source provenance', partial: 'Partial transcript · review limitations',
      sourceId: 'Study source ID', originId: 'Origin ID', revision: 'Snapshot revision',
      collected: 'Collected at', noRevision: 'Not recorded',
      metrics: 'Observed platform counters', views: 'Views', likes: 'Likes',
      comments: 'Comments', interactions: 'Observed interactions per view',
      relativeViews: 'Views vs same-creator peer median', peers: 'eligible peers',
      noPeerComparison: 'No supported same-creator peer comparison',
      features: 'Transcript features across selected sources',
      selectedSources: 'selected sources', transcriptCaveat: 'Transcript hypotheses, not proof of audience retention.',
      states: { planned: 'Planned', awaiting_approval: 'Awaiting approval', running: 'Running',
        cancelling: 'Cancelling', paused: 'Paused', needs_input: 'Needs input',
        blocked: 'Blocked', completed: 'Completed', failed: 'Failed', cancelled: 'Cancelled' }
    },
    ar: {
      title: 'بحث الدراسة', reportLabel: 'تقرير بحث الدراسة', privacy: 'تحليل محلي لمصادر الدراسة المحفوظة. دون طلب مدفوع أو اتصال بمزود الذكاء الاصطناعي.',
      refresh: 'تحديث', prepare: 'تحضير البحث', approve: 'الموافقة على التحليل المحلي',
      deny: 'رفض', cancel: 'إلغاء', recover: 'تسوية التشغيل المحفوظ',
      noSources: 'أرفق واحفظ مصدراً بحثياً أو نصاً لتتمكن من تحليله.',
      selected: 'محدد', empty: 'لا يوجد تقرير بحثي محفوظ بعد.',
      observations: 'ملاحظات إبداعية · ليست حقائق متحققاً منها بشكل مستقل',
      limits: 'القيود', unavailable: 'تعذر فتح التقرير المحفوظ بسبب تغيّر المصدر أو الصلاحية.',
      state: 'حالة التشغيل المحفوظ', error: 'عملية البحث غير متاحة',
      choose: 'اختر مصدراً محفوظاً واحداً على الأقل.', limit: 'الحد الأقصى ثمانية مصادر لكل تشغيل.',
      loading: 'جارٍ تحميل البحث المحفوظ…', transcript: 'نص مفرغ', research: 'بحث',
      approvalScope: 'الموافقة على هذه المصادر تحديدًا', sourceCount: 'مصادر',
      missingScope: 'أحد المصادر المختارة لم يعد متاحًا. ارفض التشغيل وأعد تحضير طلب جديد.',
      saveBeforeApproval: 'احفظ تغييرات الدراسة قبل تحضير البحث المحلي أو الموافقة عليه.',
      provenance: 'مصدر الأدلة', partial: 'نص مفرغ جزئي · راجع القيود',
      sourceId: 'معرّف مصدر الدراسة', originId: 'معرّف المصدر الأصلي', revision: 'إصدار اللقطة',
      collected: 'تاريخ الجمع', noRevision: 'غير مسجل',
      metrics: 'إحصاءات المنصة المرصودة', views: 'المشاهدات', likes: 'الإعجابات',
      comments: 'التعليقات', interactions: 'التفاعلات المرصودة لكل مشاهدة',
      relativeViews: 'المشاهدات مقارنة بوسيط مقاطع المنشئ المماثلة', peers: 'مقاطع مقارنة مؤهلة',
      noPeerComparison: 'لا تتوفر مقارنة مدعومة لمقاطع المنشئ',
      features: 'سمات النصوص عبر المصادر المختارة',
      selectedSources: 'مصادر مختارة', transcriptCaveat: 'فرضيات مستمدة من النصوص، وليست دليلاً على الاحتفاظ بالمشاهدين.',
      states: { planned: 'مخطط', awaiting_approval: 'بانتظار الموافقة', running: 'جارٍ التنفيذ',
        cancelling: 'جارٍ الإلغاء', paused: 'متوقف مؤقتًا', needs_input: 'يتطلب إدخالًا',
        blocked: 'محظور', completed: 'مكتمل', failed: 'فشل', cancelled: 'ملغى' }
    }
  } as const;

  const featureNames: Record<string, { en: string; ar: string }> = {
    hook: { en: 'Opening hook', ar: 'الافتتاح الجاذب' },
    opening_promise: { en: 'Opening promise', ar: 'وعد المقدمة' },
    narrative_setup: { en: 'Narrative setup', ar: 'تمهيد السرد' },
    narrative_turn: { en: 'Narrative turn', ar: 'تحول السرد' },
    spoken_pacing: { en: 'Spoken pacing', ar: 'إيقاع الكلام' },
    payoff: { en: 'Payoff', ar: 'الخاتمة أو النتيجة' },
    call_to_action: { en: 'Call to action', ar: 'دعوة لاتخاذ إجراء' },
    rhetorical_device: { en: 'Rhetorical device', ar: 'أسلوب بلاغي' }
  };
  function featureName(id: string): string {
    return featureNames[id]?.[$locale === 'ar' ? 'ar' : 'en'] ?? id.replace(/_/g, ' ');
  }
  function observedCount(count: number): string {
    return new Intl.NumberFormat($locale === 'ar' ? 'ar-AE' : 'en-US').format(count);
  }

  $: t = labels[$locale === 'ar' ? 'ar' : 'en'];
  let sources: StudyResearchSource[] = [];
  let selectedIds: string[] = [];
  let activeRun: StudyResearchRun | null = null;
  let report: StudyResearchReport | null = null;
  let busy = false;
  let loading = false;
  let errorText = '';
  let epoch = 0;
  let alive = true;

  onDestroy(() => { alive = false; epoch += 1; });
  onMount(() => { void refresh(); });

  function scoped(run: StudyResearchRun): boolean {
    return run.plan.project_id === projectId && run.plan.study_id === studyId &&
      run.plan.steps.length === 1 && run.plan.steps[0].input.tool === 'analyze_evidence' &&
      run.plan.steps[0].requires_approval === true &&
      run.plan.steps[0].estimate?.tokens === 0 &&
      run.plan.steps[0].estimate?.cost_microusd === 0;
  }

  function hasSelectedSources(): boolean {
    return !pendingEdits && selectedIds.length >= 1 && selectedIds.length <= 8 &&
      new Set(selectedIds).size === selectedIds.length &&
      selectedIds.every((id) => sources.some((source) => source.id === id && source.available));
  }

  function hasCurrentRunSources(run: StudyResearchRun): boolean {
    const ids = run.plan.steps[0]?.input.source_ids;
    return !pendingEdits && Array.isArray(ids) && ids.length >= 1 && ids.length <= 8 &&
      new Set(ids).size === ids.length &&
      ids.every((id) => sources.some((source) => source.id === id && source.available));
  }

  function showError(cause: unknown): string {
    return cause instanceof Error ? cause.message : t.error;
  }

  async function openReport(run: StudyResearchRun, requestEpoch: number): Promise<void> {
    const artifact = run.steps[0]?.output?.artifact_id;
    if (!artifact || !scoped(run) || run.state !== 'completed') return;
    try {
      const saved = await api.readStudyResearchReport(projectId, studyId, artifact);
      if (!alive || requestEpoch !== epoch) return;
      if (!matchesResearchReportScope(saved, run, sources, projectId, studyId)) {
        throw new Error(t.unavailable);
      }
      report = saved;
    } catch (cause) {
      if (!alive || requestEpoch !== epoch) return;
      report = null;
      errorText = showError(cause);
    }
  }

  async function refresh(): Promise<void> {
    const requestEpoch = ++epoch;
    loading = true;
    errorText = '';
    report = null;
    // A failed refresh must never leave an old approval panel actionable.
    activeRun = null;
    sources = [];
    try {
      const [attached, runs] = await Promise.all([
        api.listStudyResearchSources(projectId, studyId),
        api.listStudyResearchRuns(projectId, studyId)
      ]);
      if (!alive || requestEpoch !== epoch) return;
      sources = attached;
      const available = new Set(attached.filter((source) => source.available).map((source) => source.id));
      const prior = selectedIds.filter((id) => available.has(id));
      selectedIds = (prior.length ? prior : attached.filter((source) => source.available)
        .slice(0, 8).map((source) => source.id));
      activeRun = runs.find(scoped) ?? null;
      if (activeRun?.state === 'awaiting_approval') {
        // The pending approval is for the saved immutable run selection, not a
        // freshly auto-selected set of sources after reopening Studio.
        selectedIds = [...activeRun.plan.steps[0].input.source_ids];
      }
      if (activeRun?.state === 'completed') await openReport(activeRun, requestEpoch);
    } catch (cause) {
      if (alive && requestEpoch === epoch) errorText = showError(cause);
    } finally {
      if (alive && requestEpoch === epoch) loading = false;
    }
  }

  function toggle(id: string): void {
    if (busy || loading || activeRun?.state === 'awaiting_approval') return;
    if (selectedIds.includes(id)) {
      selectedIds = selectedIds.filter((value) => value !== id);
    } else if (selectedIds.length < 8) {
      selectedIds = [...selectedIds, id];
    } else {
      errorText = t.limit;
      return;
    }
    errorText = '';
  }

  async function prepare(): Promise<void> {
    if (busy || loading || pendingEdits || activeRun?.state === 'awaiting_approval') return;
    if (!hasSelectedSources()) { errorText = t.choose; return; }
    const requested = [...selectedIds];
    const requestEpoch = ++epoch;
    busy = true;
    report = null;
    errorText = '';
    try {
      const run = await api.startStudyResearch(projectId, studyId, requested);
      if (!alive || requestEpoch !== epoch) return;
      const returned = run.plan.steps[0]?.input.source_ids;
      if (!scoped(run) || run.state !== 'awaiting_approval' ||
          !Array.isArray(returned) || returned.length !== requested.length ||
          new Set(returned).size !== requested.length ||
          requested.some((id) => !returned.includes(id))) throw new Error(t.unavailable);
      activeRun = run;
    } catch (cause) {
      if (alive && requestEpoch === epoch) errorText = showError(cause);
    } finally {
      if (alive && requestEpoch === epoch) busy = false;
    }
  }

  async function decide(action: 'approve' | 'deny' | 'cancel' | 'recover'): Promise<void> {
    if (!activeRun || busy || loading || !scoped(activeRun) ||
        ((action === 'approve' || action === 'recover') && pendingEdits) ||
        (action === 'approve' && !hasCurrentRunSources(activeRun))) return;
    const requestEpoch = ++epoch;
    busy = true;
    errorText = '';
    const runId = activeRun.plan.id;
    const approvedSources = [...activeRun.plan.steps[0].input.source_ids];
    try {
      const run = await api.decideStudyResearch(projectId, studyId, runId, action);
      if (!alive || requestEpoch !== epoch) return;
      if (!scoped(run) || run.plan.id !== runId ||
          run.plan.steps[0].input.source_ids.length !== approvedSources.length ||
          approvedSources.some((id) => !run.plan.steps[0].input.source_ids.includes(id))) {
        throw new Error(t.unavailable);
      }
      activeRun = run;
      report = null;
      if (run.state === 'completed') await openReport(run, requestEpoch);
    } catch (cause) {
      if (alive && requestEpoch === epoch) errorText = showError(cause);
    } finally {
      if (alive && requestEpoch === epoch) busy = false;
    }
  }
</script>

<section class="panel study-research-review" aria-label={t.title} aria-busy={loading || busy} dir={$locale === 'ar' ? 'rtl' : 'ltr'}>
  <div class="review-heading">
    <h2>{t.title}</h2>
    <button type="button" class="text-button" disabled={busy || loading}
      on:click={() => void refresh()}>{t.refresh}</button>
  </div>
  <p class="muted">{t.privacy}</p>
  {#if pendingEdits}<p class="muted" role="status">{t.saveBeforeApproval}</p>{/if}
  {#if loading}
    <div class="research-loading" role="status" aria-label={t.loading}>
      <span class="research-loading-line"></span><span class="research-loading-line short"></span>
      <span class="sr-only">{t.loading}</span>
    </div>
  {/if}
  {#if sources.length}
    <div class="research-attachments" role="group" aria-label={t.title}>
      {#each sources as source (source.id)}
        <button type="button" class="research-source" class:selected={selectedIds.includes(source.id)}
          aria-pressed={selectedIds.includes(source.id)} disabled={!source.available || busy || loading || activeRun?.state === 'awaiting_approval'}
          on:click={() => toggle(source.id)} dir="auto">
          <span>{source.kind === 'transcript' ? t.transcript : t.research} · {source.locator}</span>
          <small>{selectedIds.includes(source.id) ? t.selected : source.available ? '' : t.unavailable}</small>
        </button>
      {/each}
    </div>
  {:else if !loading}
    <p class="muted">{t.noSources}</p>
  {/if}
  <button type="button" class="button secondary" disabled={busy || loading || pendingEdits || !hasSelectedSources() || activeRun?.state === 'awaiting_approval'}
    on:click={() => void prepare()}>{t.prepare}</button>
  {#if activeRun}
    <p class="muted" role="status">{t.state}: {t.states[activeRun.state]}</p>
    <div class="review-actions">
      {#if activeRun.state === 'awaiting_approval'}
        <details class="research-approval-scope">
          <summary>{t.approvalScope} · {activeRun.plan.steps[0].input.source_ids.length} {t.sourceCount}</summary>
          <ul>
            {#each activeRun.plan.steps[0].input.source_ids as sourceId (sourceId)}
              <li>
                <span dir="auto">{sources.find((source) => source.id === sourceId)?.locator ?? sourceId}</span>
                <code dir="ltr">{sourceId}</code>
              </li>
            {/each}
          </ul>
        </details>
        {#if !pendingEdits && !hasCurrentRunSources(activeRun)}
          <p class="studio-error" role="alert">{t.missingScope}</p>
        {/if}
        <button type="button" class="button primary" disabled={busy || loading || pendingEdits || !hasCurrentRunSources(activeRun)}
          on:click={() => void decide('approve')}>{t.approve}</button>
        <button type="button" class="button secondary" disabled={busy || loading}
          on:click={() => void decide('deny')}>{t.deny}</button>
      {:else if activeRun.state === 'blocked'}
        <button type="button" class="button secondary" disabled={busy || loading || pendingEdits}
          on:click={() => void decide('recover')}>{t.recover}</button>
      {:else if ['planned', 'running', 'paused', 'needs_input'].includes(activeRun.state)}
        <button type="button" class="button secondary" disabled={busy || loading}
          on:click={() => void decide('cancel')}>{t.cancel}</button>
      {/if}
    </div>
  {/if}
  {#if errorText}<p role="alert" class="studio-error">{errorText}</p>{/if}
  {#if report && !pendingEdits}
    <div class="research-result" role="region" aria-label={t.reportLabel}>
      {#if report.feature_support.length}
        <details class="research-feature-support">
          <summary>{t.features}</summary>
          <p class="muted">{t.transcriptCaveat}</p>
          <ul>
            {#each report.feature_support as feature, index (index)}
              <li><strong>{featureName(feature.feature)}</strong> ·
                {observedCount(feature.distinct_selected_sources)} {t.selectedSources}</li>
            {/each}
          </ul>
        </details>
      {/if}
      {#each report.sources as item (item.study_source_id)}
        <div class="result-source">
          <strong dir="auto">{item.kind === 'transcript' ? t.transcript : t.research} · {item.locator}</strong>
          {#if item.access === 'partial'}<p class="muted" role="status">{t.partial}</p>{/if}
          <p>{item.observations.length} {t.observations}</p>
          <details class="research-provenance">
            <summary>{t.provenance}</summary>
            <dl>
              <dt>{t.sourceId}</dt><dd><code dir="ltr">{item.study_source_id}</code></dd>
              <dt>{t.originId}</dt><dd><code dir="ltr">{item.reference_id}</code></dd>
              <dt>{t.revision}</dt><dd dir="auto">{item.origin_revision ?? t.noRevision}</dd>
              <dt>{t.collected}</dt><dd dir="auto">{item.collected_at}</dd>
            </dl>
          </details>
          {#if item.metrics}
            <details class="research-metrics">
              <summary>{t.metrics}</summary>
              <dl>
                {#if item.metrics.views !== null}
                  <dt>{t.views}</dt><dd>{observedCount(item.metrics.views)}</dd>
                {/if}
                {#if item.metrics.likes !== null}
                  <dt>{t.likes}</dt><dd>{observedCount(item.metrics.likes)}</dd>
                {/if}
                {#if item.metrics.comments !== null}
                  <dt>{t.comments}</dt><dd>{observedCount(item.metrics.comments)}</dd>
                {/if}
                {#if item.metrics.observed_interactions_per_view !== null}
                  <dt>{t.interactions}</dt>
                  <dd>{(item.metrics.observed_interactions_per_view * 100).toFixed(2)}%</dd>
                {/if}
                {#if item.metrics.creator_comparison.status === 'observed'}
                  <dt>{t.relativeViews}</dt>
                  <dd>{item.metrics.creator_comparison.views_relative_to_peer_median.toFixed(2)}× ·
                    {observedCount(item.metrics.creator_comparison.peer_count)} {t.peers}</dd>
                {:else}
                  <dt>{t.relativeViews}</dt><dd>{t.noPeerComparison}</dd>
                {/if}
              </dl>
            </details>
          {/if}
          {#if item.observations.length}
            <ul class="observations">
              {#each item.observations as observation, i (i)}
                <li>
                  <strong dir="auto">{observation.statement}</strong>
                  <blockquote dir="auto">{observation.evidence.excerpt}</blockquote>
                  <small dir="auto">
                    {observation.evidence.locator}
                    {#if observation.evidence.segment_id}
                      · {observation.evidence.segment_id}
                    {/if}
                    {#if observation.evidence.start_ms !== null && observation.evidence.end_ms !== null}
                      · {(observation.evidence.start_ms / 1000).toFixed(2)}–{(observation.evidence.end_ms / 1000).toFixed(2)}s
                    {/if}
                  </small>
                  {#if observation.limitations.length}
                    <ul>{#each observation.limitations as limitation, j (j)}
                      <li dir="auto">{limitation}</li>
                    {/each}</ul>
                  {/if}
                </li>
              {/each}
            </ul>
          {/if}
          {#if item.limitations.length}
            <details><summary>{t.limits}</summary>
              <ul>{#each item.limitations as limitation, i (i)}
                <li dir="auto">{limitation}</li>
              {/each}</ul>
            </details>
          {/if}
        </div>
      {/each}
      {#if report.limitations.length}
        <details><summary>{t.limits}</summary>
          <ul>{#each report.limitations as limitation, i (i)}
            <li dir="auto">{limitation}</li>
          {/each}</ul>
        </details>
      {/if}
    </div>
  {:else if !activeRun && !loading}
    <p class="muted">{t.empty}</p>
  {/if}
</section>

<style>
  h2 { margin: 0; font-size: var(--text-md); }
  .review-heading, .review-actions { display: flex; gap: 8px; align-items: center; justify-content: space-between; flex-wrap: wrap; }
  .muted { font-size: var(--text-xs); color: var(--color-text-muted); line-height: 1.5; }
  .research-loading { display: grid; gap: 9px; padding-block: 12px; }
  .research-loading-line { display: block; width: 100%; height: 12px; border-radius: 6px; background: var(--color-surface-raised); }
  .research-loading-line.short { width: 65%; }
  .research-attachments { display: grid; gap: 6px; margin-block: 10px; }
  .research-source { display: flex; flex-direction: column; gap: 3px; align-items: start; text-align: start;
    border: 1px solid var(--color-border); border-radius: var(--radius-sm); background: transparent;
    color: var(--color-text); padding: 8px; overflow-wrap: anywhere; font-size: var(--text-xs); }
  .research-source.selected { border-color: var(--color-border-strong); background: var(--color-surface-raised); }
  .research-source:disabled { opacity: .55; cursor: not-allowed; }
  .research-source small, .result-source p { font-size: var(--text-xs); color: var(--color-text-muted); }
  .result-source { border-top: 1px solid var(--color-border); padding-block: 9px; overflow-wrap: anywhere; }
  .research-result { margin-block: 12px; }
  .research-provenance, .research-metrics, .research-feature-support { margin-block: 8px; }
  .research-provenance dl, .research-metrics dl { display: grid; grid-template-columns: minmax(0, auto) minmax(0, 1fr); gap: 5px 12px; padding-inline-start: 8px; }
  .research-provenance dt, .research-metrics dt { color: var(--color-text-muted); }
  .research-provenance dd, .research-metrics dd { margin: 0; overflow-wrap: anywhere; min-width: 0; }
  .research-provenance code { font-size: var(--text-xs); white-space: normal; overflow-wrap: anywhere; }
  .research-feature-support ul { display: grid; gap: 6px; padding-inline-start: 20px; }
  .research-approval-scope { width: 100%; font-size: var(--text-xs); }
  .research-approval-scope ul { margin: 8px 0; padding-inline-start: 20px; display: grid; gap: 6px; }
  .research-approval-scope li { overflow-wrap: anywhere; display: grid; gap: 3px; }
  .research-approval-scope code { font-size: var(--text-xs); color: var(--color-text-muted); overflow-wrap: anywhere; }
  .observations { display: grid; gap: 8px; padding-inline-start: 20px; }
  .observations blockquote { margin: 5px 0; padding-inline-start: 10px; border-inline-start: 2px solid var(--color-border-strong); white-space: pre-wrap; overflow-wrap: anywhere; }
  .observations li { font-size: var(--text-xs); }
  .observations small { display: block; overflow-wrap: anywhere; color: var(--color-text-muted); }
  details { font-size: var(--text-xs); }
  .studio-error { color: var(--color-danger, #b44); font-size: var(--text-xs); overflow-wrap: anywhere; }
</style>
