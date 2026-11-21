'use strict';
/*
 * TEST-ONLY preload for running original Vite/Vitest tooling with an
 * experimental VM-bound regex helper. Do not use for production/release.
 * This module is loaded only AFTER original npm ci by an explicit test
 * workflow with NODE_OPTIONS=--require=...; original picomatch stays locked.
 */
const { install } = require('./agent11_vm_regex_wrapper_candidate.cjs');
const handle = install({ timeoutMs:80 });
const counts = {calls:0,timeouts:0,errors:0};
const original = handle.GuardedRegExp.prototype.exec;
handle.GuardedRegExp.prototype.exec = function boundedInstrumented(input){
  counts.calls++;
  try {
    return original.call(this,input);
  } catch(e) {
    if(e && e.code==='ERR_AGENT11_PICOMATCH_DEADLINE') counts.timeouts++;
    else counts.errors++;
    throw e;
  }
};
process.on('exit',() => {
  // Counts are process-scoped and MUST NOT be treated as proof that ALL
  // native regex engines/import paths or preloaded dependencies are covered.
  console.error('AGENT11_TEST_ONLY_VM_REGEX_PRELOAD_STATS '+JSON.stringify({
    pid:process.pid,...counts,hostRun:'original pinned npm lock',
    releaseApproval:false,parserFullyProtected:false
  }));
});
