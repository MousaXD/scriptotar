import type { ScriptotarApi } from './client';
import { createMockClient } from './mockClient';

let client: ScriptotarApi = createMockClient();

export function getApi(): ScriptotarApi { return client; }
export function setApi(next: ScriptotarApi): void { client = next; }
export type { ScriptotarApi, AiPromptInput, ResearchQuery } from './client';
export { createMockClient } from './mockClient';
export { createTauriClient } from './tauriClient';
