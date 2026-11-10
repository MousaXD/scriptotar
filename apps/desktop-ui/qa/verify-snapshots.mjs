// Approved digest comparison: no automatic snapshot updating or silent tolerance.
import { createHash } from 'node:crypto';
import { readFile, readdir, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';

const evidence = resolve(process.cwd(), 'qa', 'evidence');
const manifestFile = resolve(process.cwd(), 'qa', 'baselines', 'manifest.json');
const execution = JSON.parse(await readFile(resolve(evidence, 'report.json'), 'utf8'));
if (!/^[a-f0-9]{40}$/.test(execution.sha || '') ||
    (process.env.QA_TESTED_SHA && execution.sha !== process.env.QA_TESTED_SHA) ||
    execution.screenshotCount !== 140 || execution.results?.length !== 12 ||
    !Array.isArray(execution.failures) || execution.failures.length > 0) {
  throw new Error('Cannot certify screenshots without successful, exact-SHA, 140-case browser evidence.');
}
const images = (await readdir(evidence)).filter((name) => name.endsWith('.png')).sort();
if (images.length !== 140) {
  throw new Error('Screenshot evidence missing or duplicated: expected 140 real PNG files, got ' + images.length);
}
let manifest;
try {
  manifest = JSON.parse(await readFile(manifestFile, 'utf8'));
} catch (error) {
  if (error.code !== 'ENOENT') throw error;
  console.log('NO APPROVED VISUAL BASELINE: capture-only until Agent 00 approves screenshots.');
  // Passing rendered interactions does not imply approved image-diff certification.
  await writeFile(resolve(evidence, 'comparison.json'),
    JSON.stringify({ comparison: 'NOT_CERTIFIED', reason: 'No manually approved visual baseline' }, null, 2) + '\n');
  process.exit(0);
}
const approved = manifest.screenshots ?? {};
const expected = Object.keys(approved).sort();
const differences = [];
if (!/^[a-f0-9]{40}$/.test(manifest.approvedSourceSha || '')) {
  differences.push('Baseline is missing an exact 40-character approvedSourceSha.');
}
if (JSON.stringify(images) !== JSON.stringify(expected)) {
  differences.push('Screenshot inventory changed: expected ' + expected.length + ', got ' + images.length);
}
for (const image of images) {
  const sum = createHash('sha256').update(await readFile(resolve(evidence, image))).digest('hex');
  if (sum !== approved[image]) differences.push(image + ': screenshot differs from the approved image hash');
}
const report = {
  baselineCommit: manifest.approvedSourceSha,
  screenshotCount: images.length,
  comparison: differences.length ? 'FAIL' : 'PASS',
  differences
};
await writeFile(resolve(evidence, 'comparison.json'), JSON.stringify(report, null, 2) + '\n');
console.log(JSON.stringify(report, null, 2));
if (differences.length) process.exitCode = 1;
