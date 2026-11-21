#!/usr/bin/env node
/**
 * Agent11 supplemental source-exact API compatibility check.
 *
 * Generates reference outputs from ORIGINAL published locked 4.0.7 before
 * the disposable node_modules patch, then compares after. Bounded finite
 * differential evidence is NOT proof of all-language equivalence or safety.
 * Preserves separate original 19, original upstream 10, and expanded 12 gates.
 *
 * Each pattern/option pair is executed in a killable subprocess.
 */
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { createRequire } from 'node:module';
import { existsSync, readFileSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';

const phase=process.argv[2];
assert.ok(phase==='baseline'||phase==='candidate',
  'Usage: node ...agent11_picomatch_bounded_semantics_diff.mjs baseline|candidate');
const snapshot=resolve('.agent11-bounded-semantics-original-407.json');
const uiRequire=createRequire(resolve('apps/desktop-ui/package.json'));
const modulePath=uiRequire.resolve('picomatch');
const patterns=[
  '+(*)','+(*)c','+(*a)','*(*a)','+(*a)c','*(a*)b',
  '+(a*)','+(a*b)','+(a*b)c','+(a*b*c)','+(ab*)',
  '+(a|b)','+(a|aa)','+(a|b|ab)','+([ab]|b)',
  '+([ab]|b)c','+(@(+([ab])))','+(@(*a))','+(@(*))',
  '*(a*|b)','*(*.js)','*(!(foo))','+(!(b)a)',
  '*(+(a))','+(+(a))','+(a|a*)','+([a-c]|c)',
  '+([AB]|B)','+(@(+([AB])))','+(A|B|AB)'
];
const options=[
  {},{capture:true},{nocase:true},{maxExtglobRecursion:false},
  {bash:true,windows:true},{dot:true}
];
const alphabet=['a','b','c','A','B','/','*'];
const strings=new Set(['']);
for(const a of alphabet){strings.add(a);for(const b of alphabet)strings.add(a+b);}
for(const more of [
  'aaa','aab','aba','abc','abb','ababc','abab',
  'a/b','a/bc','Aab','ABAB','[ab]','[AB]',
  'abc/','ab/','a*','*a','foo','bar','baz',
  'xay','xabaay','abcabc','abc.js','foo.js',
  'my/folder','aaaaaaa','bbbbbbb','BBBBBBB'
])strings.add(more);
const inputs=[...strings].sort();
assert.ok(inputs.length>=80&&inputs.length<=100,'Unexpected finite input set size');
assert.ok(inputs.every(s=>s.length<=10),'Unexpected long input');
const child="\nconst pm=require(process.argv[1]);\nconst record=JSON.parse(process.argv[2]);\nconst inputs=JSON.parse(process.argv[3]);\nconst opts=record.opts;\nconst results=[];\nlet regex;\ntry {regex=pm.makeRe(record.pattern,opts);}\ncatch(e) {console.log(JSON.stringify({compileError:e.name+': '+e.message}));process.exit();}\nfor(const input of inputs){\n  try{\n    const actual=pm.isMatch(input,record.pattern,opts);\n    const captures=opts.capture===true?(regex.exec(input)?.slice(1)??null):undefined;\n    results.push(opts.capture===true?{actual,captures}:{actual});\n  }catch(e){results.push({error:e.name+': '+e.message});}\n}\nconsole.log(JSON.stringify({results}));\n";
const records=[];
const cases=patterns.flatMap(pattern=>options.map(opts=>({pattern,opts})));
for(const tc of cases){
  const p=spawnSync(process.execPath,['-e',child,modulePath,JSON.stringify(tc),JSON.stringify(inputs)],{
    cwd:process.cwd(),encoding:'utf8',timeout:1600,maxBuffer:512*1024
  });
  if(p.error||p.status!==0){
    console.error(JSON.stringify({phase,pattern:tc.pattern,opts:tc.opts,status:p.status,
      error:p.error?.code??p.error?.message,stderr:p.stderr?.slice(0,300)}));
    process.exit(2);
  }
  try{records.push(JSON.parse(p.stdout.trim()));}
  catch(e){console.error('Malformed isolated matcher result',tc,e.message);process.exit(2);}
}
const fingerprint=createHash('sha256').update(JSON.stringify({patterns,options,inputs})).digest('hex');
const result={version:1,fingerprint,patterns,options,inputs,records};
if(phase==='baseline'){
  const pkg=JSON.parse(readFileSync('apps/desktop-ui/node_modules/picomatch/package.json','utf8'));
  assert.equal(pkg.version,'4.0.7','Original installed package manifest changed');
  assert.ok(!existsSync(snapshot),'Refuse stale reference snapshot (clean job required)');
  writeFileSync(snapshot,JSON.stringify(result)+'\n',{flag:'wx'});
  console.log('SOURCE-EXACT ORIGINAL 4.0.7 BOUNDED DIFFERENTIAL BASELINE: '+
    JSON.stringify({cases:cases.length,inputs:inputs.length,fingerprint}));
  process.exit(0);
}
assert.ok(existsSync(snapshot),'Baseline must be computed before parser mutation');
const baseline=JSON.parse(readFileSync(snapshot,'utf8'));
assert.equal(baseline.fingerprint,fingerprint,'Differential input corpus changed across phases');
assert.equal(baseline.records.length,result.records.length,'Reference count changed');
let changes=0;
const examples=[];
for(let i=0;i<cases.length;i++){
  const old=baseline.records[i],now=result.records[i];
  if(JSON.stringify(old)===JSON.stringify(now))continue;
  changes++;
  if(examples.length<15){
    const oldResults=old.results??[],newResults=now.results??[];
    const differingInputs=[];
    for(let j=0;j<inputs.length;j++){
      if(JSON.stringify(oldResults[j])!==JSON.stringify(newResults[j])){
        differingInputs.push({input:inputs[j],original:oldResults[j],candidate:newResults[j]});
        if(differingInputs.length>=3)break;
      }
    }
    examples.push({pattern:cases[i].pattern,opts:cases[i].opts,differingInputs,
      originalCompileError:old.compileError,candidateCompileError:now.compileError});
  }
}
console.log('SOURCE-EXACT BOUNDED MATCHER DIFFERENTIAL: '+JSON.stringify({
  compared:cases.length,inputs:inputs.length,changedPatternOptionPairs:changes,
  examples,scope:'Finite short input and public API returns only; NOT all-language equivalence',
  approvedPatch:false
}));
if(changes>0){
  console.error('FAIL-CLOSED: original published 4.0.7 API semantics changed; NOT a compatible drop-in');
  process.exitCode=1;
}
