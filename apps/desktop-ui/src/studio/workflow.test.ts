import { act, fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { setLocale } from '../i18n';
import type { ContentStudy, StudioGateway } from './contract';
import type { StudioAgentRunGateway, StudioRun } from './agenticRuntime';
import { isSameSourceSet, limitsValid, validatesStudioRun } from './agenticRuntime';
import StudioAgentReview from './StudioAgentReview.svelte';
import StudioCandidateCompare from './StudioCandidateCompare.svelte';

afterEach(() => setLocale('en'));

function fixture(id = 'study-a'): ContentStudy {
  return {
    id, projectId: 'p1', revision: 4,
    brief: { title: 'Study', goal: 'Original script', audience: 'Creators', platform: 'TikTok',
      language: 'English', duration: '40', style: 'Clear' },
    sources: [{ kind: 'transcript', id: 't-1' }, { kind: 'research', id: 'r-2' }],
    draft: 'Draft B', draftId: 'd-2', draftRevision: 2,
    drafts: [
      { id: 'd-1', title: 'A', revision: 1 },
      { id: 'd-2', title: 'B', revision: 2 },
      { id: 'd-3', title: 'C', revision: 3 }
    ],
    draftVersions: []
  };
}

function runFor(study = fixture()): StudioRun {
  return {
    runId: 'run-1', projectId: study.projectId, studyId: study.id,
    studyRevision: study.revision, state: 'awaiting_approval', provider: 'OpenAI',
    model: 'test-model', exactPrompt: 'Exact attached evidence prompt',
    selectedSources: [...study.sources], maxTokens: 6000, maxUsd: 1
  };
}

function gatewayWith(bridge: StudioAgentRunGateway): StudioGateway {
  return bridge as unknown as StudioGateway;
}

async function enterModel() {
  await fireEvent.input(screen.getByRole('textbox', { name: 'Model' }), { target: { value: 'test-model' } });
}

describe('Studio run review: no fabricated provider execution', () => {
  it('supports a local exact-prompt preview but cannot approve without host integration', async () => {
    render(StudioAgentReview, { props: {
      projectId: 'p1', study: fixture(), gateway: gatewayWith({}),
      prompt: 'Exact attached evidence prompt'
    } });
    expect(screen.getByText(/Provider execution is not connected/)).toBeInTheDocument();
    const preview = screen.getByText(/Exact data to be sent/);
    await fireEvent.click(preview);
    expect(screen.getByText('Exact attached evidence prompt')).toBeInTheDocument();
    expect(screen.getByText('transcript · t-1')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Approve execution' })).not.toBeInTheDocument();
  });

  it('requires a durable server proposal and explicit consent before approval', async () => {
    const study = fixture(), pending = runFor(study);
    const propose = vi.fn(async () => pending);
    const decide = vi.fn(async (_project: string, _study: string, _run: string, action: string) =>
      ({ ...pending, state: action === 'approve' ? 'running' : 'cancelled' } as StudioRun));
    render(StudioAgentReview, { props: {
      projectId: 'p1', study, prompt: pending.exactPrompt,
      gateway: gatewayWith({ proposeStudioRun: propose, decideStudioRun: decide })
    } });
    await enterModel();
    await fireEvent.click(screen.getByRole('button', { name: 'Request approval' }));
    expect(propose).toHaveBeenCalledWith('p1', study.id, study.revision,
      expect.objectContaining({ exactPrompt: pending.exactPrompt, maxTokens: 6000, maxUsd: 1,
        selectedSources: study.sources }));
    const approve = await screen.findByRole('button', { name: 'Approve execution' });
    expect(approve).toBeDisabled();
    expect(decide).not.toHaveBeenCalled();
    await fireEvent.click(screen.getByRole('checkbox', { name: /I authorize sending this exact prompt/ }));
    expect(approve).toBeEnabled();
    await fireEvent.click(approve);
    await waitFor(() => expect(decide).toHaveBeenCalledWith('p1', study.id, pending.runId, 'approve'));
    expect(screen.getByText('Running')).toBeInTheDocument();
  });

  it('records denial through the backend without sending an approval', async () => {
    const pending = runFor();
    const decide = vi.fn(async () => ({ ...pending, state: 'cancelled' as const }));
    render(StudioAgentReview, { props: {
      projectId: 'p1', study: fixture(), prompt: pending.exactPrompt,
      gateway: gatewayWith({ proposeStudioRun: async () => pending, decideStudioRun: decide })
    } });
    await enterModel();
    await fireEvent.click(screen.getByRole('button', { name: 'Request approval' }));
    await fireEvent.click(await screen.findByRole('button', { name: 'Deny' }));
    await waitFor(() => expect(decide).toHaveBeenCalledWith('p1', 'study-a', 'run-1', 'deny'));
    expect(screen.getByText('Cancelled / denied')).toBeInTheDocument();
  });

  it('rejects cross-project and tampered prompt responses', async () => {
    const badRun = { ...runFor(), projectId: 'other-project' };
    const decide = vi.fn();
    render(StudioAgentReview, { props: {
      projectId: 'p1', study: fixture(), prompt: badRun.exactPrompt,
      gateway: gatewayWith({ proposeStudioRun: async () => badRun, decideStudioRun: decide })
    } });
    await enterModel();
    await fireEvent.click(screen.getByRole('button', { name: 'Request approval' }));
    expect(await screen.findByRole('alert')).toHaveTextContent(/changed/i);
    expect(screen.queryByRole('button', { name: 'Approve execution' })).not.toBeInTheDocument();
    expect(decide).not.toHaveBeenCalled();
  });

  it('discards an in-flight proposal when switching studies', async () => {
    let resolve!: (value: StudioRun) => void;
    const pending = new Promise<StudioRun>(yes => { resolve = yes; });
    const study = fixture();
    const view = render(StudioAgentReview, { props: {
      projectId: 'p1', study, prompt: 'Exact attached evidence prompt',
      gateway: gatewayWith({ proposeStudioRun: () => pending, decideStudioRun: vi.fn() })
    } });
    await enterModel();
    await fireEvent.click(screen.getByRole('button', { name: 'Request approval' }));
    await view.rerender({ projectId: 'p1', study: fixture('study-b'), prompt: 'Different prompt' });
    await act(async () => { resolve(runFor(study)); await pending; });
    expect(screen.queryByRole('button', { name: 'Approve execution' })).not.toBeInTheDocument();
  });

  it('shows authoritative blocked state after reopening, with a reconciliation action', async () => {
    const saved: StudioRun = { ...runFor(), state: 'blocked' };
    const recover = vi.fn(async () => ({ ...saved, state: 'failed' as const }));
    render(StudioAgentReview, { props: {
      projectId: 'p1', study: fixture(),
      gateway: gatewayWith({ listStudioRuns: async () => [saved], decideStudioRun: recover,
        proposeStudioRun: async () => saved })
    } });
    await waitFor(() => expect(screen.getByText('Blocked — reconciliation required')).toBeInTheDocument());
    await fireEvent.click(screen.getByRole('button', { name: 'Reconcile interrupted run' }));
    expect(recover).toHaveBeenCalledWith('p1', 'study-a', 'run-1', 'recover');
  });

  it('supports Arabic RTL and enforces numeric spending bounds', async () => {
    setLocale('ar');
    const { container } = render(StudioAgentReview, { props: { projectId: 'p1', study: fixture() } });
    expect(container.querySelector('.studio-run-review')).toHaveAttribute('dir', 'rtl');
    expect(screen.getByRole('heading', { name: 'مراجعة تنفيذ الذكاء الاصطناعي' })).toBeInTheDocument();
    expect(limitsValid('6000', '1.00')).toBe(true);
    expect(limitsValid('0', '1')).toBe(false);
    expect(limitsValid('200001', '1')).toBe(false);
    expect(limitsValid('6000', '-2')).toBe(false);
  });
});

describe('saved candidate comparison', () => {
  function comparisonGateway() {
    const getStudy = vi.fn(async (projectId: string, studyId: string, draftId?: string) => {
      const study = fixture(studyId), candidate = study.drafts!.find(item => item.id === draftId)!;
      return { ...study, projectId, draftId: candidate.id, draftRevision: candidate.revision,
        draft: 'Body of ' + candidate.title };
    });
    return { getStudy, gateway: { getStudy } as unknown as StudioGateway };
  }

  it('loads three independently saved candidate bodies and opens the selected editor', async () => {
    const { gateway, getStudy } = comparisonGateway();
    const choose = vi.fn();
    render(StudioCandidateCompare, { props: {
      projectId: 'p1', study: fixture(), gateway, onChoose: choose
    } });
    const button = screen.getByRole('button', { name: 'Compare script candidates (3)' });
    button.focus();
    expect(document.activeElement).toBe(button);
    expect(button.tagName).toBe('BUTTON');
    await fireEvent.click(button);
    await waitFor(() => expect(screen.getByText('Body of C')).toBeInTheDocument());
    expect(getStudy).toHaveBeenCalledTimes(3);
    const card = screen.getByText('Body of A').closest('article')!;
    await fireEvent.click(within(card).getByRole('button', { name: 'Edit this candidate' }));
    expect(choose).toHaveBeenCalledWith('d-1');
  });

  it('blocks saved comparison while Study edits are unsaved', async () => {
    const { gateway, getStudy } = comparisonGateway();
    render(StudioCandidateCompare, { props: {
      projectId: 'p1', study: fixture(), gateway, pendingEdits: true
    } });
    const compare = screen.getByRole('button', { name: 'Compare script candidates (3)' });
    expect(compare).toBeDisabled();
    expect(compare).toHaveAttribute('aria-expanded', 'false');
    expect(screen.getByRole('status')).toHaveTextContent(
      'Save Study changes before comparing saved script candidates.');
    await fireEvent.click(compare);
    expect(getStudy).not.toHaveBeenCalled();
    expect(screen.queryByRole('button', { name: 'Edit this candidate' })).not.toBeInTheDocument();
    expect(screen.queryByText('Unsaved local draft')).not.toBeInTheDocument();
  });

  it('fails closed on foreign-project candidate hydration', async () => {
    const { gateway } = comparisonGateway();
    const unsafe = { ...gateway, getStudy: async () => ({ ...fixture(), projectId: 'other' }) };
    render(StudioCandidateCompare, { props: {
      projectId: 'p1', study: fixture(), gateway: unsafe
    } });
    await fireEvent.click(screen.getByRole('button', { name: 'Compare script candidates (3)' }));
    expect(await screen.findByRole('alert')).toHaveTextContent(/Cannot compare/);
    expect(screen.queryByText('Body of A')).not.toBeInTheDocument();
  });

  it('renders Arabic RTL and preserves durable source validation', () => {
    setLocale('ar');
    const { gateway } = comparisonGateway();
    const { container } = render(StudioCandidateCompare, { props: {
      projectId: 'p1', study: fixture(), gateway
    } });
    expect(container.querySelector('.candidate-compare')).toHaveAttribute('dir', 'rtl');
    expect(screen.getByRole('button', { name: 'مقارنة نسخ النصوص (3)' })).toBeInTheDocument();
    expect(isSameSourceSet(fixture().sources, [...fixture().sources].reverse())).toBe(true);
    expect(validatesStudioRun({ ...runFor(), projectId: 'wrong' }, 'p1', 'study-a')).toBe(false);
  });
});