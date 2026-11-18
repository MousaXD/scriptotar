#!/usr/bin/env node
/**
 * Bounded, opt-in diagnostic for GHSA-fc5q-39xc-vjpf.
 * This is NOT a substitute for npm audit or a full parser security review.
 * Execute from apps/desktop-ui after npm ci.
 */
import { spawnSync } from 'node:child_process';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
try {
  require.resolve('picomatch');
} catch {
  console.error('picomatch not installed; first run npm ci from apps/desktop-ui');
  process.exit(2);
}

const assertPatched = process.argv.includes('--assert-patched');
const reportOnly = process.argv.includes('--report');
if (assertPatched === reportOnly) {
  console.error('Usage: node scripts/probe-picomatch-redos.mjs --report|--assert-patched [--extended]');
  process.exit(2);
}

const childSource = String.raw`
  const picomatch = require('picomatch');
  const pattern = process.argv[1];
  const input = process.argv[2];
  const expected = process.argv[3] === 'true';
  const began = performance.now();
  const actual = picomatch(pattern)(input);
  console.log(JSON.stringify({actual, expected, ms: Math.round(performance.now() - began)}));
  if (actual !== expected) process.exitCode = 3;
`;

// The trailing mismatch is essential. All inputs are fixed, not user-controlled.
// Run each matcher in a killable subprocess; never invoke a hostile pattern
// in the parent process, including for warm-up or compilation.
const baselineCases = [
  { label: 'single-branch plus-star', pattern: '+(*a)', input: 'a'.repeat(36) + 'b', matches: false },
  { label: 'single-branch star-star', pattern: '*(*a)', input: 'a'.repeat(36) + 'b', matches: false },
  { label: 'safe control', pattern: '+(a|b)', input: 'abab', matches: true }
];


// Adversarial variants beyond the original GHSA minimal reproduction.
// A final slash cannot be matched by default segment-scoped glob wildcards.
// Keep all hostile patterns in individually killable subprocesses.
const extendedCases = [
  { label: 'single branch suffix-star', pattern: '+(a*)', input: 'a'.repeat(36) + '/', matches: false },
  { label: 'single branch infix-stars', pattern: '+(*a*)', input: 'a'.repeat(36) + '/', matches: false },
  { label: 'repeated multi-branch wildcards', pattern: '+(*a|b)', input: 'a'.repeat(36) + '/', matches: false },
  { label: 'nested wrapper star', pattern: '+(@(*a))', input: 'a'.repeat(36) + '/', matches: false },
  { label: 'nested wrapper bare star', pattern: '+(@(*))', input: 'a'.repeat(36) + '/', matches: false },
  { label: 'ambiguously repeated character classes', pattern: '+([ab]|b)', input: 'b'.repeat(36) + '/', matches: false },
  { label: 'ambiguously repeated literals', pattern: '+(a|b|ab)', input: 'ab'.repeat(22) + '/', matches: false },
  { label: 'wrapped nested repeated class', pattern: '+(@(+([ab])))', input: 'a'.repeat(36) + '/', matches: false }
];
const extended = process.argv.includes('--extended');
const cases = extended ? [...baselineCases, ...extendedCases] : baselineCases;

let failed = false;
for (const entry of cases) {
  const child = spawnSync(
    process.execPath,
    ['-e', childSource, entry.pattern, entry.input, String(entry.matches)],
    { cwd: process.cwd(), encoding: 'utf8', timeout: 1200, maxBuffer: 64 * 1024 }
  );
  const timedOut = child.error?.code === 'ETIMEDOUT';
  let data;
  try { data = JSON.parse(child.stdout.trim()); } catch { /* no valid child output */ }
  const passed = !timedOut && child.status === 0 && data?.actual === entry.matches;
  const status = timedOut ? 'TIMEOUT (probable ReDoS)' : passed ? 'PASS' : 'ERROR/REGRESSION';
  console.log(`${entry.label}: ${status}${Number.isFinite(data?.ms) ? ` (${data.ms}ms)` : ''}`);
  if (!passed) {
    failed = true;
    if (!timedOut) console.error(child.stderr || child.error?.message || child.stdout || 'unknown failure');
  }
}

if (failed) {
  console.error('SECURITY PROBE NOT CLEAN: a known ReDoS pattern timed out or functionality regressed.');
} else {
  console.log('SECURITY PROBE CLEAN for this bounded sample only; review advisory, full corpus, and npm audit.');
}
if (assertPatched && failed) process.exitCode = 1;
if (reportOnly && failed) console.log('REPORT-ONLY mode preserves the existing CI security gate; this result is NOT remediation.');
