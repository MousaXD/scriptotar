import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import App from './App.svelte';
import { createMockClient } from './api/mockClient';

async function ready(api = createMockClient()) {
  render(App, { props: { api } });
  await screen.findByRole('heading', { name: 'Creator Lab' });
  return api;
}

describe('desktop workstation', () => {
  it('switches projects through the backend contract', async () => {
    const api = createMockClient();
    const selectProject = vi.spyOn(api, 'selectProject');
    await ready(api);
    const select = screen.getByLabelText('Project');
    await fireEvent.change(select, { target: { value: 'p-client-a' } });
    await waitFor(() => expect(selectProject).toHaveBeenCalledWith('p-client-a'));
    expect(await screen.findByRole('heading', { name: 'Client A' })).toBeInTheDocument();
  });

  it('renders persisted job states including failure and interruption', async () => {
    await ready();
    await fireEvent.click(screen.getByRole('button', { name: /Jobs/ }));
    expect(screen.getByTestId('job-transcribing')).toBeInTheDocument();
    expect(screen.getByTestId('job-failed')).toBeInTheDocument();
    expect(screen.getByTestId('job-interrupted')).toBeInTheDocument();
  });

  it('switches AI Studio between Copy Prompt and BYOK without persisting a key in UI state', async () => {
    await ready();
    await fireEvent.click(screen.getByRole('button', { name: /AI Studio/ }));
    expect(screen.queryByLabelText('API key')).not.toBeInTheDocument();
    await fireEvent.click(screen.getByRole('button', { name: /BYOK/ }));
    expect(screen.getByTestId('byok-fields')).toBeInTheDocument();
    expect(screen.getByLabelText('API key')).toHaveAttribute('autocomplete', 'off');
  });

  it('filters and sorts research results', async () => {
    await ready();
    await fireEvent.click(screen.getByRole('button', { name: /Research/ }));
    const sort = screen.getByLabelText('Research sort');
    await fireEvent.change(sort, { target: { value: 'date' } });
    expect(screen.getAllByTestId(/research-r-/)[0]).toHaveAttribute('data-testid', 'research-r-2');
    const filter = screen.getByLabelText('Filter research');
    await fireEvent.input(filter, { target: { value: 'caption pacing' } });
    expect(screen.getByTestId('research-r-4')).toBeInTheDocument();
    expect(screen.queryByTestId('research-r-1')).not.toBeInTheDocument();
  });

  it('searches timestamped transcript segments', async () => {
    await ready();
    await fireEvent.click(screen.getByRole('button', { name: /Transcript/ }));
    const search = screen.getByLabelText('Search transcript');
    await fireEvent.input(search, { target: { value: 'النتيجة' } });
    expect(screen.getByText(/ابدأ بالنتيجة/)).toBeInTheDocument();
    expect(screen.queryByText(/أول ثلاث ثواني/)).not.toBeInTheDocument();
  });

  it('renders Arabic transcript content with RTL direction', async () => {
    await ready();
    await fireEvent.click(screen.getByRole('button', { name: /Transcript/ }));
    expect(screen.getByTestId('transcript-content')).toHaveAttribute('dir', 'rtl');
  });

  it('shows a recoverable error state when bootstrap fails', async () => {
    render(App, { props: { api: createMockClient({ bootstrap: async () => { throw new Error('Database unavailable'); } }) } });
    const alert = await screen.findByRole('alert');
    expect(alert).toHaveTextContent('Database unavailable');
    expect(screen.getByRole('button', { name: 'Try again' })).toBeInTheDocument();
  });
});
