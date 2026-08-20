import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import App from './App.svelte';
import { createMockClient } from './api/mockClient';

beforeEach(() => {
  window.localStorage.clear();
  delete document.documentElement.dataset.theme;
});

async function ready(api = createMockClient()) {
  render(App, { props: { api } });
  await screen.findByRole('heading', { name: 'Creator Lab' });
  return api;
}

describe('project management and entity navigation', () => {
  it('creates a project through the backend contract and selects it immediately', async () => {
    const api = createMockClient();
    const createProject = vi.spyOn(api, 'createProject');
    await ready(api);

    await fireEvent.click(screen.getByRole('button', { name: 'New project' }));
    const name = screen.getByLabelText('Project name');
    await fireEvent.input(name, { target: { value: 'Campaign Lab' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Create project' }));

    await waitFor(() => expect(createProject).toHaveBeenCalledWith('Campaign Lab'));
    expect(await screen.findByRole('heading', { name: 'Campaign Lab' })).toBeInTheDocument();
    expect(screen.getByLabelText('Project')).toHaveValue('p-created-1');
  });

  it('keeps project creation open and shows backend validation errors', async () => {
    const api = createMockClient();
    vi.spyOn(api, 'createProject').mockRejectedValueOnce(new Error('A project with that name already exists'));
    await ready(api);

    await fireEvent.click(screen.getByRole('button', { name: 'New project' }));
    await fireEvent.input(screen.getByLabelText('Project name'), { target: { value: 'Creator Lab' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Create project' }));

    expect(await screen.findByRole('alert')).toHaveTextContent('A project with that name already exists');
    expect(screen.getByRole('heading', { name: 'Create project' })).toBeInTheDocument();
  });

  it('opens the exact transcript selected from the dashboard', async () => {
    await ready();

    await fireEvent.click(screen.getByRole('button', { name: /Hook breakdown — Arabic sample/ }));

    expect(await screen.findByRole('heading', { name: 'Transcript workspace' })).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: 'Hook breakdown — Arabic sample' })).toBeInTheDocument();
  });
});
