import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import DashboardView from './DashboardView.svelte';
import { setLocale } from '../i18n';
import type { AiRun, Creator, Job, Project, Transcript } from '../types';

const project: Project = { id: 'p1', name: 'Creator project', updatedAt: 'now', itemCount: 0 };
const failed: Job = { id: 'bad', title: 'Failed media', source: 'clip', state: 'failed', stageLabel: '', updatedAt: 'now', detail: 'Decoder crashed' };
const processing: Job = { id: 'active', title: 'Processing media', source: '/media/a.mp4', state: 'transcribing', stageLabel: 'Transcribing', progress: 32, updatedAt: 'now' };
const transcript: Transcript = { id: 't1', projectId: 'p1', title: 'Latest transcript', language: 'Arabic', direction: 'rtl', source: '', platform: '', durationSeconds: 19, createdAt: 'now', text: '', segments: [] };
const creator: Creator = { id: 'c1', name: 'Creator A', handle: '@creator', platform: 'YouTube', watchlisted: true };
const aiRun: AiRun = { id: 'a1', title: 'Idea draft', task: 'Outline', mode: 'copy', createdAt: 'now', status: 'completed' };

function setup(overrides: Partial<{ project: Project; jobs: Job[]; transcripts: Transcript[]; creators: Creator[]; aiRuns: AiRun[] }> = {}) {
  const onNavigate = vi.fn();
  const onCreateProject = vi.fn(async (_name: string) => {});
  const onOpenTranscript = vi.fn();
  const result = render(DashboardView, { props: {
    project, jobs: [], transcripts: [], creators: [], aiRuns: [], onNavigate, onCreateProject, onOpenTranscript, ...overrides
  } });
  return { ...result, onNavigate, onCreateProject, onOpenTranscript };
}

beforeEach(() => { window.localStorage.clear(); setLocale('en'); });

describe('dashboard creator workflow', () => {
  it('starts transcription from the primary action and retains research, queue and project actions', async () => {
    const { onNavigate, onCreateProject } = setup();
    await fireEvent.click(screen.getAllByRole('button', { name: 'Add transcription job' })[0]);
    expect(onNavigate).toHaveBeenCalledWith('jobs');
    await fireEvent.click(screen.getByRole('button', { name: 'Jobs' }));
    expect(onNavigate).toHaveBeenCalledWith('jobs');
    await fireEvent.click(screen.getAllByRole('button', { name: 'New research scan' })[0]);
    expect(onNavigate).toHaveBeenCalledWith('research');
    await fireEvent.click(screen.getByRole('button', { name: 'New project' }));
    await fireEvent.input(screen.getByLabelText('Project name'), { target: { value: 'Second project' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Create project' }));
    await waitFor(() => expect(onCreateProject).toHaveBeenCalledWith('Second project'));
  });

  it('keeps project creation visible after the backend rejects the name', async () => {
    const { onCreateProject } = setup();
    onCreateProject.mockRejectedValueOnce(new Error('Duplicate project'));
    await fireEvent.click(screen.getByRole('button', { name: 'New project' }));
    await fireEvent.input(screen.getByLabelText('Project name'), { target: { value: 'Duplicate' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Create project' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('Duplicate project');
    expect(screen.getByRole('heading', { name: 'Create project' })).toBeInTheDocument();
  });

  it('places failed jobs ahead of active jobs with diagnostics and active progress', () => {
    const { container } = setup({ jobs: [processing, failed], transcripts: [transcript], creators: [creator], aiRuns: [aiRun] });
    const rows = Array.from(container.querySelectorAll('[data-testid^="job-"]'));
    expect(rows.map((row) => row.getAttribute('data-testid'))).toEqual(['job-failed', 'job-transcribing']);
    expect(screen.getByText('Decoder crashed')).toBeInTheDocument();
    expect(screen.getByRole('progressbar')).toHaveAttribute('aria-valuenow', '32');
    expect(screen.getByText('1 needs attention')).toBeInTheDocument();
  });

  it('still shows active jobs when failed jobs would otherwise fill the dashboard', () => {
    const failedJobs = Array.from({ length: 8 }, (_, i) => ({ ...failed, id: `failed-${i}` }));
    const activeJobs = Array.from({ length: 3 }, (_, i) => ({ ...processing, id: `active-${i}` }));
    const { container } = setup({ jobs: [...failedJobs, ...activeJobs] });
    const rows = Array.from(container.querySelectorAll('[data-testid^="job-"]'));
    expect(rows).toHaveLength(6);
    expect(rows.filter((row) => row.getAttribute('data-testid') === 'job-failed')).toHaveLength(4);
    expect(rows.filter((row) => row.getAttribute('data-testid') === 'job-transcribing')).toHaveLength(2);
  });

  it('opens recent transcripts and keeps secondary workflows accessible', async () => {
    const { onOpenTranscript, onNavigate } = setup({ transcripts: [transcript], creators: [creator], aiRuns: [aiRun] });
    const recent = screen.getByRole('region', { name: 'Recent transcripts' });
    await fireEvent.click(within(recent).getByRole('button', { name: /Latest transcript/ }));
    expect(onOpenTranscript).toHaveBeenCalledWith('t1');
    await fireEvent.click(screen.getByRole('button', { name: /Idea draft/ }));
    expect(onNavigate).toHaveBeenCalledWith('ai');
    await fireEvent.click(screen.getByRole('button', { name: /Creator A/ }));
    expect(onNavigate).toHaveBeenCalledWith('research');
  });

  it('renders Arabic and long project names without modifying their content', () => {
    setLocale('ar');
    const name = 'مشروع طويل جداً '.repeat(10);
    setup({ project: { ...project, name } });
    expect(screen.getByRole('heading', { level: 1 })).toHaveTextContent(name.trim());
    expect(screen.getByRole('button', { name: 'إضافة مهمة تفريغ' })).toBeInTheDocument();
  });

  it('shows the selected project identity while switching remains in the application shell', () => {
    setup({ project: { ...project, id: 'p2', name: 'Other project' } });
    expect(screen.getByRole('heading', { name: 'Other project' })).toBeInTheDocument();
  });

  it('shows a queue entry point for an empty project', () => {
    const { onNavigate } = setup();
    expect(screen.getByRole('region', { name: 'Current jobs' })).toBeInTheDocument();
    expect(screen.queryByRole('progressbar')).not.toBeInTheDocument();
    expect(onNavigate).not.toHaveBeenCalled();
  });
});
