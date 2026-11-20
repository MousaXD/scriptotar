#!/usr/bin/env node
/**
 * Agent11 independent VM-guard BYPASS NEGATIVE test: no production changes.
 * Expected: direct native RegExp.prototype.exec.call and public picomatch.test
 * with user-supplied native RegExp bypass the experimental override.
 * Native dangerous regex is ALWAYS confined to a hard-kill subprocess.
 * GREEN workflow confirms UNFIXED BYPASS, NOT security certification.
 */
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { resolve } from 'node:path';

const repo=resolve('.');
const pattern='+([ab]|b)';
const input='b'.repeat(36)+'/';
const maxMs=1250;
const child="\nconst { install } = require('./tests/defragmentation/security/agent11_vm_compile_guard_candidate.cjs');\nconst handle = install({execTimeoutMs:80,compileTimeoutMs:150});\nconst pm=require('./apps/desktop-ui/node_modules/picomatch');\nconst mode=process.argv[1],pattern=process.argv[2],input=process.argv[3];\nconst guarded=pm.makeRe(pattern);\nif (!(guarded instanceof handle.GuardedRegExp)) throw new Error('Experimental wrapper not active');\nconst run=()=>{\n  switch(mode) {\n    case 'guarded-exec': return guarded.exec(input);\n    case 'guarded-public-test': return pm.test(input,guarded);\n    case 'native-prototype-bypass': return RegExp.prototype.exec.call(guarded,input);\n    case 'public-native-regexp-bypass':\n      return pm.test(input,new RegExp(guarded.source,guarded.flags));\n    default: throw new Error('unexpected mode');\n  }\n};\ntry {\n  const result=run();\n  console.log('CHILD COMPLETED',JSON.stringify({mode,matches:result!==null}));\n} catch(e) {\n  if (e?.code==='ERR_AGENT11_PICOMATCH_DEADLINE') {\n    console.log('CHILD CORRECTLY FAIL-CLOSED',JSON.stringify({mode,code:e.code}));\n  } else throw e;\n}\n";
const guardedModes=['guarded-exec','guarded-public-test'];
const bypassModes=['native-prototype-bypass','public-native-regexp-bypass'];
for(const mode of [...guardedModes,...bypassModes]){
  const proc=spawnSync(process.execPath,['-e',child,mode,pattern,input],{
    cwd:repo,encoding:'utf8',timeout:maxMs,maxBuffer:512*1024,windowsHide:true
  });
  const actual={
    mode,exitCode:proc.status,errorCode:proc.error?.code??null,
    stdout:(proc.stdout??'').slice(-1000),stderr:(proc.stderr??'').slice(-1000)
  };
  if(guardedModes.includes(mode)){
    assert.equal(proc.status,0,'Controlled wrapper mode failed: '+JSON.stringify(actual));
    assert.ok(actual.stdout.includes('CHILD CORRECTLY FAIL-CLOSED'),
      'Guarded route did not raise typed deadline: '+JSON.stringify(actual));
    assert.equal(proc.error,undefined,'Guarded route hit OS subprocess deadline');
  } else {
    assert.equal(proc.error?.code,'ETIMEDOUT',
      'Native bypass did not actually hang; inspect before claiming bypass: '+JSON.stringify(actual));
    assert.equal(proc.status,null,'Native bypass surprisingly completed');
  }
  console.log('AGENT11 VM DIRECT-NATIVE BYPASS:',JSON.stringify({
    mode,classification:guardedModes.includes(mode)?
      'guarded_typed_deadline':'native_regex_bypassed_vm_and_hit_process_timeout',
    processDeadlineMs:maxMs,noUnboundedNativeRegexInParent:true,releaseApproval:false
  }));
}
console.log('AGENT11 VM BYPASS HARD VERDICT:',JSON.stringify({
  guardedRoutes:guardedModes.length,unprotectedRoutes:bypassModes.length,
  expectedNegativeControlsPass:true,
  securityAssessment:'NO-GO: native prototype / public native regex bypass remains',
  parserRemediated:false,releaseApproved:false
}));
