import { afterEach, describe, expect, it, vi } from 'vitest';
import { hasNativeFileDrop, subscribeToNativeFileDrop } from './tauriRuntime';

type Handler = (event: { payload: { type: string; paths?: string[] } }) => void;

afterEach(() => {
  delete (window as Window & { __TAURI__?: unknown }).__TAURI__;
});

describe('native Tauri file drop bridge', () => {
  it('reports unavailable outside the Tauri webview runtime', () => {
    expect(hasNativeFileDrop()).toBe(false);
  });

  it('normalizes native drop events and exposes the unlisten function', async () => {
    let handler: Handler | undefined;
    const unlisten = vi.fn();
    const onDragDropEvent = vi.fn(async (next: Handler) => {
      handler = next;
      return unlisten;
    });

    (window as Window & { __TAURI__?: unknown }).__TAURI__ = {
      webview: {
        getCurrentWebview: () => ({ onDragDropEvent })
      }
    };

    expect(hasNativeFileDrop()).toBe(true);

    const events: Array<{ type: string; paths: string[] }> = [];
    const stop = await subscribeToNativeFileDrop((event) => events.push(event));

    handler?.({ payload: { type: 'over' } });
    handler?.({ payload: { type: 'drop', paths: ['/tmp/clip.mp4', '/tmp/second.mov'] } });
    handler?.({ payload: { type: 'leave' } });

    expect(events).toEqual([
      { type: 'over', paths: [] },
      { type: 'drop', paths: ['/tmp/clip.mp4', '/tmp/second.mov'] },
      { type: 'leave', paths: [] }
    ]);

    stop?.();
    expect(unlisten).toHaveBeenCalledOnce();
  });
});
