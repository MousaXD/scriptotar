import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { createMockClient } from '../api/mockClient';
import { setLocale } from '../i18n';
import StudioView from '../views/StudioView.svelte';
import type { ContentStudy, StudioGateway, StudyWritingBatch } from './contract';

afterEach(() => setLocale('en'));

function setup() {
  setLocale('en');
  const projectId = 'project-1';
  const studyId = 'study-1';
  const ids = ['first-generated', 'second-generated', 'third-generated'] as const;
  const study: ContentStudy = {
    id: studyId, projectId, revision: 3,
    brief: { title: 'Receipt Study', goal: 'Draft three original scripts',
      audience: 'Creators', platform: 'Reels', language: 'English',
      duration: '30', style: 'Informative' },
    sources: [], draft: 'Manual body', draftVersions: [],
    draftId: 'manual', draftRevision: 1,
    drafts: [{ id: 'manual', title: 'Manual script', revision: 1 },
      ...ids.map((id, i) => ({ id, title: `Generated ${i + 1}`, revision: 1 }))]
  };
  const batches = vi.fn(async (): Promise<StudyWritingBatch[]> => [{
    runId: 'canonical-run-1', projectId, studyId,
    draftIds: [...ids], committedAt: '2026-11-17T17:00:00Z'
  }]);
  const getStudy = vi.fn(async (_project: string, _study: string, draftId?: string) => ({
    ...study,
    draftId: draftId || 'manual',
    draft: draftId ? `Exact body for ${draftId}` : 'Manual body'
  }));
  const gateway: StudioGateway = {
    listStudies: async () => [{ id: studyId, projectId,
      brief: study.brief, revision: study.revision }],
    createStudy: async () => study,
    getStudy,
    saveStudy: async () => study,
    listStudyWritingBatches: batches
  };
  const api = createMockClient();
  return { projectId, study, batches, getStudy, gateway, api };
}

async function openStudy() {
  await fireEvent.click(await screen.findByRole('combobox', { name: 'Open study' }));
  await fireEvent.click(await screen.findByRole('option', { name: 'Receipt Study' }));
}

describe('Studio read-only canonical three-script receipts', () => {
  it('opens only the exact receipt-linked candidate without another provider request', async () => {
    const { projectId, gateway, api, getStudy, batches } = setup();
    const runAi = vi.spyOn(api, 'runAi');
    render(StudioView, { props: { api, projectId, gateway } });
    await openStudy();
    expect(await screen.findByRole('heading', { name: 'Saved three-script batches' })).toBeInTheDocument();
    await waitFor(() => expect(batches).toHaveBeenCalledWith(projectId, 'study-1'));
    await fireEvent.click(screen.getByRole('button', { name: 'Generated 2' }));
    await waitFor(() => expect(getStudy).toHaveBeenCalledWith(projectId, 'study-1', 'second-generated'));
    expect(screen.getByRole('textbox', { name: 'Script draft' }))
      .toHaveValue('Exact body for second-generated');
    expect(runAi).not.toHaveBeenCalled();
  });

  it('quarantines a foreign-project receipt without rendering its draft actions', async () => {
    const { projectId, gateway, api, batches } = setup();
    batches.mockResolvedValueOnce([{
      runId: 'foreign-run', projectId: 'another-project', studyId: 'study-1',
      draftIds: ['first-generated', 'second-generated', 'third-generated'],
      committedAt: '2026-11-17T17:00:00Z'
    }]);
    render(StudioView, { props: { api, projectId, gateway } });
    await openStudy();
    expect(await screen.findByText('This study belongs to a different project.')).toHaveAttribute('role', 'alert');
    expect(screen.queryByRole('button', { name: 'Generated 2' })).not.toBeInTheDocument();
  });

  it('rejects malformed four-candidate receipts rather than showing a fourth draft', async () => {
    const { projectId, gateway, api, batches } = setup();
    batches.mockResolvedValueOnce([{
      runId: 'malformed-run', projectId, studyId: 'study-1',
      draftIds: ['first-generated', 'second-generated', 'third-generated', 'forged-fourth'] as unknown as [string, string, string],
      committedAt: '2026-11-17T17:00:00Z'
    }]);
    render(StudioView, { props: { api, projectId, gateway } });
    await openStudy();
    expect(await screen.findByText('This study belongs to a different project.')).toHaveAttribute('role', 'alert');
    expect(screen.queryByRole('button', { name: 'Generated 2' })).not.toBeInTheDocument();
  });
  it('exports the exact three receipt-linked scripts locally without AI egress', async () => {
    const { projectId, gateway, api, getStudy } = setup();
    const createObjectURL = vi.fn((_blob: Blob) => 'blob:receipt-export');
    Object.defineProperty(URL, 'createObjectURL', { configurable: true, value: createObjectURL });
    Object.defineProperty(URL, 'revokeObjectURL', { configurable: true, value: vi.fn() });
    const anchorClick = vi.spyOn(HTMLAnchorElement.prototype, 'click').mockImplementation(() => {});
    const runAi = vi.spyOn(api, 'runAi');
    render(StudioView, { props: { api, projectId, gateway } });
    await openStudy();
    const exportButton = await screen.findByRole('button', { name: 'Export 3 scripts' });
    await fireEvent.click(exportButton);
    await waitFor(() => expect(createObjectURL).toHaveBeenCalledTimes(1));
    expect(getStudy.mock.calls.filter(call => !!call[2]).map(call => call[2]))
      .toEqual(['first-generated', 'second-generated', 'third-generated']);
    const blob = createObjectURL.mock.calls[0][0] as Blob;
    expect(blob).toBeInstanceOf(Blob);
    expect(blob.type).toBe('text/markdown;charset=utf-8');
    expect(anchorClick).toHaveBeenCalledOnce();
    expect(runAi).not.toHaveBeenCalled();
    anchorClick.mockRestore();
  });

  it('refuses a mixed-revision batch without creating a download', async () => {
    const { projectId, gateway, api, getStudy, study } = setup();
    const createObjectURL = vi.fn((_blob: Blob) => 'blob:should-not-exist');
    Object.defineProperty(URL, 'createObjectURL', { configurable: true, value: createObjectURL });
    getStudy.mockImplementation(async (_project, _study, draftId) => ({
      ...study,
      revision: draftId === 'second-generated' ? study.revision + 1 : study.revision,
      draftId: draftId || 'manual',
      draft: draftId ? 'Exact ' + draftId : 'Manual body'
    }));
    render(StudioView, { props: { api, projectId, gateway } });
    await openStudy();
    await fireEvent.click(await screen.findByRole('button', { name: 'Export 3 scripts' }));
    expect(await screen.findByText('This study belongs to a different project.')).toHaveAttribute('role', 'alert');
    expect(createObjectURL).not.toHaveBeenCalled();
  });

  it('denies export if any returned script belongs to another project', async () => {
    const { projectId, gateway, api, getStudy, study } = setup();
    const createObjectURL = vi.fn(() => 'blob:should-not-exist');
    Object.defineProperty(URL, 'createObjectURL', { configurable: true, value: createObjectURL });
    getStudy.mockImplementation(async (_project, _study, draftId) => ({
      ...study,
      projectId: draftId === 'third-generated' ? 'foreign-project' : projectId,
      draftId: draftId || 'manual',
      draft: draftId ? 'Exact ' + draftId : 'Manual body'
    }));
    render(StudioView, { props: { api, projectId, gateway } });
    await openStudy();
    await fireEvent.click(await screen.findByRole('button', { name: 'Export 3 scripts' }));
    expect(await screen.findByText('This study belongs to a different project.')).toHaveAttribute('role', 'alert');
    expect(createObjectURL).not.toHaveBeenCalled();
  });
});
