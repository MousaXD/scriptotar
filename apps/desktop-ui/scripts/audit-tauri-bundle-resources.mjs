#!/usr/bin/env node
/**
 * Bounded Tauri bundle resource provenance check for the CURRENT build workflows.
 * The checked-in tauri.conf.json is not the final bundle configuration.
 *
 * --manifest /path/to/list --platform linux|windows checks a REAL package file
 * inventory (dpkg-deb --contents for Linux; recursively enumerated installed
 * files for Windows). Without --manifest this only checks workflow source.
 *
 * This does NOT prove that bundled binaries are harmless or replace npm audit.
 */
import { existsSync, readFileSync } from 'node:fs';
import { dirname, resolve, join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';

const ui = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const root = resolve(ui, '..', '..');
const problems = [];
const expected = {
  'resources/transcription-runtime/': 'transcription-runtime/',
  'resources/mcp-tools/': 'mcp-tools/'
};
const definitions = [
  { workflow: 'linux-tauri.yml', platform: 'linux', target: 'deb', generator: "cat > src-tauri/tauri.linux.conf.json <<'JSON'", end: 'JSON' },
  { workflow: 'windows-tauri.yml', platform: 'windows', target: 'nsis', generator: "'@ | Set-Content -Encoding utf8 src-tauri/tauri.windows.conf.json", end: "@'" },
  { workflow: 'installed-webview-acceptance.yml', platform: 'linux', target: 'deb', generator: "cat > src-tauri/tauri.linux.conf.json <<'JSON'", end: 'JSON' },
  { workflow: 'installed-webview-acceptance.yml', platform: 'windows', target: 'nsis', generator: "'@ | Set-Content -Encoding utf8 src-tauri/tauri.windows.conf.json", end: "@'" }
];
function extractConfig(content, def) {
  const lines = content.split(/\r?\n/);
  if (def.platform === 'linux') {
    const positions = lines.flatMap((line, i) => line.includes(def.generator) ? [i] : []);
    if (positions.length !== 1) throw Error(`Expected precisely one Linux generator in ${def.workflow}, found ${positions.length}`);
    const start = positions[0] + 1;
    const end = lines.findIndex((line, i) => i >= start && line.trim() === def.end);
    if (end < 0) throw Error(`Unterminated Linux bundle JSON in ${def.workflow}`);
    return JSON.parse(lines.slice(start, end).join('\n'));
  }
  const endPositions = lines.flatMap((line, i) => line.includes(def.generator) ? [i] : []);
  if (endPositions.length !== 1) throw Error(`Expected precisely one Windows config generator in ${def.workflow}`);
  const end = endPositions[0];
  let start = end - 1;
  while (start >= 0 && lines[start].trim() !== "@'") start--;
  if (start < 0) throw Error(`Unterminated Windows bundle heredoc in ${def.workflow}`);
  return JSON.parse(lines.slice(start + 1, end).join('\n'));
}
const out = [];
for (const def of definitions) {
  const file = join(root, '.github', 'workflows', def.workflow);
  if (!existsSync(file)) { problems.push(`Missing packaging workflow: ${def.workflow}`); continue; }
  const source = readFileSync(file, 'utf8');
  try {
    const config = extractConfig(source, def);
    const resources = config.bundle?.resources;
    const actual = Object.entries(resources || {}).sort();
    const wanted = Object.entries(expected).sort();
    if (JSON.stringify(actual) !== JSON.stringify(wanted)) {
      problems.push(`Unexpected generated ${def.platform} bundle resource map in ${def.workflow}: ${JSON.stringify(actual)}`);
    }
    if (config.bundle?.active !== true || !config.bundle?.targets?.includes(def.target)) {
      problems.push(`Generated bundle target/activation changed in ${def.workflow} / ${def.platform}`);
    }
    // Source checks only: actual package inventory must be supplied via --manifest.
    if (!source.includes('sidecars/transcription/build_runtime.py') ||
        !source.includes('sidecars/transcription/validate_runtime.py') ||
        !source.includes('scripts/prepare_mcp_tools.py')) {
      problems.push(`Runtime/tool preparation provenance changed in ${def.workflow}`);
    }
    out.push({ workflow: def.workflow, platform: def.platform, resourceNames: Object.keys(resources || {}), configVerified: true });
  } catch (error) {
    problems.push(`Invalid generated Tauri config in ${def.workflow}/${def.platform}: ${error.message}`);
  }
}
function containsForbiddenPackagePath(entry) {
  const normalized = entry.replace(/\\/g, '/').toLowerCase();
  return /(?:^|\/)node_modules(?:\/|$)|(?:^|\/)(?:picomatch|tinyglobby|fdir|vite|vitest)(?:\/|\.|$)/.test(normalized);
}
function inspectManifest(manifest, platform, findings) {
  if (!['linux', 'windows'].includes(platform)) {
    findings.push('Manifest requires --platform linux or --platform windows');
    return { entries: 0, runtime: false, tools: false };
  }
  const raw = readFileSync(manifest, 'utf8').split(/\r?\n/).filter(Boolean);
  if (raw.length < 8) findings.push('Unexpectedly small package manifest, cannot certify package');
  let runtime = false, tools = false;
  for (const line of raw) {
    // Linux dpkg-deb --contents starts with permission, uid/gid, size, date.
    // Windows lines may be absolute paths from Get-ChildItem -Recurse -File.
    const path = line.replace(/\\/g, '/').toLowerCase();
    if (containsForbiddenPackagePath(line)) {
      findings.push(`Unexpected frontend parser/development module in package inventory: ${line.slice(-240)}`);
    }
    runtime ||= path.includes('transcription-runtime/');
    tools ||= path.includes('mcp-tools/');
  }
  if (!runtime) findings.push('Actual package manifest lacks transcription-runtime resource paths');
  if (!tools) findings.push('Actual package manifest lacks mcp-tools resource paths');
  return { entries: raw.length, runtime, tools };
}
const argv = process.argv.slice(2);
const idx = argv.indexOf('--manifest');
const pidx = argv.indexOf('--platform');
let inventory = null;
if (idx >= 0 || pidx >= 0) {
  if (idx < 0 || pidx < 0 || !argv[idx + 1] || !argv[pidx + 1]) {
    problems.push('Real package inventory validation requires BOTH --manifest and --platform');
  } else {
    const path = resolve(process.cwd(), argv[idx + 1]);
    if (!existsSync(path)) problems.push(`Actual package manifest missing: ${path}`);
    else inventory = inspectManifest(path, argv[pidx + 1], problems);
  }
}
// The SAME path-detection function used against actual package inventories
// must reject prohibited filenames, while leaving legitimate runtime files alone.
const manifestPathControls = [
  { name: 'Linux node_modules', value: './usr/share/scriptotar/node_modules/picomatch/index.js', forbidden: true },
  { name: 'Linux direct parser', value: './usr/share/scriptotar/picomatch/lib/parse.js', forbidden: true },
  { name: 'Windows node_modules', value: 'C:\\Scriptotar\\node_modules\\vite\\dist\\index.js', forbidden: true },
  { name: 'Windows nested parser', value: 'C:\\Scriptotar\\Resources\\tinyglobby\\index.js', forbidden: true },
  { name: 'Linux transcription runtime', value: './usr/lib/scriptotar/transcription-runtime/RUNTIME-VERSIONS.txt', forbidden: false },
  { name: 'Linux MCP tools', value: './usr/lib/scriptotar/mcp-tools/MCP-TOOLS-PROVENANCE.json', forbidden: false },
  { name: 'Windows transcription runtime', value: 'C:\\Scriptotar\\transcription-runtime\\scriptotar-engine.exe', forbidden: false },
  { name: 'Windows MCP tools', value: 'C:\\Scriptotar\\mcp-tools\\cloudflared.exe', forbidden: false }
];
for (const check of manifestPathControls) {
  if (containsForbiddenPackagePath(check.value) !== check.forbidden) {
    problems.push(`Package manifest path scanner failed its ${check.forbidden ? 'malicious' : 'clean'} fixture: ${check.name}`);
  }
}
const report = {
  title: 'Tauri generated config source and optional ACTUAL package manifest security inventory',
  configSources: out,
  packageManifest: inventory,
  pathMatcherFixtures: manifestPathControls.length,
  actualPackageManifestExamined: idx >= 0 && inventory !== null,
  issues: problems
};
console.log(JSON.stringify(report, null, 2));
if (problems.length) {
  console.error('Packaging resource contract changed or manifest contains unexpected parser modules.');
  process.exitCode = 1;
} else {
  console.log('Generated config contract PASS; real package-manifest result is separate and requires --manifest.');
  console.log('Not a binary content analysis, npm audit waiver or picomatch remediation.');
}
