import { mount } from 'svelte';
import App from './App.svelte';
import { createTauriClient, setApi } from './api';
import './app.css';

type GlobalTauri = {
  core?: {
    invoke?: <T>(command: string, args?: Record<string, unknown>) => Promise<T>;
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
  setApi(createTauriClient(globalTauri.core.invoke));
} else if (isTauriRuntime) {
  throw new Error('Scriptotar is running inside Tauri, but the Tauri IPC bridge is unavailable.');
}

mount(App, {
  target: document.getElementById('app')!
});
