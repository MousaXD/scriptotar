import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { setLocale } from '../i18n';
import McpSetupWizard from './McpSetupWizard.svelte';
import { createMockClient } from '../api/mockClient';

async function setup() {
  const api = createMockClient();
  render(McpSetupWizard, { props: { api } });
  await screen.findByTestId('mcp-step-permissions');
  return api;
}

async function enableReadOnly() {
  const api = await setup();
  await fireEvent.click(screen.getByLabelText('Enable Scriptotar MCP'));
  await fireEvent.click(screen.getByLabelText('Read projects, jobs and transcripts'));
  await fireEvent.click(screen.getByRole('button', { name: 'Save permissions & continue' }));
  await screen.findByTestId('mcp-step-connection');
  return api;
}

describe('local MCP connection setup wizard', () => {
  afterEach(() => setLocale('en'));

  it('defaults to deny, reviews permissions then connects via Cloudflare and revokes URL', async () => {
    const api = await enableReadOnly();
    const connect = vi.spyOn(api, 'mcpConnectCloudflare');
    expect(screen.getByRole('radio', { name: /Cloudflare Quick Tunnel/ })).toBeChecked();
    await fireEvent.click(screen.getByRole('button', { name: 'Connect' }));
    await screen.findByTestId('mcp-step-finish');
    await waitFor(() => expect(connect).toHaveBeenCalledTimes(1));
    expect(screen.getByTestId('mcp-public-url').textContent).toMatch(/^https:\/\/sample-cloudflare\.trycloudflare\.com\/mcp\/p\/[a-f0-9]{64}$/);
    expect(screen.getByText(/treat this url like a password/i)).toBeInTheDocument();
    await fireEvent.click(screen.getByRole('button', { name: 'Disconnect & revoke URL' }));
    expect(await screen.findByTestId('mcp-step-connection')).toBeInTheDocument();
    expect(screen.queryByTestId('mcp-public-url')).not.toBeInTheDocument();
  });

  it('requires local approval to save powerful write and execute permissions', async () => {
    const api = await setup();
    const save = vi.spyOn(api, 'saveMcpPermissions');
    await fireEvent.click(screen.getByLabelText('Enable Scriptotar MCP'));
    await fireEvent.click(screen.getByRole('button', { name: 'Show advanced file and command permissions' }));
    await fireEvent.click(screen.getByLabelText('Execute approved programs'));
    expect(screen.getByRole('button', { name: 'Save permissions & continue' })).toBeDisabled();
    await fireEvent.click(screen.getByLabelText(/I approve powerful file and command access/));
    await fireEvent.click(screen.getByRole('button', { name: 'Save permissions & continue' }));
    await screen.findByTestId('mcp-step-connection');
    expect(save).toHaveBeenCalledWith(expect.objectContaining({ executeCommands: true }), true);
  });

  it('requires explicit opt-in before saving OpenAI key and supports an HTTPS hostname', async () => {
    const api = await enableReadOnly();
    const openai = vi.spyOn(api, 'mcpConnectOpenai');
    await fireEvent.click(screen.getByRole('radio', { name: /OpenAI Secure Tunnel/ }));
    await fireEvent.click(screen.getByRole('button', { name: 'Connect' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('Invalid tunnel ID');
    await fireEvent.input(screen.getByPlaceholderText('tunnel_0123456789abcdef0123456789abcdef'), { target: { value: 'tunnel_' + 'a'.repeat(32) } });
    await fireEvent.input(screen.getByPlaceholderText('sk-...'), { target: { value: 'sk-test-secret' } });
    const remember = screen.getByLabelText('Remember key in the OS keyring (never plain-text settings)');
    expect(remember).not.toBeChecked();
    await fireEvent.click(remember);
    expect(remember).toBeChecked();
    await fireEvent.click(screen.getByRole('button', { name: 'Connect' }));
    await screen.findByTestId('mcp-step-finish');
    expect(openai).toHaveBeenCalledWith('tunnel_' + 'a'.repeat(32), 'sk-test-secret', true);
    await fireEvent.click(screen.getByRole('button', { name: 'Disconnect & revoke URL' }));
    await fireEvent.click(screen.getByRole('radio', { name: /My own HTTPS hostname/ }));
    await fireEvent.input(screen.getByPlaceholderText('https://mcp.example.com'), { target: { value: 'https://mcp.example.com' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Connect' }));
    expect(await screen.findByTestId('mcp-public-url')).toHaveTextContent('https://mcp.example.com/mcp/p/');
  });

  it('shows unsaved permissions, disables next step and allows discarding without a backend write', async () => {
    const api = await setup();
    const save = vi.spyOn(api, 'saveMcpPermissions');
    const enabled = screen.getByLabelText('Enable Scriptotar MCP');
    expect(enabled).not.toBeChecked();
    await fireEvent.click(enabled);
    expect(screen.getByText('Unsaved permission changes')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /2 Connection/ })).toBeDisabled();
    await fireEvent.click(screen.getByRole('button', { name: 'Discard permission changes' }));
    expect(enabled).not.toBeChecked();
    expect(save).not.toHaveBeenCalled();
    expect(screen.queryByText('Unsaved permission changes')).not.toBeInTheDocument();
  });

  it('keeps newer permission edits unsaved when an earlier backend save completes', async () => {
    const api = await setup();
    const initiallySaved = await api.getMcpPermissions();
    let completeSave: ((policy: typeof initiallySaved) => void) | undefined;
    const save = vi.spyOn(api, 'saveMcpPermissions').mockImplementation(
      () => new Promise((resolve) => { completeSave = resolve; })
    );

    await fireEvent.click(screen.getByLabelText('Enable Scriptotar MCP'));
    await fireEvent.click(screen.getByLabelText('Read projects, jobs and transcripts'));
    await fireEvent.click(screen.getByRole('button', { name: 'Save permissions & continue' }));
    await waitFor(() => expect(save).toHaveBeenCalledOnce());
    expect(screen.getByRole('button', { name: 'Save permissions & continue' })).toBeDisabled();

    // The user changes a different grant while the first save is still in flight.
    await fireEvent.click(screen.getByLabelText('Search public creators'));
    completeSave?.({ ...initiallySaved, enabled: true, readData: true });

    await waitFor(() => expect(screen.getByRole('button', { name: 'Save permissions & continue' })).toBeEnabled());
    expect(screen.getByLabelText('Search public creators')).toBeChecked();
    expect(screen.getByText('Unsaved permission changes')).toBeInTheDocument();
    expect(screen.getByTestId('mcp-step-permissions')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /2 Connection/ })).toBeDisabled();
    expect(save).toHaveBeenCalledWith(expect.objectContaining({ searchCreators: false }), false);
  });

  it('renders Arabic permissions directly and preserves policy values when language changes', async () => {
    const api = await setup();
    const save = vi.spyOn(api, 'saveMcpPermissions');
    setLocale('ar');
    expect(await screen.findByRole('heading', { name: 'الخطوة 1 · الأذونات المحلية' })).toBeInTheDocument();
    await fireEvent.click(screen.getByLabelText('تفعيل MCP في Scriptotar'));
    await fireEvent.click(screen.getByLabelText('قراءة المشاريع والمهام والنصوص المفرغة'));
    expect(screen.getByText('تغييرات أذونات غير محفوظة')).toBeInTheDocument();
    await fireEvent.click(screen.getByRole('button', { name: 'حفظ الأذونات والمتابعة' }));
    await screen.findByTestId('mcp-step-connection');
    expect(save).toHaveBeenCalledWith(expect.objectContaining({ enabled: true, readData: true }), false);
    expect(screen.getByRole('heading', { name: 'الخطوة 2 · اختر طريقة الاتصال' })).toBeInTheDocument();
    expect(screen.getByText(/حُفظت الأذونات على هذا الجهاز/)).toBeInTheDocument();
    setLocale('en');
    expect(await screen.findByText(/Saved on this computer/)).toBeInTheDocument();
  });
  it('reveals existing approved paths and keeps identifiers LTR in Arabic', async () => {
    const original = createMockClient();
    const policy = await original.getMcpPermissions();
    policy.allowedPaths = ['/home/user/Approved'];
    const api = createMockClient({ getMcpPermissions: async () => policy });
    render(McpSetupWizard, { props: { api } });
    await screen.findByTestId('mcp-step-permissions');
    expect(screen.getByText(/Powerful access:/)).toBeInTheDocument();
    const paths = screen.getByPlaceholderText('/home/user/Documents/Scriptotar') as HTMLTextAreaElement;
    expect(paths).toHaveValue('/home/user/Approved');
    setLocale('ar');
    expect(paths).toHaveAttribute('dir', 'ltr');
    expect(paths).toHaveValue('/home/user/Approved');
    expect(document.documentElement.dir).toBe('rtl');
  });

  it('keeps the native setup steps keyboard focusable', async () => {
    await enableReadOnly();
    const step = screen.getByRole('button', { name: /2 Connection/ });
    step.focus();
    expect(step).toHaveFocus();
    expect(step).toHaveAttribute('aria-current', 'step');
    const first = screen.getByRole('button', { name: /1 Permissions/ });
    first.focus();
    expect(first).toHaveFocus();
  });

  it('invalidates powerful-access approval if the approved scope changes before save', async () => {
    await setup();
    await fireEvent.click(screen.getByLabelText('Enable Scriptotar MCP'));
    await fireEvent.click(screen.getByRole('button', { name: 'Show advanced file and command permissions' }));
    await fireEvent.click(screen.getByLabelText('Execute approved programs'));
    await fireEvent.click(screen.getByLabelText(/I approve powerful file and command access/));
    expect(screen.getByRole('button', { name: 'Save permissions & continue' })).toBeEnabled();
    await fireEvent.input(screen.getByPlaceholderText('/home/user/Documents/Scriptotar'), {
      target: { value: '/tmp/scriptotar-workspace' }
    });
    expect(screen.getByRole('button', { name: 'Save permissions & continue' })).toBeDisabled();
  });

});
