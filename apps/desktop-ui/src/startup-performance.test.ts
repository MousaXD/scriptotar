import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
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
    expect(screen.getByRole('heading', { name: 'Creator Lab' })).toBeInTheDocument();
    expect(screen.getByLabelText('Project')).toHaveValue('p-creator-lab');
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
