import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
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
    expect(await screen.findByText(/Awaiting approval/)).toBeInTheDocument();
    await fireEvent.click(approve);
    await waitFor(() => expect(screen.getByText(/Completed/)).toBeInTheDocument());
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
  it('quarantines a completed report with a mismatched observation source', async () => {
    const { api, projectId, studyId } = await fixture();
    const canonical = api.readStudyResearchReport.bind(api);
    api.readStudyResearchReport = async (project, study, artifact) => {
      const report = await canonical(project, study, artifact);
      const source = report.sources[0];
      source.observations = [{
        statement: 'Claim from a different source', beat: null, feature: null,
        confidence: 'source_observation', limitations: [],
        evidence: {
          study_source_id: source.study_source_id,
          reference_id: 'foreign-origin',
          locator: source.locator, transcript_id: 'foreign-origin',
          transcript_revision: '1', segment_id: 'segment-1',
          start_ms: 100, end_ms: 200, excerpt: 'Untrusted excerpt'
        }
      }];
      return report;
    };
    render(StudioResearchReview, { props: { api, projectId, studyId } });
    await waitFor(() => expect(screen.getByRole('button', { name: 'Prepare research' })).toBeEnabled());
    await fireEvent.click(screen.getByRole('button', { name: 'Prepare research' }));
    await fireEvent.click(await screen.findByRole('button', { name: 'Approve local analysis' }));
    expect(await screen.findByRole('alert')).toHaveTextContent(/saved report cannot be reopened/i);
    expect(screen.queryByText('Claim from a different source')).not.toBeInTheDocument();
    expect(screen.queryByLabelText('Study research report')).not.toBeInTheDocument();
  });

  it('rejects a prepare response whose canonical source selection was changed', async () => {
    const { api, projectId, studyId } = await fixture();
    const original = api.startStudyResearch.bind(api);
    api.startStudyResearch = async (project, study, ids) => {
      const run = await original(project, study, ids);
      run.plan.steps[0].input.source_ids = ['a-different-attached-source'];
      return run;
    };
    render(StudioResearchReview, { props: { api, projectId, studyId } });
    await waitFor(() => expect(screen.getByRole('button', { name: 'Prepare research' })).toBeEnabled());
    await fireEvent.click(screen.getByRole('button', { name: 'Prepare research' }));
    expect(await screen.findByRole('alert')).toHaveTextContent(/saved report cannot be reopened/i);
    expect(screen.queryByRole('button', { name: 'Approve local analysis' })).not.toBeInTheDocument();
  });

  it('removes stale approval controls if a source refresh fails', async () => {
    const { api, projectId, studyId } = await fixture();
    render(StudioResearchReview, { props: { api, projectId, studyId } });
    await waitFor(() => expect(screen.getByRole('button', { name: 'Prepare research' })).toBeEnabled());
    await fireEvent.click(screen.getByRole('button', { name: 'Prepare research' }));
    expect(await screen.findByRole('button', { name: 'Approve local analysis' })).toBeEnabled();
    api.listStudyResearchSources = async () => { throw new Error('Source listing unavailable'); };
    await fireEvent.click(screen.getByRole('button', { name: 'Refresh' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('Source listing unavailable');
    expect(screen.queryByRole('button', { name: 'Approve local analysis' })).not.toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Prepare research' })).toBeDisabled();
  });

  it('reopens the latest pending research run instead of an older completed run', async () => {
    const { api, projectId, studyId } = await fixture();
    const sources = await api.listStudyResearchSources(projectId, studyId);
    const first = await api.startStudyResearch(projectId, studyId, [sources[0].id]);
    await api.decideStudyResearch(projectId, studyId, first.plan.id, 'approve');
    const latest = await api.startStudyResearch(projectId, studyId, [sources[1].id]);
    const stored = await api.listStudyResearchRuns(projectId, studyId);
    expect(stored.map((run) => run.plan.id)).toEqual([latest.plan.id, first.plan.id]);

    render(StudioResearchReview, { props: { api, projectId, studyId } });
    expect(await screen.findByText(/Awaiting approval/)).toBeInTheDocument();
    const scope = await screen.findByText(/Approve these exact sources/);
    await fireEvent.click(scope);
    expect(screen.getByText(sources[1].id)).toBeInTheDocument();
    expect(screen.queryByText(sources[0].id)).not.toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Approve local analysis' })).toBeEnabled();
  });

  it('reopens an older research run updated after a newer pending run, matching SQLite', async () => {
    const { api, projectId, studyId } = await fixture();
    const sources = await api.listStudyResearchSources(projectId, studyId);
    const oldPending = await api.startStudyResearch(projectId, studyId, [sources[0].id]);
    const newPending = await api.startStudyResearch(projectId, studyId, [sources[1].id]);
    await api.decideStudyResearch(projectId, studyId, oldPending.plan.id, 'approve');

    const runs = await api.listStudyResearchRuns(projectId, studyId);
    expect(runs.map((run) => run.plan.id)).toEqual([oldPending.plan.id, newPending.plan.id]);
    render(StudioResearchReview, { props: { api, projectId, studyId } });
    expect(await screen.findByText(/Completed/)).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Approve local analysis' })).not.toBeInTheDocument();
  });

  it('refuses approval controls for a nonzero-cost backend research plan', async () => {
    const { api, projectId, studyId } = await fixture();
    const sources = await api.listStudyResearchSources(projectId, studyId);
    await api.startStudyResearch(projectId, studyId, [sources[0].id]);
    const previous = api.listStudyResearchRuns.bind(api);
    api.listStudyResearchRuns = async (project, study) => {
      const runs = await previous(project, study);
      runs[0].plan.steps[0].estimate.cost_microusd = 1;
      return runs;
    };
    render(StudioResearchReview, { props: { api, projectId, studyId } });
    await waitFor(() => expect(screen.getByRole('button', { name: 'Prepare research' })).toBeEnabled());
    expect(screen.queryByRole('button', { name: 'Approve local analysis' })).not.toBeInTheDocument();
  });

  it('uses RTL and Arabic labels for the immutable saved approval scope', async () => {
    setLocale('ar');
    const { api, projectId, studyId } = await fixture();
    const view = render(StudioResearchReview, { props: { api, projectId, studyId } });
    const prepare = await screen.findByRole('button', { name: 'تحضير البحث' });
    await waitFor(() => expect(prepare).toBeEnabled());
    await fireEvent.click(prepare);
    expect(await screen.findByText(/الموافقة على هذه المصادر/)).toBeInTheDocument();
    expect(view.container.querySelector('.study-research-review')).toHaveAttribute('dir', 'rtl');
  });

  it('cancels an interrupted native research run without showing a fabricated report', async () => {
    const { api, projectId, studyId } = await fixture();
    const available = await api.listStudyResearchSources(projectId, studyId);
    const pending = await api.startStudyResearch(projectId, studyId, [available[0].id]);
    const running = { ...pending, state: 'running' as const };
    api.listStudyResearchRuns = async () => [running];
    const decide = vi.fn(async () => ({ ...running, state: 'cancelled' as const }));
    api.decideStudyResearch = decide;

    render(StudioResearchReview, { props: { api, projectId, studyId } });
    const cancel = await screen.findByRole('button', { name: 'Cancel' });
    await fireEvent.click(cancel);
    await waitFor(() => expect(decide).toHaveBeenCalledWith(
      projectId, studyId, pending.plan.id, 'cancel'));
    expect(await screen.findByText(/Cancelled/)).toBeInTheDocument();
    expect(screen.queryByLabelText('Study research report')).not.toBeInTheDocument();
  });

});
