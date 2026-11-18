import { act, fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { createMockClient, mockBootstrap } from '../api/mockClient';
import type { AiPromptInput } from '../api/client';
import { setLocale } from '../i18n';
import type { ContentStudy, CreativeBrief, StudioGateway } from '../studio/contract';
import { emptyBrief } from '../studio/contract';
import App from '../App.svelte';
import StudioView from './StudioView.svelte';

afterEach(() => setLocale('en'));

function studyFixture(): { gateway: StudioGateway; get: () => ContentStudy[] } {
  const records = new Map<string, ContentStudy>();
  let index = 0;
  const gateway: StudioGateway = {
    async listStudies(projectId) {
      return [...records.values()].filter((item) => item.projectId === projectId)
        .map(({ id, projectId: owner, brief, revision }) => ({ id, projectId: owner, brief, revision }));
    },
    async createStudy(projectId, brief) {
      const id = 'study-' + ++index;
      const result: ContentStudy = { id, projectId, revision: 1, brief: { ...brief }, sources: [], draft: '', draftVersions: [] };
      records.set(id, structuredClone(result));
      return structuredClone(result);
    },
    async getStudy(projectId, studyId) {
      const item = records.get(studyId);
      if (!item || item.projectId !== projectId) throw new Error('Study not found in this project');
      return structuredClone(item);
    },
    async saveStudy(projectId, studyId, revision, patch) {
      const item = records.get(studyId);
      if (!item || item.projectId !== projectId) throw new Error('Study not found in this project');
      if (item.revision !== revision) throw new Error('Concurrent revision conflict');
      const next: ContentStudy = { ...item, ...patch, revision: revision + 1,
        draftVersions: patch.draft !== undefined && patch.draft !== item.draft
          ? [...item.draftVersions, { id: 'v-' + (revision + 1), revision: revision + 1, content: patch.draft, createdAt: '2026-11-13T12:00:00Z' }]
          : item.draftVersions };
      records.set(studyId, structuredClone(next));
      return structuredClone(next);
    }
  };
  return { gateway, get: () => [...records.values()] };
}

function mount(gateway?: StudioGateway, extra: Record<string, unknown> = {}) {
  return render(StudioView, {
    props: {
      api: createMockClient(),
      projectId: 'p-creator-lab',
      research: mockBootstrap.research,
      transcripts: mockBootstrap.transcripts,
      jobs: mockBootstrap.jobs,
      gateway, ...extra
    }
  });
}

describe('Content Study Studio', () => {
  it('never claims a production save when the study service is not connected', async () => {
    mount();
    expect(screen.getByRole('heading', { name: 'Studio' })).toBeInTheDocument();
    expect(screen.getByText(/unsaved workspace; export your script/)).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Save changes' })).not.toBeInTheDocument();
    await fireEvent.input(screen.getByRole('textbox', { name: 'Script draft' }), {
      target: { value: 'An original voice-over draft' }
    });
    expect(screen.getByRole('textbox', { name: 'Script draft' })).toHaveValue('An original voice-over draft');
  });

  it('drops stale local prompts and unblocks new preparation when the brief changes', async () => {
    let resolvePrompt!: (value: string) => void;
    const pending = new Promise<string>((resolve) => { resolvePrompt = resolve; });
    const api = createMockClient({ buildAiPrompt: async () => pending });
    mount(undefined, { api });
    const prepare = screen.getByRole('button', { name: 'Prepare writing prompt' });
    await fireEvent.click(prepare);
    expect(prepare).toBeDisabled();
    await fireEvent.input(screen.getByRole('textbox', { name: 'Creative goal' }), {
      target: { value: 'A different angle' }
    });
    expect(prepare).toBeEnabled();
    await act(async () => { resolvePrompt('OUTDATED prompt'); await pending; });
    expect(screen.queryByText('OUTDATED prompt')).not.toBeInTheDocument();
  });

  it('creates, attaches an exact transcript, versions the draft and reopens it', async () => {
    const { gateway, get } = studyFixture();
    const view = mount(gateway);
    await fireEvent.input(screen.getByRole('textbox', { name: 'Study name' }), { target: { value: 'Arabic launch' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Create study' }));
    await waitFor(() => expect(get()).toHaveLength(1));

    await fireEvent.click(screen.getByText('Choose source material', { selector: 'summary' }));
    await fireEvent.click(screen.getByRole('checkbox', { name: 'Hook breakdown — Arabic sample' }));
    await fireEvent.input(screen.getByRole('textbox', { name: 'Script draft' }), { target: { value: 'مرحبا بكم في الإطلاق' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Save changes' }));
    await waitFor(() => expect(get()[0].draft).toBe('مرحبا بكم في الإطلاق'));
    expect(get()[0].sources).toEqual([{ kind: 'transcript', id: 't-ar' }]);
    expect(get()[0].draftVersions).toHaveLength(1);

    view.unmount();
    mount(gateway);
    await fireEvent.click(screen.getByRole('combobox', { name: 'Open study' }));
    await fireEvent.click(screen.getByRole('option', { name: 'Arabic launch' }));
    expect(await screen.findByRole('textbox', { name: 'Script draft' })).toHaveValue('مرحبا بكم في الإطلاق');
    expect(screen.getAllByText('Hook breakdown — Arabic sample').length).toBeGreaterThan(0);
    await fireEvent.click(screen.getByRole('button', { name: 'Version 2' }));
    expect(screen.getByRole('textbox', { name: 'Script draft' })).toHaveAttribute('readonly');
    await fireEvent.click(screen.getByRole('button', { name: 'Current draft' }));
    expect(screen.getByRole('textbox', { name: 'Script draft' })).not.toHaveAttribute('readonly');
  });


  it('restores a historical script as a new immutable draft version and reopens it', async () => {
    const { gateway, get } = studyFixture();
    const view = mount(gateway);
    await fireEvent.input(screen.getByRole('textbox', { name: 'Study name' }), {
      target: { value: 'Revision history' }
    });
    await fireEvent.click(screen.getByRole('button', { name: 'Create study' }));
    await waitFor(() => expect(get()).toHaveLength(1));

    const editor = screen.getByRole('textbox', { name: 'Script draft' });
    await fireEvent.input(editor, { target: { value: 'First original script' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Save changes' }));
    await waitFor(() => expect(get()[0].draft).toBe('First original script'));

    await fireEvent.input(editor, { target: { value: 'Revised script' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Save changes' }));
    await waitFor(() => expect(get()[0].draft).toBe('Revised script'));
    expect(get()[0].draftVersions).toHaveLength(2);

    await fireEvent.click(screen.getByRole('button', { name: 'Version 2' }));
    expect(editor).toHaveAttribute('readonly');
    await fireEvent.click(screen.getByRole('button', { name: 'Restore as new version' }));
    await waitFor(() => expect(get()[0].draft).toBe('First original script'));
    expect(get()[0].draftVersions.map((item) => item.content)).toEqual([
      'First original script', 'Revised script', 'First original script'
    ]);
    expect(editor).not.toHaveAttribute('readonly');

    view.unmount();
    mount(gateway);
    await fireEvent.click(screen.getByRole('combobox', { name: 'Open study' }));
    await fireEvent.click(screen.getByRole('option', { name: 'Revision history' }));
    expect(await screen.findByRole('textbox', { name: 'Script draft' }))
      .toHaveValue('First original script');
  });

  it('saves an unverified exact source observation and reopens it from the same study', async () => {
    const api = createMockClient();
    const view = render(App, { props: { api } });
    await screen.findByRole('heading', { name: 'Creator Lab' });
    await fireEvent.click(screen.getByRole('button', { name: 'Studio' }));
    await fireEvent.input(screen.getByRole('textbox', { name: 'Study name' }), {
      target: { value: 'Referenced research' }
    });
    await fireEvent.click(screen.getByRole('button', { name: 'Create study' }));
    await fireEvent.click(screen.getByText('Choose source material', { selector: 'summary' }));
    await fireEvent.click(screen.getByRole('checkbox', { name: 'Hook breakdown — Arabic sample' }));
    await fireEvent.click(screen.getByRole('button', { name: 'Hook breakdown — Arabic sample' }));
    await fireEvent.input(screen.getByRole('textbox', { name: 'Observation or claim' }), {
      target: { value: 'The opening uses a question' }
    });
    await fireEvent.input(screen.getByRole('textbox', { name: 'Exact timestamp or locator' }), {
      target: { value: '0:00-0:07' }
    });
    await fireEvent.click(screen.getByRole('button', { name: 'Save observation' }));
    expect(await screen.findByText('The opening uses a question')).toBeInTheDocument();
    expect(screen.getAllByText('Unverified — not fact-checked').length).toBeGreaterThan(0);
    const study = (await api.listStudies('p-creator-lab'))[0];
    const observed = await api.listStudyEvidence('p-creator-lab', study.id);
    expect(observed).toHaveLength(1);
    expect(observed[0]).toMatchObject({
      projectId: 'p-creator-lab', studyId: study.id,
      source: { kind: 'transcript', id: 't-ar' }, locator: '0:00-0:07', certainty: 'unverified'
    });
    await expect(api.addStudyEvidence('p-client-a', study.id, {
      source: { kind: 'transcript', id: 't-ar' }, claim: 'Wrong project', locator: '0:00'
    })).rejects.toThrow();
    await expect(api.addStudyEvidence('p-creator-lab', study.id, {
      source: { kind: 'transcript', id: 'not-attached' }, claim: 'Wrong source', locator: '0:00'
    })).rejects.toThrow();
    view.unmount();
    render(App, { props: { api } });
    await screen.findByRole('heading', { name: 'Creator Lab' });
    await fireEvent.click(screen.getByRole('button', { name: 'Studio' }));
    await fireEvent.click(screen.getByRole('combobox', { name: 'Open study' }));
    await fireEvent.click(screen.getByRole('option', { name: 'Referenced research' }));
    expect(await screen.findByText('The opening uses a question')).toBeInTheDocument();
  });

  it('revokes a Study source with confirmation without losing linked evidence or immutable drafts', async () => {
    const api = createMockClient();
    const study = await api.createStudy('p-creator-lab', {
      ...emptyBrief(), title: 'Permission withdrawal', goal: 'Teach an editing concept'
    });
    const attached = await api.saveStudy('p-creator-lab', study.id, study.revision, {
      sources: [{ kind: 'transcript', id: 't-ar' }], draft: 'Original immutable script'
    });
    await api.addStudyEvidence('p-creator-lab', study.id, {
      source: { kind: 'transcript', id: 't-ar' }, claim: 'Old observation', locator: '0:02'
    });
    const original = await api.getStudy('p-creator-lab', study.id);
    expect(original.revision).toBe(attached.revision);
    const provider = vi.spyOn(api, 'runAi');
    render(App, { props: { api } });
    await screen.findByRole('heading', { name: 'Creator Lab' });
    await fireEvent.click(screen.getByRole('button', { name: 'Studio' }));
    await fireEvent.click(screen.getByRole('combobox', { name: 'Open study' }));
    await fireEvent.click(screen.getByRole('option', { name: 'Permission withdrawal' }));
    await fireEvent.click(screen.getByRole('button', { name: 'Hook breakdown — Arabic sample' }));
    await fireEvent.click(screen.getByRole('button', { name: 'Revoke source access' }));
    expect(screen.getByText(/Linked evidence and draft history remain/)).toBeInTheDocument();
    expect((await api.getStudy('p-creator-lab', study.id)).revision).toBe(original.revision);
    await fireEvent.click(screen.getByRole('button', { name: 'Confirm revoke' }));
    await screen.findByText('Study source access revoked. Historical evidence retained.');
    const changed = await api.getStudy('p-creator-lab', study.id);
    expect(changed.sources).toEqual([]);
    expect(changed.revision).toBe(original.revision + 1);
    expect(changed.draft).toBe(original.draft);
    expect(changed.draftVersions).toEqual(original.draftVersions);
    expect((await api.listStudyEvidence('p-creator-lab', study.id))[0].sourceAvailable).toBe(false);
    await expect(api.saveStudy('p-creator-lab', study.id, changed.revision, {
      sources: [{ kind: 'transcript', id: 't-ar' }]
    })).rejects.toThrow(/Revoked Study source/);
    await expect(api.revokeStudySourceAccess('p-client-a', study.id, changed.revision,
      { kind: 'transcript', id: 't-ar' })).rejects.toThrow();
    expect(await api.getStudy('p-creator-lab', study.id)).toEqual(changed);
    expect(provider).not.toHaveBeenCalled();
  });

  it('prepares a local writing prompt with exact saved evidence and makes zero paid calls', async () => {
    const built = vi.fn(async (_input: AiPromptInput) => 'Prepared locally');
    const paid = vi.fn(async (_input: AiPromptInput) => 'Should never run');
    const api = createMockClient({ buildAiPrompt: built, runAi: paid });
    render(App, { props: { api } });
    await screen.findByRole('heading', { name: 'Creator Lab' });
    await fireEvent.click(screen.getByRole('button', { name: 'Studio' }));
    await fireEvent.input(screen.getByRole('textbox', { name: 'Study name' }), {
      target: { value: 'Evidence writing context' }
    });
    await fireEvent.click(screen.getByRole('button', { name: 'Create study' }));
    await fireEvent.click(screen.getByText('Choose source material', { selector: 'summary' }));
    await fireEvent.click(screen.getByRole('checkbox', { name: 'Hook breakdown — Arabic sample' }));
    await fireEvent.click(screen.getByRole('button', { name: 'Hook breakdown — Arabic sample' }));
    await fireEvent.input(screen.getByRole('textbox', { name: 'Observation or claim' }), {
      target: { value: 'The opening mentions a decision point' }
    });
    await fireEvent.input(screen.getByRole('textbox', { name: 'Exact timestamp or locator' }), {
      target: { value: '0:00–0:07' }
    });
    await fireEvent.click(screen.getByRole('button', { name: 'Save observation' }));
    expect(await screen.findByText('The opening mentions a decision point')).toBeInTheDocument();
    await fireEvent.click(screen.getByRole('button', { name: 'Prepare writing prompt' }));
    await waitFor(() => expect(built).toHaveBeenCalledTimes(1));
    const context = built.mock.calls[0][0].sourceText;
    expect(context).toContain('The opening mentions a decision point');
    expect(context).toContain('0:00–0:07');
    expect(context).toContain('UNTRUSTED, NOT INDEPENDENTLY FACT-CHECKED');
    expect(context).toContain('"source_id":"t-ar"');
    expect(context).toContain('Untrusted transcript text');
    expect(paid).not.toHaveBeenCalled();
  });

  it('refuses foreign evidence returned by an injected gateway before constructing a prompt', async () => {
    const { gateway } = studyFixture();
    const built = vi.fn(async (_input: AiPromptInput) => 'Should not prepare');
    const api = createMockClient({ buildAiPrompt: built });
    mount({
      ...gateway,
      listStudyEvidence: async (projectId, studyId) => [{
        id: 'foreign-evidence', projectId: 'foreign-project', studyId,
        source: null, sourceLabel: 'unknown', sourceAvailable: false,
        claim: 'Untrusted foreign observation', locator: '0:01',
        timestampSeconds: null, certainty: 'unverified', uncertaintyNote: null,
        createdAt: '2026-11-15T00:00:00Z'
      }]
    }, { api });
    await fireEvent.input(screen.getByRole('textbox', { name: 'Study name' }), {
      target: { value: 'Scope rejection' }
    });
    await fireEvent.click(screen.getByRole('button', { name: 'Create study' }));
    await fireEvent.click(screen.getByRole('button', { name: 'Prepare writing prompt' }));
    expect(await screen.findByText('Evidence belongs to another project or study.')).toBeInTheDocument();
    expect(built).not.toHaveBeenCalled();
  });

  it('imports an exact historical AI output only after confirmation, preserving prior draft versions', async () => {
    const api = createMockClient();
    const view = render(App, { props: { api } });
    await screen.findByRole('heading', { name: 'Creator Lab' });
    await fireEvent.click(screen.getByRole('button', { name: 'Studio' }));
    await fireEvent.input(screen.getByRole('textbox', { name: 'Study name' }), {
      target: { value: 'Historical import' }
    });
    await fireEvent.click(screen.getByRole('button', { name: 'Create study' }));
    await fireEvent.input(screen.getByRole('textbox', { name: 'Script draft' }), {
      target: { value: 'My existing original script' }
    });
    await fireEvent.click(screen.getByRole('button', { name: 'Save changes' }));
    const study = (await api.listStudies('p-creator-lab'))[0];
    await waitFor(async () =>
      expect((await api.getStudy('p-creator-lab', study.id)).draft).toBe('My existing original script'));

    await fireEvent.click(screen.getByRole('button', { name: 'Reopen saved result: Restaurant launch hook set' }));
    expect(await screen.findByText('Saved output for Hook ideas')).toBeInTheDocument();
    await fireEvent.click(screen.getByRole('button', { name: 'Import AI result as draft' }));
    expect((await api.getStudy('p-creator-lab', study.id)).draft).toBe('My existing original script');
    expect(screen.getByText(/Earlier versions remain/)).toBeInTheDocument();
    await fireEvent.click(screen.getByRole('button', { name: 'Confirm import' }));
    await waitFor(async () =>
      expect((await api.getStudy('p-creator-lab', study.id)).draft).toBe('Saved output for Hook ideas'));
    const stored = await api.getStudy('p-creator-lab', study.id);
    expect(stored.draftVersions.map((version) => version.content)).toEqual([
      'My existing original script', 'Saved output for Hook ideas'
    ]);

    await fireEvent.click(screen.getByRole('button', { name: 'Reopen saved result: Caption pacing breakdown' }));
    expect(await screen.findByText('Saved prompt for Viral breakdown')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Import AI result as draft' })).not.toBeInTheDocument();

    view.unmount();
    render(App, { props: { api } });
    await screen.findByRole('heading', { name: 'Creator Lab' });
    await fireEvent.click(screen.getByRole('button', { name: 'Studio' }));
    await fireEvent.click(screen.getByRole('combobox', { name: 'Open study' }));
    await fireEvent.click(screen.getByRole('option', { name: 'Historical import' }));
    expect(await screen.findByRole('textbox', { name: 'Script draft' }))
      .toHaveValue('Saved output for Hook ideas');
  });

  it('reviews disputed claims without letting them enter local writing prompts, then reopens them', async () => {
    const built = vi.fn(async (_input: AiPromptInput) => 'Prepared locally');
    const paid = vi.fn(async (_input: AiPromptInput) => 'Should not run');
    const api = createMockClient({ buildAiPrompt: built, runAi: paid });
    const view = render(App, { props: { api } });
    await screen.findByRole('heading', { name: 'Creator Lab' });
    await fireEvent.click(screen.getByRole('button', { name: 'Studio' }));
    await fireEvent.input(screen.getByRole('textbox', { name: 'Study name' }), {
      target: { value: 'Evidence claim review' }
    });
    await fireEvent.click(screen.getByRole('button', { name: 'Create study' }));
    await fireEvent.click(screen.getByText('Choose source material', { selector: 'summary' }));
    await fireEvent.click(screen.getByRole('checkbox', { name: 'Hook breakdown — Arabic sample' }));
    await fireEvent.click(screen.getByRole('button', { name: 'Hook breakdown — Arabic sample' }));
    await fireEvent.input(screen.getByRole('textbox', { name: 'Observation or claim' }), {
      target: { value: 'UNTRUSTWORTHY source claim' }
    });
    await fireEvent.input(screen.getByRole('textbox', { name: 'Exact timestamp or locator' }), {
      target: { value: '0:01-0:04' }
    });
    await fireEvent.click(screen.getByRole('button', { name: 'Save observation' }));
    expect(await screen.findByText('UNTRUSTWORTHY source claim')).toBeInTheDocument();
    const study = (await api.listStudies('p-creator-lab'))[0];
    const original = (await api.listStudyEvidence('p-creator-lab', study.id))[0];
    await fireEvent.click(screen.getByRole('button', { name: 'Flag as disputed' }));
    await fireEvent.input(screen.getByRole('textbox', { name: 'Reason for evidence review' }), {
      target: { value: 'The recording contradicts this interpretation' }
    });
    await fireEvent.click(screen.getByRole('button', { name: 'Confirm dispute' }));
    expect(await screen.findByText('Disputed — excluded from writing prompts')).toBeInTheDocument();
    await expect(api.reviewStudyEvidence('p-creator-lab', study.id, original.id, {
      expectedCertainty: 'unverified', action: 'dispute', note: 'Stale review'
    })).rejects.toThrow();
    await expect(api.reviewStudyEvidence('p-client-a', study.id, original.id, {
      expectedCertainty: 'disputed', action: 'reopen', note: 'Wrong project'
    })).rejects.toThrow();
    await fireEvent.click(screen.getByRole('button', { name: 'Prepare writing prompt' }));
    await waitFor(() => expect(built).toHaveBeenCalledTimes(1));
    expect(built.mock.calls[0][0].sourceText).not.toContain('UNTRUSTWORTHY source claim');
    expect(paid).not.toHaveBeenCalled();
    view.unmount();

    render(App, { props: { api } });
    await screen.findByRole('heading', { name: 'Creator Lab' });
    await fireEvent.click(screen.getByRole('button', { name: 'Studio' }));
    await fireEvent.click(screen.getByRole('combobox', { name: 'Open study' }));
    await fireEvent.click(screen.getByRole('option', { name: 'Evidence claim review' }));
    expect(await screen.findByText('Disputed — excluded from writing prompts')).toBeInTheDocument();
    expect((await api.listStudyEvidence('p-creator-lab', study.id))[0].reviewHistory).toHaveLength(1);
    await fireEvent.click(screen.getByRole('button', { name: 'Return to unverified' }));
    await fireEvent.input(screen.getByRole('textbox', { name: 'Reason for evidence review' }), {
      target: { value: 'Reviewed the entire segment and corrected the context' }
    });
    await fireEvent.click(screen.getByRole('button', { name: 'Confirm reopening' }));
    await waitFor(async () => {
      const latest = (await api.listStudyEvidence('p-creator-lab', study.id))[0];
      expect(latest.certainty).toBe('unverified');
      expect(latest.reviewHistory).toHaveLength(2);
      expect(latest.reviewHistory?.[0].note).toBe('The recording contradicts this interpretation');
      expect(latest.reviewHistory?.[1].note).toBe('Reviewed the entire segment and corrected the context');
    });
    await fireEvent.click(screen.getByText(/Recent review history/));
    expect(await screen.findByText('The recording contradicts this interpretation')).toBeInTheDocument();
    await fireEvent.click(screen.getByRole('button', { name: 'Prepare writing prompt' }));
    await waitFor(() => expect(built).toHaveBeenCalledTimes(2));
    expect(built.mock.calls[1][0].sourceText).toContain('UNTRUSTWORTHY source claim');
    expect(paid).not.toHaveBeenCalled();
  });


  it('duplicates a script into a separate candidate without replacing original versions', async () => {
    const api = createMockClient();
    const view = render(App, { props: { api } });
    await screen.findByRole('heading', { name: 'Creator Lab' });
    await fireEvent.click(screen.getByRole('button', { name: 'Studio' }));
    await fireEvent.input(screen.getByRole('textbox', { name: 'Study name' }), {
      target: { value: 'Two original directions' }
    });
    await fireEvent.click(screen.getByRole('button', { name: 'Create study' }));
    const chooser = await screen.findByRole('combobox', { name: 'Script candidates' });
    expect(chooser).toBeInTheDocument();
    const editor = screen.getByRole('textbox', { name: 'Script draft' });
    await fireEvent.input(editor, { target: { value: 'The untouched first direction' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Save changes' }));
    const study = (await api.listStudies('p-creator-lab'))[0];
    await waitFor(async () => {
      expect((await api.getStudy('p-creator-lab', study.id)).draft).toBe('The untouched first direction');
    });
    const first = await api.getStudy('p-creator-lab', study.id);
    const firstId = first.draftId!;
    await fireEvent.click(screen.getByRole('button', { name: 'Duplicate current script' }));
    await waitFor(async () => {
      expect((await api.getStudy('p-creator-lab', study.id)).drafts).toHaveLength(2);
    });
    const second = await api.getStudy('p-creator-lab', study.id);
    expect(second.draftId).not.toBe(firstId);
    expect(second.draft).toBe('The untouched first direction');
    await expect(api.duplicateStudyDraft('p-creator-lab', study.id, second.draftId!, 0))
      .rejects.toThrow();
    await expect(api.duplicateStudyDraft('p-client-a', study.id, firstId, 2))
      .rejects.toThrow();
    await fireEvent.input(editor, { target: { value: 'Independent second direction' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Save changes' }));
    await waitFor(async () => {
      expect((await api.getStudy('p-creator-lab', study.id)).draft).toBe('Independent second direction');
    });
    await fireEvent.click(screen.getByRole('button', { name: 'Rename script' }));
    await fireEvent.input(screen.getByRole('textbox', { name: 'Script candidate name' }), {
      target: { value: 'Arabic follow-up version' }
    });
    await fireEvent.click(screen.getByRole('button', { name: 'Save name' }));
    await waitFor(async () => {
      const current = await api.getStudy('p-creator-lab', study.id);
      expect(current.drafts?.some((item) => item.title === 'Arabic follow-up version')).toBe(true);
      expect(current.draft).toBe('Independent second direction');
    });
    await fireEvent.click(screen.getByRole('combobox', { name: 'Script candidates' }));
    await fireEvent.click(screen.getByRole('option', { name: 'Two original directions' }));
    expect(await screen.findByRole('textbox', { name: 'Script draft' }))
      .toHaveValue('The untouched first direction');
    const old = await api.getStudy('p-creator-lab', study.id, firstId);
    expect(old.draftVersions.some((snapshot) => snapshot.content === 'Independent second direction')).toBe(false);
    await fireEvent.click(screen.getByRole('combobox', { name: 'Script candidates' }));
    await fireEvent.click(screen.getByRole('option', { name: 'Arabic follow-up version' }));
    expect(await screen.findByRole('textbox', { name: 'Script draft' }))
      .toHaveValue('Independent second direction');
    await fireEvent.click(screen.getByRole('button', { name: 'Archive script' }));
    expect(screen.getByText(/hidden from active candidates/)).toBeInTheDocument();
    await fireEvent.click(screen.getByRole('button', { name: 'Confirm archive' }));
    await waitFor(async () => {
      const latest = await api.getStudy('p-creator-lab', study.id);
      expect(latest.drafts).toHaveLength(1);
      expect(latest.archivedDrafts).toHaveLength(1);
    });
    const archived = await api.getStudy('p-creator-lab', study.id);
    await expect(api.setStudyDraftArchived('p-creator-lab', study.id, firstId,
      archived.drafts![0].revision, true)).rejects.toThrow();
    await expect(api.setStudyDraftArchived('p-client-a', study.id,
      archived.archivedDrafts![0].id, archived.archivedDrafts![0].revision, false))
      .rejects.toThrow();
    expect(screen.queryByRole('button', { name: 'Archive script' })).not.toBeInTheDocument();
    await fireEvent.click(screen.getByRole('button', { name: 'Restore archived script' }));
    await waitFor(async () => {
      const latest = await api.getStudy('p-creator-lab', study.id);
      expect(latest.drafts).toHaveLength(2);
      expect(latest.archivedDrafts).toHaveLength(0);
      expect(latest.draft).toBe('Independent second direction');
    });
    view.unmount();

    render(App, { props: { api } });
    await screen.findByRole('heading', { name: 'Creator Lab' });
    await fireEvent.click(screen.getByRole('button', { name: 'Studio' }));
    await fireEvent.click(screen.getByRole('combobox', { name: 'Open study' }));
    await fireEvent.click(screen.getByRole('option', { name: 'Two original directions' }));
    expect(await screen.findByRole('combobox', { name: 'Script candidates' })).toBeInTheDocument();
    await fireEvent.click(screen.getByRole('combobox', { name: 'Script candidates' }));
    await fireEvent.click(screen.getByRole('option', { name: 'Two original directions' }));
    expect(await screen.findByRole('textbox', { name: 'Script draft' }))
      .toHaveValue('The untouched first direction');
  });

  it('filters a large candidate selector without losing the active script or defaulting to system controls', async () => {
    const api = createMockClient();
    const brief = { ...emptyBrief(), title: 'Candidate index' };
    let study = await api.createStudy('p-creator-lab', brief);
    for (let i = 0; i < 10; i++) {
      study = await api.duplicateStudyDraft('p-creator-lab', study.id, study.draftId!, study.draftRevision!);
      study = await api.renameStudyDraft('p-creator-lab', study.id, study.draftId!, study.draftRevision!, `Angle ${i}`);
    }
    const view = render(App, { props: { api } });
    await screen.findByRole('heading', { name: 'Creator Lab' });
    await fireEvent.click(screen.getByRole('button', { name: 'Studio' }));
    await fireEvent.click(screen.getByRole('combobox', { name: 'Open study' }));
    await fireEvent.click(screen.getByRole('option', { name: 'Candidate index' }));
    const search = await screen.findByRole('searchbox', { name: 'Search scripts' });
    await fireEvent.input(search, { target: { value: 'Angle 3' } });
    const picker = screen.getByRole('combobox', { name: 'Script candidates' });
    await fireEvent.click(picker);
    expect(screen.getByRole('option', { name: 'Angle 3' })).toBeInTheDocument();
    expect(screen.getByRole('option', { name: 'Angle 9' })).toBeInTheDocument();
    expect(screen.queryByRole('option', { name: 'Angle 4' })).not.toBeInTheDocument();
    await fireEvent.click(screen.getByRole('option', { name: 'Angle 3' }));
    expect(picker).toHaveAttribute('aria-haspopup', 'listbox');
    expect(await screen.findByRole('textbox', { name: 'Script draft' })).toBeInTheDocument();
    view.unmount();
  });

  it('prepares an evidence-aware three-angle prompt without calling any provider or including disputed claims', async () => {
    const api = createMockClient();
    const provider = vi.spyOn(api, 'runAi');
    const brief = { ...emptyBrief(), title: 'Three-angle study',
      goal: 'Make a fresh restaurant introduction', audience: 'Local food lovers' };
    const initial = await api.createStudy('p-creator-lab', brief);
    const sources = [
      { kind: 'transcript' as const, id: 't-ar' },
      { kind: 'transcript' as const, id: 't-en' },
      { kind: 'research' as const, id: 'r-1' }
    ];
    const saved = await api.saveStudy('p-creator-lab', initial.id, initial.revision, { sources });
    const observation = await api.addStudyEvidence('p-creator-lab', saved.id, {
      source: sources[0], claim: 'An unverified Arabic opening observation',
      locator: '0:00-0:07'
    });
    await expect(api.prepareStudyAngles('p-other', saved.id)).rejects.toThrow();
    const result = await api.prepareStudyAngles('p-creator-lab', saved.id);
    expect(result.attachedSourceCount).toBe(3);
    expect(result.transcriptExcerptCount).toBe(3);
    expect(result.unverifiedObservationCount).toBe(1);
    expect(result.prompt).toContain('Propose exactly 3');
    expect(result.prompt).toContain('An unverified Arabic opening observation');
    expect(provider).not.toHaveBeenCalled();

    render(App, { props: { api } });
    await screen.findByRole('heading', { name: 'Creator Lab' });
    await fireEvent.click(screen.getByRole('button', { name: 'Studio' }));
    await fireEvent.click(screen.getByRole('combobox', { name: 'Open study' }));
    await fireEvent.click(screen.getByRole('option', { name: 'Three-angle study' }));
    await fireEvent.click(screen.getByRole('button', { name: 'Prepare 3 creative angles' }));
    expect(await screen.findByText(/Three-angle prompt prepared locally/)).toBeInTheDocument();
    expect((screen.getByRole('textbox', { name: 'Prepared prompt' }) as HTMLTextAreaElement).value)
      .toContain('Propose exactly 3');
    expect(provider).not.toHaveBeenCalled();

    await api.reviewStudyEvidence('p-creator-lab', saved.id, observation.id, {
      expectedCertainty: 'unverified', action: 'dispute', note: 'Contradicted by full recording'
    });
    const disputed = await api.prepareStudyAngles('p-creator-lab', saved.id);
    expect(disputed.unverifiedObservationCount).toBe(0);
    expect(disputed.prompt).not.toContain('An unverified Arabic opening observation');
    const nineSources = [...sources, ...Array.from({ length: 6 }, (_, i) => ({
      kind: 'research' as const, id: 'extra-source-' + i
    }))];
    await api.saveStudy('p-creator-lab', saved.id, saved.revision, { sources: nineSources });
    await api.addStudyEvidence('p-creator-lab', saved.id, {
      source: nineSources[8], claim: 'Ninth-source observation must stay out of preview',
      locator: 'manual note'
    });
    const bounded = await api.prepareStudyAngles('p-creator-lab', saved.id);
    expect(bounded.attachedSourceCount).toBe(9);
    expect(bounded.limitations).toContain('Only the first eight selected sources were inspected.');
    expect(bounded.prompt).not.toContain('Ninth-source observation must stay out of preview');
    expect(provider).not.toHaveBeenCalled();
  });

  it('reviews model-proposed angles locally and prepares a chosen script prompt without provider egress', async () => {
    const api = createMockClient();
    const provider = vi.spyOn(api, 'runAi');
    const brief = { ...emptyBrief(), title: 'Original angle choices',
      goal: 'Make a clear short reel', audience: 'Local viewers' };
    const study = await api.createStudy('p-creator-lab', brief);
    const prepared = await api.prepareStudyAngles('p-creator-lab', study.id);
    const response = JSON.stringify({ angles: ['Problem', 'Contrast', 'Checklist'].map((name, i) => ({
      id: 'angle-' + i, approach: name + ' approach', hook: name + ' opening hook',
      promise: 'An original useful promise for the viewer',
      outline: [{ role: 'hook', intent: name + ' introduction', estimated_seconds: 8 }],
      evidence_ids: []
    })) });
    const interpreted = await api.interpretStudyAngles('p-creator-lab', study.id,
      study.revision, prepared.prompt, response);
    expect(interpreted.angles).toHaveLength(3);
    expect(interpreted.draftPrompt).toBeNull();
    await expect(api.interpretStudyAngles('p-client-a', study.id, study.revision,
      prepared.prompt, response)).rejects.toThrow();
    await expect(api.interpretStudyAngles('p-creator-lab', study.id, study.revision,
      'old context', response)).rejects.toThrow();
    await expect(api.interpretStudyAngles('p-creator-lab', study.id, study.revision,
      prepared.prompt, response.replace('"evidence_ids":[]', '"evidence_ids":["missing"]')))
      .rejects.toThrow();
    render(App, { props: { api } });
    await screen.findByRole('heading', { name: 'Creator Lab' });
    await fireEvent.click(screen.getByRole('button', { name: 'Studio' }));
    await fireEvent.click(screen.getByRole('combobox', { name: 'Open study' }));
    await fireEvent.click(screen.getByRole('option', { name: 'Original angle choices' }));
    await fireEvent.click(screen.getByRole('button', { name: 'Prepare 3 creative angles' }));
    expect(await screen.findByText(/Three-angle prompt prepared locally/)).toBeInTheDocument();
    await fireEvent.input(screen.getByRole('textbox', { name: 'Paste three-angle JSON response' }), {
      target: { value: response }
    });
    await fireEvent.click(screen.getByRole('button', { name: 'Validate 3 angles' }));
    expect(await screen.findByText('Problem approach')).toBeInTheDocument();
    expect(screen.getByText('Contrast approach')).toBeInTheDocument();
    expect(screen.getByText('Checklist approach')).toBeInTheDocument();
    const choose = screen.getAllByRole('button', { name: 'Prepare script for this angle' });
    await fireEvent.click(choose[1]);
    const single = await screen.findByRole('textbox', { name: 'Selected script prompt' });
    expect((single as HTMLTextAreaElement).value).toContain('Write one ORIGINAL ScriptDraft');
    expect((single as HTMLTextAreaElement).value).toContain('angle-1');
    expect(provider).not.toHaveBeenCalled();
    const unchanged = await api.getStudy('p-creator-lab', study.id);
    expect(unchanged.draft).toBe('');
    expect(unchanged.drafts).toHaveLength(1);
    const scriptJson = JSON.stringify({
      id: 'model-script-1', angle_id: 'angle-1',
      title: 'Fresh second angle script', language: 'Arabic',
      beats: [{
        id: 'hook-1', role: 'hook',
        narration: 'An original concrete opening for this audience',
        on_screen_text: null, shots: [], evidence_ids: [], manually_edited: true
      }],
      cta: null, caption: null,
      claims: [{ id: 'claim-1', beat_id: 'hook-1',
        assertion: 'A proposed claim requiring independent checking',
        status: 'unverified', evidence_ids: [] }]
    });
    const scriptPreview = await api.reviewStudyScript('p-creator-lab', study.id,
      study.revision, prepared.prompt, response, 'angle-1',
      (single as HTMLTextAreaElement).value, scriptJson, false);
    expect(scriptPreview.savedStudy).toBeNull();
    expect(scriptPreview.needsFactReviewCount).toBe(1);
    await expect(api.reviewStudyScript('p-client-a', study.id,
      study.revision, prepared.prompt, response, 'angle-1',
      (single as HTMLTextAreaElement).value, scriptJson, true)).rejects.toThrow();
    await fireEvent.input(screen.getByRole('textbox', { name: 'Paste structured script JSON' }), {
      target: { value: scriptJson }
    });
    await fireEvent.click(screen.getByRole('button', { name: 'Validate script locally' }));
    expect(await screen.findByText('Fresh second angle script')).toBeInTheDocument();
    expect((await api.getStudy('p-creator-lab', study.id)).drafts).toHaveLength(1);
    await fireEvent.click(screen.getByRole('button', { name: 'Save as new script candidate' }));
    expect((await api.getStudy('p-creator-lab', study.id)).drafts).toHaveLength(1);
    await fireEvent.click(screen.getByRole('button', { name: 'Confirm new candidate' }));
    await waitFor(async () => {
      const current = await api.getStudy('p-creator-lab', study.id);
      expect(current.drafts).toHaveLength(2);
      expect(current.draft).toContain('An original concrete opening');
    });
    await expect(api.reviewStudyScript('p-creator-lab', study.id,
      study.revision, prepared.prompt, response, 'angle-1',
      (single as HTMLTextAreaElement).value, scriptJson, true)).rejects.toThrow();
    expect((await api.getStudy('p-creator-lab', study.id)).drafts).toHaveLength(2);
    expect(provider).not.toHaveBeenCalled();
  });

  it('shows a localized accessible native angle denial for string/Error IPC failures without mutating Study data', async () => {
    const api = createMockClient();
    const provider = vi.spyOn(api, 'runAi');
    const save = vi.spyOn(api, 'saveStudy');
    const study = await api.createStudy('p-creator-lab', {
      ...emptyBrief(), title: 'Native rejection regression',
      goal: 'Describe practical original editing tips',
      audience: 'People learning video editing'
    });
    const original = await api.getStudy('p-creator-lab', study.id);
    const interpret = vi.spyOn(api, 'interpretStudyAngles');
    interpret.mockRejectedValueOnce('invalid angle response: expected exactly three angles');
    interpret.mockRejectedValueOnce(new Error('invalid angle response: model returned one angle'));

    render(App, { props: { api } });
    await screen.findByRole('heading', { name: 'Creator Lab' });
    await fireEvent.click(screen.getByRole('button', { name: 'Studio' }));
    await fireEvent.click(screen.getByRole('combobox', { name: 'Open study' }));
    await fireEvent.click(screen.getByRole('option', { name: 'Native rejection regression' }));
    await fireEvent.click(screen.getByRole('button', { name: 'Prepare 3 creative angles' }));
    expect(await screen.findByText(/Three-angle prompt prepared locally/)).toBeInTheDocument();

    const rejected = '{"angles":[{"id":"only-one","approach":"Incomplete"}]}';
    const textbox = screen.getByRole('textbox', { name: 'Paste three-angle JSON response' });
    await fireEvent.input(textbox, { target: { value: rejected } });
    await fireEvent.click(screen.getByRole('button', { name: 'Validate 3 angles' }));
    const denial = 'The angle response is invalid or its Study evidence changed.';
    const visibleDenial = await screen.findByRole('alert', { name: denial });
    expect(visibleDenial).toHaveTextContent(denial);
    expect(textbox).toHaveAttribute('aria-invalid', 'true');
    expect(textbox).toHaveAttribute('aria-describedby', 'studio-angle-error');
    await waitFor(() => expect(visibleDenial).toHaveFocus());
    expect(screen.queryByText('Incomplete')).not.toBeInTheDocument();

    await fireEvent.input(textbox, { target: { value: rejected + ' ' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Validate 3 angles' }));
    expect(await screen.findByRole('alert', { name: denial })).toHaveTextContent(denial);

    expect(await api.getStudy('p-creator-lab', study.id)).toEqual(original);
    expect(save).not.toHaveBeenCalled();
    expect(provider).not.toHaveBeenCalled();

    await act(async () => { setLocale('ar'); });
    const arabicTextbox = await screen.findByRole('textbox', {
      name: 'الصق استجابة الزوايا الثلاث بصيغة JSON'
    });
    await fireEvent.input(arabicTextbox, { target: { value: rejected } });
    interpret.mockRejectedValueOnce('invalid angle response');
    await fireEvent.click(screen.getByRole('button', { name: 'التحقق من الزوايا الثلاث' }));
    const arabicDenial = 'استجابة الزوايا غير صالحة أو تغير دليل الدراسة.';
    const arabicAlert = await screen.findByRole('alert', { name: arabicDenial });
    expect(arabicAlert).toHaveTextContent(arabicDenial);
    await waitFor(() => expect(arabicAlert).toHaveFocus());
    expect(await api.getStudy('p-creator-lab', study.id)).toEqual(original);
    expect(provider).not.toHaveBeenCalled();

    // The same installed-style UI session must recover without a reload,
    // re-approval, new Study revision, provider call, or canonical write.
    await act(async () => { setLocale('en'); });
    const recoveryJson = JSON.stringify({
      angles: ['Problem', 'Contrast', 'Checklist'].map((approach, index) => ({
        id: 'recovery-' + index,
        approach: approach + ' fresh viewpoint',
        hook: 'A distinct useful opening ' + index,
        promise: 'Useful takeaways for the reader',
        outline: [{ role: 'hook', intent: 'Introduce viewpoint ' + index, estimated_seconds: 8 }],
        evidence_ids: []
      }))
    });
    await fireEvent.input(screen.getByRole('textbox', {
      name: 'Paste three-angle JSON response'
    }), { target: { value: recoveryJson } });
    await fireEvent.click(screen.getByRole('button', { name: 'Validate 3 angles' }));
    expect(await screen.findByText('Checklist fresh viewpoint')).toBeInTheDocument();
    expect(screen.queryByRole('alert', { name: denial })).not.toBeInTheDocument();
    expect((await api.getStudy('p-creator-lab', study.id))).toEqual(original);
    expect(provider).not.toHaveBeenCalled();
    expect(save).not.toHaveBeenCalled();
  });

  it('correlates an accessible Studio rejection to its actual host nonce, not another request', async () => {
    const api = createMockClient();
    const study = await api.createStudy('p-creator-lab', {
      ...emptyBrief(), title: 'Native request binding',
      goal: 'Explain original editing', audience: 'People learning editing'
    });
    const before = await api.getStudy('p-creator-lab', study.id);
    const provider = vi.spyOn(api, 'runAi');
    const interpret = vi.spyOn(api, 'interpretStudyAngles');
    interpret.mockImplementationOnce(async (...args) => {
      throw new Error('SCRIPTOTAR_DENIAL:INVALID_STUDY_ANGLES:' + args[6]);
    });
    render(App, { props: { api } });
    await screen.findByRole('heading', { name: 'Creator Lab' });
    await fireEvent.click(screen.getByRole('button', { name: 'Studio' }));
    await fireEvent.click(screen.getByRole('combobox', { name: 'Open study' }));
    await fireEvent.click(screen.getByRole('option', { name: 'Native request binding' }));
    await fireEvent.click(screen.getByRole('button', { name: 'Prepare 3 creative angles' }));
    const textbox = await screen.findByRole('textbox', { name: 'Paste three-angle JSON response' });
    await fireEvent.input(textbox, { target: { value: '{"angles":[{"id":"bad"}]}' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Validate 3 angles' }));
    const id = interpret.mock.calls[0]?.[6];
    expect(id).toMatch(/^[0-9a-f]{32}$/);
    const alert = await screen.findByRole('alert');
    expect(alert).toHaveTextContent('Request ID: ' + id);
    expect(alert).toHaveAttribute('aria-label',
      'The angle response is invalid or its Study evidence changed. · request ' + id);
    await waitFor(() => expect(alert).toHaveFocus());

    interpret.mockImplementationOnce(async () => {
      throw new Error('SCRIPTOTAR_DENIAL:INVALID_STUDY_ANGLES:ffffffffffffffffffffffffffffffff');
    });
    await fireEvent.input(textbox, { target: { value: '{"angles":[]}' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Validate 3 angles' }));
    const unbound = await screen.findByRole('alert');
    expect(unbound).not.toHaveTextContent('Request ID:');
    expect(await api.getStudy('p-creator-lab', study.id)).toEqual(before);
    expect(provider).not.toHaveBeenCalled();
  });

  it('discards stale asynchronous angle validation after the user replaces the pasted response', async () => {
    const api = createMockClient();
    const study = await api.createStudy('p-creator-lab', {
      ...emptyBrief(), title: 'Angle cancellation regression',
      goal: 'Create a new short reel', audience: 'Local creators'
    });
    let rejectPrevious: (cause: unknown) => void = () => {};
    const interpret = vi.spyOn(api, 'interpretStudyAngles');
    interpret.mockImplementationOnce(() => new Promise((_, reject) => {
      rejectPrevious = reject;
    }));
    const original = await api.getStudy('p-creator-lab', study.id);
    render(App, { props: { api } });
    await screen.findByRole('heading', { name: 'Creator Lab' });
    await fireEvent.click(screen.getByRole('button', { name: 'Studio' }));
    await fireEvent.click(screen.getByRole('combobox', { name: 'Open study' }));
    await fireEvent.click(screen.getByRole('option', { name: 'Angle cancellation regression' }));
    await fireEvent.click(screen.getByRole('button', { name: 'Prepare 3 creative angles' }));
    expect(await screen.findByText(/Three-angle prompt prepared locally/)).toBeInTheDocument();
    const textbox = screen.getByRole('textbox', { name: 'Paste three-angle JSON response' });
    await fireEvent.input(textbox, { target: { value: '{"angles":[]}' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Validate 3 angles' }));
    await fireEvent.input(textbox, { target: { value: '{"angles":[{"id":"replaced"}]}' } });
    await act(async () => { rejectPrevious('invalid angle response for superseded JSON'); });
    expect(screen.queryByRole('alert', { name: /angle response is invalid/i })).not.toBeInTheDocument();
    expect(await api.getStudy('p-creator-lab', study.id)).toEqual(original);
    expect(screen.queryByText('Three angles validated locally. Choose one to prepare its script prompt.')).not.toBeInTheDocument();
  });

  it('requires approval for a durable local source check and never calls a provider', async () => {
    const api = createMockClient();
    const provider = vi.spyOn(api, 'runAi');
    const view = render(App, { props: { api } });
    await screen.findByRole('heading', { name: 'Creator Lab' });
    await fireEvent.click(screen.getByRole('button', { name: 'Studio' }));
    await fireEvent.input(screen.getByRole('textbox', { name: 'Study name' }), {
      target: { value: 'Local source audit' }
    });
    await fireEvent.click(screen.getByRole('button', { name: 'Create study' }));
    await fireEvent.click(screen.getByText('Choose source material', { selector: 'summary' }));
    await fireEvent.click(screen.getByRole('checkbox', { name: 'Hook breakdown — Arabic sample' }));
    await fireEvent.click(screen.getByRole('button', { name: 'Hook breakdown — Arabic sample' }));
    const prepare = await screen.findByRole('button', { name: 'Prepare local check' });
    await fireEvent.click(prepare);
    expect(await screen.findByText(/Awaiting your approval/)).toBeInTheDocument();
    expect(provider).not.toHaveBeenCalled();
    await fireEvent.click(screen.getByRole('button', { name: 'Approve local read' }));
    expect(await screen.findByText(/Transcript checked locally/)).toBeInTheDocument();
    expect(provider).not.toHaveBeenCalled();
    view.unmount();
    render(App, { props: { api } });
    await screen.findByRole('heading', { name: 'Creator Lab' });
    await fireEvent.click(screen.getByRole('button', { name: 'Studio' }));
    await fireEvent.click(screen.getByRole('combobox', { name: 'Open study' }));
    await fireEvent.click(screen.getByRole('option', { name: 'Local source audit' }));
    expect(await screen.findByText(/Transcript checked locally/)).toBeInTheDocument();
  });

  it('opens evidence without guessing title links and rejects wrong-project responses', async () => {
    const api = createMockClient({
      getTranscript: async (id) => ({ ...mockBootstrap.transcripts.find((item) => item.id === id)!, projectId: 'p-client-a' })
    });
    render(StudioView, { props: {
      api, projectId: 'p-creator-lab', research: mockBootstrap.research,
      transcripts: mockBootstrap.transcripts, jobs: [],
      incoming: { kind: 'transcript', id: 't-ar', projectId: 'p-creator-lab' }
    } });
    expect(await screen.findByRole('alert')).toHaveTextContent('different project');
    expect(screen.queryByText('أول ثلاث ثواني هي المكان الذي يقرر فيه المشاهد إذا كان سيكمل.')).not.toBeInTheDocument();
  });

  it('ignores a stale transcript response after switching the inspected source', async () => {
    let resolveOld!: (value: typeof mockBootstrap.transcripts[0]) => void;
    const oldPromise = new Promise<typeof mockBootstrap.transcripts[0]>((resolve) => { resolveOld = resolve; });
    const api = createMockClient({
      getTranscript: async (id) => id === 't-ar' ? oldPromise : mockBootstrap.transcripts[1]
    });
    const view = render(StudioView, { props: {
      api, projectId: 'p-creator-lab', research: [], transcripts: mockBootstrap.transcripts, jobs: [],
      incoming: { kind: 'transcript', id: 't-ar', projectId: 'p-creator-lab' }
    } });
    await fireEvent.click(screen.getByText('Choose source material', { selector: 'summary' }));
    await fireEvent.click(screen.getByRole('checkbox', { name: 'Caption pacing breakdown' }));
    await fireEvent.click(screen.getByRole('button', { name: 'Caption pacing breakdown' }));
    expect(await screen.findByText(/The caption is doing two jobs/)).toBeInTheDocument();
    resolveOld(mockBootstrap.transcripts[0]);
    await oldPromise;
    await waitFor(() => expect(screen.queryByText(/أول ثلاث ثواني هي المكان/)).not.toBeInTheDocument());
    view.unmount();
  });

  it('renders complete Arabic controls and preserves mixed-direction source names', async () => {
    setLocale('ar');
    mount(undefined, { incoming: { kind: 'transcript', id: 't-ar', projectId: 'p-creator-lab' } });
    expect(screen.getByRole('heading', { name: 'الاستوديو' })).toBeInTheDocument();
    expect(screen.getByRole('textbox', { name: 'مسودة النص' })).toHaveAttribute('dir', 'auto');
    expect(await screen.findByText(/أول ثلاث ثواني هي المكان/)).toBeInTheDocument();
    expect(screen.getAllByText('Hook breakdown — Arabic sample').length).toBeGreaterThan(0);
  });
});

describe('App integration with legacy expert tools', () => {
  it('routes the exact research item to Studio and keeps queue accessible', async () => {
    render(App, { props: { api: createMockClient() } });
    await screen.findByRole('heading', { name: 'Creator Lab' });
    await fireEvent.click(screen.getByRole('button', { name: /Research/ }));
    const row = screen.getByTestId('research-r-1');
    await fireEvent.click(within(row).getByRole('button', { name: 'Use in Studio' }));
    expect(await screen.findByRole('heading', { name: 'Studio' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Three cuts that make a hook feel faster' })).toBeInTheDocument();
    await fireEvent.click(screen.getByRole('button', { name: 'Open full queue' }));
    expect(await screen.findByRole('heading', { name: 'Jobs' })).toBeInTheDocument();
  });

  it('reopens the exact saved AI artifact from Library when the detail API is available', async () => {
    const { gateway } = studyFixture();
    const detail = vi.fn(async (projectId: string, runId: string) => ({
      id: runId, projectId, task: 'Viral breakdown', title: 'Caption pacing breakdown',
      prompt: 'Original prompt', result: 'Exactly saved answer from the historic run',
      createdAt: '2026-11-13T12:00:00Z'
    }));
    render(App, { props: { api: createMockClient(), studioGateway: { ...gateway, getAiArtifact: detail } } });
    await screen.findByRole('heading', { name: 'Creator Lab' });
    await fireEvent.click(screen.getByRole('button', { name: /Library/ }));
    await fireEvent.click(screen.getByRole('button', { name: 'Open AI run: Caption pacing breakdown' }));
    expect(await screen.findByText('Exactly saved answer from the historic run')).toBeInTheDocument();
    expect(detail).toHaveBeenCalledWith('p-creator-lab', 'a-1');
  });

  it('reopens saved Copy Prompt artifacts without fabricating a generated result', async () => {
    const { gateway } = studyFixture();
    render(App, { props: {
      api: createMockClient(), studioGateway: {
        ...gateway,
        getAiArtifact: async (projectId: string, id: string) => ({
          id, projectId, task: 'Viral breakdown', title: 'Caption pacing breakdown',
          prompt: 'Exact saved copy-only prompt', result: null,
          createdAt: '2026-11-13T12:00:00Z'
        })
      }
    } });
    await screen.findByRole('heading', { name: 'Creator Lab' });
    await fireEvent.click(screen.getByRole('button', { name: /Library/ }));
    await fireEvent.click(screen.getByRole('button', { name: 'Open AI run: Caption pacing breakdown' }));
    expect(await screen.findByText('Exact saved copy-only prompt')).toBeInTheDocument();
    expect(screen.queryByText('Exactly saved answer from the historic run')).not.toBeInTheDocument();
  });

  it('opens saved Copy Prompt history without inventing a generated answer', async () => {
    render(App, { props: { api: createMockClient() } });
    await screen.findByRole('heading', { name: 'Creator Lab' });
    await fireEvent.click(screen.getByRole('button', { name: /Library/ }));
    await fireEvent.click(screen.getByRole('button', { name: 'Open AI run: Caption pacing breakdown' }));
    expect(await screen.findByText('Saved prompt for Viral breakdown')).toBeInTheDocument();
    expect(screen.queryByText('Exactly saved answer from the historic run')).not.toBeInTheDocument();
  });

    it('keeps local draft edits when switching between Studio and Discover', async () => {
    render(App, { props: { api: createMockClient() } });
    await screen.findByRole('heading', { name: 'Creator Lab' });
    await fireEvent.click(screen.getByRole('button', { name: 'Studio' }));
    await fireEvent.input(screen.getByRole('textbox', { name: 'Script draft' }), {
      target: { value: 'A draft not yet saved to the backend' }
    });
    await fireEvent.click(screen.getByRole('button', { name: 'Discover more' }));
    expect(await screen.findByRole('heading', { name: /Research/ })).toBeInTheDocument();
    await fireEvent.click(screen.getByRole('button', { name: 'Studio' }));
    expect(screen.getByRole('textbox', { name: 'Script draft' }))
      .toHaveValue('A draft not yet saved to the backend');
    expect(screen.queryByText(/unsaved workspace; export your script/)).not.toBeInTheDocument();
  });

  it('opens the activity interface with real failure details', async () => {
    render(App, { props: { api: createMockClient() } });
    await screen.findByRole('heading', { name: 'Creator Lab' });
    await fireEvent.click(screen.getByRole('button', { name: 'Show activity' }));
    const drawer = screen.getByRole('region', { name: 'Job activity' });
    expect(within(drawer).getByText('Extractor returned an authentication error')).toBeInTheDocument();
    await fireEvent.click(within(drawer).getByRole('button', { name: 'Open full queue' }));
    expect(await screen.findByRole('heading', { name: 'Jobs' })).toBeInTheDocument();
  });
});
