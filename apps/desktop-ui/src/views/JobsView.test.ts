import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import JobsView from './JobsView.svelte';
import { setLocale } from '../i18n';
import type { Job } from '../types';

const job = (id: string, state: Job['state']): Job => ({ id, title: `Job ${id}`, source: `/media/${id}.mp4`, state, stageLabel: 'Working', progress: 51, updatedAt: 'now', detail: state === 'failed' ? 'Permission denied' : undefined });
function setup(jobs: Job[] = []) {
  const props = {
    jobs, onCancel: vi.fn(async (_id: string) => {}), onRetry: vi.fn(async (_id: string) => {}),
    onChooseLocal: vi.fn(async () => '/safe/recording.mp4'),
    onEnqueueLocal: vi.fn(async (_path: string) => {}), onEnqueueUrl: vi.fn(async (_url: string) => {}),
    onOpenCompleted: vi.fn(async (_job: Job) => {})
  };
  return { ...render(JobsView, { props }), ...props };
}

beforeEach(() => { window.localStorage.clear(); setLocale('en'); delete (window as Window & { __TAURI__?: unknown }).__TAURI__; });

describe('jobs workspace', () => {
  it('offers a direct recovery action when a status filter has no matching jobs', async () => {
    const { container } = setup([job('finished', 'completed')]);
    await fireEvent.click(screen.getByRole('button', { name: /Needs attention/ }));
    expect(screen.getByText('No jobs in this filter')).toBeInTheDocument();
    await fireEvent.click(screen.getByRole('button', { name: 'Show all jobs' }));
    expect(container.querySelectorAll('[data-testid^="job-"]')).toHaveLength(1);
  });

  it('handles safe native picker selection and queues the exact selected path', async () => {
    const { onChooseLocal, onEnqueueLocal } = setup();
    await fireEvent.click(screen.getByRole('button', { name: 'Choose video' }));
    await waitFor(() => expect(onChooseLocal).toHaveBeenCalledOnce());
    expect(screen.getByText('recording.mp4')).toBeInTheDocument();
    expect(screen.getByText('/safe/recording.mp4')).toHaveAttribute('dir', 'ltr');
    await fireEvent.click(screen.getByRole('button', { name: 'Queue selected' }));
    await waitFor(() => expect(onEnqueueLocal).toHaveBeenCalledWith('/safe/recording.mp4'));
    expect(screen.queryByText('recording.mp4')).not.toBeInTheDocument();
  });

  it('accepts URL entry and preserves URL submission', async () => {
    const { onEnqueueUrl } = setup();
    await fireEvent.input(screen.getByRole('textbox', { name: 'Media URL' }), { target: { value: 'https://example.com/clip' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Queue URL' }));
    await waitFor(() => expect(onEnqueueUrl).toHaveBeenCalledWith('https://example.com/clip', expect.stringMatching(/^[0-9a-f]{32}$/)));
  });

  it('announces a rejected file: URL to assistive tech, without creating a job, and recovers on valid input', async () => {
    const { onEnqueueUrl, container } = setup();
    onEnqueueUrl.mockRejectedValueOnce('validation failed: research URL is invalid: only http and https are supported');
    const input = screen.getByRole('textbox', { name: 'Media URL' });
    await fireEvent.input(input, { target: { value: 'file:///scriptotar-native-qa-denied-media.mp4' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Queue URL' }));
    const message = 'Invalid media URL. Only HTTP and HTTPS URLs are supported.';
    const alert = await screen.findByRole('alert', { name: message });
    expect(alert).toHaveTextContent(message);
    await waitFor(() => expect(alert).toHaveFocus());
    expect(alert).toHaveAttribute('aria-live', 'assertive');
    expect(alert).toHaveAttribute('aria-atomic', 'true');
    expect(input).toHaveAttribute('aria-invalid', 'true');
    expect(input).toHaveAttribute('aria-describedby', 'queue-action-error');
    expect(onEnqueueUrl).toHaveBeenCalledWith('file:///scriptotar-native-qa-denied-media.mp4', expect.stringMatching(/^[0-9a-f]{32}$/));
    expect(container.querySelectorAll('[data-testid^="job-"]')).toHaveLength(0);

    await fireEvent.input(input, { target: { value: 'https://example.com/safe-video' } });
    expect(screen.queryByRole('alert')).not.toBeInTheDocument();
    expect(input).not.toHaveAttribute('aria-invalid');
    await fireEvent.click(screen.getByRole('button', { name: 'Queue URL' }));
    await waitFor(() => expect(onEnqueueUrl).toHaveBeenCalledWith('https://example.com/safe-video', expect.stringMatching(/^[0-9a-f]{32}$/)));
    expect(screen.queryByRole('alert')).not.toBeInTheDocument();
  });

  it('exposes the same backend URL rejection in Arabic with a named native alert', async () => {
    setLocale('ar');
    const { onEnqueueUrl } = setup();
    onEnqueueUrl.mockRejectedValueOnce(new Error('validation failed: research URL is invalid: only http and https are supported'));
    const input = screen.getByRole('textbox', { name: 'رابط الوسائط' });
    await fireEvent.input(input, { target: { value: 'file:///scriptotar-native-qa-denied-media.mp4' } });
    await fireEvent.click(screen.getByRole('button', { name: 'إضافة الرابط للطابور' }));
    const arabicMessage = 'رابط الوسائط غير صالح. يُسمح بروابط HTTP وHTTPS فقط.';
    const alert = await screen.findByRole('alert', { name: arabicMessage });
    expect(alert).toHaveTextContent(arabicMessage);
    await waitFor(() => expect(alert).toHaveFocus());
    expect(alert).toHaveAttribute('dir', 'auto');
    expect(input).toHaveAttribute('aria-invalid', 'true');
    expect(onEnqueueUrl).toHaveBeenCalledTimes(1);
  });

  it('exposes only the actual host-echoed Queue denial identity and ignores spoofed IDs', async () => {
    const { onEnqueueUrl } = setup();
    onEnqueueUrl.mockImplementationOnce(async (_url: string, requestId?: string) => {
      throw new Error('SCRIPTOTAR_DENIAL:INVALID_MEDIA_URL:' + requestId);
    });
    const input = screen.getByRole('textbox', { name: 'Media URL' });
    await fireEvent.input(input, { target: { value: 'file:///host-rejected.mp4' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Queue URL' }));
    const currentId = onEnqueueUrl.mock.calls[0]?.[1] as string;
    expect(currentId).toMatch(/^[0-9a-f]{32}$/);
    const alert = await screen.findByRole('alert');
    expect(alert).toHaveAttribute('aria-label',
      'Invalid media URL. Only HTTP and HTTPS URLs are supported. · request ' + currentId);
    expect(alert).toHaveTextContent('Request ID: ' + currentId);
    await waitFor(() => expect(alert).toHaveFocus());

    onEnqueueUrl.mockImplementationOnce(async () => {
      throw new Error('SCRIPTOTAR_DENIAL:INVALID_MEDIA_URL:ffffffffffffffffffffffffffffffff');
    });
    await fireEvent.input(input, { target: { value: 'file:///another-rejection.mp4' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Queue URL' }));
    const newer = await screen.findByRole('alert');
    expect(newer).not.toHaveTextContent('Request ID:');
    expect(newer).not.toHaveAttribute('aria-label', expect.stringContaining(' · request '));
  });

  it('allows cancel, retry and open completed transcript without hiding failure details', async () => {
    const { onCancel, onRetry, onOpenCompleted } = setup([job('1', 'transcribing'), job('2', 'failed'), job('3', 'completed')]);
    expect(screen.getByText('Permission denied')).toBeInTheDocument();
    await fireEvent.click(screen.getByRole('button', { name: 'Cancel' }));
    await fireEvent.click(screen.getByRole('button', { name: 'Retry' }));
    await fireEvent.click(screen.getByRole('button', { name: 'Open transcript' }));
    await waitFor(() => {
      expect(onCancel).toHaveBeenCalledWith('1');
      expect(onRetry).toHaveBeenCalledWith('2');
      expect(onOpenCompleted).toHaveBeenCalledWith(expect.objectContaining({ id: '3' }));
    });
  });

  it('filters active, interrupted, failed, cancelled and completed states', async () => {
    const { container } = setup([job('1', 'queued'), job('2', 'interrupted'), job('3', 'failed'), job('4', 'cancelled'), job('5', 'completed')]);
    await fireEvent.click(screen.getByRole('button', { name: /Needs attention/ }));
    expect(container.querySelectorAll('[data-testid^="job-"]')).toHaveLength(2);
    await fireEvent.click(screen.getByRole('button', { name: /Finished/ }));
    expect(container.querySelectorAll('[data-testid^="job-"]')).toHaveLength(2);
    await fireEvent.click(screen.getByRole('button', { name: /Active/ }));
    expect(container.querySelectorAll('[data-testid^="job-"]')).toHaveLength(1);
  });

  it('limits initial DOM work for large lists and exposes more jobs on demand', async () => {
    const { container } = setup(Array.from({ length: 75 }, (_, i) => job(`${i}`, 'completed')));
    expect(container.querySelectorAll('[data-testid^="job-"]')).toHaveLength(30);
    await fireEvent.click(screen.getByRole('button', { name: /Show more jobs/ }));
    expect(container.querySelectorAll('[data-testid^="job-"]')).toHaveLength(60);
  });

  it('handles URL drops and refuses unsafe filename-only browser file drops', async () => {
    const { container, onEnqueueLocal } = setup();
    const target = container.querySelector('.ingestion-workspace') as HTMLElement;
    await fireEvent.drop(target, { dataTransfer: { getData: (type: string) => type === 'text/plain' ? 'https://example.com/video' : '', files: [] } });
    expect(screen.getByRole('textbox', { name: 'Media URL' })).toHaveValue('https://example.com/video');
    await fireEvent.drop(target, { dataTransfer: { getData: () => '', files: [{ name: 'untrusted.mp4' }] } });
    expect(onEnqueueLocal).not.toHaveBeenCalled();
    expect(screen.getByRole('alert')).toHaveTextContent('A dropped filename is not a usable file path');
  });


  it('accepts native desktop drop paths and queues them through backend validation', async () => {
    let nativeHandler: ((event: { payload: { type: 'drop'; paths: string[] } }) => void) | undefined;
    (window as Window & { __TAURI__?: unknown }).__TAURI__ = {
      webview: {
        getCurrentWebview: () => ({
          onDragDropEvent: async (handler: typeof nativeHandler) => { nativeHandler = handler; return () => {}; }
        })
      }
    };
    const { onEnqueueLocal } = setup();
    await waitFor(() => expect(nativeHandler).toBeDefined());
    nativeHandler?.({ payload: { type: 'drop', paths: ['/safe/native-drop.mp4'] } });
    await screen.findByText('native-drop.mp4');
    await fireEvent.click(screen.getByRole('button', { name: 'Queue selected' }));
    await waitFor(() => expect(onEnqueueLocal).toHaveBeenCalledWith('/safe/native-drop.mp4'));
  });

  it('keeps manual paths behind a disclosure and submits them unchanged', async () => {
    const { onEnqueueLocal } = setup();
    await fireEvent.click(screen.getByText('Advanced: enter a local path manually'));
    await fireEvent.input(screen.getByRole('textbox', { name: 'Manual local media path' }), { target: { value: '/home/media/movie.mp4' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Queue path' }));
    await waitFor(() => expect(onEnqueueLocal).toHaveBeenCalledWith('/home/media/movie.mp4'));
  });

  it('queues all clipboard URLs rather than silently dropping extras', async () => {
    const readText = vi.fn(async () => 'https://example.org/1 https://example.org/2');
    Object.defineProperty(navigator, 'clipboard', { configurable: true, value: { readText } });
    const { onEnqueueUrl } = setup();
    await fireEvent.click(screen.getByRole('button', { name: 'Clipboard URLs' }));
    await waitFor(() => expect(onEnqueueUrl).toHaveBeenCalledTimes(2));
    expect(onEnqueueUrl).toHaveBeenNthCalledWith(1, 'https://example.org/1');
    expect(onEnqueueUrl).toHaveBeenNthCalledWith(2, 'https://example.org/2');
  });

  it('keeps controls accessible in Arabic without changing backend identifiers', async () => {
    setLocale('ar');
    const { onRetry } = setup([job('failed-ar', 'failed')]);
    await fireEvent.click(screen.getByRole('button', { name: 'إعادة المحاولة' }));
    expect(onRetry).toHaveBeenCalledWith('failed-ar');
  });
});
