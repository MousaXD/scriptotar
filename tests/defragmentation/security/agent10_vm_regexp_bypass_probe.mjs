#!/usr/bin/env node
/**
 * Agent 10 independent, bounded negative research probe.
 * Source-exact Agent11 VM regex wrapper is research-only, NOT a parser fix.
 * Only disposable Node child processes execute pathological expressions;
 * parent kills them after a strict deadline. Never run native bypass in parent.
 */
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '../../..');
const child = String.raw`
'use strict';
const assert = require('node:assert/strict');
const {createRequire} = require('node:module');
const {resolve} = require('node:path');
const req = createRequire(resolve('apps/desktop-ui/package.json'));
const pm = req('picomatch');
assert.equal(req('picomatch/package.json').version, '4.0.7');
const mode = process.env.AGENT10_ESCAPE_CASE;
const pattern = '+(a|b|ab)';
const input = 'ab'.repeat(32) + '/';  // known hostile near miss
let oldRegex;
if (mode === 'precompiled') oldRegex = pm.makeRe(pattern, {capture:true});
const {install} = require(resolve('tests/defragmentation/security/agent11_vm_regex_wrapper_candidate.cjs'));
const handle = install({timeoutMs:80});
const regex = oldRegex || pm.makeRe(pattern, {capture:true});
assert.equal(regex instanceof handle.GuardedRegExp, mode !== 'precompiled');
if (mode === 'guarded') {
  assert.equal(pm.makeRe('src/**/*.js').test('src/a.js'), true);
  try {
    regex.exec(input);
    console.log('UNEXPECTED_NATIVE_FINISH');
    process.exitCode = 6;
  } catch (err) {
    if (err?.code !== 'ERR_AGENT11_PICOMATCH_DEADLINE') throw err;
    console.log('EXPECTED_TYPED_FAIL_CLOSED_DENIAL');
  }
} else if (mode === 'prototype') {
  // Direct native prototype call bypasses the GuardedRegExp.exec override.
  RegExp.prototype.exec.call(regex, input);
  console.log('UNEXPECTED_QUICK_PROTOTYPE_RETURN');
} else if (mode === 'precompiled') {
  // Regex compiled before installing the override is still a native RegExp.
  regex.exec(input);
  console.log('UNEXPECTED_QUICK_PRECOMPILED_RETURN');
} else {
  throw new Error('Unknown adversarial mode');
}
`;

function run(mode) {
  const attempt = spawnSync(process.execPath, ['-e', child], {
    cwd: root, encoding: 'utf8', timeout: 1600,
    windowsHide: true, maxBuffer: 128 * 1024,
    env: {...process.env, NODE_OPTIONS:'', AGENT10_ESCAPE_CASE: mode},
  });
  return {
    mode, exit: attempt.status, signal: attempt.signal,
    error: attempt.error?.code ?? null,
    stdout: (attempt.stdout ?? '').trim().slice(-500),
    stderr: (attempt.stderr ?? '').trim().slice(-500),
  };
}

const guarded = run('guarded');
console.log('AGENT10 RESEARCH POSITIVE:', JSON.stringify(guarded));
assert.equal(guarded.error, null, 'Guarded 80ms case must not exceed parent deadline');
assert.equal(guarded.exit, 0, 'Guarded run must throw typed denial, never silently return false');
assert.match(guarded.stdout, /EXPECTED_TYPED_FAIL_CLOSED_DENIAL/);

for (const mode of ['prototype', 'precompiled']) {
  const bypass = run(mode);
  console.log('AGENT10 RESEARCH NEGATIVE:', JSON.stringify(bypass));
  assert.equal(bypass.error, 'ETIMEDOUT',
    'Expected actual native unbounded matcher bypass in killable child for '+mode);
}
console.log('AGENT10 VERIFIED: guarded path denies, prototype and precompiled paths bypass deadline in bounded children.');
console.log('LIMIT: capability proof only; no claim that genuine Vite build currently calls either bypass.');
console.log('VERDICT: experimental VM wrapper is NOT complete picomatch parser remediation or release approval.');
