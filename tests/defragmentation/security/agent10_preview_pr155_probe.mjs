#!/usr/bin/env node
/**
 * Agent10 isolated source-exact preview owner approval revalidation.
 * No secret/env reads, release API, privileged tokens, tags or publication.
 * Candidate PR155 is separate from main and never merged by this test.
 */
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
const oldRoot='.agent10-preview-main';
const candidateRoot='.agent10-preview-pr155';
const workflowPath='.github/workflows/tauri-next-release.yml';
const gatePath='scripts/release_ci_gate.py';
const original=readFileSync(join(oldRoot,workflowPath),'utf8');
const candidate=readFileSync(join(candidateRoot,workflowPath),'utf8');
const oldGate=readFileSync(join(oldRoot,gatePath));
const newGate=readFileSync(join(candidateRoot,gatePath));
const sha=bytes=>createHash('sha256').update(bytes).digest('hex');
assert.equal(sha(oldGate),sha(newGate),'Package exact-SHA release gate changed');
assert.match(original,/head_commit\.message == 'owner-approve-scriptotar-next-1\.4\.1-hotfix-publish-20261112'/);
assert.match(original,/\)\s*\|\|\s*github\.event\.workflow_run\.head_commit\.message/);
const block=source=>source.match(/(?:^|\n)  publish-preview:\n([\s\S]*?)(?=\n  [a-z][\w-]+:\n|$)/)?.[1];
assert.ok(block(original)&&block(candidate),'Publisher job disappeared/renamed');
const expression=block(candidate).match(/^\s+if:\s*(.+)$/m)?.[1];
const expected="github.event_name == 'workflow_run' && "+
  "github.event.workflow_run.head_branch == 'main' && "+
  "vars.SCRIPTOTAR_NEXT_DISTRIBUTION_APPROVED == 'true' && "+
  "vars.SCRIPTOTAR_NEXT_1_4_1_DISTRIBUTION_APPROVED == 'true'";
assert.equal(expression,expected,'Only exact AND condition is accepted');
assert.doesNotMatch(block(candidate),/head_commit\.message|\|\|/);
assert.match(block(candidate),/contents:\s*write/);
assert.match(block(candidate),/ref:\s*\$\{\{ github\.event\.workflow_run\.head_sha \}\}/);
assert.match(block(candidate),/python3 scripts\/release_ci_gate\.py/);
assert.match(block(candidate),/gh run download/);
assert.match(candidate,/workflow_run:\s*\n\s+workflows:\s*\n\s+- Windows Tauri Installer/);
assert.match(candidate,/types:\s*\[completed\]/);
assert.match(candidate,/branches:\s*\[main\]/);
assert.match(candidate,/pull_request:/);
// Ensure only comment and job-level condition changed from the original source.
const strip=source=>source.split('\n').filter(line=>!line.includes('One-time repository owner approval for the 1.4.1 preview') &&
  !line.includes('Publishing requires owner-managed repository approvals') &&
  !line.trimStart().startsWith("if: github.event_name == 'workflow_run'")).join('\n');
assert.equal(strip(candidate),strip(original),'Owner changed more than authorization comment and if clause');
let count=0;
for(const event of ['workflow_run','pull_request','workflow_dispatch']) {
  for(const branch of ['main','agent-10-test']) {
    for(const globalApproval of [false,true]){
      for(const versionApproval of [false,true]){
        for(const message of ['owner-approve-scriptotar-next-1.4.1-hotfix-publish-20261112','ordinary commit']) {
          const authorized=event==='workflow_run'&&branch==='main'&&globalApproval&&versionApproval;
          assert.equal(authorized,event==='workflow_run'&&branch==='main'&&globalApproval===true&&versionApproval===true);
          if(event==='workflow_run'&&branch==='main'&&!globalApproval&&!versionApproval)
            assert.equal(authorized,false,'Commit message improperly bypassed owner approvals');
          count++;
        }
      }
    }
  }
}
console.log(JSON.stringify({
  source:{old_main:"69e60f91c0b00560332dac06b732fc2259c1d632",
    owner_pr155:"1f9e661f3c72c28a5695d1c7dfa124280f5b1494"},
  checks:count,verdict:'PASS: source-only policy approval correction',
  mandatoryGateSha256:sha(oldGate),noPublish:true,
  limitations:'Does not verify GitHub admin vars, branch protection or live publishing',
},null,2));
