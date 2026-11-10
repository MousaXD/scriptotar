import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import SettingsView from './SettingsView.svelte';
import { createMockClient, mockBootstrap } from '../api/mockClient';
import { setLocale } from '../i18n';

function setup() {
  const api = createMockClient();
  const onSaved = vi.fn();
  render(SettingsView, {
    props: {
      api,
      settings: structuredClone(mockBootstrap.settings),
      migrationStatus: structuredClone(mockBootstrap.migrationStatus),
      onSaved
    }
  });
  return { api, onSaved };
}

describe('Settings UX', () => {
  afterEach(() => setLocale('en'));

  it('starts clean, protects unsaved changes, and discards without saving', async () => {
    const { api } = setup();
    const save = vi.spyOn(api, 'saveSettings');
    const theme = screen.getByRole('combobox', { name: 'Theme' }) as HTMLSelectElement;
    const original = theme.value;
    expect(screen.getByRole('button', { name: 'Save changes' })).toBeDisabled();
    await fireEvent.change(theme, { target: { value: original === 'dark' ? 'system' : 'dark' } });
    expect(screen.getByText('Unsaved changes')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Save changes' })).toBeEnabled();
    await fireEvent.click(screen.getByRole('button', { name: 'Discard changes' }));
    expect(theme.value).toBe(original);
    expect(screen.queryByText('Unsaved changes')).not.toBeInTheDocument();
    expect(save).not.toHaveBeenCalled();
  });

  it('persists the selected appearance only on Save', async () => {
    const { api, onSaved } = setup();
    const save = vi.spyOn(api, 'saveSettings');
    const theme = screen.getByRole('combobox', { name: 'Theme' }) as HTMLSelectElement;
    const next = theme.value === 'system' ? 'dark' : 'system';
    await fireEvent.change(theme, { target: { value: next } });
    expect(save).not.toHaveBeenCalled();
    await fireEvent.click(screen.getByRole('button', { name: 'Save changes' }));
    await waitFor(() => expect(save).toHaveBeenCalledWith(expect.objectContaining({ appearance: next })));
    expect(onSaved).toHaveBeenCalledWith(expect.objectContaining({ appearance: next }));
    await waitFor(() => expect(screen.queryByText('Unsaved changes')).not.toBeInTheDocument());
    expect(screen.getByRole('button', { name: 'Save changes' })).toBeDisabled();
  });

  it('switches interface language to Arabic without changing transcription language', async () => {
    const { api } = setup();
    const save = vi.spyOn(api, 'saveSettings');
    const select = screen.getByRole('combobox', { name: 'Settings interface language' });
    await fireEvent.change(select, { target: { value: 'ar' } });
    expect(screen.getByRole('heading', { name: 'الإعدادات' })).toBeInTheDocument();
    expect(document.documentElement.dir).toBe('rtl');
    expect(screen.getByRole('combobox', { name: 'لغة الواجهة في الإعدادات' })).toHaveValue('ar');
    expect(save).not.toHaveBeenCalled();
  });

  it('keeps legacy migration controls and warnings visible', () => {
    setup();
    expect(screen.getByTestId('migration-status')).toBeInTheDocument();
    expect(screen.getByText(/Discovery uses a read-only, WAL-aware SQLite snapshot/)).toBeInTheDocument();
  });
});
