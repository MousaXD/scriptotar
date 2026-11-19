#!/usr/bin/env node
/**
 * Agent10 independent, read-only release authorization review.
 * Does NOT invoke GitHub release APIs, create artifacts, change refs, or publish.
 * Preview release 1.4.1 is distinct from the blocked defragmentation P0 release.
 * Fail closed on an untrusted commit message being accepted in place of a
 * repository-owner/governance approval. Its exact-SHA job gates still apply.
 */
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';

const publisher = readFileSync('.github/workflows/tauri-next-release.yml','utf8');
const gate = readFileSync('scripts/release_ci_gate.py','utf8');
const assertOwnerOnly=process.argv.includes('--assert-owner-only');
if(process.argv.length!==2 && !(process.argv.length===3 && assertOwnerOnly)){
  console.error('Usage: node agent10_preview_release_approval_probe.mjs [--assert-owner-only]');
  process.exit(2);
}
const block=publisher.match(/(?:^|\n)  publish-preview:\n([\s\S]*?)(?=\n  [a-z][\w-]+:\n|$)/);
assert.ok(block, 'Preview publisher job not found; rereview required');
const expression=block[1].match(/^\s+if:\s*(.+)$/m)?.[1];
assert.ok(expression,'Publisher approval condition not found');
assert.ok(expression.includes("github.event_name == 'workflow_run'"),'Event condition changed');
assert.ok(expression.includes("github.event.workflow_run.head_branch == 'main'"),'Branch restriction changed');
const variableGate=expression.includes("vars.SCRIPTOTAR_NEXT_DISTRIBUTION_APPROVED == 'true'") &&
  expression.includes("vars.SCRIPTOTAR_NEXT_1_4_1_DISTRIBUTION_APPROVED == 'true'");
assert.ok(variableGate,'Explicit repository variables changed; do not silently certify');
const messageMatch=expression.match(/github\.event\.workflow_run\.head_commit\.message\s*==\s*'([^']+)'/);
const messageOnlyOverride=!!messageMatch && expression.includes('|| github.event.workflow_run.head_commit.message');
const preview=messageMatch?.[1]??'';
const simulate=({headBranch='main',event='workflow_run',approved=false,message=''})=>
  event==='workflow_run' && headBranch==='main' && (approved || (messageOnlyOverride && message===preview));
const noFlagsNoMessage=simulate({approved:false,message:'ordinary-maintenance-change'});
const noFlagsForgeMessage=simulate({approved:false,message:preview});
const bothFlags=simulate({approved:true,message:'ordinary-maintenance-change'});
const wrongBranch=simulate({headBranch:'feature',approved:true,message:preview});
assert.equal(noFlagsNoMessage,false);
assert.equal(bothFlags,true);
assert.equal(wrongBranch,false);
assert.ok(publisher.includes('contents: write'),'Publisher privileges changed; rereview');
assert.ok(publisher.includes('run: python3 scripts/release_ci_gate.py'),'Mandatory exact-SHA gates changed; rereview');
const gatedWorkflows=[...gate.matchAll(/workflow_file="([^"]+)"/g)].map(m=>m[1]);
assert.deepEqual(gatedWorkflows,[
 'integration.yml','security-hygiene.yml','windows-tauri.yml','linux-tauri.yml','mcp-e2e.yml'
],'Preview release gate set changed; rereview');
console.log(JSON.stringify({
  source:'.github/workflows/tauri-next-release.yml',
  sourceProvenance:'git branch HEAD; main/PR149 byte-identical publisher blob 3127500db3c4694a551df4fb660514e9871f2392',
  risk:'GITHUB_RELEASE_OWNER_APPROVAL_CONTROLLED_BY_MAIN_COMMIT_MESSAGE',
  proof:{event:'workflow_run',branch:'main',repositoryApprovals:'both FALSE',
    commitMessageMatchesLegacyOverride:!!preview,jobConditionAllowsPublish:noFlagsForgeMessage,
    noApprovalNoSpecialMessageDenied:!noFlagsNoMessage,nonMainDenied:!wrongBranch},
  mandatoryExactShaJobGatesStillApply:true,
  privilegedPublisherContentsWrite:true,
  previewGateWorkflows:gatedWorkflows,
  nativeInstalledP0GateIncluded:false,
  disclaimer:'Static job-if policy proof only; does not run publisher, prove package gate bypass, or authorize release'
},null,2));
if(assertOwnerOnly && noFlagsForgeMessage){
 console.error('RELEASE APPROVAL POLICY RED: exact main commit message substitutes for explicit owner-governed approval');
 process.exitCode=1;
}else if(assertOwnerOnly) console.log('PASS: no commit-message-only preview release approval branch');
