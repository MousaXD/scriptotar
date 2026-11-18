#!/usr/bin/env node
/**
 * Static-only inventory of whether dependency-tree picomatch can be seen in
 * Scriptotar's packaged Tauri frontend input. NOT a ReDoS safety certificate
 * and NEVER a replacement for the unfiltered mandatory npm advisory gate.
 * Run after npm ci && npm run build from apps/desktop-ui.
 */
import { readFileSync, readdirSync, statSync, existsSync } from 'node:fs';
import { dirname, extname, join, relative, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';

const uiRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const repoRoot = resolve(uiRoot, '..', '..');
const pkg = JSON.parse(readFileSync(join(uiRoot, 'package.json'), 'utf8'));
const lock = JSON.parse(readFileSync(join(uiRoot, 'package-lock.json'), 'utf8'));
const configPath = join(repoRoot, 'apps', 'desktop', 'src-tauri', 'tauri.conf.json');
const config = JSON.parse(readFileSync(configPath, 'utf8'));
const dist = resolve(dirname(configPath), config.build?.frontendDist || '');
const expectedDist = resolve(uiRoot, 'dist');
const watched = ['picomatch', 'tinyglobby', 'fdir', 'vite', 'vitest'];
const embedded = ['picomatch', 'tinyglobby', 'fdir'];
const problems = [];

if (dist !== expectedDist) problems.push('Tauri frontendDist no longer resolves to desktop-ui/dist');
if (Object.keys(pkg.dependencies || {}).length > 0) {
  problems.push('Unexpected desktop-ui runtime dependencies; re-review full npm runtime graph');
}
for (const name of watched) {
  const entries = Object.entries(lock.packages || {}).filter(([p]) =>
    p === `node_modules/${name}` || p.endsWith(`/node_modules/${name}`)
  );
  if (!entries.length) problems.push(`No locked entry found for ${name}; update inventory`);
  for (const [path, entry] of entries) {
    if (entry.dev !== true) problems.push(`Runtime/non-dev lockfile edge: ${path}`);
  }
}
if (config.bundle?.resources !== undefined &&
    (Array.isArray(config.bundle.resources) ? config.bundle.resources.length > 0
      : Object.keys(config.bundle.resources || {}).length > 0)) {
  problems.push('Tauri bundle.resources now configured; manually audit node_modules packaging');
}
if (!existsSync(dist) || !statSync(dist).isDirectory()) {
  problems.push('Frontend distribution directory missing: build before running audit');
}

const files = [];
function visit(dir) {
  for (const d of readdirSync(dir, { withFileTypes: true })) {
    const path = join(dir, d.name);
    if (d.isDirectory()) visit(path);
    else if (d.isFile()) files.push(path);
  }
}
if (existsSync(dist)) visit(dist);
if (files.length === 0) problems.push('Built frontend contains zero files');
const pattern = /(?:^|[/\\])node_modules[/\\](?:picomatch|tinyglobby|fdir)(?:[/\\]|$)/i;
for (const path of files) {
  const ext = extname(path);
  if (ext === '.map') {
    try {
      const map = JSON.parse(readFileSync(path, 'utf8'));
      for (const source of map.sources || []) {
        if (pattern.test(source)) {
          problems.push(`Bundled parser source map entry: ${relative(dist, path)} / ${source}`);
        }
      }
    } catch {
      problems.push(`Cannot parse built frontend source map: ${relative(dist, path)}`);
    }
  } else if (['.js', '.css', '.html'].includes(ext)) {
    const bytes = readFileSync(path, 'utf8');
    for (const name of embedded) {
      if (bytes.includes(name)) problems.push(`Parser textual indicator ${name} in ${relative(dist, path)}`);
    }
  }
}
console.log(JSON.stringify({
  check: 'static Tauri frontend input, NOT installed-package or runtime exploit certification',
  tauriFrontendDist: relative(repoRoot, dist).split(sep).join('/'),
  bundleResourcesConfigured: config.bundle?.resources !== undefined,
  scannedAssets: files.length,
  lockedDevOnly: watched.map(name => ({ name, entries: Object.keys(lock.packages || {}).filter(p =>
    p === `node_modules/${name}` || p.endsWith(`/node_modules/${name}`)) })),
  issues: problems
}, null, 2));
if (problems.length) {
  console.error('Release input/dependency boundary changed: independent manual source and package review required.');
  process.exitCode = 1;
} else {
  console.log('No named parser modules in built static source maps/assets and affected lockfile nodes are dev-only.');
  console.log('This is LIMITED STATIC EVIDENCE; untrusted build-time globs can still be dangerous.');
  console.log('Does not waive npm audit, claim arbitrary ReDoS safety, or certify installed desktop packages.');
}
