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

async function chooseSetting(name: string, option: string) {
  await fireEvent.click(screen.getByRole('combobox', { name }));
  await fireEvent.click(screen.getByRole('option', { name: option }));
}

describe('Settings UX', () => {
  afterEach(() => setLocale('en'));

  it('starts clean, protects unsaved changes, and discards without saving', async () => {
    const { api } = setup();
    const save = vi.spyOn(api, 'saveSettings');
    const theme = screen.getByRole('combobox', { name: 'Theme' });
    const original = mockBootstrap.settings.appearance;
    expect(screen.getByRole('button', { name: 'Save changes' })).toBeDisabled();
    await chooseSetting('Theme', original === 'dark' ? 'System' : 'Dark');
    expect(screen.getByText('Unsaved changes')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Save changes' })).toBeEnabled();
    await fireEvent.click(screen.getByRole('button', { name: 'Discard changes' }));
    expect(theme).toHaveTextContent(original === 'dark' ? 'Dark' : 'System');
    expect(screen.queryByText('Unsaved changes')).not.toBeInTheDocument();
    expect(save).not.toHaveBeenCalled();
  });

  it('persists the selected appearance only on Save', async () => {
    const { api, onSaved } = setup();
    const save = vi.spyOn(api, 'saveSettings');
    const next = mockBootstrap.settings.appearance === 'system' ? 'dark' : 'system';
    await chooseSetting('Theme', next === 'dark' ? 'Dark' : 'System');
    expect(save).not.toHaveBeenCalled();
    await fireEvent.click(screen.getByRole('button', { name: 'Save changes' }));
    await waitFor(() => expect(save).toHaveBeenCalledWith(expect.objectContaining({ appearance: next })));
    expect(onSaved).toHaveBeenCalledWith(expect.objectContaining({ appearance: next }));
    await waitFor(() => expect(screen.queryByText('Unsaved changes')).not.toBeInTheDocument());
    expect(screen.getByRole('button', { name: 'Save changes' })).toBeDisabled();
    expect(screen.getByText(/Settings saved/)).toBeInTheDocument();
    await chooseSetting('Settings interface language', 'Arabic');
    expect(screen.getByText(/تم حفظ الإعدادات/)).toBeInTheDocument();
  });

  it('does not treat edits made during an in-flight save as persisted', async () => {
    const { api } = setup();
    let completeSave: (() => void) | undefined;
    const save = vi.spyOn(api, 'saveSettings').mockImplementation(
      () => new Promise<void>((resolve) => { completeSave = resolve; })
    );
    const device = screen.getByRole('combobox', { name: 'Device' });
    const originalDevice = mockBootstrap.settings.device;
    const changedDevice = originalDevice === 'cpu' ? 'auto' : 'cpu';
    await chooseSetting('Theme', mockBootstrap.settings.appearance === 'dark' ? 'System' : 'Dark');
    await fireEvent.click(screen.getByRole('button', { name: 'Save changes' }));
    await waitFor(() => expect(save).toHaveBeenCalledOnce());

    await chooseSetting('Device', changedDevice === 'auto' ? 'Automatic' : 'cpu');
    completeSave?.();
    await waitFor(() => expect(screen.getByRole('button', { name: 'Save changes' })).toBeEnabled());
    expect(device).toHaveTextContent(changedDevice === 'auto' ? 'Automatic' : 'cpu');
    expect(screen.getByText('Unsaved changes')).toBeInTheDocument();
    expect(save).toHaveBeenCalledWith(expect.objectContaining({ device: originalDevice }));
  });

  it('switches interface language to Arabic without changing transcription language', async () => {
    const { api } = setup();
    const save = vi.spyOn(api, 'saveSettings');
    await chooseSetting('Settings interface language', 'Arabic');
    expect(screen.getByRole('heading', { name: 'الإعدادات' })).toBeInTheDocument();
    expect(document.documentElement.dir).toBe('rtl');
    expect(screen.getByRole('combobox', { name: 'لغة الواجهة في الإعدادات' })).toHaveTextContent('العربية');
    expect(save).not.toHaveBeenCalled();
  });

  it('keeps filesystem paths left-to-right in Arabic Settings', () => {
    const settings = structuredClone(mockBootstrap.settings);
    settings.outputDirectory = '/home/مستخدم/الملفات';
    render(SettingsView, {
      props: {
        api: createMockClient(),
        settings,
        migrationStatus: structuredClone(mockBootstrap.migrationStatus)
      }
    });
    setLocale('ar');
    expect(screen.getByText('/home/مستخدم/الملفات')).toHaveAttribute('dir', 'ltr');
  });

  it('keeps legacy migration controls and warnings visible', () => {
    setup();
    expect(screen.getByTestId('migration-status')).toBeInTheDocument();
    expect(screen.getByText(/Discovery uses a read-only, WAL-aware SQLite snapshot/)).toBeInTheDocument();
  });
  it('keeps legacy migration retry actions connected to the host API', async () => {
    const { api } = setup();
    const retry = vi.spyOn(api, 'retryLegacyMigration');
    await fireEvent.click(screen.getByRole('button', { name: 'Retry migration discovery' }));
    await waitFor(() => expect(retry).toHaveBeenCalledOnce());
    expect(screen.getByTestId('migration-status')).toHaveAttribute('data-state', 'no_legacy_db');
  });

});
