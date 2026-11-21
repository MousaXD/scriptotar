import { describe, expect, it } from 'vitest';
import type { StudyResearchReport, StudyResearchRun, StudyResearchSource } from './agent';
import { matchesResearchReportScope } from './researchReportGuard';

const projectId = 'project';
const studyId = 'study';
const source: StudyResearchSource = {
  id: 'attached-one', originId: 'transcript-one', kind: 'transcript',
  locator: 'source://one', collectedAt: '2026-11-21T00:00:00Z', available: true
};
const run = {
  state: 'completed',
  plan: { project_id: projectId, study_id: studyId,
    steps: [{ input: { tool: 'analyze_evidence', source_ids: [source.id] } }] }
} as StudyResearchRun;
const report: StudyResearchReport = {
  project_id: projectId, study_id: studyId, analyzed_at: null,
  source_text_is_untrusted_data: true, feature_support: [], limitations: [],
  sources: [{
    study_source_id: source.id, reference_id: source.originId, kind: source.kind,
    locator: source.locator, collected_at: source.collectedAt, access: 'available',
    origin_revision: '1', metrics: null, limitations: [],
    observations: [{
      statement: 'Directly quoted observation', beat: null, feature: null,
      confidence: 'source_observation', limitations: [],
      evidence: { study_source_id: source.id, reference_id: source.originId,
        locator: 'source://one#t=1-2', transcript_id: source.originId,
        transcript_revision: '1', segment_id: 's1',
        start_ms: 1000, end_ms: 2000, excerpt: 'Direct excerpt' }
    }]
  }]
};

const valid = (value: StudyResearchReport, selected = run,
  attached: StudyResearchSource[] = [source]) =>
  matchesResearchReportScope(value, selected, attached, projectId, studyId);

describe('Study research report display scope', () => {
  it('accepts canonical selected-source evidence with a segment-specific locator', () => {
    expect(valid(report)).toBe(true);
  });

  it('fails closed on malformed IPC report and run shapes instead of crashing the review', () => {
    const missingRunPlan = { ...run, plan: null } as unknown as StudyResearchRun;
    expect(valid(report, missingRunPlan)).toBe(false);
    const missingSource = { ...report, sources: [null] } as unknown as StudyResearchReport;
    expect(valid(missingSource)).toBe(false);
    const malformedObservation = structuredClone(report);
    malformedObservation.sources[0].observations = [null as never];
    expect(valid(malformedObservation)).toBe(false);
  });

  it('rejects unrelated reports, duplicate source rows and unrequested sources', () => {
    expect(valid({ ...report, project_id: 'foreign' })).toBe(false);
    expect(valid({ ...report, sources: [report.sources[0], report.sources[0]] })).toBe(false);
    expect(valid({ ...report, sources: [{ ...report.sources[0], study_source_id: 'foreign' }] })).toBe(false);
  });

  it('rejects revoked, replaced and no-longer-attached sources at reopen', () => {
    expect(valid(report, run, [])).toBe(false);
    expect(valid(report, run, [{ ...source, available: false }])).toBe(false);
    expect(valid(report, run, [{ ...source, originId: 'different' }])).toBe(false);
    expect(valid(report, run, [{ ...source, collectedAt: 'changed' }])).toBe(false);
  });

  it('rejects duplicate or malformed attached source snapshots at the display boundary', () => {
    expect(valid(report, run, [source, { ...source }])).toBe(false);
    expect(valid(report, run, null as unknown as StudyResearchSource[])).toBe(false);
    expect(valid(report, run, [null as never])).toBe(false);
  });

  it('quarantines forged observation lineage and incoherent time ranges', () => {
    const changed = structuredClone(report);
    changed.sources[0].observations[0].evidence.reference_id = 'foreign-transcript';
    expect(valid(changed)).toBe(false);
    const clock = structuredClone(report);
    clock.sources[0].observations[0].evidence.start_ms = 3000;
    expect(valid(clock)).toBe(false);
  });

  it('retains partial transcript findings but quarantines revoked/denied source access', () => {
    expect(valid({ ...report, sources: [{ ...report.sources[0], access: 'partial' }] })).toBe(true);
    for (const access of ['revoked', 'access_denied', 'deleted', 'unsupported'] as const) {
      expect(valid({ ...report, sources: [{ ...report.sources[0], access }] })).toBe(false);
    }
  });

  it('requires a transcript observation to retain exact origin and revision', () => {
    const mismatchedOrigin = structuredClone(report);
    mismatchedOrigin.sources[0].observations[0].evidence.transcript_id = 'foreign';
    expect(valid(mismatchedOrigin)).toBe(false);
    const mismatchedRevision = structuredClone(report);
    mismatchedRevision.sources[0].observations[0].evidence.transcript_revision = 'stale';
    expect(valid(mismatchedRevision)).toBe(false);
    const missingRevision = structuredClone(report);
    missingRevision.sources[0].observations[0].evidence.transcript_revision = '';
    expect(valid(missingRevision)).toBe(false);
  });

  it('rejects fabricated creator-peer observations, invalid counters and feature bounds', () => {
    const value = structuredClone(report);
    value.sources[0].metrics = {
      views: 40, likes: null, comments: 2, observed_interactions_per_view: 0.05,
      creator_comparison: { status: 'unavailable', reason: 'insufficient_peers', eligible_peers: 0 }
    };
    expect(valid(value)).toBe(true);
    const fakeMetric = structuredClone(value);
    fakeMetric.sources[0].metrics!.views = -1;
    expect(valid(fakeMetric)).toBe(false);
    const forgedPeer = structuredClone(value);
    forgedPeer.sources[0].metrics!.creator_comparison = {
      status: 'observed', peer_count: 1, median_peer_views: 99,
      views_relative_to_peer_median: 1.1, interpretation: 'Unverified comparison'
    };
    expect(valid(forgedPeer)).toBe(false);
    const fakeSupport = structuredClone(value);
    fakeSupport.feature_support = [{
      feature: 'hook', distinct_selected_sources: 2,
      meaning: 'Cannot exceed the one selected source'
    }];
    expect(valid(fakeSupport)).toBe(false);
  });

  it('enforces the native 96 KB report budget and 160-codepoint transcript excerpts', () => {
    const oversized = structuredClone(report);
    oversized.limitations = ['x'.repeat(96_001)];
    expect(valid(oversized)).toBe(false);

    const overlongExcerpt = structuredClone(report);
    overlongExcerpt.sources[0].observations[0].evidence.excerpt = 'e'.repeat(161);
    expect(valid(overlongExcerpt)).toBe(false);

    // Rust bounds excerpts by Unicode scalar count, not UTF-16 code units;
    // 160 Arabic or emoji characters should remain permitted.
    const unicodeExcerpt = structuredClone(report);
    unicodeExcerpt.sources[0].observations[0].evidence.excerpt = '😀'.repeat(160);
    expect(valid(unicodeExcerpt)).toBe(true);

    const tooManyObservations = structuredClone(report);
    tooManyObservations.sources[0].observations = Array.from({ length: 49 },
      () => structuredClone(report.sources[0].observations[0]));
    expect(valid(tooManyObservations)).toBe(false);
  });

  it('requires a completed authorized run with its exact saved selection', () => {
    expect(valid(report, { ...run, state: 'cancelled' })).toBe(false);
    expect(valid(report, { ...run, plan: { ...run.plan,
      steps: [{ ...run.plan.steps[0], input: { tool: 'analyze_evidence',
        source_ids: [source.id, 'other'] } }] } })).toBe(false);
  });
});
