import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import App from './App.svelte';
import { createMockClient, mockBootstrap } from './api/mockClient';
import { setLocale } from './i18n';
import type { BootstrapData, Job } from './types';

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason: Error) => void;
  const promise = new Promise<T>((resolvePromise, rejectPromise) => {
    resolve = resolvePromise;
    reject = rejectPromise;
  });
  return { promise, resolve, reject };
}

beforeEach(() => {
  window.localStorage.clear();
  setLocale('en');
});

describe('progressive desktop startup', () => {
  it('renders the shell and allows navigation before a slow bootstrap finishes', async () => {
    const bootstrap = deferred<BootstrapData>();
    const api = createMockClient({ bootstrap: () => bootstrap.promise });
    render(App, { props: { api } });

    expect(screen.getByRole('main')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Settings' })).toBeInTheDocument();
    expect(screen.getByRole('status')).toHaveAttribute('aria-busy', 'true');
    expect(screen.getByLabelText('Project')).toBeDisabled();
    expect(document.querySelector('.boot-screen')).not.toBeInTheDocument();

    await fireEvent.click(screen.getByRole('button', { name: 'Settings' }));
    bootstrap.resolve(structuredClone(mockBootstrap));
    expect(await screen.findByRole('heading', { name: 'Settings' })).toBeInTheDocument();
    expect(screen.queryByRole('status')).not.toBeInTheDocument();
  });

  it('keeps the shell mounted during a bootstrap failure and recovers on retry', async () => {
    let attempts = 0;
    const api = createMockClient({
      bootstrap: async () => {
        if (++attempts === 1) throw new Error('The database is locked');
        return structuredClone(mockBootstrap);
      }
    });
    render(App, { props: { api } });

    expect(await screen.findByRole('alert')).toHaveTextContent('The database is locked');
    expect(screen.getByRole('button', { name: 'Settings' })).toBeInTheDocument();
    await fireEvent.click(screen.getByRole('button', { name: 'Try again' }));
    expect(await screen.findByRole('heading', { name: 'Creator Lab' })).toBeInTheDocument();
    expect(attempts).toBe(2);
  });

  it('finds a project near the end of a large workspace without prebuilding every palette command', async () => {
    const projects = Array.from({ length: 75 }, (_, index) => ({
      ...mockBootstrap.projects[0],
      id: `project-${index}`,
      name: `Ordinary workspace ${index}`
    }));
    projects.push({ ...mockBootstrap.projects[0], id: 'project-target', name: 'Distinctive Zebra Workspace' });
    const api = createMockClient({
      bootstrap: async () => ({
        ...structuredClone(mockBootstrap),
        projects,
        activeProjectId: 'project-0'
      })
    });
    render(App, { props: { api } });
    await screen.findByRole('heading', { name: 'Ordinary workspace 0' });
    await fireEvent.input(screen.getByLabelText('Search workspace'), {
      target: { value: 'Distinctive Zebra' }
    });
    const matches = await screen.findAllByRole('button', { name: /Distinctive Zebra Workspace.*Project/ });
    expect(matches.length).toBeGreaterThan(0);
  });

  it('reserves listbox typeahead letters for the focused custom selector', async () => {
    render(App, { props: { api: createMockClient() } });
    await screen.findByRole('heading', { name: 'Creator Lab' });

    const listbox = document.createElement('div');
    listbox.setAttribute('role', 'listbox');
    listbox.setAttribute('tabindex', '0');
    screen.getByRole('main').appendChild(listbox);
    listbox.focus();
    await fireEvent.keyDown(listbox, { key: 'r' });

    expect(screen.getByRole('heading', { name: 'Creator Lab' })).toBeInTheDocument();
    expect(screen.queryByRole('heading', { name: 'Research' })).not.toBeInTheDocument();

    listbox.remove();
    await fireEvent.keyDown(window, { key: 'r' });
    expect(await screen.findByRole('heading', { name: 'Research' })).toBeInTheDocument();
  });

  it('reserves workspace letter shortcuts for plaintext-only editors and ARIA textboxes', async () => {
    render(App, { props: { api: createMockClient() } });
    await screen.findByRole('heading', { name: 'Creator Lab' });

    for (const attributes of [
      { contenteditable: 'plaintext-only' },
      { role: 'textbox' },
      { role: 'searchbox' }
    ]) {
      const editor = document.createElement('div');
      for (const [name, value] of Object.entries(attributes)) editor.setAttribute(name, value);
      editor.setAttribute('tabindex', '0');
      screen.getByRole('main').appendChild(editor);
      try {
        editor.focus();
        await fireEvent.keyDown(editor, { key: 'r' });
        expect(screen.getByRole('heading', { name: 'Creator Lab' })).toBeInTheDocument();
        expect(screen.queryByRole('heading', { name: 'Research' })).not.toBeInTheDocument();
      } finally {
        editor.remove();
      }
    }

    await fireEvent.keyDown(window, { key: 'r' });
    expect(await screen.findByRole('heading', { name: 'Research' })).toBeInTheDocument();
  });

  it('reserves workspace letter shortcuts for an open native dialog', async () => {
    render(App, { props: { api: createMockClient() } });
    await screen.findByRole('heading', { name: 'Creator Lab' });

    const dialog = document.createElement('dialog');
    dialog.setAttribute('open', '');
    document.body.appendChild(dialog);
    try {
      await fireEvent.keyDown(dialog, { key: 'r' });
      expect(screen.getByRole('heading', { name: 'Creator Lab' })).toBeInTheDocument();
      expect(screen.queryByRole('heading', { name: 'Research' })).not.toBeInTheDocument();
    } finally {
      dialog.remove();
    }

    await fireEvent.keyDown(window, { key: 'r' });
    expect(await screen.findByRole('heading', { name: 'Research' })).toBeInTheDocument();
  });

  it('prevents the browser shortcut when the command palette shortcut repeats', async () => {
    render(App, { props: { api: createMockClient() } });
    await screen.findByRole('heading', { name: 'Creator Lab' });

    await fireEvent.keyDown(window, { key: 'k', ctrlKey: true });
    expect(await screen.findByRole('dialog', { name: 'Command palette' })).toBeInTheDocument();

    for (const modifier of ['ctrlKey', 'metaKey'] as const) {
      const repeat = new KeyboardEvent('keydown', {
        key: 'k',
        [modifier]: true,
        bubbles: true,
        cancelable: true
      });
      window.dispatchEvent(repeat);
      expect(repeat.defaultPrevented).toBe(true);
      expect(screen.getByRole('dialog', { name: 'Command palette' })).toBeInTheDocument();
    }

    await fireEvent.keyDown(window, { key: 'Escape' });
    expect(screen.queryByRole('dialog', { name: 'Command palette' })).not.toBeInTheDocument();
  });

  it('keeps the command palette closed while another modal owns keyboard focus', async () => {
    render(App, { props: { api: createMockClient() } });
    await screen.findByRole('heading', { name: 'Creator Lab' });

    const modal = document.createElement('div');
    modal.setAttribute('role', 'dialog');
    modal.setAttribute('aria-modal', 'true');
    modal.setAttribute('aria-label', 'Project confirmation');
    document.body.appendChild(modal);
    try {
      await fireEvent.keyDown(modal, { key: 'k', ctrlKey: true });
      expect(screen.queryByRole('dialog', { name: 'Command palette' })).not.toBeInTheDocument();
    } finally {
      modal.remove();
    }

    await fireEvent.keyDown(window, { key: 'k', ctrlKey: true });
    expect(await screen.findByRole('dialog', { name: 'Command palette' })).toBeInTheDocument();
    await fireEvent.keyDown(window, { key: 'Escape' });
    expect(screen.queryByRole('dialog', { name: 'Command palette' })).not.toBeInTheDocument();
  });

  it('closes the command palette to expose an ordinary project-switch rejection', async () => {
    const api = createMockClient();
    vi.spyOn(api, 'selectProject').mockRejectedValueOnce(new Error('Project switch denied'));
    render(App, { props: { api } });
    await screen.findByRole('heading', { name: 'Creator Lab' });

    await fireEvent.input(screen.getByLabelText('Search workspace'), {
      target: { value: 'Client A' }
    });
    const palette = await screen.findByRole('dialog', { name: 'Command palette' });
    const projectCommand = within(palette).getAllByRole('button', { name: /Client A.*Project/ })[0];
    await fireEvent.click(projectCommand);

    expect(await screen.findByRole('alert')).toHaveTextContent('Project switch denied');
    expect(screen.queryByRole('dialog', { name: 'Command palette' })).not.toBeInTheDocument();
    expect(screen.getByRole('heading', { name: 'Creator Lab' })).toBeInTheDocument();
  });

  it('fails closed if a project switch changes the backend before its response fails', async () => {
    const api = createMockClient();
    const actualSelect = api.selectProject.bind(api);
    vi.spyOn(api, 'selectProject').mockImplementationOnce(async (projectId) => {
      await actualSelect(projectId);
      throw new Error('Transport interrupted after selecting the project');
    });
    render(App, { props: { api } });
    await screen.findByRole('heading', { name: 'Creator Lab' });

    await fireEvent.change(screen.getByLabelText('Project'), { target: { value: 'p-client-a' } });
    expect(await screen.findByRole('alert')).toHaveTextContent('The active project could not be verified');
    expect(screen.queryByRole('heading', { name: 'Creator Lab' })).not.toBeInTheDocument();
    expect(screen.getByLabelText('Project')).toBeDisabled();

    await fireEvent.click(screen.getByRole('button', { name: 'Try again' }));
    expect(await screen.findByRole('heading', { name: 'Client A' })).toBeInTheDocument();
  });

  it('fails closed if a rejected project switch cannot verify the backend state', async () => {
    const api = createMockClient();
    const actualBootstrap = api.bootstrap.bind(api);
    vi.spyOn(api, 'bootstrap')
      .mockImplementationOnce(actualBootstrap)
      .mockRejectedValueOnce(new Error('Connection unavailable while verifying'));
    vi.spyOn(api, 'selectProject').mockRejectedValueOnce(new Error('Project switch was rejected'));
    render(App, { props: { api } });
    await screen.findByRole('heading', { name: 'Creator Lab' });

    await fireEvent.change(screen.getByLabelText('Project'), { target: { value: 'p-client-a' } });
    expect(await screen.findByRole('alert')).toHaveTextContent('The active project could not be verified');
    expect(screen.queryByRole('heading', { name: 'Creator Lab' })).not.toBeInTheDocument();
    await fireEvent.click(screen.getByRole('button', { name: 'Try again' }));
    expect(await screen.findByRole('heading', { name: 'Creator Lab' })).toBeInTheDocument();
  });

  it('fails closed when project creation commits but its response is lost', async () => {
    const api = createMockClient();
    const actualCreate = api.createProject.bind(api);
    vi.spyOn(api, 'createProject').mockImplementationOnce(async (name) => {
      await actualCreate(name);
      throw new Error('Transport interrupted after creating the project');
    });
    render(App, { props: { api } });
    await screen.findByRole('heading', { name: 'Creator Lab' });
    await fireEvent.click(screen.getByRole('button', { name: 'New project' }));
    await fireEvent.input(screen.getByLabelText('Project name'), {
      target: { value: 'Durable Workspace' }
    });
    await fireEvent.click(screen.getByRole('button', { name: 'Create project' }));

    expect(await screen.findByRole('alert')).toHaveTextContent('The active project could not be verified');
    expect(screen.queryByRole('heading', { name: 'Creator Lab' })).not.toBeInTheDocument();
    await fireEvent.click(screen.getByRole('button', { name: 'Try again' }));
    expect(await screen.findByRole('heading', { name: 'Durable Workspace' })).toBeInTheDocument();
  });

  it('does not schedule job refreshes from an event after unmount', async () => {
    let notify: ((id: string) => void) | undefined;
    const listJobs = vi.fn(async () => [] as Job[]);
    const api = createMockClient({
      listJobs,
      subscribeJobChanges: async (listener) => {
        notify = listener;
        return () => {};
      }
    });
    const view = render(App, { props: { api } });
    await screen.findByRole('heading', { name: 'Creator Lab' });
    view.unmount();

    vi.useFakeTimers();
    try {
      notify?.('late-event');
      await vi.advanceTimersByTimeAsync(200);
      expect(listJobs).not.toHaveBeenCalled();
    } finally {
      vi.useRealTimers();
    }
  });

  it('does not apply stale job responses after switching projects', async () => {
    const jobs = deferred<Job[]>();
    let notify: ((id: string) => void) | undefined;
    const listJobs = vi.fn(() => jobs.promise);
    const api = createMockClient({
      listJobs,
      subscribeJobChanges: async (listener) => {
        notify = listener;
        return () => {};
      }
    });
    render(App, { props: { api } });
    await screen.findByRole('heading', { name: 'Creator Lab' });

    notify?.('j-1');
    await waitFor(() => expect(listJobs).toHaveBeenCalledTimes(1));
    await fireEvent.change(screen.getByLabelText('Project'), { target: { value: 'p-client-a' } });
    expect(await screen.findByRole('heading', { name: 'Client A' })).toBeInTheDocument();

    jobs.resolve([{ ...mockBootstrap.jobs[0], id: 'stale-job', title: 'STALE JOB FROM OTHER PROJECT' }]);
    await new Promise((resolve) => setTimeout(resolve, 0));
    expect(screen.getByRole('heading', { name: 'Client A' })).toBeInTheDocument();
    expect(screen.queryByText('STALE JOB FROM OTHER PROJECT')).not.toBeInTheDocument();
  });

  it('lets the new project refresh while the previous project poll is still pending', async () => {
    const stalled = deferred<Job[]>();
    let notify: ((id: string) => void) | undefined;
    const latestJob: Job = { ...mockBootstrap.jobs[0], id: 'new-project-job', title: 'NEW PROJECT JOB' };
    const listJobs = vi.fn()
      .mockImplementationOnce(() => stalled.promise)
      .mockResolvedValueOnce([latestJob]);
    const api = createMockClient({
      listJobs,
      subscribeJobChanges: async (listener) => {
        notify = listener;
        return () => {};
      }
    });
    render(App, { props: { api } });
    await screen.findByRole('heading', { name: 'Creator Lab' });

    notify?.('j-1');
    await waitFor(() => expect(listJobs).toHaveBeenCalledTimes(1));
    await fireEvent.change(screen.getByLabelText('Project'), { target: { value: 'p-client-a' } });
    expect(await screen.findByRole('heading', { name: 'Client A' })).toBeInTheDocument();

    notify?.('new-project-job');
    await waitFor(() => expect(listJobs).toHaveBeenCalledTimes(2));
    expect(await screen.findByText('NEW PROJECT JOB')).toBeInTheDocument();

    stalled.resolve([{ ...mockBootstrap.jobs[0], id: 'old-project-job', title: 'OLD PROJECT JOB' }]);
    await new Promise((resolve) => setTimeout(resolve, 0));
    expect(screen.queryByText('OLD PROJECT JOB')).not.toBeInTheDocument();
    expect(screen.getByText('NEW PROJECT JOB')).toBeInTheDocument();
  });

  it('rejects an empty bootstrap snapshot that still claims an active project', async () => {
    const api = createMockClient({
      bootstrap: async () => ({
        ...structuredClone(mockBootstrap),
        projects: [],
        activeProjectId: 'ghost-project'
      })
    });
    render(App, { props: { api } });

    expect(await screen.findByRole('alert')).toHaveTextContent('did not identify an active project');
    expect(screen.queryByRole('heading', { name: 'Create your first project' })).not.toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Try again' })).toBeInTheDocument();
  });

  it('fails closed on an inconsistent background bootstrap while retaining retry recovery', async () => {
    let bootstrapCalls = 0;
    let notify: ((id: string) => void) | undefined;
    const api = createMockClient({
      bootstrap: async () => {
        bootstrapCalls += 1;
        if (bootstrapCalls === 2) {
          return { ...structuredClone(mockBootstrap), activeProjectId: 'missing-after-refresh' };
        }
        return structuredClone(mockBootstrap);
      },
      listJobs: async () => structuredClone(mockBootstrap.jobs).map((job) =>
        job.id === 'j-1' ? { ...job, state: 'completed' as const } : job
      ),
      subscribeJobChanges: async (listener) => {
        notify = listener;
        return () => {};
      }
    });
    render(App, { props: { api } });
    await screen.findByRole('heading', { name: 'Creator Lab' });
    await waitFor(() => expect(notify).toBeTypeOf('function'));
    notify?.('j-1');

    expect(await screen.findByRole('alert')).toHaveTextContent('did not identify an active project');
    expect(screen.queryByRole('heading', { name: 'Creator Lab' })).not.toBeInTheDocument();
    expect(bootstrapCalls).toBe(2);
    await fireEvent.click(screen.getByRole('button', { name: 'Try again' }));
    expect(await screen.findByRole('heading', { name: 'Creator Lab' })).toBeInTheDocument();
    expect(bootstrapCalls).toBe(3);
  });

  it('does not show a different project when bootstrap returns an invalid active ID', async () => {
    const api = createMockClient({
      bootstrap: async () => ({ ...structuredClone(mockBootstrap), activeProjectId: 'missing-project' })
    });
    render(App, { props: { api } });

    expect(await screen.findByRole('alert')).toHaveTextContent('did not identify an active project');
    expect(screen.queryByRole('heading', { name: 'Creator Lab' })).not.toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Try again' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Settings' })).toBeInTheDocument();
  });

  it('keeps the previous project selected when the backend switch snapshot is inconsistent', async () => {
    const api = createMockClient({
      selectProject: async () => ({
        ...structuredClone(mockBootstrap),
        activeProjectId: 'p-client-a',
        projects: [structuredClone(mockBootstrap.projects[0])]
      })
    });
    render(App, { props: { api } });
    await screen.findByRole('heading', { name: 'Creator Lab' });
    await fireEvent.change(screen.getByLabelText('Project'), { target: { value: 'p-client-a' } });
    expect(await screen.findByRole('alert')).toHaveTextContent('did not match the backend response');
    expect(screen.queryByRole('heading', { name: 'Creator Lab' })).not.toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Try again' })).toBeInTheDocument();
    expect(screen.getByLabelText('Project')).toBeDisabled();
    await fireEvent.click(screen.getByRole('button', { name: 'Try again' }));
    expect(await screen.findByRole('heading', { name: 'Creator Lab' })).toBeInTheDocument();
    expect(screen.queryByText('The selected project did not match the backend response.')).not.toBeInTheDocument();
  });

  it('renders an RTL shell and localized skeleton while Arabic bootstrap is pending', async () => {
    setLocale('ar');
    const bootstrap = deferred<BootstrapData>();
    const api = createMockClient({ bootstrap: () => bootstrap.promise });
    render(App, { props: { api } });

    expect(document.documentElement.dir).toBe('rtl');
    expect(screen.getByRole('main')).toBeInTheDocument();
    expect(screen.getByRole('status')).toHaveAttribute('aria-busy', 'true');
    bootstrap.resolve(structuredClone(mockBootstrap));
    expect(await screen.findByRole('heading', { name: 'Creator Lab' })).toBeInTheDocument();
    expect(document.documentElement.dir).toBe('rtl');
  });

  it('warns against duplicate creation when a rejected request already persisted a project', async () => {
    const newlyCreated = {
      ...structuredClone(mockBootstrap.projects[0]),
      id: 'p-partially-created',
      name: 'Partially Created'
    };
    const api = createMockClient({
      bootstrap: vi.fn()
        .mockResolvedValueOnce(structuredClone(mockBootstrap))
        .mockResolvedValueOnce({
          ...structuredClone(mockBootstrap),
          projects: [...structuredClone(mockBootstrap.projects), newlyCreated]
        }),
      createProject: vi.fn().mockRejectedValueOnce(new Error('Failed to activate project'))
    });
    render(App, { props: { api } });
    await screen.findByRole('heading', { name: 'Creator Lab' });
    await fireEvent.click(screen.getByRole('button', { name: 'New project' }));
    await fireEvent.input(screen.getByLabelText('Project name'), {
      target: { value: 'Partially Created' }
    });
    await fireEvent.click(screen.getByRole('button', { name: 'Create project' }));

    expect(await screen.findByText(/may already have been created\. Check the project selector/)).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: 'Creator Lab' })).toBeInTheDocument();
    expect(screen.getByRole('option', { name: 'Partially Created' })).toBeInTheDocument();
    expect(api.createProject).toHaveBeenCalledTimes(1);
  });

  it('does not let a job event invalidate project creation while the mutation is pending', async () => {
    const creating = deferred<BootstrapData>();
    const listJobs = vi.fn(async () => structuredClone(mockBootstrap.jobs));
    let notify: ((id: string) => void) | undefined;
    const api = createMockClient({
      createProject: async () => creating.promise,
      listJobs,
      subscribeJobChanges: async (listener) => {
        notify = listener;
        return () => {};
      }
    });
    render(App, { props: { api } });
    await screen.findByRole('heading', { name: 'Creator Lab' });
    await waitFor(() => expect(notify).toBeTypeOf('function'));

    await fireEvent.click(screen.getByRole('button', { name: 'New project' }));
    await fireEvent.input(screen.getByLabelText('Project name'), {
      target: { value: 'Protected Project' }
    });
    await fireEvent.click(screen.getByRole('button', { name: 'Create project' }));
    expect(screen.getByRole('heading', { name: 'Create project' })).toBeInTheDocument();

    notify?.('queued-while-creating');
    await new Promise((resolve) => setTimeout(resolve, 170));
    expect(listJobs).not.toHaveBeenCalled();

    const project = {
      ...structuredClone(mockBootstrap.projects[0]),
      id: 'p-protected',
      name: 'Protected Project'
    };
    creating.resolve({
      ...structuredClone(mockBootstrap),
      projects: [...structuredClone(mockBootstrap.projects), project],
      activeProjectId: project.id
    });

    expect(await screen.findByRole('heading', { name: 'Protected Project' })).toBeInTheDocument();
    expect(screen.getByLabelText('Project')).toHaveValue('p-protected');
  });

  it('fails closed when creating a project returns the previously active project', async () => {
    const api = createMockClient({
      createProject: async () => structuredClone(mockBootstrap)
    });
    render(App, { props: { api } });
    await screen.findByRole('heading', { name: 'Creator Lab' });
    await fireEvent.click(screen.getByRole('button', { name: 'New project' }));
    await fireEvent.input(screen.getByLabelText('Project name'), {
      target: { value: 'A Newly Created Project' }
    });
    await fireEvent.click(screen.getByRole('button', { name: 'Create project' }));

    expect(await screen.findByRole('alert')).toHaveTextContent('did not match the backend response');
    expect(screen.queryByRole('heading', { name: 'Creator Lab' })).not.toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Try again' })).toBeInTheDocument();
  });

  it('rejects an existing different project as the result of createProject', async () => {
    const api = createMockClient({
      createProject: async () => ({
        ...structuredClone(mockBootstrap),
        activeProjectId: 'p-client-a'
      })
    });
    render(App, { props: { api } });
    await screen.findByRole('heading', { name: 'Creator Lab' });
    await fireEvent.click(screen.getByRole('button', { name: 'New project' }));
    await fireEvent.input(screen.getByLabelText('Project name'), {
      target: { value: 'Actually New Project' }
    });
    await fireEvent.click(screen.getByRole('button', { name: 'Create project' }));

    expect(await screen.findByRole('alert')).toHaveTextContent('did not match the backend response');
    expect(screen.queryByRole('heading', { name: 'Client A' })).not.toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Try again' })).toBeInTheDocument();
  });

  it('rejects a fresh project with an unexpected name in the create response', async () => {
    const api = createMockClient({
      createProject: async () => ({
        ...structuredClone(mockBootstrap),
        projects: [...structuredClone(mockBootstrap.projects), {
          ...structuredClone(mockBootstrap.projects[0]),
          id: 'new-but-wrong-name',
          name: 'Unexpected Project'
        }],
        activeProjectId: 'new-but-wrong-name'
      })
    });
    render(App, { props: { api } });
    await screen.findByRole('heading', { name: 'Creator Lab' });
    await fireEvent.click(screen.getByRole('button', { name: 'New project' }));
    await fireEvent.input(screen.getByLabelText('Project name'), {
      target: { value: 'Correct Project' }
    });
    await fireEvent.click(screen.getByRole('button', { name: 'Create project' }));

    expect(await screen.findByRole('alert')).toHaveTextContent('did not match the backend response');
    expect(screen.queryByRole('heading', { name: 'Unexpected Project' })).not.toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Try again' })).toBeInTheDocument();
  });

  it('fails closed on a malformed project creation result instead of exposing the old project', async () => {
    const api = createMockClient({
      createProject: async () => ({
        ...structuredClone(mockBootstrap),
        projects: [],
        activeProjectId: 'missing-created-project'
      })
    });
    render(App, { props: { api } });
    await screen.findByRole('heading', { name: 'Creator Lab' });
    await fireEvent.click(screen.getByRole('button', { name: 'New project' }));
    await fireEvent.input(screen.getByLabelText('Project name'), {
      target: { value: 'Uncertain Workspace' }
    });
    await fireEvent.click(screen.getByRole('button', { name: 'Create project' }));

    expect(await screen.findByRole('alert')).toHaveTextContent('created project was not available');
    expect(screen.queryByRole('heading', { name: 'Creator Lab' })).not.toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Try again' })).toBeInTheDocument();
  });

  it('prevents project switching and creation while an active-project watchlist write is pending', async () => {
    const saving = deferred<BootstrapData>();
    const saveWatchlist = vi.fn(() => saving.promise);
    const api = createMockClient({ saveWatchlist });
    const selectProject = vi.spyOn(api, 'selectProject');
    const createProject = vi.spyOn(api, 'createProject');
    render(App, { props: { api } });
    await screen.findByRole('heading', { name: 'Creator Lab' });
    await fireEvent.click(screen.getByRole('button', { name: 'Research' }));
    await screen.findByRole('heading', { name: 'Research' });
    await fireEvent.input(screen.getByRole('textbox', { name: 'Creator profile URL' }), {
      target: { value: 'https://example.com/pending-watchlist' }
    });
    await fireEvent.click(screen.getByRole('button', { name: 'Save watchlist' }));
    await waitFor(() => expect(saveWatchlist).toHaveBeenCalledTimes(1));

    const projectSelector = screen.getByLabelText('Project');
    expect(projectSelector).toBeDisabled();
    await fireEvent.change(projectSelector, { target: { value: 'p-client-a' } });
    expect(selectProject).not.toHaveBeenCalled();

    await fireEvent.keyDown(window, { key: 'd' });
    await screen.findByRole('heading', { name: 'Creator Lab' });
    await fireEvent.click(screen.getByRole('button', { name: 'New project' }));
    await fireEvent.input(screen.getByLabelText('Project name'), {
      target: { value: 'Must Wait' }
    });
    await fireEvent.click(screen.getByRole('button', { name: 'Create project' }));
    expect(createProject).not.toHaveBeenCalled();
    expect(await screen.findByText('Finish saving the watchlist before creating a project.')).toBeInTheDocument();

    saving.resolve(structuredClone(mockBootstrap));
    await waitFor(() => expect(projectSelector).not.toBeDisabled());
    await fireEvent.change(projectSelector, { target: { value: 'p-client-a' } });
    expect(await screen.findByRole('heading', { name: 'Client A' })).toBeInTheDocument();
  });

  it('releases the project-selection guard after a watchlist save fails', async () => {
    const saving = deferred<BootstrapData>();
    const api = createMockClient({ saveWatchlist: () => saving.promise });
    render(App, { props: { api } });
    await screen.findByRole('heading', { name: 'Creator Lab' });
    await fireEvent.click(screen.getByRole('button', { name: 'Research' }));
    await screen.findByRole('heading', { name: 'Research' });
    await fireEvent.input(screen.getByRole('textbox', { name: 'Creator profile URL' }), {
      target: { value: 'https://example.com/rejected-watchlist' }
    });
    await fireEvent.click(screen.getByRole('button', { name: 'Save watchlist' }));
    const projectSelector = screen.getByLabelText('Project');
    expect(projectSelector).toBeDisabled();

    saving.reject(new Error('Save refused'));
    await waitFor(() => expect(projectSelector).not.toBeDisabled());
    await fireEvent.change(projectSelector, { target: { value: 'p-client-a' } });
    await waitFor(() => expect(projectSelector).toHaveValue('p-client-a'));
    // Switching projects preserves the current view; the project title is on Dashboard.
    await fireEvent.keyDown(window, { key: 'd' });
    expect(await screen.findByRole('heading', { name: 'Client A' })).toBeInTheDocument();
  });

  it('fails closed when a watchlist save returns a different active project', async () => {
    const api = createMockClient({
      saveWatchlist: async () => ({
        ...structuredClone(mockBootstrap),
        activeProjectId: 'p-client-a'
      })
    });
    render(App, { props: { api } });
    await screen.findByRole('heading', { name: 'Creator Lab' });
    await fireEvent.click(screen.getByRole('button', { name: 'Research' }));
    await screen.findByRole('heading', { name: 'Research' });
    await fireEvent.input(screen.getByRole('textbox', { name: 'Creator profile URL' }), {
      target: { value: 'https://example.com/creator' }
    });
    await fireEvent.click(screen.getByRole('button', { name: 'Save watchlist' }));

    expect(await screen.findByRole('alert')).toHaveTextContent('watchlist response did not match the active project');
    expect(screen.queryByRole('heading', { name: 'Research' })).not.toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Try again' })).toBeInTheDocument();
  });

  it('allows creating a project from an otherwise empty workspace', async () => {
    const api = createMockClient({
      bootstrap: async () => ({ ...structuredClone(mockBootstrap), projects: [], activeProjectId: '' })
    });
    const createProject = vi.spyOn(api, 'createProject');
    render(App, { props: { api } });

    expect(await screen.findByRole('heading', { name: 'Create your first project' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Settings' })).toBeInTheDocument();
    await fireEvent.input(screen.getByLabelText('Project name'), { target: { value: 'My First Workspace' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Create project' }));
    await waitFor(() => expect(createProject).toHaveBeenCalledWith('My First Workspace'));
    expect(await screen.findByRole('heading', { name: 'My First Workspace' })).toBeInTheDocument();
  });
});
