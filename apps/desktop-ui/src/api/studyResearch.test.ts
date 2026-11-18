import { describe, expect, it } from 'vitest';
import { createMockClient } from './mockClient';

const brief = {
  title: 'Grounded creative study', goal: 'Compare formats', audience: 'Creators',
  platform: 'YouTube', language: 'English', duration: '45', style: 'Clear'
};

async function withAttachedResearch() {
  const api = createMockClient();
  const projectId = (await api.bootstrap()).activeProjectId;
  const study = await api.createStudy(projectId, brief);
  await api.saveStudy(projectId, study.id, study.revision, {
    sources: [ { kind: 'research', id: 'saved-research-1' }, { kind: 'transcript', id: 'saved-transcript-2' } ]
  });
  const sources = await api.listStudyResearchSources(projectId, study.id);
  return { api, projectId, studyId: study.id, sources };
}

describe('scoped AnalyzeEvidence mock/IPC contract', () => {
  it('returns canonical attachment IDs, requires approval and reopens only an applied report', async () => {
    const { api, projectId, studyId, sources } = await withAttachedResearch();
    expect(sources).toHaveLength(2);
    expect(sources[0].id).not.toEqual(sources[0].originId);
    await expect(api.startStudyResearch(projectId, studyId, [sources[0].originId]))
      .rejects.toThrow(/not attached/);
    const prepared = await api.startStudyResearch(projectId, studyId, sources.map((source) => source.id));
    expect(prepared.state).toBe('awaiting_approval');
    expect(prepared.plan.steps[0].input.tool).toBe('analyze_evidence');
    expect(prepared.plan.steps[0].estimate.cost_microusd).toBe(0);
    expect((await api.listStudyResearchRuns(projectId, studyId))).toHaveLength(1);
    expect(prepared.steps[0].output).toBeNull();
    await expect(api.decideStudyResearch('foreign-project', studyId, prepared.plan.id, 'approve'))
      .rejects.toThrow();
    const completed = await api.decideStudyResearch(projectId, studyId, prepared.plan.id, 'approve');
    expect(completed.state).toBe('completed');
    const artifactId = completed.steps[0].output?.artifact_id;
    expect(artifactId).toBeTruthy();
    const report = await api.readStudyResearchReport(projectId, studyId, artifactId!);
    expect(report.sources.map((item) => item.study_source_id)).toEqual(sources.map((item) => item.id));
    expect(report.source_text_is_untrusted_data).toBe(true);
    expect(report.sources.every((source) => source.observations.length === 0)).toBe(true);
    await expect(api.decideStudyResearch(projectId, studyId, prepared.plan.id, 'approve'))
      .rejects.toThrow();
    await expect(api.readStudyResearchReport(projectId, studyId, 'missing-artifact'))
      .rejects.toThrow();
    await expect(api.readStudyResearchReport('foreign-project', studyId, artifactId!))
      .rejects.toThrow();
  });

  it('denial, duplicate selection and stale Study revision never create readable artifacts', async () => {
    const { api, projectId, studyId, sources } = await withAttachedResearch();
    await expect(api.startStudyResearch(projectId, studyId, [sources[0].id, sources[0].id]))
      .rejects.toThrow(/distinct/);
    const denied = await api.startStudyResearch(projectId, studyId, [sources[0].id]);
    const finishedDenial = await api.decideStudyResearch(projectId, studyId, denied.plan.id, 'deny');
    expect(finishedDenial.state).toBe('cancelled');
    expect(finishedDenial.steps[0].output).toBeNull();
    const stale = await api.startStudyResearch(projectId, studyId, [sources[0].id]);
    const study = await api.getStudy(projectId, studyId);
    await api.saveStudy(projectId, studyId, study.revision, { sources: [] });
    await expect(api.decideStudyResearch(projectId, studyId, stale.plan.id, 'approve'))
      .rejects.toThrow(/changed/);
  });
});
