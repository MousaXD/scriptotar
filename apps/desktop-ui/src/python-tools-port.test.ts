import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import JobsView from './views/JobsView.svelte';
import AiStudioView from './views/AiStudioView.svelte';
import { extractUrlsFromText, countWords, formatSpeakingDuration } from './utils/textUtils';
import { createMockClient, mockBootstrap } from './api/mockClient';

describe('Python tools port helpers', () => {
  it('extracts unique URLs from arbitrary text', () => {
    const raw = `
      Check out this video: https://www.youtube.com/watch?v=12345
      And also this short: https://youtube.com/shorts/abcdef
      Duplicate: https://www.youtube.com/watch?v=12345
      Not a url: random text and ftp://invalid
    `;
    const urls = extractUrlsFromText(raw);
    expect(urls).toHaveLength(2);
    expect(urls).toContain('https://www.youtube.com/watch?v=12345');
    expect(urls).toContain('https://youtube.com/shorts/abcdef');
  });

  it('handles word counting and duration formatting correctly', () => {
    expect(countWords('')).toBe(0);
    expect(countWords('   hello world test   ')).toBe(3);

    expect(formatSpeakingDuration(0)).toBe('0:00');
    expect(formatSpeakingDuration(45)).toBe('0:45');
    expect(formatSpeakingDuration(75)).toBe('1:15');
    expect(formatSpeakingDuration(3665)).toBe('61:05');
  });
});

describe('JobsView clipboard and drag-drop tools', () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  it('extracts and enqueues URLs from the clipboard on button click', async () => {
    const enqueueUrlMock = vi.fn();
    const clipboardText = 'https://www.tiktok.com/@creator/video/987654\nhttps://www.youtube.com/watch?v=55555';

    Object.assign(navigator, {
      clipboard: {
        readText: vi.fn().mockResolvedValue(clipboardText),
      },
    });

    render(JobsView, {
      props: {
        jobs: mockBootstrap.jobs,
        onCancel: vi.fn(),
        onRetry: vi.fn(),
        onChooseLocal: vi.fn(),
        onEnqueueLocal: vi.fn(),
        onEnqueueUrl: enqueueUrlMock,
        onOpenCompleted: vi.fn(),
      },
    });

    const clipboardBtn = screen.getByTitle('Clipboard URLs');
    await fireEvent.click(clipboardBtn);

    await waitFor(() => {
      expect(enqueueUrlMock).toHaveBeenCalledTimes(2);
      expect(enqueueUrlMock).toHaveBeenCalledWith('https://www.tiktok.com/@creator/video/987654');
      expect(enqueueUrlMock).toHaveBeenCalledWith('https://www.youtube.com/watch?v=55555');
      expect(screen.getByText('Queued 2 URLs from clipboard.')).toBeInTheDocument();
    });
  });

  it('accepts web links dropped onto the capture panel', async () => {
    const enqueueUrlMock = vi.fn();

    render(JobsView, {
      props: {
        jobs: mockBootstrap.jobs,
        onCancel: vi.fn(),
        onRetry: vi.fn(),
        onChooseLocal: vi.fn(),
        onEnqueueLocal: vi.fn(),
        onEnqueueUrl: enqueueUrlMock,
        onOpenCompleted: vi.fn(),
      },
    });

    const capturePanel = screen.getByLabelText('Add transcription job');
    const droppedUrl = 'https://www.youtube.com/watch?v=dropped123';

    await fireEvent.drop(capturePanel, {
      dataTransfer: {
        getData: (format: string) => (format === 'text/plain' ? droppedUrl : ''),
      },
    });

    await waitFor(() => {
      const urlInput = screen.getByLabelText('Media URL');
      expect(urlInput).toHaveValue(droppedUrl);
      expect(screen.getByText(`Dropped link ${droppedUrl}. Ready to queue.`)).toBeInTheDocument();
    });
  });
});

describe('AiStudioView interactive speaking calculator', () => {
  it('updates spoken duration in real-time when text changes and pace changes', async () => {
    const api = createMockClient();
    render(AiStudioView, {
      props: {
        api,
        transcripts: [],
      },
    });

    const textarea = screen.getByPlaceholderText('Paste or load transcript/research text…');
    // 50 words
    const sampleText = Array(50).fill('word').join(' ');
    await fireEvent.input(textarea, { target: { value: sampleText } });

    // Standard pace is 2.5 w/s -> 50 / 2.5 = 20s
    expect(screen.getByText(/50 words · 249 chars/)).toBeInTheDocument();
    expect(screen.getByText(/~0:20 spoken/)).toBeInTheDocument();

    // Switch pace to Relaxed (Slow: 2.1 w/s) -> 50 / 2.1 = 23.8 -> 24s -> 0:24
    const paceSelect = screen.getByLabelText('Speaking pace');
    await fireEvent.change(paceSelect, { target: { value: 'slow' } });

    expect(screen.getByText(/~0:24 spoken/)).toBeInTheDocument();

    // Switch pace to Fast (2.9 w/s) -> 50 / 2.9 = 17.2 -> 18s -> 0:18
    await fireEvent.change(paceSelect, { target: { value: 'fast' } });

    expect(screen.getByText(/~0:18 spoken/)).toBeInTheDocument();
  });
});
