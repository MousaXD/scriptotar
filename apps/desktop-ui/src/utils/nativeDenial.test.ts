import { describe, expect, it } from 'vitest';
import { matchesNativeDenial, newNativeRequestId } from './nativeDenial';

describe('native request provenance', () => {
  it('uses distinct, cryptographically generated per-action 128-bit identities', () => {
    const first = newNativeRequestId();
    const second = newNativeRequestId();
    expect(first).toMatch(/^[0-9a-f]{32}$/);
    expect(second).toMatch(/^[0-9a-f]{32}$/);
    expect(first).not.toBe(second);
  });

  it('accepts only the exact typed real Tauri denial response for this request', () => {
    const id = '0123456789abcdef0123456789abcdef';
    const prefix = 'SCRIPTOTAR_DENIAL:INVALID_STUDY_ANGLES:';
    expect(matchesNativeDenial(new Error(prefix + id), 'INVALID_STUDY_ANGLES', id)).toBe(true);
    expect(matchesNativeDenial(prefix + id, 'INVALID_STUDY_ANGLES', id)).toBe(true);
    expect(matchesNativeDenial(prefix + id, 'INVALID_MEDIA_URL', id)).toBe(false);
    expect(matchesNativeDenial(prefix + 'ffffffffffffffffffffffffffffffff',
      'INVALID_STUDY_ANGLES', id)).toBe(false);
    expect(matchesNativeDenial('old invalid angle', 'INVALID_STUDY_ANGLES', id)).toBe(false);
    expect(matchesNativeDenial({ message: prefix + id }, 'INVALID_STUDY_ANGLES', id)).toBe(false);
  });
});
