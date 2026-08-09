import { mount } from 'svelte';
import App from './App.svelte';
import { createTauriClient, setApi } from './api';
import './app.css';

type GlobalTauri = {
  core?: {
    invoke?: <T>(command: string, args?: Record<string, unknown>) => Promise<T>;
  };
};

const globalTauri = (window as Window & { __TAURI__?: GlobalTauri }).__TAURI__;
if (globalTauri?.core?.invoke) {
  setApi(createTauriClient(globalTauri.core.invoke));
}

mount(App, {
  target: document.getElementById('app')!
});
