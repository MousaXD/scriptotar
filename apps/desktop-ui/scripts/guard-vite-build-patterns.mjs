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

const argv = process.argv.slice(2);
const trustedDefaultRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const isWorkspaceAudit = argv.length === 2 && argv[0] === '--workspace';
const root = isWorkspaceAudit ? resolve(process.cwd(), argv[1]) : trustedDefaultRoot;
const repo = resolve(root, '..', '..');
const trustedGitBlobShas = Object.freeze({
  'apps/desktop-ui/package.json': '19edb804b2c442be96311e2f39f7788dfb286dda',
  'apps/desktop-ui/package-lock.json': 'eebc9228dd404f0b9f9bd55f2665ab3e00fe9dfa',
  'apps/desktop-ui/tsconfig.json': 'aab6eff7f7e345f6fb93b86e16d69aa328d8e140',
  'apps/desktop/src-tauri/tauri.conf.json': 'b28cc6a5057032479f94d5b94ed01f2ea23fa03e'
});
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
function gitBlobShaCanonical(file, findings) {
  const content = readFileSync(file, 'utf8');
  if (/\r(?!\n)/.test(content)) findings.push('Bare carriage return in reviewed build contract: '+relative(repo,file));
  const canonical = Buffer.from(content.replace(/\r\n/g, '\n'), 'utf8');
  return createHash('sha1').update('blob '+canonical.byteLength+'\0').update(canonical).digest('hex');
}
function audit() {
  const findings = [];
  const configFile = join(root,'vite.config.ts');
  const packageFile = join(root,'package.json');
  const packageLockFile = join(root,'package-lock.json');
  const tsconfigFile = join(root,'tsconfig.json');
  const tauriFile = join(repo,'apps','desktop','src-tauri','tauri.conf.json');
  for (const path of [configFile,packageFile,packageLockFile,tsconfigFile,tauriFile]) {
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
    for (const [rel,expected] of Object.entries(trustedGitBlobShas)) {
      const path = join(repo,rel);
      const actual = gitBlobShaCanonical(path,findings);
      if (actual !== expected) findings.push('Unreviewed build manifest/lock/Tauri contract '+rel+'; expected Git blob '+expected+' observed '+actual);
    }
    const pkg = JSON.parse(readFileSync(packageFile,'utf8'));
    const tauri = JSON.parse(readFileSync(tauriFile,'utf8'));
    if (pkg.scripts?.build !== 'vite build') findings.push('Unreviewed npm build command');
    if (tauri.build?.beforeBuildCommand !== 'cd ../desktop-ui && npm ci && npm run build')
      findings.push('Unreviewed Tauri beforeBuildCommand');
    if (tauri.build?.frontendDist !== '../../desktop-ui/dist')
      findings.push('Unreviewed Tauri frontendDist');
  }
  // npm-shrinkwrap.json overrides package-lock.json. A PR adding .npmrc or a
  // parent workspace manifest may also change the install graph despite a
  // correctly pinned package-lock. Reject these BEFORE npm ci in every job.
  // Unreviewed Vite/rollup/PostCSS configs or Vite .env files also change
  // compiler behavior before the ordinary frontend import scanner can help.
  const shadowInstall = /^(?:npm-shrinkwrap\.json|\.npmrc|\.yarnrc(?:\.yml)?|pnpm-lock\.yaml|pnpm-workspace\.yaml|yarn\.lock|bun\.lock(?:b)?|\.pnp\.(?:cjs|js)|\.env(?:\..*)?)$/i;
  const extraBuildConfig = /^(?:vite|svelte|postcss|tailwind|rollup|webpack|esbuild)\.config\.[mc]?[jt]sx?$/i;
  for (const dir of [repo,join(repo,'apps'),root]) {
    if (!existsSync(dir)) { findings.push('Missing reviewed source parent: '+dir); continue; }
    for (const entry of readdirSync(dir,{withFileTypes:true})) {
      const file = join(dir,entry.name);
      if (shadowInstall.test(entry.name) ||
          (extraBuildConfig.test(entry.name) && file !== configFile) ||
          (dir !== root && (entry.name === 'package.json' || entry.name === 'package-lock.json'))) {
        findings.push('Unreviewed shadow npm/build configuration: '+relative(repo,file));
      }
    }
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
    sourceContractBlobPins:trustedGitBlobShas,
    scannedFirstPartyFiles:files.length, findings,
    scope:'desktop-ui/src, index.html, Vite config, build contracts; excludes third-party/plugin-generated glob patterns',
    result: findings.length ? 'REVIEW REQUIRED BEFORE VITE' : 'NO FIRST-PARTY GLOB MARKERS DETECTED; NOT FULL SECURITY CERTIFICATION'
  },null,2));
  if (findings.length) process.exitCode = 1;
}
if (argv.length === 1 && argv[0] === '--self-test') selfTest();
else if (argv.length === 0 || isWorkspaceAudit) audit();
else { console.error('Usage: node scripts/guard-vite-build-patterns.mjs [--self-test | --workspace TARGET_PATH]'); process.exitCode=2; }
