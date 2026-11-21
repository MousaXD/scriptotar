#!/usr/bin/env node
/**
 * Agent11 containment-only test; NEVER weaken immutable original 19 and 12
 * case suites. Their original strict bool semantics REMAIN RED on timeout:
 * VM research candidate throws a typed error rather than returning false.
 *
 * Analyze the untouched suites in child processes with their original
 * per-case kill deadlines; require zero child ETIMEDOUT and explicit typed
 * bounded errors on any unresolved parser pattern. No release certification.
 */
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { existsSync,readFileSync } from 'node:fs';
import { resolve,join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { dirname } from 'node:path';
const root=resolve(dirname(fileURLToPath(import.meta.url)),'..','..','..');
const original=join(root,'apps','desktop-ui','scripts','probe-picomatch-redos.mjs');
const expanded=join(root,'tests','defragmentation','security','agent11_picomatch_extended_adversarial.mjs');
const preload=join(root,'tests','defragmentation','security','agent11_vm_regex_wrapper_preload.cjs');
assert.ok(existsSync(original)&&existsSync(expanded)&&existsSync(preload),
  'Requires immutable original security probe and test-only preload');
const immutable=readFileSync(original,'utf8');
assert.ok(immutable.includes("const baselineCases = [")&&immutable.includes("const extendedCases = ["),
  'Original pinned security corpus unexpected shape');
const env={...process.env,NODE_OPTIONS:'--require='+preload};
const trials=[
  {name:'immutable original 19-case --assert-patched --extended',
    argv:[original,'--assert-patched','--extended'],want:19,
    line:/:\s+(PASS|TIMEOUT(?: \(probable ReDoS\))?|ERROR\/REGRESSION)\b/g},
  {name:'independent expanded 12-case original adversarial',
    argv:[expanded],want:12,
    line:/^EXTENDED: (.+): (PASS|TIMEOUT|MISMATCH\/ERROR);/gm}
];
for(const test of trials) {
  const outcome=spawnSync(process.execPath,test.argv,{
    cwd:test.name.startsWith('immutable')?join(root,'apps','desktop-ui'):root,
    env,encoding:'utf8',timeout:65000,maxBuffer:3*1024*1024,
    windowsHide:true
  });
  assert.notEqual(outcome.error?.code,'ETIMEDOUT','Whole original corpus hung in a VM-protected child');
  const stdout=outcome.stdout||'';
  const stderr=outcome.stderr||'';
  const rows=test.name.startsWith('immutable')
    ? stdout.split(/\r?\n/).filter(l=>/:\s+(PASS|TIMEOUT|ERROR\/REGRESSION)(?:\s|$)/.test(l))
    : stdout.split(/\r?\n/).filter(l=>/^EXTENDED: .*: (PASS|TIMEOUT|MISMATCH\/ERROR);/.test(l));
  const timeouts=rows.filter(l=>l.includes('TIMEOUT'));
  const errors=rows.filter(l=>l.includes('ERROR/REGRESSION')||l.includes('MISMATCH/ERROR'));
  const passes=rows.filter(l=>l.includes(': PASS'));
  const typed=(stderr.match(/ERR_AGENT11_PICOMATCH_DEADLINE/g)||[]).length;
  console.log('IMMUTABLE ORIGINAL CORPUS WRAPPER CONTAINMENT:',JSON.stringify({
    name:test.name,unchangedCases:test.want,observed:rows.length,
    passingOriginalBooleanCases:passes.length,typedFailClosedCases:errors.length,
    originalChildHardTimeouts:timeouts.length,typedErrorMentions:typed,
    originalExitCode:outcome.status,releaseApproval:false,
    sampleOriginalUnchangedStatus:rows.slice(0,4),
    caveat:'Original boolean-security suite is intentionally RED; typed exception is fail-closed mitigation only'
  }));
  assert.equal(rows.length,test.want,'Expected exact unchanged original security corpus count');
  assert.equal(timeouts.length,0,'VM execution did not bound all original hostile cases');
  assert.ok(errors.length>0,'No attempted expensive parser case produced required explicit failure');
  assert.ok(typed>=errors.length,'Missing typed fail-closed deadline for original errored security cases');
  assert.equal(outcome.status,1,
    'Original strict parser-safety suite MUST remain RED, cannot reclassify timeout exception as boolean PASS');
}
console.log('AGENT11 VM PRELOAD CONTAINMENT EVIDENCE: original 19 and 12 end within deadlines but original strict tests remain RED; no production fix');
