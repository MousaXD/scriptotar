// Manual-only action after reviewing screenshot artifacts, NEVER run from CI.
import { createHash } from 'node:crypto';
import { readFile, readdir, mkdir, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';

const sha = process.env.QA_APPROVED_SHA;
if (!/^[a-f0-9]{40}$/.test(sha || '')) {
  throw new Error('Set QA_APPROVED_SHA to the exact reviewed 40-character commit.');
}
const directory = resolve(process.cwd(), 'qa');
const evidence = resolve(directory, 'evidence');
const report = JSON.parse(await readFile(resolve(evidence, 'report.json'), 'utf8'));
if (report.sha !== sha || report.failures.length || report.screenshotCount !== 140) {
  throw new Error('Refusing unpinned, incomplete or failing screenshot evidence.');
}
const images = (await readdir(evidence)).filter((name) => name.endsWith('.png')).sort();
if (images.length !== 140) throw new Error('Expected 140 real rendered screenshots.');
const screenshots = {};
for (const name of images) {
  screenshots[name] = createHash('sha256')
    .update(await readFile(resolve(evidence, name))).digest('hex');
}
await mkdir(resolve(directory, 'baselines'), { recursive: true });
await writeFile(resolve(directory, 'baselines', 'manifest.json'),
  JSON.stringify({ approvedSourceSha: sha, reviewedBy: 'Agent 00 manual review required',
    note: 'Review all browser artifact screenshots before committing this manifest.', screenshots }, null, 2) + '\n');
console.log('Baseline manifest generated. Commit only AFTER screenshot review and explicit approval.');
