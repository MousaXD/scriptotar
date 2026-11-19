#!/usr/bin/env node
/**
 * Agent10 independent BOUNDED actual Vite compiler reachability probe.
 * The policy guard is tested by agent10_prebuild_scope_probe.mjs; this script
 * deliberately asks the compiler to process a hostile but local source pattern
 * in a disposable CI source checkout.  No app is installed, no credentials,
 * remote network input, or user data are involved.  Always kill after 12 sec.
 *
 * A timed-out hostile Vite build is evidence of vulnerability, NOT an approval
 * or a safe parser test.  The original 19-pattern parser RED remains separate.
 */
import assert from 'node:assert/strict';
import { existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const repo = resolve(dirname(fileURLToPath(import.meta.url)), '..', '..', '..');
const ui = join(repo, 'apps', 'desktop-ui');
const viteBin = join(ui, 'node_modules', 'vite', 'bin', 'vite.js');
assert.ok(existsSync(viteBin), 'Run npm ci in apps/desktop-ui before the Vite reachability probe');

const main = join(ui, 'src', 'main.ts');
const original = readFileSync(main, 'utf8');
const qa = join(ui, 'agent10-scope-fixture');
const staging = join(ui, 'src', 'agent10-staging');
const saveQa = existsSync(qa);
const saveStaging = existsSync(staging);
assert.equal(saveQa, false, 'Do not touch an existing qa directory');
assert.equal(saveStaging, false, 'Do not touch an existing src/staging directory');

function build(timeout) {
  return spawnSync(process.execPath, [viteBin, 'build'], {
    cwd: ui, encoding: 'utf8', timeout, maxBuffer: 1024 * 1024,
    windowsHide: true, env: { ...process.env, CI: 'true' },
  });
}
function excerpt(r) {
  return ((r.stdout || '') + '\n' + (r.stderr || '')).slice(-1600);
}
try {
  // Distinguish CI/package slowness and baseline failure from a crafted pattern.
  const baseline = build(45000);
  assert.equal(baseline.status, 0,
    'Baseline Vite build not healthy; cannot attribute hostile timeouts: ' + excerpt(baseline));
  console.log('BASELINE: Vite compiled original Scriptotar frontend successfully');

  mkdirSync(qa, { recursive: false });
  mkdirSync(staging, { recursive: false });
  writeFileSync(join(qa, 'hostile.ts'),
    "export const modules = import.meta.glob('./+(*a)');\n");
  writeFileSync(join(qa, 'a'.repeat(36) + 'b'),
    'near-miss name, NOT a pattern; the pattern is in hostile.ts\n');
  // Independent false-positive control: the same >400-byte normalized path
  // must resolve and compile when its target is ordinary benign source.
  // Otherwise a malicious-build timeout could be attributed to path length.
  writeFileSync(join(qa, 'benign.ts'), 'export const expectedSafe = 1;\n');
  const harmlessLongSpecifier = './agent10-staging/' + './'.repeat(220) +
    '../../agent10-scope-fixture/benign.ts';
  assert.ok(harmlessLongSpecifier.length > 400);
  writeFileSync(main, original + '\nimport ' + JSON.stringify(harmlessLongSpecifier) + ';\n');
  const harmlessLongBuild = build(45000);
  console.log(JSON.stringify({
    test: 'long import path resolves to safe TypeScript module',
    specifier_length: harmlessLongSpecifier.length,
    outcome: harmlessLongBuild.status === 0 ? 'PASS' : 'NOT VALIDATED',
    status: harmlessLongBuild.status,
    error_code: harmlessLongBuild.error?.code || null,
    log_tail: excerpt(harmlessLongBuild)
  }));
  assert.equal(harmlessLongBuild.status, 0,
    'Overlength import did not resolve harmless source; malicious timeout could be confounded: '+excerpt(harmlessLongBuild));

  // Actual Vite positive controls distinguish encoded-string decoding from
  // generic parsing failures. Benign external target must compile cleanly.
  const escapedBenignVariants = [
    ['unicode', String.raw`.\u002e/agent10-scope-fixture/benign.ts`],
    ['unicode-brace', String.raw`.\u{2e}/agent10-scope-fixture/benign.ts`],
    ['hex', String.raw`.\x2e/agent10-scope-fixture/benign.ts`],
  ];
  for (const [kind, specifier] of escapedBenignVariants) {
    writeFileSync(main, original + "\nimport '" + specifier + "';\n");
    const ok=build(45000);
    console.log(JSON.stringify({test:'JS '+kind+' escaped ../ safe module import',
      outcome:ok.status===0?'PASS':'NOT VALIDATED',status:ok.status,
      error_code:ok.error?.code||null,log_tail:excerpt(ok)}));
    assert.equal(ok.status,0,'Encoded relative import failed safe resolution for '+kind+': '+excerpt(ok));
  }
  const variants = [
    { label:'Vite hexadecimal-escaped relative traversal import',
      specifier:String.raw`.\x2e/agent10-scope-fixture/hostile.ts`,directLiteral:true },
    { label:'Vite Unicode-brace escaped relative traversal import',
      specifier:String.raw`.\u{2e}/agent10-scope-fixture/hostile.ts`,directLiteral:true },
    { label:'Vite Unicode-escaped relative traversal import',
      specifier:String.raw`.\u002e/agent10-scope-fixture/hostile.ts`,
      directLiteral:true },
    { label: 'Vite project-root-absolute import', specifier: '/agent10-scope-fixture/hostile.ts' },
    { label: 'Vite normalized relative import escape', specifier: './agent10-staging/../../agent10-scope-fixture/hostile.ts' },
    { label: 'Vite over-400-character normalized relative import escape', specifier: './agent10-staging/' + './'.repeat(220) + '../../agent10-scope-fixture/hostile.ts' },
  ];
  for (const variant of variants) {
    const sourceLiteral = variant.directLiteral ? ("'" + variant.specifier + "'") : JSON.stringify(variant.specifier);
    writeFileSync(main, original + '\nimport ' + sourceLiteral + ';\n');
    const hostile = build(12000);
    const timeout = hostile.error?.code === 'ETIMEDOUT';
    console.log(JSON.stringify({
      test: variant.label, specifier: variant.specifier, timeout_ms: 12000,
      outcome: timeout ? 'HOSTILE_VITE_BUILD_TIMEOUT (vulnerability evidence)' :
        hostile.status === 0 ? 'BUILD_FINISHED (not reproduced)' : 'BUILD_ERROR (not reproduced)',
      exit_code: hostile.status, error_code: hostile.error?.code || null,
      log_tail: excerpt(hostile),
    }));
    assert.ok(timeout,
      'Did not reproduce a bounded hostile-pattern Vite timeout via '+variant.specifier+
      '; this is NOT PASS, independently review import resolution and compiler logs');
  }
  // Independent compiler-side confirmation for the .mts scanner blind spot.
  // The .mts file is inside src (not an out-of-root import); the old scanner's missing suffix
  // previously prevented the guard from reading its import.meta.glob macro.
  const mtsDir=join(ui,'src','agent10-unreviewed-mts');
  assert.equal(existsSync(mtsDir),false,'Refusing to overwrite a real source directory');
  mkdirSync(mtsDir,{recursive:false});
  try {
    writeFileSync(join(mtsDir,'hostile.mts'),
      "export const modules = import.meta.glob('./+(*a)');\n");
    writeFileSync(join(mtsDir,'a'.repeat(36)+'b'),'near miss only\n');
    writeFileSync(main,original+"\nimport './agent10-unreviewed-mts/hostile.mts';\n");
    const hostileMts=build(12000);
    const mtsTimeout=hostileMts.error?.code==='ETIMEDOUT';
    console.log(JSON.stringify({test:'Vite imports unreviewed .mts module containing malicious glob',
      timeout_ms:12000, outcome:mtsTimeout?'HOSTILE_VITE_BUILD_TIMEOUT (real compiler reachability)':
        hostileMts.status===0?'BUILD_FINISHED (not reproduced)':'BUILD_ERROR (not reproduced)',
      exit_code:hostileMts.status,error_code:hostileMts.error?.code||null,log_tail:excerpt(hostileMts)}));
    assert.ok(mtsTimeout,'Vite .mts hostile module was not shown to time out; review logs before claiming exploitability');
  } finally {
    writeFileSync(main,original);
    rmSync(mtsDir,{recursive:true,force:true});
  }
  console.log('Seven independent hostile Vite build routes TIMED OUT; parser NOT fixed.');
} finally {
  writeFileSync(main, original);
  if (!saveQa) rmSync(qa, { recursive: true, force: true });
  if (!saveStaging) rmSync(staging, { recursive: true, force: true });
}
