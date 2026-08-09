import type { AppSettings } from './types';

export type AppearanceMode = AppSettings['appearance'];
export const APPEARANCE_STORAGE_KEY = 'scriptotar.appearance';

export function loadAppearance(fallback: AppearanceMode): AppearanceMode {
  if (typeof window === 'undefined') return fallback;
  try {
    const stored = window.localStorage.getItem(APPEARANCE_STORAGE_KEY);
    return stored === 'dark' || stored === 'system' ? stored : fallback;
  } catch {
    return fallback;
  }
}

export function applyAppearance(mode: AppearanceMode): void {
  if (typeof document !== 'undefined') {
    document.documentElement.dataset.theme = mode;
  }
}

export function persistAppearance(mode: AppearanceMode): void {
  if (typeof window !== 'undefined') {
    try { window.localStorage.setItem(APPEARANCE_STORAGE_KEY, mode); }
    catch { /* Keep the in-memory theme even when browser storage is unavailable. */ }
  }
  applyAppearance(mode);
}
