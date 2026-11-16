import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { afterEach, describe, expect, it } from 'vitest';
import { setLocale } from '../i18n';
import { createMockClient } from '../api/mockClient';
import StudioResearchReview from './StudioResearchReview.svelte';

afterEach(() => setLocale('en'));

const brief = {
  title: 'Local research', goal: 'Observe saved sources', audience: 'Creators',
  platform: 'YouTube', language: 'English', duration: '45', style: 'Clear'
};

async function fixture() {
  const api = createMockClient();
  const projectId = (await api.bootstrap()).activeProjectId;
  const study = await api.createStudy(projectId, brief);
  await api.saveStudy(projectId, study.id, study.revision, {
    sources: [{ kind: 'transcript', id: 'transcript-1' }, { kind: 'research', id: 'research-2' }]
  });
  return { api, projectId, studyId: study.id };
}

describe('Study research is a local, explicitly authorized workflow', () => {
  it('prepares, approves, reads and reopens the durable source-scoped report', async () => {
    const { api, projectId, studyId } = await fixture();
    const readCanonical = api.readStudyResearchReport.bind(api);
    api.readStudyResearchReport = async (project, study, artifact) => {
      const report = await readCanonical(project, study, artifact);
      const source = report.sources[0];
      source.observations = [{
        statement: 'Observed opening question', beat: null, feature: null,
        confidence: 'source_observation',
        evidence: {
          study_source_id: source.study_source_id,
          reference_id: source.reference_id,
          locator: source.locator,
          transcript_id: source.reference_id,
          transcript_revision: 'mock:unchanged',
          segment_id: 'segment-7', start_ms: 1250, end_ms: 3100,
          excerpt: 'What would you do first?'
        },
        limitations: ['Quoted text is not an independently verified fact']
      }];
      return report;
    };
    const view = render(StudioResearchReview, { props: { api, projectId, studyId } });
    const prepare = screen.getByRole('button', { name: 'Prepare research' });
    await waitFor(() => expect(prepare).toBeEnabled());
    expect(screen.getByText(/No AI provider or paid request/)).toBeInTheDocument();
    await fireEvent.click(prepare);
    const approve = await screen.findByRole('button', { name: 'Approve local analysis' });
    expect(await screen.findByText(/awaiting_approval/)).toBeInTheDocument();
    await fireEvent.click(approve);
    await waitFor(() => expect(screen.getByText(/completed/)).toBeInTheDocument());
    expect(screen.getAllByText(/not independently fact-checked/).length).toBe(2);
    expect(screen.getByText('Observed opening question')).toBeInTheDocument();
    expect(screen.getByText('What would you do first?')).toBeInTheDocument();
    expect(screen.getByText(/segment-7/)).toBeInTheDocument();
    expect(screen.getByText(/1\.25–3\.10s/)).toBeInTheDocument();
    const runs = await api.listStudyResearchRuns(projectId, studyId);
    expect(runs).toHaveLength(1);
    expect(runs[0].plan.steps[0].input.tool).toBe('analyze_evidence');
    const artifactId = runs[0].steps[0].output?.artifact_id;
    expect(artifactId).toBeTruthy();
    view.unmount();
    render(StudioResearchReview, { props: { api, projectId, studyId } });
    expect((await screen.findAllByText(/not independently fact-checked/)).length).toBe(2);
    expect((await api.readStudyResearchReport(projectId, studyId, artifactId!)).sources).toHaveLength(2);
  });

  it('denies an approval after Study source revocation and does not show a report', async () => {
    const { api, projectId, studyId } = await fixture();
    render(StudioResearchReview, { props: { api, projectId, studyId } });
    await waitFor(() => expect(screen.getByRole('button', { name: 'Prepare research' })).toBeEnabled());
    await fireEvent.click(screen.getByRole('button', { name: 'Prepare research' }));
    const approve = await screen.findByRole('button', { name: 'Approve local analysis' });
    const study = await api.getStudy(projectId, studyId);
    await api.saveStudy(projectId, studyId, study.revision, { sources: [] });
    await fireEvent.click(approve);
    expect(await screen.findByRole('alert')).toHaveTextContent(/changed/i);
    expect(screen.queryByLabelText('Study research report')).not.toBeInTheDocument();
    expect((await api.listStudyResearchRuns(projectId, studyId))[0].steps[0].output).toBeNull();
  });

  it('shows Arabic consent and privacy labels', async () => {
    setLocale('ar');
    const { api, projectId, studyId } = await fixture();
    render(StudioResearchReview, { props: { api, projectId, studyId } });
    expect(await screen.findByText('بحث الدراسة')).toBeInTheDocument();
    expect(screen.getByText(/دون طلب مدفوع/)).toBeInTheDocument();
    expect(await screen.findByRole('button', { name: 'تحضير البحث' })).toBeInTheDocument();
  });
});
