import type { StudyResearchReport, StudyResearchRun, StudyResearchSource } from './agent';

// Keep the display boundary no more permissive than the canonical Rust
// study_research.rs report admission (MAX_REPORT_BYTES = 96_000).
const MAX_DISPLAY_REPORT_BYTES = 96_000;
const MAX_DISPLAY_OBSERVATIONS_PER_SOURCE = 48;
const MAX_DISPLAY_EXCERPT_CODEPOINTS = 160;

/**
 * Defense in depth at the Studio display boundary. The Rust Study gateway is
 * authoritative for access and freshness; this check stops a stale, malformed
 * or wrong-run IPC response from being rendered as source-backed evidence.
 *
 * The run contains canonical study_sources IDs, not transcript or video IDs.
 */
export function matchesResearchReportScope(
  report: StudyResearchReport,
  run: StudyResearchRun,
  attached: StudyResearchSource[],
  projectId: string,
  studyId: string
): boolean {
  const step = run.plan.steps[0];
  const requested = step?.input.source_ids;
  if (run.state !== 'completed' || run.plan.project_id !== projectId ||
      run.plan.study_id !== studyId || run.plan.steps.length !== 1 ||
      step?.input.tool !== 'analyze_evidence' ||
      !Array.isArray(requested) || requested.length < 1 || requested.length > 8 ||
      requested.some((id) => typeof id !== 'string' || !id) ||
      new Set(requested).size !== requested.length ||
      report.project_id !== projectId || report.study_id !== studyId ||
      report.source_text_is_untrusted_data !== true ||
      !Array.isArray(report.sources) || report.sources.length !== requested.length ||
      !Array.isArray(report.feature_support) || report.feature_support.length > 8 ||
      !Array.isArray(report.limitations)) {
    return false;
  }

  // A malformed IPC reply must never allocate an unbounded evidence DOM.
  // Rust checks the serialized artifact on disk; this also limits replies
  // supplied by a stale/mock/incorrect frontend adapter before rendering.
  try {
    const serialized = JSON.stringify(report);
    if (!serialized || new TextEncoder().encode(serialized).byteLength > MAX_DISPLAY_REPORT_BYTES) {
      return false;
    }
  } catch {
    return false;
  }

  // These values can be rendered as observed facts only if their shape,
  // finite numbers and selected-source bounds survive an untrusted IPC reply.
  if (report.feature_support.some((feature) =>
    !feature || typeof feature.feature !== 'string' ||
    !feature.feature || typeof feature.meaning !== 'string' ||
    feature.meaning.length > 2000 ||
    !Number.isInteger(feature.distinct_selected_sources) ||
    feature.distinct_selected_sources < 1 ||
    feature.distinct_selected_sources > requested.length)) return false;

  const plausibleCount = (value: number | null): boolean =>
    value === null || (typeof value === 'number' && Number.isFinite(value) && value >= 0);

  const allowed = new Set(requested);
  const seen = new Set<string>();
  for (const finding of report.sources) {
    const id = finding.study_source_id;
    if (!allowed.has(id) || seen.has(id)) return false;
    seen.add(id);
    const source = attached.find((item) => item.id === id && item.available);
    // A detached source, changed origin or replaced collected snapshot is not
    // proof for the original research run even if the human label looks alike.
    if (!source || source.originId !== finding.reference_id ||
        source.kind !== finding.kind || source.locator !== finding.locator ||
        source.collectedAt !== finding.collected_at ||
        // A partial transcript remains useful with its limitations; revoked,
        // deleted, unsupported or denied material never becomes usable evidence.
        !['available', 'partial'].includes(finding.access) ||
        !Array.isArray(finding.observations) ||
        finding.observations.length > MAX_DISPLAY_OBSERVATIONS_PER_SOURCE ||
        !Array.isArray(finding.limitations)) return false;

    const metrics = finding.metrics;
    if (metrics) {
      if (!plausibleCount(metrics.views) || !plausibleCount(metrics.likes) ||
          !plausibleCount(metrics.comments) ||
          !plausibleCount(metrics.observed_interactions_per_view)) return false;
      const comparison = metrics.creator_comparison;
      if (!comparison || !['observed', 'unavailable'].includes(comparison.status)) return false;
      if (comparison.status === 'observed' &&
          (!Number.isInteger(comparison.peer_count) || comparison.peer_count < 3 ||
            !Number.isFinite(comparison.median_peer_views) || comparison.median_peer_views < 0 ||
            !Number.isFinite(comparison.views_relative_to_peer_median) ||
            comparison.views_relative_to_peer_median < 0 ||
            typeof comparison.interpretation !== 'string')) return false;
      if (comparison.status === 'unavailable' &&
          (!Number.isInteger(comparison.eligible_peers) || comparison.eligible_peers < 0 ||
            typeof comparison.reason !== 'string')) return false;
    }

    for (const observation of finding.observations) {
      const evidence = observation.evidence;
      if (!evidence || evidence.study_source_id !== id ||
          evidence.reference_id !== finding.reference_id ||
          // Real Rust anchors carry the same canonical transcript origin and
          // revision as the selected source, not a different video's excerpt.
          (finding.kind === 'transcript' &&
            (evidence.transcript_id !== finding.reference_id ||
              typeof evidence.transcript_revision !== 'string' ||
              !evidence.transcript_revision.trim() ||
              (finding.origin_revision !== null &&
                evidence.transcript_revision !== finding.origin_revision))) ||
          typeof evidence.locator !== 'string' || !evidence.locator ||
          typeof evidence.excerpt !== 'string' || !evidence.excerpt.trim() ||
          Array.from(evidence.excerpt).length > MAX_DISPLAY_EXCERPT_CODEPOINTS ||
          typeof observation.statement !== 'string' || !observation.statement.trim() ||
          !Array.isArray(observation.limitations)) return false;
      // Temporal spans may be absent for metadata-only sources; when present
      // they must be coherent and cannot be mistaken for another source.
      const start = evidence.start_ms;
      const end = evidence.end_ms;
      if ((start === null) !== (end === null) ||
          (start !== null && (!Number.isFinite(start) || !Number.isFinite(end) ||
            start < 0 || end! < start))) return false;
    }
  }
  return seen.size === allowed.size;
}
