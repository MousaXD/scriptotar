/** Cryptographic, per-action identity; used only to correlate native denials.
 * This is NOT a consent token, a project authority, or a credential.
 */
export function newNativeRequestId(): string {
  if (!globalThis.crypto?.getRandomValues) {
    throw new Error('Secure request identity is unavailable');
  }
  const bytes = new Uint8Array(16);
  globalThis.crypto.getRandomValues(bytes);
  return Array.from(bytes, (value) => value.toString(16).padStart(2, '0')).join('');
}

/** Never trust a stale, unbound, or differently-scoped error's request marker. */
export function matchesNativeDenial(cause: unknown, code: string, requestId: string): boolean {
  if (!/^[0-9a-f]{32}$/.test(requestId) || !/^[A-Z_]+$/.test(code)) return false;
  const value = typeof cause === 'string' ? cause : cause instanceof Error ? cause.message : '';
  return value === `SCRIPTOTAR_DENIAL:${code}:${requestId}`;
}
