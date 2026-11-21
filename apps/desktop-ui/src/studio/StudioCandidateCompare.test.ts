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

  it('invalidates receipt-linked comparison when a candidate beyond the first three changes', async () => {
    const { study, gateway, ids, projectId } = fixture();
    const view = render(StudioCandidateCompare, { props: { study, gateway, projectId } });
    const compare = screen.getByRole('button', { name: /Compare script candidates/ });
    await fireEvent.click(compare);
    expect(await screen.findByText('Saved generated-c')).toBeInTheDocument();
    expect(compare).toHaveAttribute('aria-expanded', 'true');

    // Generated C is the fourth current summary because a manual draft
    // precedes the receipt; the Study revision alone is not enough to notice.
    const edited = { ...study, drafts: study.drafts?.map(candidate =>
      candidate.id === ids[2] ? { ...candidate, revision: 2 } : candidate) };
    await view.rerender({ study: edited, gateway, projectId });
    expect(compare).toHaveAttribute('aria-expanded', 'false');
    expect(screen.queryByText('Saved generated-c')).not.toBeInTheDocument();
  });

  it('hides saved candidates and blocks their actions when the visible Study becomes dirty', async () => {
    const { study, gateway, projectId } = fixture();
    const choose = vi.fn();
    const view = render(StudioCandidateCompare, {
      props: { study, gateway, projectId, pendingEdits: false, onChoose: choose }
    });
    const compare = screen.getByRole('button', { name: /Compare script candidates/ });
    await fireEvent.click(compare);
    expect(await screen.findByText('Saved generated-c')).toBeInTheDocument();
    expect(screen.getAllByRole('button', { name: 'Edit this candidate' })).toHaveLength(3);

    await view.rerender({ study, gateway, projectId, pendingEdits: true, onChoose: choose });
    expect(compare).toBeDisabled();
    expect(compare).toHaveAttribute('aria-expanded', 'false');
    expect(screen.queryByText('Saved generated-c')).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Edit this candidate' })).not.toBeInTheDocument();
    expect(screen.getByText('Save Study changes before comparing saved script candidates.')).toBeInTheDocument();
    await fireEvent.click(compare);
    expect(choose).not.toHaveBeenCalled();

    await view.rerender({ study, gateway, projectId, pendingEdits: false, onChoose: choose });
    expect(compare).toBeEnabled();
  });

  it('drops an in-flight candidate read when unsaved edits invalidate its snapshot', async () => {
    const { study, gateway, read, projectId } = fixture();
    let releaseReads: (() => void) | undefined;
    const gate = new Promise<void>((resolve) => { releaseReads = resolve; });
    read.mockImplementation(async (_project, _study, draftId) => {
      await gate;
      return { ...study, draftId, draftRevision: 1, draft: 'Saved ' + draftId };
    });
    const view = render(StudioCandidateCompare, {
      props: { study, gateway, projectId, pendingEdits: false }
    });
    await fireEvent.click(screen.getByRole('button', { name: /Compare script candidates/ }));
    await waitFor(() => expect(read).toHaveBeenCalledTimes(3));
    await view.rerender({ study, gateway, projectId, pendingEdits: true });
    releaseReads?.();
    await Promise.resolve();
    expect(screen.getByRole('button', { name: /Compare script candidates/ })).toBeDisabled();
    expect(screen.queryByText('Saved generated-a')).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Edit this candidate' })).not.toBeInTheDocument();
  });

  it('refuses a foreign-project receipt before opening any candidate', async () => {
    const { study, gateway, read, batches, batch, projectId } = fixture();
    batches.mockResolvedValue([{ ...batch, projectId: 'foreign' }]);
    render(StudioCandidateCompare, { props: { study, gateway, projectId } });
    await fireEvent.click(screen.getByRole('button', { name: /Compare script candidates/ }));
    expect(await screen.findByRole('alert')).toHaveTextContent(/Cannot compare/);
    expect(read).not.toHaveBeenCalled();
  });
  it('refuses a candidate read with changed Study source attachments even at the same revision', async () => {
    const { study, gateway, read, projectId } = fixture();
    read.mockImplementation(async (_project, _study, draftId) =>
      ({ ...study, sources: [{ kind: 'transcript' as const, id: 'substituted-source' }],
        draftId, draftRevision: 1, draft: 'Wrong-source ' + draftId }));
    render(StudioCandidateCompare, { props: { study, gateway, projectId } });
    await fireEvent.click(screen.getByRole('button', { name: /Compare script candidates/ }));
    expect(await screen.findByRole('alert')).toHaveTextContent(/Cannot compare/);
    expect(screen.queryByText('Wrong-source generated-a')).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Edit this candidate' })).not.toBeInTheDocument();
  });

  it('refuses inconsistent draft summaries even when the returned body claims a current revision', async () => {
    const { study, gateway, read, projectId } = fixture();
    read.mockImplementation(async (_project, _study, draftId) => ({
      ...study, draftId, draftRevision: 1, draft: 'Forged ' + draftId,
      drafts: study.drafts?.map(d => d.id === draftId ? { ...d, revision: 2 } : d)
    }));
    render(StudioCandidateCompare, { props: { study, gateway, projectId } });
    await fireEvent.click(screen.getByRole('button', { name: /Compare script candidates/ }));
    expect(await screen.findByRole('alert')).toHaveTextContent(/Cannot compare/);
    expect(screen.queryByText('Forged generated-a')).not.toBeInTheDocument();
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
