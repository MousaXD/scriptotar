#!/usr/bin/env node
/**
 * Agent10 independent adversarial pre-Vite *scope* regression.
 *
 * Drives the actual separately checked-out immutable guard against disposable
 * copies of the real source/config, without installing npm or invoking Vite.
 * This test explicitly revalidates the previous RED against the newer
 * hardened 9dd384 guard, not its older vulnerable 75ec9e ancestor. It does not prove the original 19-pattern parser vulnerability is
 * patched or establish installed desktop-app security.
 */
import assert from 'node:assert/strict';
import {
  cpSync, existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, symlinkSync, writeFileSync,
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

  // Independent Agent10 negative controls beyond Agent11's fixture matrix.
  // Require a typed source/config finding, not just an arbitrary nonzero exit.
  function requireBlocked(label, mutate, reason) {
    const undo = mutate();
    try {
      const out = run();
      verdict(label, out, true);
      assert.notEqual(out.status, 0, label + ' unexpectedly accepted');
      assert.match(out.stdout, reason, label + ': missing expected finding');
    } finally { undo(); }
  }
  for (const [label, stmt, reason] of [
    ['static absolute import', "import '/qa/hostile.ts';", /Unreviewed absolute\/root-relative module import/],
    ['export-from absolute', "export { modules } from '/qa/hostile.ts';", /Unreviewed absolute\/root-relative module import/],
    ['dynamic absolute', "void import('/qa/hostile.ts');", /Unreviewed absolute\/root-relative module import/],
    ['commented dynamic absolute', "void import(/* review */ '/qa/hostile.ts');", /Unreviewed absolute\/root-relative module import/],
    ['absolute backtick dynamic', "void import(\x60/qa/hostile.ts\x60);", /Unreviewed absolute\/root-relative module import/],
    ['nested relative traversal', "import './stage/../../qa/hostile.ts';", /Unreviewed out-of-scope relative source reference/],
    ['export-from traversal', "export { modules } from '../qa/hostile.ts';", /Unreviewed out-of-scope relative source reference/],
  ]) {
    requireBlocked(label, () => {
      write(main, original + '\n' + stmt + '\n');
      return () => write(main, original);
    }, reason);
  }
  // Policy/config paths that independently influence actual source compilation.
  for (const [label, path, body] of [
    ['shadow workspace npmrc', join(ws, '.npmrc'), 'ignore-scripts=false\n'],
    ['shadow ancestor npmrc', join(tmp, 'apps', '.npmrc'), 'registry=https://example.invalid\n'],
    ['shadow package shrinkwrap', join(ws, 'npm-shrinkwrap.json'), '{}\n'],
    ['shadow ancestor PostCSS', join(tmp, '.postcssrc.yaml'), 'plugins: []\n'],
    ['unreviewed PostCSS TOML', join(ws, '.postcssrc.toml'), 'plugins = []\n'],
    ['alternate PostCSS YAML config', join(ws, 'postcss.config.yaml'), 'plugins: []\n'],
    ['unreviewed alternate Vite config', join(ws, 'vite.config.mjs'), 'export default {}\n'],
    ['shadow Vite environment', join(ws, '.env.production'), 'VITE_MODE=evil\n'],
  ]) {
    requireBlocked(label, () => {
      write(path, body);
      return () => rmSync(path, {force:true});
    }, /Unreviewed shadow npm\/build configuration|New unreviewed build configuration file/);
  }
  const pkg = join(ws, 'package.json');
  const pkgOriginal = readFileSync(pkg, 'utf8');
  requireBlocked('alternate npm compiler command', () => {
    write(pkg, pkgOriginal.replace('"vite build"', '"vite build --config evil.config.ts"'));
    return () => write(pkg, pkgOriginal);
  }, /Unreviewed build manifest\/lock\/Tauri contract/);
  const viteConfig = join(ws, 'vite.config.ts');
  const viteOriginal = readFileSync(viteConfig, 'utf8');
  requireBlocked('unreviewed Vite plugin and source alias', () => {
    write(viteConfig, viteOriginal + '\n// eslint-plugin dynamic alias\n');
    return () => write(viteConfig, viteOriginal);
  }, /Unreviewed Vite config SHA-256/);
  const index = join(ws, 'index.html');
  const htmlOriginal = readFileSync(index, 'utf8');
  requireBlocked('alternate HTML module entrypoint', () => {
    write(index, htmlOriginal + '\n<script type="module" src="/qa/hostile.ts"></script>\n');
    return () => write(index, htmlOriginal);
  }, /Unreviewed build manifest\/lock\/Tauri contract/);
  // Windows runners may disallow creating symlinks without privilege. Record
  // NOT TESTED rather than silently converting unsupported into PASS.
  const symlinkPath = join(ws, 'src', 'agent10-hostile-symlink.ts');
  let linked = false;
  try {
    symlinkSync(qa, symlinkPath, 'file');
    linked = true;
  } catch (error) {
    console.log(JSON.stringify({test:'source symlink escape',outcome:'NOT TESTED',reason:error.code}));
  }
  if (linked) {
    try {
      const out = run();
      verdict('source symlink escape', out, true);
      assert.notEqual(out.status,0);
      assert.match(out.stdout,/Source symlink requires review/);
    } finally {rmSync(symlinkPath,{force:true});}
  }
  // Deliberately demonstrate the boundary of this tool: the scanner does not
  // inspect workflow files. A malicious PR can bypass an unenforced workflow.
  // This is a governance RED, not an exploit result against the new source scanner.
  const untrustedWorkflow = join(tmp,'.github','workflows','ci.yml');
  write(untrustedWorkflow,'name: bypassed-guard\njobs: {}\n');
  const workflowGap=run();
  console.log(JSON.stringify({test:'PR-authored workflow replacement',guard_status:workflowGap.status,
    classification:'UNPROTECTED CI POLICY GAP: scanner cannot independently enforce its own invocation'}));
  assert.equal(workflowGap.status,0,'Workflow-file-only change must not be misclassified as scanner coverage');
  rmSync(join(tmp,'.github'),{recursive:true,force:true});
  
  assert.deepEqual(bypasses, [], 'UNBLOCKED PREBUILD GUARD SCOPES: ' + bypasses.join(', '));
  console.log('PASS: hardened guard rejected tested source and build-config escapes; workflow governance remains separate');
} finally {
  rmSync(tmp, { recursive: true, force: true });
}
