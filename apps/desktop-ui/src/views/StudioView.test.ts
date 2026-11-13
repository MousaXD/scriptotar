import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { createMockClient, mockBootstrap } from '../api/mockClient';
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

  it('does not invent an old AI answer without a detail API', async () => {
    render(App, { props: { api: createMockClient() } });
    await screen.findByRole('heading', { name: 'Creator Lab' });
    await fireEvent.click(screen.getByRole('button', { name: /Library/ }));
    await fireEvent.click(screen.getByRole('button', { name: 'Open AI run: Caption pacing breakdown' }));
    expect(await screen.findByText(/Exact AI history is unavailable/)).toBeInTheDocument();
    expect(screen.queryByText('Exactly saved answer from the historic run')).not.toBeInTheDocument();
  });

    it('keeps unsaved local draft edits when switching between Studio and Discover', async () => {
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
    expect(screen.getByText(/unsaved workspace; export your script/)).toBeInTheDocument();
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
