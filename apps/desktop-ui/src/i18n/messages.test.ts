import { describe, expect, it } from 'vitest';
import { shellMessages } from './messages';

describe('English and Arabic UI message coverage', () => {
  it('has identical translation keys for both interface languages', () => {
    expect(Object.keys(shellMessages.ar).sort()).toEqual(Object.keys(shellMessages.en).sort());
  });

  it('translates all dedicated MCP wizard labels and warnings in both languages', () => {
    const keys = Object.keys(shellMessages.en).filter((key) => key.startsWith('mcp.'));
    expect(keys.length).toBeGreaterThan(70);
    for (const key of keys) {
      const en = shellMessages.en[key as keyof typeof shellMessages.en];
      const ar = shellMessages.ar[key as keyof typeof shellMessages.ar];
      expect(en.trim().length).toBeGreaterThan(0);
      expect(ar.trim().length).toBeGreaterThan(0);
      expect(ar).not.toBe(en);
    }
  });
  it('retains the same interpolation placeholders in both locales', () => {
    const placeholders = (message: string) =>
      [...message.matchAll(/\{([a-zA-Z0-9_]+)\}/g)].map((match) => match[1]).sort();
    for (const key of Object.keys(shellMessages.en) as Array<keyof typeof shellMessages.en>) {
      expect(placeholders(shellMessages.ar[key]), key).toEqual(placeholders(shellMessages.en[key]));
    }
  });

});
