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

// Fail closed if the reviewed build entry contract changes. The current
// desktop release command only embeds Vite's static dist, not npm tools.
if (pkg.scripts?.build !== 'vite build') {
  problems.push('desktop-ui build script changed; trace untrusted source inputs into new build tooling');
}
if (config.build?.beforeBuildCommand !== 'cd ../desktop-ui && npm ci && npm run build') {
  problems.push('Tauri beforeBuildCommand changed; review any executable build-time inputs');
}

// A source-to-glob API inventory, NOT a complete dataflow proof. In particular,
// this does not validate glob patterns constructed internally by third-party
// Vite/Svelte plugins or from unreviewed files in other workspaces.
const globApiRules = [
  { label: 'picomatch-family module reference', test: /\b(?:picomatch|micromatch|tinyglobby|fdir|fast-glob|globby)\b/i },
  { label: 'Vite import.meta.glob macro', test: /\bimport\s*\.\s*meta\s*(?:\.\s*glob(?:Eager)?\b|\[\s*['"`]glob(?:Eager)?['"`]\s*\])/i },
  { label: 'direct glob API invocation', test: /\b(?:glob|globSync|globStream|globIterate|fastGlob)\s*\(/i }
];
const attackFixtures = [
  "import picomatch from 'picomatch';",
  "import.meta.glob(untrustedPattern)",
  "import.meta['glob'](input)",
  "globSync(userSuppliedPattern)",
  "require('tinyglobby')"
];
for (const attack of attackFixtures) {
  if (!globApiRules.some(rule => rule.test.test(attack))) {
    problems.push('Source-to-glob analyzer failed its own adversarial detection fixture');
  }
}
const cleanFixtures = [
  'const fileName = userInput;',
  'const result = await invoke("enqueue_url", { url });',
  'const globName = "not-executed";'
];
for (const clean of cleanFixtures) {
  if (globApiRules.some(rule => rule.test.test(clean))) {
    problems.push('Source-to-glob analyzer rejected a clean control');
  }
}

const sourceDir = join(uiRoot, 'src');
const reviewedSourcePaths = [join(uiRoot, 'vite.config.ts')];
function collectFirstPartySources(dir) {
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const full = join(dir, entry.name);
    if (entry.isSymbolicLink()) {
      problems.push(`Unexpected source symlink: ${relative(uiRoot, full)}`);
    } else if (entry.isDirectory()) {
      collectFirstPartySources(full);
    } else if (entry.isFile() && /\.(?:ts|tsx|js|jsx|mjs|svelte)$/.test(entry.name)) {
      reviewedSourcePaths.push(full);
    }
  }
}
if (!existsSync(sourceDir)) {
  problems.push('Source directory missing; cannot establish source-to-glob boundary');
} else {
  collectFirstPartySources(sourceDir);
}
if (reviewedSourcePaths.length < 40) {
  problems.push('Unexpectedly few frontend sources inventoried; review scanner scope');
}
for (const filePath of reviewedSourcePaths) {
  const source = readFileSync(filePath, 'utf8');
  for (const rule of globApiRules) {
    if (rule.test.test(source)) {
      problems.push(`Unexpected ${rule.label}: ${relative(uiRoot, filePath)}`);
    }
  }
}

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
  reviewedFirstPartySources: reviewedSourcePaths.length,
  buildScript: pkg.scripts?.build,
  globApiNegativeFixtures: attackFixtures.length,
  globApiCleanFixtures: cleanFixtures.length,
  lockedDevOnly: watched.map(name => ({ name, entries: Object.keys(lock.packages || {}).filter(p =>
    p === `node_modules/${name}` || p.endsWith(`/node_modules/${name}`)) })),
  issues: problems
}, null, 2));
if (problems.length) {
  console.error('Release input/dependency boundary changed: independent manual source and package review required.');
  process.exitCode = 1;
} else {
  console.log('No named parser modules in built assets/source maps; reviewed first-party source surface contains no direct glob API indicators.');
  console.log('This is LIMITED STATIC EVIDENCE; untrusted build-time globs can still be dangerous.');
  console.log('Does not waive npm audit, claim arbitrary ReDoS safety, or certify installed desktop packages.');
}
