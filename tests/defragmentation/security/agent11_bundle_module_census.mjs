#!/usr/bin/env node
/**
 * Agent11 READ-ONLY release surface audit, NOT a parser security fix.
 *
 * Build the exact locked Scriptotar desktop UI using the real Vite config.
 * Enumerate Rollup's ACTUAL compiled module graph; do not infer shipping
 * contents solely from npm install, npm audit, or searching minified text.
 *
 * Scope: browser assets only. Tauri Rust, npm lifecycle/build-tool execution,
 * installers and sidecars are OUT OF SCOPE. A PASS DOES NOT waive issue #129.
 */
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { existsSync, mkdtempSync, readFileSync, readdirSync, rmSync, statSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve, relative, sep } from 'node:path';
import { createRequire } from 'node:module';
import { pathToFileURL } from 'node:url';

const repo = process.cwd();
const ui = resolve(repo, 'apps/desktop-ui');
const lock = resolve(ui, 'package-lock.json');
const config = resolve(ui, 'vite.config.ts');
const originalLockBlob = 'eebc9228dd404f0b9f9bd55f2665ab4fb73e00fe9dfa';
assert.ok(existsSync(lock) && existsSync(config), 'Real desktop Vite config and lockfile are required');
const blob = execFileSync('git', ['hash-object', lock], { cwd: repo, encoding: 'utf8' }).trim();
assert.equal(blob, originalLockBlob, 'Use independently reviewed exact locked desktop npm tree');
const locked = JSON.parse(readFileSync(lock, 'utf8'));
assert.equal(locked.packages['node_modules/picomatch'].version, '4.0.7',
  'Do not reuse this historical provenance assertion after a dependency change');

const requireUi = createRequire(resolve(ui, 'package.json'));
const { build } = await import(pathToFileURL(requireUi.resolve('vite')).href);

const graph = new Set();
let observed = false;
const collector = {
  name: 'agent11-production-module-census-experiment',
  apply: 'build',
  generateBundle(_options, output) {
    observed = true;
    for (const id of this.getModuleIds()) graph.add(id.replaceAll('\\', '/'));
    // Outputs provided by actual compiler, not an independent string finder.
    const names = Object.keys(output);
    assert.ok(names.length > 0, 'No generated bundle output visible to Rollup');
  }
};
const temp = mkdtempSync(join(tmpdir(), 'scriptotar-agent11-browser-'));
let result;
try {
  await build({
    configFile: config,
    root: ui,
    mode: 'production',
    plugins: [collector],
    build: {
      outDir: temp,
      emptyOutDir: true,
      sourcemap: true,
      manifest: true
    },
    logLevel: 'warn'
  });
  assert.ok(observed, 'Compiled module graph plugin did not execute');
  assert.ok(graph.size > 0, 'Compiled module graph unexpectedly empty');
  const graphPaths = [...graph].sort();
  const banned = /\/node_modules\/(?:picomatch|tinyglobby|fdir)(?:\/|$)/i;
  const shipped = graphPaths.filter(p => banned.test(p));
  let files = 0, jsFiles = 0, jsBytes = 0;
  const walk = folder => {
    for (const name of readdirSync(folder)) {
      const p = join(folder, name);
      if (statSync(p).isDirectory()) walk(p);
      else {
        files++;
        if (/\.(?:js|mjs)$/.test(name)) {
          jsFiles++;
          jsBytes += statSync(p).size;
        }
      }
    }
  };
  walk(temp);
  assert.ok(jsFiles > 0 && jsBytes > 0, 'No actual browser JS artifacts emitted');
  result = {
    source_lock_blob: blob,
    picomatch_in_clean_npm_install: true,
    inspected_entry: relative(repo, config),
    compiler: 'real Vite production build and Rollup getModuleIds',
    transformed_graph_modules: graph.size,
    compiled_js_assets: jsFiles,
    compiled_js_bytes: jsBytes,
    output_files: files,
    compiled_modules_from_picomatch_tinyglobby_fdir: shipped,
    excluded_scope: [
      'Vite/build-time parser execution (still vulnerable)',
      'Tauri Rust and sidecar executables',
      'Linux .deb and Windows NSIS installers',
      'source or dependency patterns supplied by future changes'
    ],
    verdict: shipped.length ? 'BROWSER MODULE EXPOSURE — REVIEW' :
      'NO MATCHER MODULE IN THIS SOURCE-EXACT BROWSER BUNDLE',
    issue_129_release_decision: 'NO-GO: parser risk not remediated'
  };
  console.log('AGENT11 SOURCE-EXACT BROWSER BUNDLE CENSUS: ' + JSON.stringify(result));
  assert.deepEqual(shipped, [], 'A glob parser module reached the browser output graph');
} finally {
  rmSync(temp, { recursive: true, force: true });
}
