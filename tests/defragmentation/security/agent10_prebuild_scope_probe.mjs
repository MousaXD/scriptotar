#!/usr/bin/env node
/**
 * Agent10 independent adversarial pre-Vite *scope* regression.
 *
 * Drives the actual separately checked-out immutable guard against disposable
 * copies of the real source/config, without installing npm or invoking Vite.
 * This test is intentionally RED until every source-root import escape is
 * rejected. It does not prove the original 19-pattern parser vulnerability is
 * patched or establish installed desktop-app security.
 */
import assert from 'node:assert/strict';
import {
  cpSync, existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync,
} from 'node:fs';
import { spawnSync } from 'node:child_process';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const repo = resolve(dirname(fileURLToPath(import.meta.url)), '..', '..', '..');
const guard = join(repo, '.agent10-trusted-guard', 'apps', 'desktop-ui', 'scripts', 'guard-vite-build-patterns.mjs');
const source = join(repo, 'apps', 'desktop-ui');
assert.ok(existsSync(guard), 'Separately pinned guard checkout is missing');
const tmp = mkdtempSync(join(tmpdir(), 'agent10-prebuild-scope-'));
const ws = join(tmp, 'apps', 'desktop-ui');
const main = join(ws, 'src', 'main.ts');
const qa = join(ws, 'qa', 'hostile.ts');

function run() {
  const result = spawnSync(process.execPath, [guard, '--workspace', ws], {
    cwd: tmp, encoding: 'utf8', timeout: 12000, maxBuffer: 512 * 1024, windowsHide: true,
  });
  if (result.error) throw result.error;
  return result;
}
function write(path, value) {
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, value);
}
const bypasses = [];
function verdict(label, output, requiredDenial) {
  const success = output.status === 0;
  const evidence = {
    label, status: output.status, fail_closed: !success,
    findings: output.stdout.includes('findings') ? output.stdout.slice(0, 1700) : output.stderr.slice(0, 700),
  };
  console.log(JSON.stringify(evidence));
  if (requiredDenial && success) bypasses.push(label);
}
try {
  mkdirSync(ws, { recursive: true });
  cpSync(join(source, 'src'), join(ws, 'src'), { recursive: true });
  for (const filename of ['vite.config.ts', 'package.json', 'package-lock.json', 'tsconfig.json', 'index.html']) {
    cpSync(join(source, filename), join(ws, filename));
  }
  const tauriDir = join(tmp, 'apps', 'desktop', 'src-tauri');
  mkdirSync(tauriDir, { recursive: true });
  cpSync(join(repo, 'apps', 'desktop', 'src-tauri', 'tauri.conf.json'), join(tauriDir, 'tauri.conf.json'));

  const clean = run();
  assert.equal(clean.status, 0, 'Clean exact source failed trusted guard: ' + clean.stdout + clean.stderr);
  console.log('CONTROL CLEAN SOURCE: PASS');

  // Independently verify that Agent10's previous dotfile issue really is fixed.
  const dotfile = join(ws, '.postcssrc.cjs');
  write(dotfile, 'module.exports = { plugins: {} };\n');
  const postcss = run();
  verdict('dotfile .postcssrc.cjs correctly blocked', postcss, true);
  assert.match(postcss.stdout, /Unreviewed shadow npm\/build configuration/);
  rmSync(dotfile);

  // Vite root-absolute module specifiers resolve from the project root,
  // but the present guard's relativeLiteral only sees quoted ../ strings.
  const original = readFileSync(main, 'utf8');
  write(qa, "const modules = import.meta.glob('./+(*a)'); export { modules };\n");
  write(main, original + "\nimport '/qa/hostile.ts';\n");
  const absolute = run();
  verdict('Vite root-absolute /qa/hostile.ts bypass', absolute, true);
  write(main, original);

  // A path beginning ./ can normalize outside src without beginning ../ .
  // This is an independent string-level scope escape negative control;
  // source-compiled Vite reachability must be separately tested.
  mkdirSync(join(ws, 'src', 'staging'), { recursive: true });
  write(main, original + "\nimport './staging/../../qa/hostile.ts';\n");
  const normalized = run();
  verdict('normalized ./staging/../../qa/hostile.ts bypass', normalized, true);
  write(main, original);
  assert.deepEqual(bypasses, [], 'UNBLOCKED PREBUILD GUARD SCOPES: ' + bypasses.join(', '));
  console.log('PASS: both out-of-scope malicious source import shapes were denied before build');
} finally {
  rmSync(tmp, { recursive: true, force: true });
}
