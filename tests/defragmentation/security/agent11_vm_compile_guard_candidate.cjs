'use strict';
/*
 * Agent11 RESEARCH-ONLY, NOT an approved parser replacement.
 * Experimental Node vm.Script synchronous deadline for the ORIGINAL
 * picomatch public compilation/scan methods, composed with the separately
 * verified VM RegExp.exec subclass.
 *
 * The original npm package source/lock is unchanged. This only hooks the
 * CJS module instance resolved through the reviewed UI package path;
 * direct module imports or alternative bundled copies may still bypass.
 *
 * There is NO memory/heap cap or general compilation proof. The 19/12
 * immutable security boolean probes remain RED on typed runtime errors.
 */
const vm=require('node:vm');
const {createRequire}=require('node:module');
const {resolve}=require('node:path');
const {install:installExec}=require('./agent11_vm_regex_wrapper_candidate.cjs');

const script=new vm.Script('out=source.apply(null,params)',{
  filename:'agent11-picomatch-vm-compile-deadline.cjs'
});
function runBounded(source,params,timeoutMs,name) {
  const ctx=vm.createContext({source,params,out:null});
  try {
    script.runInContext(ctx,{timeout:timeoutMs});
    return ctx.out;
  } catch(e) {
    if(e&&e.code==='ERR_SCRIPT_EXECUTION_TIMEOUT') {
      const err=new Error('Agent11 research: picomatch '+name+' compilation exceeded '+timeoutMs+' ms');
      err.code='ERR_AGENT11_PICOMATCH_COMPILE_DEADLINE';
      throw err;
    }
    throw e;
  } finally {
    ctx.source=null;
    ctx.params=null;
    ctx.out=null;
  }
}
function install(options={}) {
  const compileTimeoutMs=options.compileTimeoutMs??150;
  if(!Number.isSafeInteger(compileTimeoutMs)||compileTimeoutMs<20||compileTimeoutMs>1500) {
    throw new TypeError('Experimental compile budget must be integer 20-1500ms');
  }
  const handle=installExec({timeoutMs:options.execTimeoutMs??80});
  const r=createRequire(resolve(__dirname,'../../../apps/desktop-ui/package.json'));
  const inner=r('picomatch/lib/picomatch');
  const facade=r('picomatch');
  const keys=['toRegex','compileRe','makeRe','parse','scan'];
  const originals={};
  const stats=Object.fromEntries(keys.map(k=>[k,{calls:0,timeouts:0}]));
  for(const key of keys) {
    const fn=inner[key];
    if(typeof fn!=='function') {
      handle.uninstall();
      throw new Error('Original picomatch method not present: '+key);
    }
    originals[key]=fn;
    const bounded=function(...params) {
      stats[key].calls++;
      try {
        return runBounded(fn,params,compileTimeoutMs,key);
      } catch(e) {
        if(e&&e.code==='ERR_AGENT11_PICOMATCH_COMPILE_DEADLINE')stats[key].timeouts++;
        throw e;
      }
    };
    inner[key]=bounded;
    facade[key]=bounded;
  }
  return {
    // Expose only to test-only preload instrumentation, never to production.
    GuardedRegExp:handle.GuardedRegExp,
    stats,
    compileTimeoutMs,
    execTimeoutMs:handle.timeoutMs,
    uninstall() {
      for(const key of keys) {
        inner[key]=originals[key];
        facade[key]=originals[key];
      }
      handle.uninstall();
    }
  };
}
module.exports={install,runBounded};
