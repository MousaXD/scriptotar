// Unit tests for the evidence verifier only; these are NOT rendered UI tests.
// Synthetic bytes live in a temporary directory and never count as screenshot evidence.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdtemp, mkdir, writeFile, readFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';

const verifier = resolve('qa', 'verify-snapshots.mjs');
const sha = 'a'.repeat(40);

async function withEvidence(callback) {
  const dir = await mkdtemp(join(tmpdir(), 'scriptotar-qa-tool-'));
  try {
    await mkdir(join(dir, 'qa', 'evidence'), { recursive: true });
    return await callback(dir);
  } finally {
    await rm(dir, { recursive: true, force: true });
  }
}
function run(dir) {
  return spawnSync(process.execPath, [verifier], {
    cwd: dir,
    env: { ...process.env, QA_TESTED_SHA: sha },
    encoding: 'utf8',
    timeout: 15000
  });
}
async function populate(dir, successful = true) {
  const root = join(dir, 'qa', 'evidence');
  const images = {};
  for (let i = 0; i < 140; i += 1) {
    const name = String(i).padStart(3, '0') + '.png';
    // This deliberately tests only hashing/count/manifest logic, not image encoding.
    const bytes = Buffer.from('isolated verifier fixture ' + name);
    await writeFile(join(root, name), bytes);
    images[name] = createHash('sha256').update(bytes).digest('hex');
  }
  const report = {
    sha,
    screenshotCount: 140,
    results: Array.from({ length: 12 }, () => ({})),
    failures: successful ? [] : [{ scenario: 'fixture', error: 'simulated browser failure' }]
  };
  await writeFile(join(root, 'report.json'), JSON.stringify(report));
  return images;
}

test('missing browser report cannot be called an uncertified visual pass', async () => {
  await withEvidence(async (dir) => {
    const result = run(dir);
    assert.notEqual(result.status, 0, result.stdout + result.stderr);
  });
});
test('complete successful run without approved baseline is explicitly NOT_CERTIFIED', async () => {
  await withEvidence(async (dir) => {
    await populate(dir);
    const result = run(dir);
    assert.equal(result.status, 0, result.stdout + result.stderr);
    const comparison = JSON.parse(await readFile(join(dir, 'qa', 'evidence', 'comparison.json'), 'utf8'));
    assert.equal(comparison.comparison, 'NOT_CERTIFIED');
  });
});
test('recorded browser failures reject the evidence before baseline comparison', async () => {
  await withEvidence(async (dir) => {
    await populate(dir, false);
    const result = run(dir);
    assert.notEqual(result.status, 0, result.stdout + result.stderr);
  });
});
test('approved hashes pass unchanged and reject modified screenshot bytes', async () => {
  await withEvidence(async (dir) => {
    const screenshots = await populate(dir);
    await mkdir(join(dir, 'qa', 'baselines'), { recursive: true });
    const manifestPath = join(dir, 'qa', 'baselines', 'manifest.json');
    const manifest = {
      approvedSourceSha: sha, reviewStatus: 'PENDING_MANUAL_REVIEW',
      reviewedBy: null, reviewedAt: null, screenshots
    };
    await writeFile(manifestPath, JSON.stringify(manifest));
    let result = run(dir);
    assert.notEqual(result.status, 0, 'An unreviewed baseline must be rejected');
    manifest.reviewStatus = 'APPROVED';
    manifest.reviewedBy = 'Agent 00';
    manifest.reviewedAt = '2026-11-11T00:00:00Z';
    await writeFile(manifestPath, JSON.stringify(manifest));
    result = run(dir);
    assert.equal(result.status, 0, result.stdout + result.stderr);
    let comparison = JSON.parse(await readFile(join(dir, 'qa', 'evidence', 'comparison.json'), 'utf8'));
    assert.equal(comparison.comparison, 'PASS');

    await writeFile(join(dir, 'qa', 'evidence', '001.png'), 'changed fixture bytes');
    result = run(dir);
    assert.notEqual(result.status, 0, result.stdout + result.stderr);
    comparison = JSON.parse(await readFile(join(dir, 'qa', 'evidence', 'comparison.json'), 'utf8'));
    assert.equal(comparison.comparison, 'FAIL');
    assert.ok(comparison.differences.some((line) => line.includes('001.png')));
  });
});
