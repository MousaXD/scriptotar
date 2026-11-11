// Independent paired browser metrics comparison. Read only; no thresholds or invented speedups.
// Usage: node qa/compare-browser-metrics.mjs <baseline-report.json> <candidate-report.json>
import { readFile } from 'node:fs/promises';

const [baselinePath, candidatePath] = process.argv.slice(2);
if (!baselinePath || !candidatePath) {
  throw new Error('Usage: node qa/compare-browser-metrics.mjs <baseline-report.json> <candidate-report.json>');
}
const baseline = JSON.parse(await readFile(baselinePath, 'utf8'));
const candidate = JSON.parse(await readFile(candidatePath, 'utf8'));
const metrics = ['shellMs', 'usefulWorkspaceMs', 'warmShellMs', 'warmUsefulWorkspaceMs'];
const shaPattern = /^[a-f0-9]{40}$/;

function verify(report, name) {
  if (!shaPattern.test(report.sha || '')) throw new Error(name + ' has no immutable 40-char SHA');
  if (report.scope?.includes('NOT native Tauri') !== true) {
    throw new Error(name + ' is not labeled as browser-only evidence');
  }
  if (!Array.isArray(report.failures) || report.failures.length ||
      !Array.isArray(report.results) || report.results.length !== 12 ||
      report.screenshotCount !== 140) {
    throw new Error(name + ' has missing, incomplete, or failed screenshot/interaction evidence');
  }
  const labels = new Set();
  for (const sample of report.results) {
    if (!sample.label || sample.mode !== 'browser-mock-client' ||
        labels.has(sample.label) || !Array.isArray(sample.navigation) ||
        sample.navigation.length !== 7) {
      throw new Error(name + ': invalid or duplicate scenario');
    }
    labels.add(sample.label);
    for (const metric of metrics) {
      if (typeof sample[metric] !== 'number' ||
          !Number.isFinite(sample[metric]) || sample[metric] < 0) {
        throw new Error(name + ': missing measured ' + metric + ' on ' + sample.label);
      }
    }
    for (const row of sample.navigation) {
      if (!row.view || typeof row.navigationMs !== 'number' ||
          !Number.isFinite(row.navigationMs) || row.navigationMs < 0) {
        throw new Error(name + ': invalid navigation sample on ' + sample.label);
      }
    }
  }
}
verify(baseline, 'baseline');
verify(candidate, 'candidate');
const initial = new Map(baseline.results.map((row) => [row.label, row]));
const final = new Map(candidate.results.map((row) => [row.label, row]));
if ([...initial.keys()].sort().join('|') !== [...final.keys()].sort().join('|')) {
  throw new Error('Baseline and candidate have different viewport/theme/locale scenarios');
}
const median = (values) => {
  const sorted = [...values].sort((a, b) => a - b);
  const middle = Math.floor(sorted.length / 2);
  return sorted.length % 2 ? sorted[middle] : (sorted[middle - 1] + sorted[middle]) / 2;
};
function compare(before, after) {
  const old = median(before);
  const next = median(after);
  return {
    baselineMedianMs: Number(old.toFixed(3)),
    candidateMedianMs: Number(next.toFixed(3)),
    deltaMs: Number((next - old).toFixed(3)),
    deltaPercent: old === 0 ? null : Number(((next - old) / old * 100).toFixed(2))
  };
}
const summaries = Object.fromEntries(metrics.map((metric) => [
  metric,
  compare([...initial.values()].map((row) => row[metric]), [...final.values()].map((row) => row[metric]))
]));
const views = baseline.results[0].navigation.map((sample) => sample.view);
const navigation = Object.fromEntries(views.map((view) => {
  const before = [...initial.values()].map((row) => row.navigation.find((x) => x.view === view)?.navigationMs);
  const after = [...final.values()].map((row) => row.navigation.find((x) => x.view === view)?.navigationMs);
  if ([...before, ...after].some((value) => !Number.isFinite(value))) {
    throw new Error('Unmatched navigation view ' + view);
  }
  return [view, compare(before, after)];
}));
console.log(JSON.stringify({
  scope: 'BROWSER MOCK only; driver wall-clock latency, not native application startup',
  baselineSha: baseline.sha,
  candidateSha: candidate.sha,
  matchedScenarios: baseline.results.length,
  interpretation: 'Negative deltas are faster. No automatic statistical or release conclusion.',
  metrics: summaries,
  navigation
}, null, 2));
