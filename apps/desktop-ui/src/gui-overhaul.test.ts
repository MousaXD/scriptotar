import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import App from './App.svelte';
import { createMockClient } from './api/mockClient';
import { setLocale } from './i18n';

beforeEach(() => {
  window.localStorage.clear();
  setLocale('en');
  delete document.documentElement.dataset.theme;
});

async function ready() {
  await readyWithApi(createMockClient());
}

async function readyWithApi(api: ReturnType<typeof createMockClient>) {
  render(App, { props: { api } });
  await screen.findByRole('heading', { name: 'Creator Lab' });
}

describe('Creator Control Room GUI', () => {
  it('persists the compact sidebar preference', async () => {
    await ready();

    const collapse = screen.getByRole('button', { name: 'Collapse sidebar' });
    await fireEvent.click(collapse);

    expect(window.localStorage.getItem('scriptotar.sidebarCollapsed')).toBe('1');
    expect(screen.getByRole('button', { name: 'Expand sidebar' })).toBeInTheDocument();
  });

  it('opens the command palette from the keyboard and executes a command with Enter', async () => {
    await ready();

    await fireEvent.keyDown(window, { key: 'k', ctrlKey: true });
    expect(screen.getByRole('dialog', { name: 'Command palette' })).toBeInTheDocument();

    const input = screen.getByLabelText('Search workspace and commands');
    await fireEvent.input(input, { target: { value: 'Open Settings' } });
    await fireEvent.keyDown(input, { key: 'Enter' });

    expect(await screen.findByRole('heading', { name: 'Settings' })).toBeInTheDocument();
    expect(screen.queryByRole('dialog', { name: 'Command palette' })).not.toBeInTheDocument();
  });

  it('persists transcript rail visibility without changing transcript content', async () => {
    await ready();
    await fireEvent.click(screen.getByRole('button', { name: /Transcript/ }));

    const transcript = screen.getByTestId('transcript-content').textContent;
    await fireEvent.click(screen.getByRole('button', { name: 'Hide transcript list' }));

    expect(window.localStorage.getItem('scriptotar.transcriptListOpen')).toBe('0');
    expect(screen.getByRole('button', { name: 'Show transcript list' })).toBeInTheDocument();
    expect(screen.getByTestId('transcript-content').textContent).toBe(transcript);

    await fireEvent.click(screen.getByRole('button', { name: 'Hide details' }));
    expect(window.localStorage.getItem('scriptotar.transcriptDetailsOpen')).toBe('0');
    expect(screen.getByRole('button', { name: 'Show details' })).toBeInTheDocument();
  });

  it('surfaces unsaved settings after a local preference changes', async () => {
    await ready();
    await fireEvent.click(screen.getByRole('button', { name: /Settings/ }));

    expect(screen.queryByText('Unsaved changes')).not.toBeInTheDocument();
    await fireEvent.change(screen.getByLabelText('Theme'), { target: { value: 'system' } });

    await waitFor(() => expect(screen.getByText('Unsaved changes')).toBeInTheDocument());
  });

  it('switches the interface language from Settings and localizes new navigation chrome', async () => {
    await ready();
    await fireEvent.click(screen.getByRole('button', { name: /Settings/ }));

    await fireEvent.change(screen.getByLabelText('Settings interface language'), { target: { value: 'ar' } });

    await waitFor(() => expect(document.documentElement.dir).toBe('rtl'));
    expect(window.localStorage.getItem('scriptotar.uiLanguage')).toBe('ar');
    expect(screen.getByText('التفضيلات')).toBeInTheDocument();
    expect(screen.getByText('ترحيل البيانات القديمة')).toBeInTheDocument();
  });
  it('keeps utility navigation available to keyboard and assistive technology', async () => {
    await ready();
    expect(screen.getByRole('button', { name: 'Settings' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'MCP Setup' })).toBeInTheDocument();
    expect(screen.getByRole('link', { name: 'Skip to main workspace' })).toHaveAttribute('href', '#workspace-main');

    await fireEvent.click(screen.getByRole('button', { name: 'Collapse sidebar' }));
    expect(screen.getByRole('button', { name: 'Settings' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'MCP Setup' })).toBeInTheDocument();
  });

  it('traps Tab inside the command palette and returns focus to its opener', async () => {
    await ready();
    const opener = screen.getByRole('button', { name: 'Settings' });
    opener.focus();
    await fireEvent.keyDown(window, { key: 'k', ctrlKey: true });

    const dialog = screen.getByRole('dialog', { name: 'Command palette' });
    const search = within(dialog).getByRole('textbox', { name: 'Search workspace and commands' });
    expect(search).toHaveFocus();

    const buttons = within(dialog).getAllByRole('button');
    const last = buttons[buttons.length - 1];
    last.focus();
    await fireEvent.keyDown(last, { key: 'Tab' });
    expect(search).toHaveFocus();

    await fireEvent.keyDown(search, { key: 'Tab', shiftKey: true });
    expect(last).toHaveFocus();

    await fireEvent.keyDown(search, { key: 'Escape' });
    await waitFor(() => expect(opener).toHaveFocus());
    expect(screen.queryByRole('dialog', { name: 'Command palette' })).not.toBeInTheDocument();
  });

  it('can reopen search after Escape without a focus-driven dialog loop', async () => {
    await ready();
    const trigger = screen.getByRole('textbox', { name: 'Search workspace' });
    trigger.focus();
    const search = await screen.findByRole('textbox', { name: 'Search workspace and commands' });
    await fireEvent.keyDown(search, { key: 'Escape' });
    await waitFor(() => expect(trigger).toHaveFocus());
    expect(screen.queryByRole('dialog', { name: 'Command palette' })).not.toBeInTheDocument();

    await fireEvent.click(trigger);
    expect(await screen.findByRole('dialog', { name: 'Command palette' })).toBeInTheDocument();
  });

  it('localizes Library controls in Arabic without changing creator or transcript titles', async () => {
    await ready();
    await fireEvent.click(screen.getByRole('button', { name: 'Library' }));
    await fireEvent.change(screen.getByLabelText('Interface language'), { target: { value: 'ar' } });

    expect(await screen.findByRole('heading', { name: 'المكتبة' })).toBeInTheDocument();
    expect(screen.getByRole('group', { name: 'نوع عناصر المكتبة' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'إعداد MCP' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /فتح نص مفرغ: Hook breakdown — Arabic sample/ })).toBeInTheDocument();

    const search = screen.getByRole('textbox', { name: 'البحث في المكتبة' });
    await fireEvent.input(search, { target: { value: 'not present' } });
    expect(screen.getByText('لا توجد نتائج مطابقة')).toBeInTheDocument();
    expect(document.documentElement.dir).toBe('rtl');
  });

  it('recovers from a failed transcript read without trapping the user in a dead end', async () => {
    const api = createMockClient();
    const read = api.getTranscript.bind(api);
    const getTranscript = vi.spyOn(api, 'getTranscript').mockImplementation((id) =>
      id === 't-ar' ? Promise.reject(new Error('Transcript could not be read')) : read(id)
    );
    await readyWithApi(api);
    await fireEvent.click(screen.getByRole('button', { name: 'Transcripts' }));

    expect(await screen.findByRole('alert')).toHaveTextContent('Transcript could not be read');
    expect(screen.getByRole('button', { name: 'Retry loading' })).toBeInTheDocument();
    const picker = screen.getByRole('combobox', { name: 'Choose transcript' });
    await fireEvent.change(picker, { target: { value: 't-en' } });

    expect(await screen.findByRole('heading', { name: 'Caption pacing breakdown' })).toBeInTheDocument();
    expect(getTranscript).toHaveBeenCalledWith('t-en');
    expect(screen.getByTestId('transcript-content')).toHaveTextContent('The caption is doing two jobs');
  });

  it('localizes the dashboard while preserving the current project name', async () => {
    await ready();
    await fireEvent.change(screen.getByLabelText('Interface language'), { target: { value: 'ar' } });

    expect(await screen.findByText('المهام الحالية')).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: 'أحدث النصوص المفرغة' })).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: 'Creator Lab' })).toBeInTheDocument();

    await fireEvent.click(screen.getByRole('button', { name: 'فتح الطابور ←' }));
    expect(await screen.findByRole('heading', { name: 'المهام' })).toBeInTheDocument();
  });

});
