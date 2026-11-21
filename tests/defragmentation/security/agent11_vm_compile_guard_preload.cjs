'use strict';
/*
 * Agent11 test-only Node preload: per-call compilation and execution VM
 * deadlines for original locked picomatch 4.0.7. NOT a safe parser release.
 * This module must NEVER be included in production app/release by accident.
 */
const {install}=require('./agent11_vm_compile_guard_candidate.cjs');
const h=install({compileTimeoutMs:160,execTimeoutMs:80});
const counts={matchCalls:0,execDeadlineErrors:0,otherExecErrors:0};
const originalExec=h.GuardedRegExp.prototype.exec;
h.GuardedRegExp.prototype.exec=function boundedVmTracked(input){
  counts.matchCalls++;
  try{return originalExec.call(this,input);}
  catch(e) {
    if(e?.code==='ERR_AGENT11_PICOMATCH_DEADLINE')counts.execDeadlineErrors++;
    else counts.otherExecErrors++;
    throw e;
  }
};
process.on('exit',()=>{
  console.error('AGENT11_TEST_ONLY_VM_COMPILER_MATCH_PRELOAD '+JSON.stringify({
    pid:process.pid,...counts,
    originalPicomatchCompilation:h.stats,releaseApproval:false,
    parserGloballyFixed:false,originalStrictSecurityCorpus:'RED'
  }));
});
