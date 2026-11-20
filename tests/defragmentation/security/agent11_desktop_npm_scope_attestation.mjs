#!/usr/bin/env node
/**
 * Agent11: production-vs-build dependency boundary at one reviewed npm lock.
 * Evidence only. NO claim of safe execution of picomatch in Vite, Vitest,
 * installer workflows or the underlying Node regex engine. NO security waiver.
 *
 * This is intentionally pinned: a changed lock/manifest requires a fresh
 * full manual review rather than inheriting a stale "dev only" conclusion.
 */
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { execFileSync } from 'node:child_process';

const lockPath = 'apps/desktop-ui/package-lock.json';
const manifestPath = 'apps/desktop-ui/package.json';
const expectedLockBlob = 'eebc9228dd404f0b9f9bd55f2665ab3e00fe9dfa';
const expectedManifestBlob = '19edb804b2c442be96311e2f39f7788dfb286dda';
const expectedIntegrity = 'sha512-qcJu88Q2IWqJsDD529JKMdwGm/dvInW4HvQnRwiH9JtihJvzGOscDtHE3x1pBKeUOTysQ8kVmLnJ2kJu7yhcGA==';
const expectedImporters = new Map([
  ['node_modules/vite', {version:'7.3.7', range:'^4.0.3'}],
  ['node_modules/vitest', {version:'5.1.0', range:'^4.0.7'}],
  ['node_modules/tinyglobby', {version:'0.2.17', range:'^4.0.4'}]
]);

function verifySnapshot(lock, manifest) {
  assert.equal(lock.lockfileVersion, 3, 'unexpected npm lockfile format');
  assert.ok(lock.packages && typeof lock.packages === 'object', 'missing package entries');
  const root = lock.packages[''];
  assert.ok(root, 'missing root lockfile package');
  assert.equal(root.name, '@scriptotar/desktop-ui');
  assert.equal(root.name, manifest.name, 'manifest and lock disagree');
  assert.equal(root.version, manifest.version, 'manifest version drift');
  // Conservative: fail closed on any new root runtime, optional or peer edge.
  for (const field of ['dependencies','optionalDependencies','peerDependencies']) {
    assert.equal(Object.keys(root[field] || {}).length, 0,
      'Root package has ' + field + '; runtime reachability needs renewed review');
    assert.equal(Object.keys(manifest[field] || {}).length, 0,
      'Manifest has ' + field + '; runtime reachability needs renewed review');
  }
  assert.deepEqual(root.devDependencies, manifest.devDependencies,
    'root devDependency graph does not match manifest');
  assert.ok(Object.keys(root.devDependencies || {}).length > 0, 'No expected development deps');

  const matches = Object.entries(lock.packages).filter(([name]) => (
    name === 'node_modules/picomatch' || name.endsWith('/node_modules/picomatch')
  ));
  assert.equal(matches.length, 1,
    'Unexpected missing/duplicate/nested picomatch installation requires renewed review');
  const [path, pkg] = matches[0];
  assert.equal(path, 'node_modules/picomatch');
  assert.equal(pkg.version, '4.0.7');
  assert.equal(pkg.dev, true, 'picomatch no longer dev-only');
  assert.equal(pkg.integrity, expectedIntegrity, 'package bytes/integrity drift');
  assert.equal(pkg.resolved, 'https://registry.npmjs.org/picomatch/-/picomatch-4.0.7.tgz',
    'unexpected registry or tarball provenance');

  const found = new Map();
  for (const [name, p] of Object.entries(lock.packages)) {
    const range = p?.dependencies?.picomatch;
    const optional = p?.optionalDependencies?.picomatch;
    const peer = p?.peerDependencies?.picomatch;
    if (name !== '' && (range || optional || peer)) {
      assert.ok(range && !optional && !peer, 'Unexpected optional or peer edge: ' + name);
      assert.equal(p.dev, true, 'production importer can reach picomatch: ' + name);
      found.set(name, { version: p.version, range });
    }
  }
  assert.equal(found.size, expectedImporters.size, 'Unexpected direct picomatch importer');
  for (const [name, want] of expectedImporters) {
    assert.ok(found.has(name), 'Required verified importer missing: ' + name);
    assert.deepEqual(found.get(name), want, 'Unexpected importer version/range: ' + name);
  }
  return {
    lockfile_git_blob: expectedLockBlob,
    root_runtime_dependencies: 0,
    root_dev_dependencies: Object.keys(root.devDependencies).length,
    total_lock_package_records: Object.keys(lock.packages).length,
    picomatch_version: pkg.version,
    picomatch_dev_only: true,
    direct_dev_importers: [...found].map(([name, meta]) => ({name,...meta})),
    conclusion: 'STATIC reviewed npm-tree exposure only; browser bundle/CI build-time separately unverified',
    parser_remediation: 'NO',
    release_authorization: 'NO-GO'
  };
}
function expectFailure(lock, manifest, label, mutate) {
  const l = structuredClone(lock);
  const m = structuredClone(manifest);
  mutate(l,m);
  assert.throws(() => verifySnapshot(l,m), undefined, label+' mutation was incorrectly accepted');
  console.log('FAIL-CLOSED CONTROL PASS: '+label);
}

const fileBlob = p => execFileSync('git', ['hash-object',p], {encoding:'utf8'}).trim();
assert.equal(fileBlob(lockPath),expectedLockBlob,'desktop lock changed since source review');
assert.equal(fileBlob(manifestPath),expectedManifestBlob,'desktop package manifest changed since source review');
const lock = JSON.parse(readFileSync(lockPath,'utf8'));
const manifest = JSON.parse(readFileSync(manifestPath,'utf8'));
const result = verifySnapshot(lock, manifest);
expectFailure(lock,manifest,'runtime dependency injection',(l,m) => {
  l.packages[''].dependencies = {vite:'^7.3.7'};
  m.dependencies = {vite:'^7.3.7'};
});
expectFailure(lock,manifest,'optional runtime dependency injection',(l,m) => {
  l.packages[''].optionalDependencies = {tinyglobby:'0.2.17'};
  m.optionalDependencies = {tinyglobby:'0.2.17'};
});
expectFailure(lock,manifest,'picomatch dev flag stripped',(l) => {
  l.packages['node_modules/picomatch'].dev = false;
});
expectFailure(lock,manifest,'picomatch bytes drift',(l) => {
  l.packages['node_modules/picomatch'].integrity = 'sha512-replaced';
});
expectFailure(lock,manifest,'extra nested picomatch instance',(l) => {
  l.packages['node_modules/vite/node_modules/picomatch'] =
    structuredClone(l.packages['node_modules/picomatch']);
});
expectFailure(lock,manifest,'runtime importer flag stripped',(l) => {
  l.packages['node_modules/vite'].dev = false;
});
expectFailure(lock,manifest,'new direct importer',(l) => {
  l.packages['node_modules/jsdom'].dependencies = {
    ...(l.packages['node_modules/jsdom'].dependencies||{}),
    picomatch: '^4.0.7'
  };
});
console.log('SOURCE-EXACT NPM SCOPE ATTESTATION: '+JSON.stringify(result));
