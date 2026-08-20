export type NativeDropEvent =
  | { type: 'enter' | 'over'; paths: string[] }
  | { type: 'drop'; paths: string[] }
  | { type: 'leave'; paths: [] };

export type UnlistenFn = () => void;

type TauriDragDropPayload =
  | { type: 'enter' | 'over'; paths?: string[]; position?: { x: number; y: number } }
  | { type: 'drop'; paths: string[]; position?: { x: number; y: number } }
  | { type: 'leave' };

type TauriWebview = {
  onDragDropEvent?: (handler: (event: { payload: TauriDragDropPayload }) => void) => Promise<UnlistenFn>;
};

type TauriGlobal = {
  webview?: {
    getCurrentWebview?: () => TauriWebview;
  };
};

type RuntimeWindow = Window & {
  __TAURI__?: TauriGlobal;
};

export function hasNativeFileDrop() {
  const runtime = window as RuntimeWindow;
  return Boolean(runtime.__TAURI__?.webview?.getCurrentWebview?.()?.onDragDropEvent);
}

export async function subscribeToNativeFileDrop(handler: (event: NativeDropEvent) => void): Promise<UnlistenFn | undefined> {
  const runtime = window as RuntimeWindow;
  const currentWebview = runtime.__TAURI__?.webview?.getCurrentWebview?.();
  if (!currentWebview?.onDragDropEvent) return undefined;

  return currentWebview.onDragDropEvent((event) => {
    const payload = event.payload;
    if (payload.type === 'leave') {
      handler({ type: 'leave', paths: [] });
      return;
    }

    handler({ type: payload.type, paths: payload.paths ?? [] });
  });
}
