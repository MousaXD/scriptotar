#!/usr/bin/env node
/**
 * Agent 11 research: is Node's vm.Script synchronous timeout effective for
 * picomatch-generated RegExp with lookaheads? NOT a drop-in parser fix.
 * Actual picomatch@4.0.7 remains unmodified. No adversarial RegExp is run
 * natively; all hostile inputs pass only into a VM time-limited call.
 *
 * Does NOT bound regex compilation, arbitrary source callbacks, makeRe
 * returns, all public APIs, worker memory or native packaged software.
 */
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import vm from 'node:vm';
import { performance } from 'node:perf_hooks';
import { resolve } from 'node:path';

const r = createRequire(resolve('apps/desktop-ui/package.json'));
const pm = r('picomatch');
const manifest = r('picomatch/package.json');
assert.equal(manifest.version,'4.0.7','Only review original published locked parser');
const script = new vm.Script('out = RegExp.prototype.exec.call(regex, input)',{
  filename:'agent11-picomatch-vm-exec'
});

function resultShape(m) {
  if(m===null) return null;
  assert.ok(Array.isArray(m),'Original RegExp.exec must yield match array');
  return {
    value:m[0],captures:Array.from(m).slice(1),
    index:m.index,input:m.input,
    groups:m.groups===undefined?null:{...m.groups}
  };
}
function guarded(re,input,timeout=75){
  assert.ok(re instanceof RegExp,'Must use original native RegExp');
  assert.ok(typeof input==='string'&&input.length<=6000,'Bound test input size');
  const ctx=vm.createContext({regex:re,input,out:null});
  const start=performance.now();
  try {
    script.runInContext(ctx,{timeout,breakOnSigint:false});
    return {status:'complete',result:resultShape(ctx.out),ms:Math.ceil(performance.now()-start)};
  } catch(e) {
    if(e?.code==='ERR_SCRIPT_EXECUTION_TIMEOUT'){
      return {status:'timeout',ms:Math.ceil(performance.now()-start)};
    }
    throw e;
  } finally {
    ctx.regex=null;
    ctx.input=null;
    ctx.out=null;
  }
}

const good=[
  {pat:'+(*)c',str:'abc'},
  {pat:'+(*)c',str:'ac',options:{capture:true}},
  {pat:'+(a|b)',str:'abab'},
  {pat:'+(a*)',str:'aa',options:{capture:true}},
  {pat:'*(a*|b)',str:'ax'},
  {pat:'*(!(foo))',str:'bar',options:{bash:true,windows:true}},
  {pat:'*(!(foo))',str:'foo',options:{bash:true,windows:true}},
  {pat:'+(+(a))',str:'aaa',options:{maxExtglobRecursion:1}},
  {pat:'*(+(a))',str:'aaa',options:{maxExtglobRecursion:1}},
  {pat:'abc/*.js',str:'abc/test.js'},
  {pat:'**/*.js',str:'src/index.js'},
  {pat:'+(a*b)',str:'abab'},
  {pat:'[AB]*',str:'Bb',options:{nocase:true}}
];
let goodCount=0;
const startGood=performance.now();
for(const tc of good){
  const re=pm.makeRe(tc.pat,tc.options||{});
  const expected=resultShape(RegExp.prototype.exec.call(re,tc.str));
  const actual=guarded(re,tc.str,250);
  if(actual.status!=='complete'){
    console.log('VM POSITIVE DEADLINE EXCEEDED',JSON.stringify({pat:tc.pat,str:tc.str,timeout:250}));
    process.exitCode=1;
    continue;
  }
  assert.deepEqual(actual.result,expected,'Original captures/indices mutated for '+tc.pat);
  goodCount++;
}
const goodMs=Math.round(performance.now()-startGood);
console.log('SOURCE-EXACT VM NORMAL SEMANTICS:',JSON.stringify({passed:goodCount,total:good.length,ms:goodMs,source:'original locked picomatch 4.0.7'}));
assert.equal(goodCount,good.length,'Normal native RegExp versus VM match/capture mismatch');

const hostile=[
  {pat:'+([ab]|b)',str:'b'.repeat(36)+'/'},
  {pat:'+(a*)',str:'a'.repeat(36)+'/'},
  {pat:'+(@(+([ab])))',str:'a'.repeat(36)+'/'},
  {pat:'+(a|b|ab)',str:'ab'.repeat(22)+'/'},
  {pat:'+(@(*a))',str:'a'.repeat(36)+'/'},
  {pat:'+(a|aa)',str:'a'.repeat(36)+'/'}
];
const cutoff=80;
let deadlineCount=0;
const startHostile=performance.now();
for(const tc of hostile){
  const re=pm.makeRe(tc.pat);
  assert.equal(typeof re.source,'string');
  assert.ok(re.source.includes('(?=')||re.source.includes('(?!'),
    'Expected original source with RegExp lookaround');
  const actual=guarded(re,tc.str,cutoff);
  if(actual.status==='timeout') deadlineCount++;
  else assert.equal(actual.result,null,'Hostile trailing-slash must not match');
  console.log('SOURCE-EXACT VM HOSTILE REGEXP:',JSON.stringify({
    pattern:tc.pat,regexLookaround:true,deadline_ms:cutoff,
    status:actual.status,actual_ms:actual.ms,
    noNativeHostileExecution:true
  }));
}
const hostileMs=Math.round(performance.now()-startHostile);
console.log('SOURCE-EXACT VM BOUNDED HOSTILE SUMMARY:',JSON.stringify({
  passed:hostile.length,actuallyInterrupted:deadlineCount,deadlineMs:cutoff,totalElapsedMs:hostileMs,
  parserRepair:'NO',safeDropIn:'NO',
  caveats:'No makeRe/compiler/other APIs bounded; isolated VM exec only; CI research'
}));
assert.ok(deadlineCount>=2,'Failed to demonstrate VM termination on genuinely vulnerable picomatch regex');
assert.ok(hostileMs<5500,'Total experiment wall-clock unexpectedly long');
