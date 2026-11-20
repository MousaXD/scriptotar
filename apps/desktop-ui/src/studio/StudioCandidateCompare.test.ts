import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { setLocale } from '../i18n';
import StudioCandidateCompare from './StudioCandidateCompare.svelte';
import type { ContentStudy, StudioGateway, StudyWritingBatch } from './contract';

afterEach(() => setLocale('en'));

function fixture() {
  setLocale('en');
  const projectId = 'project-one';
  const studyId = 'study-one';
  const ids = ['generated-a', 'generated-b', 'generated-c'] as const;
  const study: ContentStudy = {
    id: studyId, projectId, revision: 5,
    brief: { title: 'Study', goal: 'Write original scripts', audience: 'Creators',
      platform: 'Reels', language: 'English', duration: '40', style: 'Clear' },
    sources: [], draft: 'Manual', draftVersions: [], draftId: 'manual', draftRevision: 1,
    drafts: ['manual', ...ids].map(id => ({ id, title: id, revision: 1 }))
  };
  const batch: StudyWritingBatch = {
    runId: 'verified', projectId, studyId, draftIds: [...ids],
    committedAt: '2026-11-17T15:00:00Z'
  };
  const read = vi.fn(async (_project: string, _study: string, draftId?: string) =>
    ({ ...study, draftId, draftRevision: 1, draft: 'Saved ' + draftId } as ContentStudy));
  const batches = vi.fn(async (): Promise<StudyWritingBatch[]> => [batch]);
  const gateway: StudioGateway = {
    listStudies: async () => [], createStudy: async () => study,
    getStudy: read, listStudyWritingBatches: batches, saveStudy: async () => study
  };
  return { study, gateway, read, batches, batch, ids, projectId };
}

describe('Verified v11 candidate comparison', () => {
  it('uses exactly the three receipt-linked draft IDs, not the old manual draft', async () => {
    const { study, gateway, read, ids, projectId } = fixture();
    render(StudioCandidateCompare, { props: { study, gateway, projectId } });
    await fireEvent.click(screen.getByRole('button', { name: /Compare script candidates/ }));
    await waitFor(() => expect(read).toHaveBeenCalledTimes(3));
    expect(read.mock.calls.map(args => args[2])).toEqual([...ids]);
    expect(screen.getByText('Saved generated-c')).toBeInTheDocument();
    expect(screen.queryByText('Saved manual')).not.toBeInTheDocument();
  });

  it('refuses a foreign-project receipt before opening any candidate', async () => {
    const { study, gateway, read, batches, batch, projectId } = fixture();
    batches.mockResolvedValue([{ ...batch, projectId: 'foreign' }]);
    render(StudioCandidateCompare, { props: { study, gateway, projectId } });
    await fireEvent.click(screen.getByRole('button', { name: /Compare script candidates/ }));
    expect(await screen.findByRole('alert')).toHaveTextContent(/Cannot compare/);
    expect(read).not.toHaveBeenCalled();
  });
  it('refuses saved bodies from a different Study revision, even for exact draft IDs', async () => {
    const { study, gateway, read, projectId } = fixture();
    read.mockImplementation(async (_project, _study, draftId) =>
      ({ ...study, revision: study.revision + 1, draftId,
        draftRevision: 1, draft: 'Stale ' + draftId }));
    render(StudioCandidateCompare, { props: { study, gateway, projectId } });
    await fireEvent.click(screen.getByRole('button', { name: /Compare script candidates/ }));
    expect(await screen.findByRole('alert')).toHaveTextContent(/Cannot compare/);
    expect(screen.queryByText('Stale generated-a')).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Edit this candidate' })).not.toBeInTheDocument();
  });

});
