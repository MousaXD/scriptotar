#!/usr/bin/env node
/**
 * Agent 11: bounded, disposable Vite source-pattern reachability probe.
 *
 * This is NOT a picomatch patch, security waiver or proof about Tauri runtime.
 * An imported source literal is the untrusted PATTERN; the near-miss filename
 * is only the match target. A separate ordinary-pattern/hostile-filename
 * control prevents confusing the two threat surfaces.
 *
 * Execute after npm ci in apps/desktop-ui. Each case is a separate killable
 * child process so a pathological Vite compiler path cannot hang CI.
 */
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const CASES = ['ordinary-glob', 'unusual-filename', 'hostile-source-pattern'];
const TIMEOUT_MS = 8000;

async function childMain(caseName) {
  assert.ok(CASES.includes(caseName), 'unknown fixture');
  const { createServer } = await import('vite');
  const root = mkdtempSync(join(tmpdir(), 'scriptotar-agent11-vite-'));
  let server;
  try {
    const unusualName = 'a'.repeat(36) + 'b';
    writeFileSync(join(root, 'normal.js'), 'export default 1;\n');
    writeFileSync(join(root, unusualName), 'export default 2;\n');
    const pattern = caseName === 'hostile-source-pattern'
      ? './+(*a)' : caseName === 'unusual-filename' ? './*' : './*.js';
    // The literal is intentionally embedded in a source file. No runtime
    // user-input APIs, dynamic expression interpolation, or glob filenames
    // masquerading as PATTERNS are used.
    writeFileSync(join(root, 'entry.js'),
      'export const modules = import.meta.glob(' + JSON.stringify(pattern) + ');\n');
    server = await createServer({
      configFile: false,
      envFile: false,
      root,
      plugins: [],
      logLevel: 'silent',
      optimizeDeps: { noDiscovery: true },
      server: { middlewareMode: true, hmr: false }
    });
    const result = await server.transformRequest('/entry.js');
    assert.ok(result?.code, 'Vite did not transform source fixture');
    assert.ok(!result.code.includes('import.meta.glob('),
      'Vite did not actually transform the glob macro');
    if (caseName === 'ordinary-glob') {
      assert.ok(result.code.includes('normal.js'),
        'ordinary glob control did not enumerate known source');
    }
    if (caseName === 'unusual-filename') {
      assert.ok(result.code.includes(unusualName),
        'unusual-filename control was not considered by ordinary glob');
    }
    console.log(JSON.stringify({ case: caseName, outcome: 'TRANSFORM_COMPLETED',
      pattern, unusualFilenameLength: unusualName.length,
      codeLength: result.code.length }));
  } finally {
    if (server) await server.close();
    rmSync(root, { recursive: true, force: true });
  }
}

function parentMain() {
  const installedVite = require('vite/package.json').version;
  const installedPicomatch = require('picomatch/package.json').version;
  assert.equal(installedVite, '7.3.7',
    'The reviewed Vite source-to-sink graph changed; manually re-review before measuring');
  assert.equal(installedPicomatch, '4.0.7',
    'The reviewed picomatch parser changed; manually re-review before measuring');
  const results = [];
  for (const fixture of CASES) {
    const child = spawnSync(process.execPath, [process.argv[1], '--child', fixture], {
      cwd: process.cwd(), encoding: 'utf8', timeout: TIMEOUT_MS,
      maxBuffer: 256 * 1024, windowsHide: true
    });
    const timeout = child.error?.code === 'ETIMEDOUT';
    const result = {
      fixture, sourcePattern: fixture === 'hostile-source-pattern' ? './+(*a)' : 'ordinary',
      status: timeout ? 'TIMEOUT' : child.status === 0 ? 'COMPLETE' : 'ERROR',
      subprocessExitCode: child.status,
      elapsedBudgetMs: TIMEOUT_MS,
      stdoutTail: (child.stdout || '').slice(-800),
      stderrTail: (child.stderr || '').slice(-800)
    };
    results.push(result);
    console.log(JSON.stringify(result));
  }
  const controls = results.slice(0, 2);
  assert.ok(controls.every(r => r.status === 'COMPLETE'),
    'Clean compiler or unusual-FILENAME controls failed; source-to-pattern experiment is invalid');
  const hostile = results[2];
  assert.notEqual(hostile.status, 'ERROR',
    'Hostile glob source fixture crashed/was rejected; repair test instead of misclassifying');
  const verdict = hostile.status === 'TIMEOUT'
    ? 'BOUNDED_VITE_SOURCE_PATTERN_TO_BUILD_TIMEOUT_REPRODUCED'
    : 'NOT_REPRODUCED_IN_THIS_FIXTURE_NOT_PROOF_OF_SAFETY';
  console.log(JSON.stringify({ check: 'Agent11 source-literal Vite import.meta.glob E2E',
    vite: installedVite, picomatch: installedPicomatch,
    verdict, securityGate: 'NOT A REMEDIATION; original 19-case probe must remain FAIL',
    scope: 'Local development/PR build-time toolchain only, not packaged desktop runtime',
    cases: results.map(({fixture,status})=>({fixture,status})) },null,2));
}

try {
  if (process.argv[2] === '--child') await childMain(process.argv[3]);
  else if (process.argv.length === 2) parentMain();
  else throw new Error('Usage: node scripts/probe-vite-pattern-reachability.mjs');
} catch (error) {
  console.error('AGENT11 VITE REACHABILITY PROBE INCONCLUSIVE/FAIL: ' + error.stack);
  process.exitCode = 1;
}
