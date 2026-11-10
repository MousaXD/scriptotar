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
  it('handles safe native picker selection and queues the exact selected path', async () => {
    const { onChooseLocal, onEnqueueLocal } = setup();
    await fireEvent.click(screen.getByRole('button', { name: 'Choose video' }));
    await waitFor(() => expect(onChooseLocal).toHaveBeenCalledOnce());
    expect(screen.getByText('recording.mp4')).toBeInTheDocument();
    await fireEvent.click(screen.getByRole('button', { name: 'Queue selected' }));
    await waitFor(() => expect(onEnqueueLocal).toHaveBeenCalledWith('/safe/recording.mp4'));
    expect(screen.queryByText('recording.mp4')).not.toBeInTheDocument();
  });

  it('accepts URL entry and preserves URL submission', async () => {
    const { onEnqueueUrl } = setup();
    await fireEvent.input(screen.getByRole('textbox', { name: 'Media URL' }), { target: { value: 'https://example.com/clip' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Queue URL' }));
    await waitFor(() => expect(onEnqueueUrl).toHaveBeenCalledWith('https://example.com/clip'));
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
    expect(screen.getByRole('alert')).toBeInTheDocument();
  });

  it('keeps controls accessible in Arabic without changing backend identifiers', async () => {
    setLocale('ar');
    const { onRetry } = setup([job('failed-ar', 'failed')]);
    await fireEvent.click(screen.getByRole('button', { name: 'إعادة المحاولة' }));
    expect(onRetry).toHaveBeenCalledWith('failed-ar');
  });
});
