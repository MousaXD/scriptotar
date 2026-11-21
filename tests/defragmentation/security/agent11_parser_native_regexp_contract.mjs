#!/usr/bin/env node
/**
 * Agent 11 independent native RegExp execution negative control.
 * Parser fixes must emit safe native regexes; intercepting a custom .exec
 * or a picomatch convenience wrapper is not sufficient.
 * Hostile cases execute ONLY in individually killable child processes.
 * This tiny supplemental corpus does not replace original 19/12/10 or 180x84.
 */
import assert from 'node:assert/strict';
import {spawnSync} from 'node:child_process';
import {createRequire} from 'node:module';
import {resolve} from 'node:path';
const modulePath=createRequire(resolve('apps/desktop-ui/package.json')).resolve('picomatch');
const cases=[
  {label:'star-led repeated extglob', pattern:'+(*a)', input:'a'.repeat(36)+'b', expected:false},
  {label:'suffix-star repeated extglob', pattern:'+(a*)', input:'a'.repeat(36)+'/', expected:false},
  {label:'overlapping repeated class', pattern:'+([ab]|b)', input:'b'.repeat(36)+'/', expected:false},
  {label:'nested repeated class', pattern:'+(@(+([ab])))', input:'a'.repeat(36)+'/', expected:false},
  {label:'redundant literal alternation', pattern:'+(a|b|ab)', input:'ab'.repeat(22)+'/', expected:false},
  {label:'positive repeated extglob', pattern:'+(a|b)', input:'abab', expected:true},
  {label:'positive simple filename', pattern:'*.js', input:'test.js', expected:true}
];
const modes=['normal','prototype','reconstructed'];
const childSource=[
  "'use strict';",
  "const pm=require(process.argv[1]);",
  "const tc=JSON.parse(process.argv[2]); const mode=process.argv[3];",
  "const re=pm.makeRe(tc.pattern,tc.options||{});",
  "if(!(re instanceof RegExp))throw Error('makeRe must return a native-compatible RegExp');",
  "let result;",
  "if(mode==='normal')result=re.exec(tc.input);",
  "else if(mode==='prototype')result=RegExp.prototype.exec.call(re,tc.input);",
  "else if(mode==='reconstructed')result=new RegExp(re.source,re.flags).exec(tc.input);",
  "else throw Error('unknown mode');",
  "const actual=result!==null;",
  "process.stdout.write(JSON.stringify({actual,mode}));",
  "if(actual!==tc.expected)process.exitCode=3;"
].join('\n');
let failed=0;let timedOut=0;
for(const tc of cases){
  for(const mode of modes){
    const p=spawnSync(process.execPath,['-e',childSource,modulePath,JSON.stringify(tc),mode],{
      cwd:process.cwd(),encoding:'utf8',timeout:1500,maxBuffer:65536,windowsHide:true
    });
    const timeout=p.error?.code==='ETIMEDOUT';
    let data;try{data=JSON.parse(p.stdout.trim())}catch{}
    const pass=!timeout&&p.status===0&&data?.actual===tc.expected&&data?.mode===mode;
    if(!pass)failed++;if(timeout)timedOut++;
    console.log('NATIVE '+tc.label+' '+mode+': '+(pass?'PASS':timeout?'TIMEOUT':'MISMATCH/ERROR'));
    if(!pass&&p.stderr.trim())console.error(p.stderr.slice(0,400));
  }
}
assert.equal(cases.length*modes.length,21);
console.log('NATIVE REGEXP SUMMARY: '+(21-failed)+'/21 passing; '+timedOut+' native execution timeouts');
if(failed){
  console.error('NO-GO: parser output native RegExp remains unsafe or incompatible');
  process.exitCode=1;
}
