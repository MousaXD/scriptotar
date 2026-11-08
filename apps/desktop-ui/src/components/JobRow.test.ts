import { render, screen } from '@testing-library/svelte';
import { beforeEach, describe, expect, it } from 'vitest';
import JobRow from './JobRow.svelte';
import StateBadge from './StateBadge.svelte';
import { setLocale } from '../i18n';
import type { Job } from '../types';

beforeEach(() => {
  window.localStorage.clear();
  setLocale('en');
});

function makeJob(progress: number | undefined, state: Job['state'] = 'transcribing'): Job {
  return {
    id: 'job-test', title: 'Arabic sample عربية', source: '/safe/example.mp4',
    updatedAt: 'now', state, stageLabel: 'Transcribing', progress,
    detail: 'Message from worker: retry /safe/example.mp4'
  };
}

describe('job status accessibility', () => {
  it('clamps out-of-range progress and preserves backend job data', () => {
    const { container } = render(JobRow, { props: { job: makeJob(150) } });
    expect(screen.getByRole('progressbar')).toHaveAttribute('aria-valuenow', '100');
    expect(screen.getByText('100%')).toBeInTheDocument();
    expect(container.querySelector('.progress-fill')).toHaveStyle({ width: '100%' });
    expect(screen.getByText('Arabic sample عربية')).toBeInTheDocument();
    expect(screen.getByText('Message from worker: retry /safe/example.mp4')).toBeInTheDocument();
  });

  it('treats invalid worker progress as unknown instead of showing NaN', () => {
    render(JobRow, { props: { job: makeJob(Number.NaN) } });
    expect(screen.queryByRole('progressbar')).not.toBeInTheDocument();
    expect(screen.getByRole('status')).toHaveTextContent('progress not reported by worker');
  });

  it('localizes failed-job guidance and state without changing the supplied error text', () => {
    setLocale('ar');
    render(JobRow, { props: { job: makeJob(undefined, 'failed') } });
    expect(screen.getByText('فشلت')).toBeInTheDocument();
    expect(screen.getByText('راجع تفاصيل الفشل، وعالج السبب المحتمل، ثم أعد محاولة المهمة.')).toBeInTheDocument();
    expect(screen.getByText('Message from worker: retry /safe/example.mp4')).toBeInTheDocument();
  });

  it('uses explicit translated state names for every job status', () => {
    setLocale('ar');
    render(StateBadge, { props: { state: 'interrupted' } });
    expect(screen.getByText('متوقفة')).toBeInTheDocument();
  });
});
