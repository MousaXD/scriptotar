#!/usr/bin/env node
/**
 * Agent11 Node-only VM-backed RegExp wrapper prequalification.
 * A fail-closed demonstration, NOT a production replacement or a claim that
 * the immutable 19/12 boolean security gates pass (timeout THROWS on purpose).
 *
 * Does not run hostile RegExp natively. The original picomatch@4.0.7 source
 * and npm package integrity remain unchanged in the checked-in repository.
 */
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { performance } from 'node:perf_hooks';
import { resolve } from 'node:path';

const r=createRequire(resolve('apps/desktop-ui/package.json'));
const pm=r('picomatch');
const internal=r('picomatch/lib/picomatch');
const {install}=r(resolve('tests/defragmentation/security/agent11_vm_regex_wrapper_candidate.cjs'));
assert.equal(r('picomatch/package.json').version,'4.0.7');
const originalToRegex=internal.toRegex;
const cases=[
 {pat:'+(*)c',input:'abc'},
 {pat:'+(*)c',input:'ac',opts:{capture:true}},
 {pat:'+(a|b)',input:'abab'},
 {pat:'+(a*)',input:'aa',opts:{capture:true}},
 {pat:'*(a*|b)',input:'ax'},
 {pat:'*(!(foo))',input:'bar',opts:{bash:true,windows:true}},
 {pat:'*(!(foo))',input:'foo',opts:{bash:true,windows:true}},
 {pat:'+(+(a))',input:'aaa',opts:{maxExtglobRecursion:1}},
 {pat:'*(+(a))',input:'aaa',opts:{maxExtglobRecursion:1}},
 {pat:'abc/*.js',input:'abc/test.js'},
 {pat:'**/*.js',input:'src/index.js'},
 {pat:'+(a*b)',input:'abab'},
 {pat:'[AB]*',input:'Bb',opts:{nocase:true}},
 {pat:'x+(*a)y',input:'xabaay'},
 {pat:'*(a*b)c',input:'abababc',opts:{capture:true}},
 {pat:'+(*)',input:'foo',opts:{capture:true}},
 {pat:'*.md',input:'src/test.md'},
 {pat:'a?b',input:'acb'},
 {pat:'ab',input:'ab'},
 {pat:'ab',input:'no'}
];
function snapshot(tc){
 const opts=tc.opts||{};
 const regex=pm.makeRe(tc.pat,opts);
 const matcher=pm(tc.pat,opts);
 const result=matcher(tc.input,true);
 const match=result.match;
 return {
   source:regex.source,flags:regex.flags,
   reInstance:regex instanceof RegExp,
   rawExec:(()=>{
      const m=regex.exec(tc.input);
      return m===null?null:{capture:Array.from(m),index:m.index,input:m.input,groups:m.groups??null};
   })(),
   boolean:result.isMatch,
   match:match===true||match===false||match==null?match:{
      capture:Array.from(match),index:match.index,input:match.input,groups:match.groups??null
   },
   output:result.output
 };
}
const baseline=cases.map(snapshot);
const handle=install({timeoutMs:80});
try {
  assert.equal(typeof handle.GuardedRegExp,'function');
  assert.ok(internal.toRegex!==originalToRegex,'Hook must affect library lexical toRegex');
  const guarded=cases.map(snapshot);
  for(let i=0;i<cases.length;i++)assert.deepEqual(guarded[i],baseline[i],
      'Original 4.0.7 picomatch public matcher/capture/source behavior changed: '+cases[i].pat);
  console.log('BOUNDED VM REGEXP WRAPPER NORMAL PARITY:',JSON.stringify({passing:cases.length,total:cases.length,method:'original matcher/makeRe + public regex.exec',production:false}));

  const source=pm.makeRe('+(a*)',{capture:true});
  assert.ok(source instanceof RegExp,'Guarded source must remain native RegExp subclass');
  assert.ok(source instanceof handle.GuardedRegExp,'Factory must return timeout-backed wrapper');
  const originalNative=RegExp.prototype.exec.call(new RegExp(source.source,source.flags),'aa');
  const wrapped=source.exec('aa');
  assert.ok(Array.isArray(wrapped),'Must return own-realm Array, not plain object');
  assert.ok(wrapped instanceof Array,'Must preserve JS main-realm result Array');
  assert.deepEqual(Array.from(wrapped),Array.from(originalNative));
  assert.equal(wrapped.index,originalNative.index);
  assert.equal(wrapped.input,originalNative.input);
  assert.equal(source.test('aa'),true,'Inherited native .test should dispatch guarded .exec');
  assert.equal(pm.matchBase('my/foo.js','*.js'),true,'matchBase must pass through bounded RegExp');

  const failing=[
    {pat:'+([ab]|b)',input:'b'.repeat(36)+'/'},
    {pat:'+(a*)',input:'a'.repeat(36)+'/'},
    {pat:'+(@(+([ab])))',input:'a'.repeat(36)+'/'},
    {pat:'+(a|b|ab)',input:'ab'.repeat(22)+'/'}
  ];
  let interrupted=0;
  for(const tc of failing) {
    const matcher=pm(tc.pat,{capture:true});
    const now=performance.now();
    try {
      const answer=matcher(tc.input);
      assert.equal(answer,false,'Original malicious nonmatch should not return true');
      console.log('BOUNDED VM REGEXP CANDIDATE:',JSON.stringify({pattern:tc.pat,status:'completed',elapsedMs:Math.round(performance.now()-now)}));
    } catch(err) {
      if(err?.code!=='ERR_AGENT11_PICOMATCH_DEADLINE')throw err;
      interrupted++;
      console.log('BOUNDED VM REGEXP CANDIDATE:',JSON.stringify({
        pattern:tc.pat,status:'fail_closed_timeout_error',elapsedMs:Math.round(performance.now()-now),
        code:err.code,notSilentFalse:true
      }));
    }
  }
  assert.ok(interrupted>=2,'Expected genuine vulnerable code interruption, not superficial wrapping');
  console.log('BOUNDED VM REGEXP CANDIDATE HARNESS SUMMARY:',JSON.stringify({
     ordinary:cases.length, hostile:failing.length,interrupted,
     overrideIsComplete:false,securityCertification:false,releaseGo:false,
     missing:'Compilation, direct supplied RegExp, prototype bypass, capture breadth, source callbacks, other engines, CI Vite throughput, package authenticity'
  }));
  const benchRe=pm.makeRe('src/**/*.js');
  const baselineRe=new RegExp(benchRe.source,benchRe.flags);
  const inputs=['src/a.js','src/deep/test.js','dist/output.html'];
  let base=0,guard=0;
  let start=performance.now();
  for(let i=0;i<750;i++)baselineRe.exec(inputs[i%inputs.length]);
  base=performance.now()-start;
  start=performance.now();
  for(let i=0;i<750;i++)benchRe.exec(inputs[i%inputs.length]);
  guard=performance.now()-start;
  console.log('BOUNDED VM WRAPPER THROUGHPUT SAMPLE:',JSON.stringify({
    matches:750,nativeElapsedMs:Math.round(base*10)/10,
    guardedElapsedMs:Math.round(guard*10)/10,
    ratioNativeToGuarded:base>0?Math.round(guard/base):null,
    provisionalOnly:true
  }));
} finally {
  handle.uninstall();
}
assert.equal(internal.toRegex,originalToRegex,'Original library must be restored after experiment');
