#!/usr/bin/env node
/**
 * Agent 11 PRE-VITE source/config review guard.
 * Run BEFORE npm ci, vite build, or the Tauri frontend build.
 * Narrow scope: reviewed desktop-ui/src, index.html, Vite config and Tauri
 * build contract. It cannot establish safety of third-party/plugin-generated
 * glob patterns, other workspaces, or a malicious CI workflow edit.
 * Failures require human review; this is NOT a picomatch parser fix.
 */
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFileSync, readdirSync, existsSync } from 'node:fs';
import { dirname, extname, join, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const repo = resolve(root, '..', '..');
const trustedConfigSha256 = '252cb3b99585c239741cc3590c9636da49f087cc6e6ed07a5c6fcef7b89e4300';
const extensions = new Set(['.js','.jsx','.mjs','.cjs','.ts','.tsx','.svelte']);
const macro = /\bimport(?:\s|\/\*[\s\S]*?\*\/)*\.(?:\s|\/\*[\s\S]*?\*\/)*meta(?:\s|\/\*[\s\S]*?\*\/)*(?:\.(?:\s|\/\*[\s\S]*?\*\/)*glob(?:Eager)?\b|\[(?:\s|\/\*[\s\S]*?\*\/)*(['"\x60])glob(?:Eager)?\1(?:\s|\/\*[\s\S]*?\*\/)*\])/i;
const modules = /\b(?:picomatch|micromatch|tinyglobby|fdir|fast-glob|globby)\b/i;
const calls = /\b(?:glob|globSync|globStream|globIterate|fastGlob)\s*\(/i;

function inspectText(text) {
  const hits = [];
  if (macro.test(text)) hits.push('Vite import.meta.glob macro');
  if (modules.test(text)) hits.push('glob matcher/build-tool reference');
  if (calls.test(text)) hits.push('direct glob API call');
  return hits;
}
function selfTest() {
  const hostile = [
    "const x = import.meta.glob('./+(*a)')",
    "const x = import /* a */ . meta /* b */ . glob('./+(*a)')",
    "const x = import.meta['glob']('./+(*a)')",
    'const x = import.meta["glob"]("./+(*a)")',
    'const x = import.meta[\x60glob\x60]("./+(*a)")',
    "const x = import.meta.globEager('./*(*a)')",
    "const x = globSync(userPattern)",
    "import matcher from 'picomatch'",
    "const tool = require('tinyglobby')"
  ];
  const clean = [
    'const title = "Scriptotar";',
    'const filename = "a".repeat(36) + "b";',
    'const queued = await invoke("enqueue_url", { url });',
    'const paths = ["normal.js", "other.js"];'
  ];
  for (const input of hostile) assert.ok(inspectText(input).length, 'missed hostile fixture: '+input);
  for (const input of clean) assert.deepEqual(inspectText(input), [], 'blocked clean control: '+input);
  console.log('SYNTHETIC pre-build guard controls '+(hostile.length+clean.length)+'/'+(hostile.length+clean.length)+' PASS; not production or parser certification.');
}
function collect(dir, problems, result) {
  if (!existsSync(dir)) { problems.push('Missing reviewed source root: '+dir); return; }
  for (const item of readdirSync(dir, { withFileTypes: true })) {
    const path = join(dir, item.name);
    if (item.isSymbolicLink()) problems.push('Source symlink requires review: '+relative(root,path));
    else if (item.isDirectory()) collect(path, problems, result);
    else if (item.isFile() && extensions.has(extname(path))) result.push(path);
  }
}
function audit() {
  const findings = [];
  const configFile = join(root,'vite.config.ts');
  const packageFile = join(root,'package.json');
  const tauriFile = join(repo,'apps','desktop','src-tauri','tauri.conf.json');
  for (const path of [configFile,packageFile,tauriFile]) {
    if (!existsSync(path)) findings.push('Missing build contract: '+relative(repo,path));
  }
  if (!findings.length) {
    // GitHub Windows checkout may convert Git LF to CRLF. Compare canonical
    // line endings, but preserve fail-closed review for all other bytes.
    const configText = readFileSync(configFile, 'utf8');
    if (/\r(?!\n)/.test(configText))
      findings.push('Unexpected bare-CR Vite config line ending: manual review required');
    const sha256 = createHash('sha256').update(configText.replace(/\r\n/g, '\n'), 'utf8').digest('hex');
    if (sha256 !== trustedConfigSha256)
      findings.push('Unreviewed Vite config SHA-256 change: inspect all plugin and PostCSS glob sinks BEFORE build');
    const pkg = JSON.parse(readFileSync(packageFile,'utf8'));
    const tauri = JSON.parse(readFileSync(tauriFile,'utf8'));
    if (pkg.scripts?.build !== 'vite build') findings.push('Unreviewed npm build command');
    if (tauri.build?.beforeBuildCommand !== 'cd ../desktop-ui && npm ci && npm run build')
      findings.push('Unreviewed Tauri beforeBuildCommand');
    if (tauri.build?.frontendDist !== '../../desktop-ui/dist')
      findings.push('Unreviewed Tauri frontendDist');
  }
  const files = [];
  collect(join(root,'src'),findings,files);
  const html = join(root,'index.html');
  if (!existsSync(html)) findings.push('Missing reviewed HTML entrypoint');
  else files.push(html);
  for (const item of readdirSync(root, { withFileTypes: true })) {
    if (item.isFile() && /^(?:postcss|tailwind|svelte|rollup|vite)\.config\./i.test(item.name) && item.name !== 'vite.config.ts')
      findings.push('New unreviewed build configuration file: '+item.name);
  }
  if (files.length < 40) findings.push('Unexpectedly few reviewed frontend sources: '+files.length);
  for (const path of files.sort()) {
    const bytes = readFileSync(path);
    if (bytes.length > 1000000) { findings.push('Large source requires review: '+relative(root,path)); continue; }
    for (const hit of inspectText(bytes.toString('utf8'))) findings.push(hit+': '+relative(root,path));
  }
  console.log(JSON.stringify({
    check:'Agent11 PRE-BUILD first-party Vite source-pattern review guard',
    reviewedViteConfigSha256:trustedConfigSha256,
    scannedFirstPartyFiles:files.length, findings,
    scope:'desktop-ui/src, index.html, Vite config, build contracts; excludes third-party/plugin-generated glob patterns',
    result: findings.length ? 'REVIEW REQUIRED BEFORE VITE' : 'NO FIRST-PARTY GLOB MARKERS DETECTED; NOT FULL SECURITY CERTIFICATION'
  },null,2));
  if (findings.length) process.exitCode = 1;
}
if (process.argv.length===3 && process.argv[2]==='--self-test') selfTest();
else if (process.argv.length===2) audit();
else { console.error('Usage: node scripts/guard-vite-build-patterns.mjs [--self-test]'); process.exitCode=2; }
