import { mount } from 'svelte';
import App from './App.svelte';
import { createTauriClient, setApi } from './api';
import { initializeLocalization } from './i18n';
import './design/tokens.css';
import './app.css';
import './design/base.css';
import './theme.css';
import './rtl.css';

type GlobalTauri = {
  core?: {
    invoke?: <T>(command: string, args?: Record<string, unknown>) => Promise<T>;
  };
  event?: {
    listen?: <T>(event: string, handler: (event: { payload: T }) => void) => Promise<() => void>;
  };
};

type RuntimeWindow = Window & {
  __TAURI__?: GlobalTauri;
  __TAURI_INTERNALS__?: unknown;
};

const runtimeWindow = window as RuntimeWindow;
const globalTauri = runtimeWindow.__TAURI__;
const isTauriRuntime = Boolean(globalTauri || runtimeWindow.__TAURI_INTERNALS__);
if (globalTauri?.core?.invoke) {
  const listen = globalTauri.event?.listen
    ? <T>(event: string, handler: (event: { payload: T }) => void) =>
        globalTauri.event!.listen!(event, handler)
    : undefined;
  setApi(createTauriClient(globalTauri.core.invoke, listen));
} else if (isTauriRuntime) {
  throw new Error('Scriptotar is running inside Tauri, but the Tauri IPC bridge is unavailable.');
}

mount(App, {
  target: document.getElementById('app')!
});

initializeLocalization();
