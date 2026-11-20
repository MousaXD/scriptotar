#!/usr/bin/env node
/**
 * Experimental Node vm deadline for original picomatch compilation/scan.
 * Tests EXTERNAL-SOURCE JavaScript timeout in child with OS deadline, then
 * verifies finite ORIGINAL API source/scan/parser/matcher parity. This is not
 * a package patch or proof of general parser/resource safety.
 */
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { createRequire } from 'node:module';
import { resolve } from 'node:path';
import { performance } from 'node:perf_hooks';

const root=resolve('tests/defragmentation/security/agent11_vm_compile_guard_candidate.cjs');
const externalSpinScript=[
  'const {runBounded}=require(process.argv[1]);',
  'try { runBounded(()=>{for(;;){}},[],40,"external-loop");',
  'process.exitCode=2; } catch(e) {',
  'console.log("EXTERNAL VM COMPILER EXECUTION RESULT",e.code);',
  'if(e.code!=="ERR_AGENT11_PICOMATCH_COMPILE_DEADLINE")process.exitCode=3;',
  '}'
].join('\n');
const spin=spawnSync(process.execPath,['-e',externalSpinScript,root],{
  cwd:process.cwd(),encoding:'utf8',timeout:2500,maxBuffer:64*1024,windowsHide:true
});
assert.notEqual(spin.error?.code,'ETIMEDOUT','VM timeout did not interrupt main-realm external infinite synchronous call');
assert.equal(spin.status,0,'External main-realm callback escaped VM compile deadline: '+spin.stderr);
assert.ok(spin.stdout.includes('ERR_AGENT11_PICOMATCH_COMPILE_DEADLINE'),
 'No typed compiler deadline for external busy-loop');
console.log('VM COMPILER EXTERNAL SYNCHRONOUS FUNCTION DEADLINE:',JSON.stringify({
  test:'infinite main-realm function invoked inside VM',result:'interrupted by 40ms VM deadline',
  noHostileLoopExecutedOutsideBoundedChild:true,releaseApproval:false
}));

const r=createRequire(resolve('apps/desktop-ui/package.json'));
const pm=r('picomatch');
const internal=r('picomatch/lib/picomatch');
assert.equal(r('picomatch/package.json').version,'4.0.7');
const pat=[
 ['+(*)c','abc',{}],
 ['+(a*)','aa',{capture:true}],
 ['+(a|b)','abab',{}],
 ['+([ab]|b)','b',{nocase:true}],
 ['**/*.js','src/index.js',{}],
 ['*.md','foo.md',{}],
 ['+(a*b)','abab',{}],
 ['*(+(a))','aaa',{maxExtglobRecursion:1}],
 ['*(!(foo))','bar',{bash:true,windows:true}],
 ['abc/*.js','abc/one.js',{}]
];
function shape([pattern,input,opts]) {
  const re=pm.makeRe(pattern,opts);
  const parsed=pm.parse(pattern,opts);
  const scanned=pm.scan(pattern,opts);
  const fromParsed=pm.compileRe(parsed,opts);
  const toRegex=pm.toRegex(re.source,opts);
  const result=pm(pattern,opts)(input,true);
  return {
    makeRe:{source:re.source,flags:re.flags,native:re instanceof RegExp},
    compiled:{source:fromParsed.source,flags:fromParsed.flags},
    toRegex:{source:toRegex.source,flags:toRegex.flags},
    parsed:{output:parsed.output,negated:parsed.negated,input:parsed.input},
    scanned:{base:scanned.base,glob:scanned.glob,prefix:scanned.prefix,isGlob:scanned.isGlob,
      negated:scanned.negated},
    match:{yes:result.isMatch,output:result.output,
      captures:Array.isArray(result.match)?Array.from(result.match):result.match}
  };
}
const baseline=pat.map(shape);
const originals={makeRe:internal.makeRe,parse:internal.parse,scan:internal.scan};
const now=performance.now();
const {install}=r(root);
const handle=install({compileTimeoutMs:160,execTimeoutMs:80});
try {
  const candidate=pat.map(shape);
  for(let i=0;i<pat.length;i++) {
    assert.deepEqual(candidate[i],baseline[i],'Bounded original compilation API changed semantics for '+pat[i][0]);
  }
  for(const key of ['makeRe','parse','scan','compileRe','toRegex']) {
    assert.ok(handle.stats[key].calls>0,'Public original picomatch method was not actually guarded: '+key);
  }
  const data={passed:pat.length,expected:pat.length,elapsedMs:Math.round(performance.now()-now),
    calls:handle.stats,compilerRuntime:'original pinned 4.0.7',
    unsupported:'direct import of parse.js/scan.js, memory/cumulative limits, out-of-process native calls'};
  console.log('BOUNDED VM COMPILER ORIGINAL PUBLIC API SNAPSHOT PARITY:',JSON.stringify(data));
  const bad=pm('+([ab]|b)');
  const input='b'.repeat(36)+'/';
  let captured=false;
  try {
    bad(input);
  } catch(e) {
    if(e.code!=='ERR_AGENT11_PICOMATCH_DEADLINE')throw e;
    captured=true;
  }
  assert.ok(captured,'Expected malicious matcher execution to retain fail-closed VM bound');
} finally {
  handle.uninstall();
}
for(const key of ['makeRe','parse','scan'])assert.equal(internal[key],originals[key],
  'Original library method not restored after experimental compiler guard: '+key);
console.log('BOUNDED VM COMPILER RESEARCH STATUS: original compilation/match functions preserved in small finite sample, not source-complete parser security approval');
