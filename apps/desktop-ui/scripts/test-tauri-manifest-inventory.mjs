#!/usr/bin/env node
/**
 * Synthetic CLI contract tests for the ACTUAL package-manifest validator.
 * These fabricated manifests test parser failures only. They are NOT packaged
 * release evidence; real .deb and NSIS file manifests must be independently fed
 * into audit-tauri-bundle-resources.mjs by production packaging workflows.
 */
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const checker = resolve(dirname(fileURLToPath(import.meta.url)), 'audit-tauri-bundle-resources.mjs');
const scratch = mkdtempSync(join(tmpdir(), 'scriptotar-synthetic-manifest-fixtures-'));
let tests = 0;
function invoke(platform, lines, expectedHash) {
  const manifest = join(scratch, `${platform}-FILES.txt`);
  const pkg = join(scratch, `${platform}-fixture-install.bin`);
  writeFileSync(manifest, lines.join('\n') + '\n', 'utf8');
  writeFileSync(pkg, `TEST-ONLY-${platform}-fake-package-not-a-real-installer`, 'utf8');
  const hash = createHash('sha256').update(readFileSync(pkg)).digest('hex');
  const child = spawnSync(process.execPath, [
    checker, '--platform', platform, '--manifest', manifest,
    '--package', pkg, '--expected-package-sha256', expectedHash ?? hash
  ], { encoding: 'utf8', timeout: 12_000, maxBuffer: 256_000 });
  assert.equal(child.error, undefined, child.error?.message);
  assert.match(child.stdout, /"actualPackageManifestExamined": true/);
  assert.match(child.stdout, /"pathMatcherFixtures": 8/);
  tests++;
  return { child, hash };
}
try {
  for (const platform of ['linux', 'windows']) {
    const root = platform === 'linux' ? './usr/lib/scriptotar' : 'C:\\Program Files\\Scriptotar';
    const delim = platform === 'linux' ? '/' : '\\';
    const path = (...parts) => [root, ...parts].join(delim);
    const lines = Array.from({ length: 12 }, (_, i) => path('assets', `fixture-${i}.txt`));
    lines.push(path('transcription-runtime', 'RUNTIME-VERSIONS.txt'));
    lines.push(path('mcp-tools', platform === 'linux' ? 'cloudflared' : 'cloudflared.exe'));
    const good = invoke(platform, lines);
    assert.equal(good.child.status, 0, good.child.stderr);
    assert.match(good.child.stdout, /"issues": \[\]/);
    const hostile = invoke(platform, [...lines, path('node_modules', 'picomatch', 'lib', 'parse.js')]);
    assert.notEqual(hostile.child.status, 0, 'parser inside installed package must fail');
    assert.match(hostile.child.stdout, /Unexpected frontend parser\/development module/);
    const missingRuntime = invoke(platform, lines.filter(s => !s.includes('transcription-runtime')));
    assert.notEqual(missingRuntime.child.status, 0, 'missing real resource must fail');
    assert.match(missingRuntime.child.stdout, /lacks transcription-runtime resource paths/);
    const incorrectChecksum = invoke(platform, lines, 'f'.repeat(64) === good.hash ? '0'.repeat(64) : 'f'.repeat(64));
    assert.notEqual(incorrectChecksum.child.status, 0, 'wrong package SHA-256 must fail');
    assert.match(incorrectChecksum.child.stdout, /SHA-256 mismatches/);
    console.log(`${platform}: synthetic CLI controls PASS: clean, injected picomatch, missing runtime, wrong package SHA256`);
  }
  assert.equal(tests, 8);
  console.log('SYNTHETIC package manifest validator fixtures 8/8 PASS; NOT installed package evidence.');
} finally {
  rmSync(scratch, { recursive: true, force: true });
}
