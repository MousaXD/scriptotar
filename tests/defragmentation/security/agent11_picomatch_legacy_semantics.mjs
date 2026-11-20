#!/usr/bin/env node
/**
 * Agent11 independent positive/negative matcher contract derived from the
 * UNMODIFIED official picomatch 4.0.7 upstream test suite.
 * This is intentionally distinct from Scriptotar's immutable 19-case DoS probe.
 *
 * Original source/tag (not PR275's rewritten tests):
 * https://github.com/micromatch/picomatch/blob/4.0.7/test/extglobs.js
 * https://github.com/micromatch/picomatch/blob/4.0.7/test/extglobs-bash.js
 * https://github.com/micromatch/picomatch/blob/4.0.7/test/special-characters.js
 * https://github.com/micromatch/picomatch/blob/4.0.7/test/options.maxExtglobRecursion.js
 */
import { spawnSync } from 'node:child_process';
import { createRequire } from 'node:module';
import { resolve } from 'node:path';

const requireUi = createRequire(resolve('apps/desktop-ui/package.json'));
const moduleFile = requireUi.resolve('picomatch');
const phase = process.argv[2] || 'candidate';

// This corpus is not edited to accommodate candidate behavior. Do not relax
// true cases to false; a false result is a matcher compatibility regression.
const cases = [
  { label: 'extglobs.js:531 plus-star matches abc', pattern: '+(*)c', input: 'abc', expected: true },
  { label: 'extglobs.js:322 repeated alternatives match ax', pattern: '*(a*|b)', input: 'ax', expected: true },
  { label: 'special-characters.js:223 filename pattern', pattern: '*/*(W*, *)*', input: 'my/folder (Work, Accts)', expected: true },
  { label: 'special-characters.js:224 double-star filename pattern', pattern: '**/*(W*, *)*', input: 'my/folder/(Work, Accts)', expected: true },
  { label: 'extglobs-bash.js:1564 negative extglob positive', pattern: '*(!(foo))', input: 'bar', opts: { bash: true, windows: true }, expected: true },
  { label: 'extglobs-bash.js:1576 negative extglob positive', pattern: '*(!(foo))', input: 'baz', opts: { bash: true, windows: true }, expected: true },
  { label: 'options.maxExtglobRecursion.js:47 enabled nested plus', pattern: '+(+(a))', input: 'aaa', opts: { maxExtglobRecursion: 1 }, expected: true },
  { label: 'options.maxExtglobRecursion.js:47 enabled nested star', pattern: '*(+(a))', input: 'aaa', opts: { maxExtglobRecursion: 1 }, expected: true },
  { label: 'extglobs.js:529 negative control', pattern: '+()c', input: 'abc', expected: false },
  { label: 'extglobs-bash.js:2068 negative control', pattern: '*(!(foo))', input: 'foo', opts: { bash: true, windows: true }, expected: false }
];

// Every matcher is executed in a separate, killable Node subprocess so an
// unexpected ReDoS hangs only its own case, not the workflow runner.
const childSource = String.raw`
const pm = require(process.argv[1]);
const tc = JSON.parse(process.argv[2]);
const actual = pm.isMatch(tc.input, tc.pattern, tc.opts || {});
process.stdout.write(JSON.stringify({actual}) + '\\n');
if (actual !== tc.expected) process.exitCode = 3;
`;
let bad = 0;
for (const entry of cases) {
  const child = spawnSync(process.execPath, ['-e', childSource, moduleFile, JSON.stringify(entry)], {
    cwd: process.cwd(), encoding: 'utf8', timeout: 1500, maxBuffer: 64 * 1024
  });
  const timedOut = child.error?.code === 'ETIMEDOUT';
  let actual;
  try { actual = JSON.parse(child.stdout.trim()).actual; } catch { /* no result */ }
  const ok = !timedOut && child.status === 0 && actual === entry.expected;
  const status = ok ? 'PASS' : timedOut ? 'TIMEOUT' : 'MISMATCH/ERROR';
  console.log(`LEGACY ${phase}: ${entry.label}: ${status}; expected=${entry.expected}; actual=${actual}`);
  if (!ok) { bad++; if (child.stderr.trim()) console.error(child.stderr.trim()); }
}
console.log(`LEGACY ${phase} SUMMARY: ${cases.length - bad}/${cases.length} passing`);
if (bad) {
  console.error('BLOCKED: original upstream 4.0.7 matcher semantics changed; NOT a compatible drop-in.');
  process.exitCode = 1;
}
