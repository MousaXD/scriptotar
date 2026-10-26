import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
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

  it('validates OpenAI tunnel ID and supports an existing HTTPS hostname', async () => {
    const api = await enableReadOnly();
    const openai = vi.spyOn(api, 'mcpConnectOpenai');
    await fireEvent.click(screen.getByRole('radio', { name: /OpenAI Secure Tunnel/ }));
    await fireEvent.click(screen.getByRole('button', { name: 'Connect' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('Invalid tunnel ID');
    await fireEvent.input(screen.getByPlaceholderText('tunnel_0123456789abcdef0123456789abcdef'), { target: { value: 'tunnel_' + 'a'.repeat(32) } });
    await fireEvent.input(screen.getByPlaceholderText('sk-...'), { target: { value: 'sk-test-secret' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Connect' }));
    await screen.findByTestId('mcp-step-finish');
    expect(openai).toHaveBeenCalledWith('tunnel_' + 'a'.repeat(32), 'sk-test-secret', true);
    await fireEvent.click(screen.getByRole('button', { name: 'Disconnect & revoke URL' }));
    await fireEvent.click(screen.getByRole('radio', { name: /My own HTTPS hostname/ }));
    await fireEvent.input(screen.getByPlaceholderText('https://mcp.example.com'), { target: { value: 'https://mcp.example.com' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Connect' }));
    expect(await screen.findByTestId('mcp-public-url')).toHaveTextContent('https://mcp.example.com/mcp/p/');
  });
});
