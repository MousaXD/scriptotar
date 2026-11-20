#!/usr/bin/env node
/**
 * Agent11 independent actual Vite compiler reachability / VM-hook falsifier.
 * Replays a single Agent10-style escaped import.meta.glob source route in an
 * isolated GH runner, after the trusted prebuild guard. Never edits source in
 * a committed tree: restores src/main.ts and deletes test fixtures finally.
 *
 * NOT a production mitigation / DO NOT bypass normal source guard in release.
 * Actual test 1: original unmodified Vite build should demonstrably timeout.
 * Actual test 2: same hostile Vite build with research-only VM-preloader must
 * error promptly with explicit timeout code, never silently succeed.
 */
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { existsSync,mkdirSync,readFileSync,rmSync,writeFileSync } from 'node:fs';
import { resolve,dirname,join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { performance } from 'node:perf_hooks';

const repo=resolve(dirname(fileURLToPath(import.meta.url)),'..','..','..');
const ui=join(repo,'apps','desktop-ui');
const main=join(ui,'src','main.ts');
const rawMain=readFileSync(main,'utf8');
const viteBin=join(ui,'node_modules','vite','bin','vite.js');
const mode=process.env.AGENT11_VM_PRELOAD_VARIANT||'exec';
assert.ok(['exec','compiler-exec'].includes(mode),'Only source-reviewed research preload variants allowed');
const preload=join(repo,'tests','defragmentation','security',
  mode==='compiler-exec'?'agent11_vm_compile_guard_preload.cjs':'agent11_vm_regex_wrapper_preload.cjs');
const fixture=join(ui,'agent11-vm-hostile-probe');
assert.ok(existsSync(viteBin)&&existsSync(preload),'Original locked npm ci and tracked VM test wrapper required');
assert.ok(!existsSync(fixture),'Refuse existing/untrusted test fixture directory');
const escapedPath='.'+'\\u002e/agent11-vm-hostile-probe/';
function build({vm=false,timeoutMs=9000}={}){
  const env={...process.env,CI:'true'};
  if(vm)env.NODE_OPTIONS='--require='+preload;
  else delete env.NODE_OPTIONS;
  const t=performance.now();
  const p=spawnSync(process.execPath,[viteBin,'build'],{
    cwd:ui,env,encoding:'utf8',timeout:timeoutMs,maxBuffer:2*1024*1024,windowsHide:true
  });
  return {status:p.status,code:p.error?.code??null,
    ms:Math.round(performance.now()-t),stdout:p.stdout??'',stderr:p.stderr??''};
}
function report(kind,r) {
  const output=r.stdout+'\n'+r.stderr;
  console.log('SOURCE-EXACT AGENT11 ACTUAL VITE:',JSON.stringify({
    kind,status:r.status,error_code:r.code,elapsedMs:r.ms,
    deadlineException:output.includes('ERR_AGENT11_PICOMATCH_DEADLINE'),
    loaderEvidence:output.includes('AGENT11_TEST_ONLY_VM_REGEX_PRELOAD_STATS')||output.includes('AGENT11_TEST_ONLY_VM_COMPILER_MATCH_PRELOAD'),
    logTail:output.slice(-1500),releaseApproval:false
  }));
}
try {
  mkdirSync(fixture);
  writeFileSync(join(fixture,'benign.ts'),'export const expectedSafe=1;\n');
  writeFileSync(join(fixture,'hostile.ts'),
    "export const modules = import.meta.glob('./+(*a)');\n");
  writeFileSync(join(fixture,'a'.repeat(36)+'b'),'near miss only\n');

  // Compiler-side positive control: escaped relative import resolves and
  // compiles when target has no malicious glob.
  writeFileSync(main,rawMain+"\nimport '"+escapedPath+"benign.ts';\n");
  const good=build({vm:true,timeoutMs:45000});
  report('benign escaped import using VM preload',good);
  assert.equal(good.status,0,'Benign encoded import should compile with VM preloaded: '+good.stderr.slice(-900));

  writeFileSync(main,rawMain+"\nimport '"+escapedPath+"hostile.ts';\n");
  const original=build({vm:false,timeoutMs:9000});
  report('unmodified actual Vite hostile escaped source',original);
  assert.equal(original.code,'ETIMEDOUT',
    'Hostile route did not independently reproduce original Vite compiler hang; do not claim VM mitigation');

  const guarded=build({vm:true,timeoutMs:9000});
  report('VM-preloaded actual Vite hostile escaped source',guarded);
  const output=guarded.stdout+'\n'+guarded.stderr;
  assert.notEqual(guarded.status,0,'Must not successfully build while silently omitting malicious source');
  assert.notEqual(guarded.code,'ETIMEDOUT',
    'VM preload did NOT interrupt actual Vite compiler security route (still full timeout)');
  assert.ok(output.includes('ERR_AGENT11_PICOMATCH_DEADLINE'),
    'Vite failed without explicit VM deadline exception; cannot claim protection');
  assert.ok(output.includes('AGENT11_TEST_ONLY_VM_REGEX_PRELOAD_STATS'),
    'No evidence VM preloader was active in the hostile Vite process');
  console.log('VM-PRELOADED ACTUAL VITE HOSTILE ROUTE: fail-closed prompt error, not drop-in parser repair');
} finally {
  writeFileSync(main,rawMain);
  rmSync(fixture,{recursive:true,force:true});
}
