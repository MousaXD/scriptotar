import type { StudyResearchReport, StudyResearchRun, StudyResearchSource } from './agent';

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
      !Array.isArray(report.sources) || report.sources.length !== requested.length) {
    return false;
  }

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
        !Array.isArray(finding.observations) || finding.observations.length > 2000 ||
        !Array.isArray(finding.limitations)) return false;

    for (const observation of finding.observations) {
      const evidence = observation.evidence;
      if (!evidence || evidence.study_source_id !== id ||
          evidence.reference_id !== finding.reference_id ||
          typeof evidence.locator !== 'string' || !evidence.locator ||
          typeof evidence.excerpt !== 'string' || !evidence.excerpt.trim() ||
          evidence.excerpt.length > 100_000 ||
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
