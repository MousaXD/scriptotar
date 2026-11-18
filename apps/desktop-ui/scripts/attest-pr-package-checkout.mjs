#!/usr/bin/env node
/**
 * Attest the Git TREE actually checked out by an original Tauri packaging
 * workflow. GitHub pull_request jobs normally build refs/pull/N/merge, not
 * the literal PR HEAD. Compare immutable merge and HEAD trees before
 * calling a package source-equivalent. This does not certify binary contents
 * or fix the vulnerable picomatch parser.
 */
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';

const args = process.argv.slice(2);
const readArg = flag => { const i = args.indexOf(flag); return i < 0 ? undefined : args[i + 1]; };
const shaRegex = /^[0-9a-f]{40}$/;

function parseCheckout(log, pr) {
  const ref = 'refs/remotes/pull/' + pr + '/merge';
  const sections = log.split('##[group]Fetching the repository').slice(1)
    .map(s => s.split('##[endgroup]')[0]);
  const found = sections.flatMap(s => [...s.matchAll(/\+([a-f0-9]{40}):refs\/remotes\/pull\/(\d+)\/merge\b/g)]
    .filter(m => Number(m[2]) === pr).map(m => m[1]));
  const unique = [...new Set(found)];
  assert.equal(unique.length, 1, 'one immutable PR merge checkout fetch SHA required');
  assert.ok(log.includes('checkout --progress --force ' + ref), 'actual checkout ref must match fetched PR merge');
  return unique[0];
}

function verify(input) {
  const { run, job, checkoutSha, merge, head, expectedHead, expectedBranch, pr } = input;
  assert.match(expectedHead || '', shaRegex);
  assert.ok(Number.isSafeInteger(pr) && pr > 0);
  assert.equal(run.head_sha, expectedHead, 'wrong original run HEAD');
  assert.equal(run.head_branch, expectedBranch, 'wrong branch');
  assert.equal(run.event, 'pull_request', 'not original PR build');
  assert.equal(run.status, 'completed');
  assert.equal(run.conclusion, 'success', 'original package run failed');
  assert.equal(job.conclusion, 'success', 'original package job failed');
  assert.equal(merge.sha, checkoutSha, 'wrong immutable checkout commit');
  assert.equal(head.sha, expectedHead, 'wrong immutable HEAD commit');
  assert.equal(merge.parents?.length, 2, 'checkout was not a two-parent merge');
  assert.match(merge.parents[0]?.sha || '', shaRegex, 'missing base SHA');
  assert.equal(merge.parents[1]?.sha, expectedHead, 'merge second parent differs from reviewed HEAD');
  assert.match(head.tree?.sha || '', shaRegex, 'missing HEAD tree');
  assert.equal(merge.tree?.sha, head.tree?.sha, 'packaged PR merge TREE does not equal reviewed HEAD TREE');
  return {
    exactSourceTreeMatch: true, originalRun: run.id, originalJob: job.id,
    pr, reviewedHead: expectedHead, checkoutCommit: checkoutSha,
    originalBase: merge.parents[0].sha, sourceTree: head.tree.sha,
    scope: 'Source-tree equality only. Generated resources, package bytes and embedded parser reachability require separate analysis.'
  };
}

function gh(path, json = true) {
  const repo = process.env.GITHUB_REPOSITORY;
  assert.match(repo || '', /^[A-Za-z0-9_.-]+\/[A-Za-z0-9_.-]+$/, 'repository required');
  // GitHub job logs legitimately contain runner ANSI formatting. gh refuses
  // terminal escape sequences by default; opt in ONLY for the read-only log
  // download, never for trusted JSON and never by stripping checkout evidence.
  const ghArgs = ['api', ...(json ? [] : ['--allow-escape-sequences']), 'repos/' + repo + '/' + path];
  const result = spawnSync('gh', ghArgs, {
    encoding: 'utf8', maxBuffer: 64 * 1024 * 1024, timeout: 120000
  });
  if (result.error || result.status !== 0) {
    throw new Error('GitHub API read failed at ' + path + ': ' +
      (result.error?.message || result.stderr?.slice(-600) || result.status));
  }
  return json ? JSON.parse(result.stdout) : result.stdout;
}

function selfTest() {
  const a = 'a'.repeat(40), b = 'b'.repeat(40), c = 'c'.repeat(40), d = 'd'.repeat(40);
  const log = '##[group]Fetching the repository\n[command]git fetch +' + a +
    ':refs/remotes/pull/132/merge\n##[endgroup]\n##[group]Checking out the ref\n' +
    '[command]git checkout --progress --force refs/remotes/pull/132/merge\n##[endgroup]';
  assert.equal(parseCheckout(log, 132), a);
  assert.throws(() => parseCheckout(log, 133));
  assert.throws(() => parseCheckout(log.replace('checkout --progress', 'checkout --wrong'), 132));
  assert.throws(() => parseCheckout(log.replace(a, 'invalid'), 132));
  const fixture = {
    run: { id: 1, head_sha: b, head_branch: 'agent-11/p0-supply-chain-recovery-20261118',
      event: 'pull_request', status: 'completed', conclusion: 'success' },
    job: { id: 2, conclusion: 'success' }, checkoutSha: a,
    merge: { sha: a, parents: [{sha:c},{sha:b}], tree:{sha:d} },
    head: {sha:b,tree:{sha:d}}, expectedHead:b,
    expectedBranch:'agent-11/p0-supply-chain-recovery-20261118', pr:132
  };
  assert.equal(verify(fixture).exactSourceTreeMatch, true);
  assert.throws(() => verify({...fixture, merge:{...fixture.merge, tree:{sha:c}}}));
  assert.throws(() => verify({...fixture, merge:{...fixture.merge, parents:[{sha:b},{sha:c}]}}));
  assert.throws(() => verify({...fixture, run:{...fixture.run, head_sha:c}}));
  assert.throws(() => verify({...fixture, job:{...fixture.job, conclusion:'failure'}}));
  console.log('SYNTHETIC 9/9 source tree controls PASS; not a real package or binary attestation.');
}

if (args.includes('--self-test')) {
  selfTest();
} else {
  try {
    const runId = Number(readArg('--run-id'));
    const pr = Number(readArg('--pr'));
    const expectedHead = readArg('--expected-head');
    const expectedBranch = readArg('--expected-branch');
    const jobName = readArg('--job-name');
    assert.ok(Number.isSafeInteger(runId) && runId > 0);
    assert.ok(Number.isSafeInteger(pr) && pr > 0);
    assert.match(expectedHead || '', shaRegex);
    assert.ok(expectedBranch && jobName, 'branch/job name required');
    const run = gh('actions/runs/' + runId);
    const jobs = gh('actions/runs/' + runId + '/jobs?per_page=100').jobs;
    const matching = jobs.filter(j => j.name === jobName);
    assert.equal(matching.length, 1, 'original packaging job name not unique');
    const job = matching[0];
    const log = gh('actions/jobs/' + job.id + '/logs', false);
    const checkoutSha = parseCheckout(log, pr);
    const merge = gh('git/commits/' + checkoutSha);
    const head = gh('git/commits/' + expectedHead);
    console.log(JSON.stringify({ check:'Immutable original packager checkout source tree',
      ...verify({run,job,checkoutSha,merge,head,expectedHead,expectedBranch,pr}) }, null, 2));
  } catch (error) {
    console.error('NOT SOURCE-CERTIFIED: ' + error.message);
    process.exitCode = 1;
  }
}
