#!/usr/bin/env node
/* Agent11 source-exact pre-Vite guard acceptance; does not patch picomatch. */
import assert from 'node:assert/strict';
import { cpSync, existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const repo = resolve(dirname(fileURLToPath(import.meta.url)),'..','..','..');
const realSource = join(repo,'apps','desktop-ui');
const guard = resolve(repo,'.agent11-trusted-guard','apps','desktop-ui','scripts','guard-vite-build-patterns.mjs');
assert.ok(existsSync(guard), 'Immutable reviewed guard checkout missing');
function runGuard(workspace) {
  const r = spawnSync(process.execPath,[guard,'--workspace',workspace],{
    cwd:repo, encoding:'utf8',timeout:7000,windowsHide:true,maxBuffer:256*1024
  });
  if(r.error) throw r.error;
  return r;
}
const temp=mkdtempSync(join(tmpdir(),'agent11-prod-guard-'));
const workspace=join(temp,'apps','desktop-ui');
const tauriDir=join(temp,'apps','desktop','src-tauri');
function fixtureWrite(relativePath,content){const path=join(temp,relativePath);mkdirSync(dirname(path),{recursive:true});writeFileSync(path,content);}
function read(relativePath){return readFileSync(join(temp,relativePath),'utf8');}
function checkBlocked(label,mutate,expected) {
  const undo=mutate();
  try {
    const out=runGuard(workspace);
    assert.notEqual(out.status,0,label+': guard unexpectedly allowed build');
    assert.match(out.stdout,expected,label+': missing fail-closed evidence');
    console.log('BLOCKED before compiler: '+label);
  } finally {undo();}
}
try {
  mkdirSync(workspace,{recursive:true});
  mkdirSync(tauriDir,{recursive:true});
  cpSync(join(realSource,'src'),join(workspace,'src'),{recursive:true});
  for(const file of ['vite.config.ts','package.json','package-lock.json','tsconfig.json','index.html']) {
    cpSync(join(realSource,file),join(workspace,file));
  }
  cpSync(join(repo,'apps','desktop','src-tauri','tauri.conf.json'),join(tauriDir,'tauri.conf.json'));
  let count=0;
  const clean=runGuard(workspace);
  assert.equal(clean.status,0,'Reviewed source failed: '+clean.stdout+' '+clean.stderr);
  console.log('SAFE SOURCE ACCEPTED before compiler; normal compiled build runs only after npm ci in workflow'); count++;

  const unusual=join(workspace,'src','a'.repeat(36)+'b.ts');
  writeFileSync(unusual,'export const harmless = 1;\n');
  const control=runGuard(workspace);
  assert.equal(control.status,0,'Weird FILENAME control rejected ordinary source'); count++;
  rmSync(unusual);

  const hostile=join(workspace,'src','agent11-hostile-glob.ts');
  writeFileSync(hostile,"const modules = import.meta.glob('./+(*a)'); export { modules };\n");
  let compilerInvoked=false;
  const hostileVerdict=runGuard(workspace);
  if(hostileVerdict.status===0) compilerInvoked=true;
  assert.equal(compilerInvoked,false,'A hostile source pattern reached the simulated Vite entrypoint');
  assert.notEqual(hostileVerdict.status,0,'Hostile source pattern guard failed open');
  assert.match(hostileVerdict.stdout,/Vite import\.meta\.glob macro/);
  console.log('HOSTILE SOURCE PATTERN STOPPED before Vite compiler invoked'); count++;
  rmSync(hostile);

  checkBlocked('package-lock drift',()=>{
    const p='apps/desktop-ui/package-lock.json',before=read(p);
    fixtureWrite(p,before+' \n');return ()=>fixtureWrite(p,before);
  },/Unreviewed build manifest\/lock\/Tauri contract/); count++;
  checkBlocked('npm package manifest drift',()=>{
    const p='apps/desktop-ui/package.json',before=read(p);
    fixtureWrite(p,before.replace('"vite build"','"vite build --unsafe-custom"'));return ()=>fixtureWrite(p,before);
  },/Unreviewed build manifest\/lock\/Tauri contract/); count++;
  checkBlocked('Tauri indirect beforeBuildCommand drift',()=>{
    const p='apps/desktop/src-tauri/tauri.conf.json',before=read(p);
    fixtureWrite(p,before.replace('npm ci && npm run build','npm run build'));return ()=>fixtureWrite(p,before);
  },/Unreviewed build manifest\/lock\/Tauri contract/); count++;
  checkBlocked('Vite plugin configuration drift',()=>{
    const p='apps/desktop-ui/vite.config.ts',before=read(p);
    fixtureWrite(p,before+'\n// manual review required\n');return ()=>fixtureWrite(p,before);
  },/Unreviewed Vite config SHA-256/); count++;
  checkBlocked('shadow npm-shrinkwrap overrides pinned lock',()=>{
    const p='apps/desktop-ui/npm-shrinkwrap.json';
    fixtureWrite(p,'{}\n');return ()=>rmSync(join(temp,p));
  },/Unreviewed shadow npm\/build configuration/); count++;
  checkBlocked('workspace npmrc override',()=>{
    const p='apps/desktop-ui/.npmrc';
    fixtureWrite(p,'ignore-scripts=false\n');return ()=>rmSync(join(temp,p));
  },/Unreviewed shadow npm\/build configuration/); count++;
  checkBlocked('parent npmrc override',()=>{
    const p='apps/.npmrc';
    fixtureWrite(p,'registry=https:\/\/example.invalid\/\n');return ()=>rmSync(join(temp,p));
  },/Unreviewed shadow npm\/build configuration/); count++;
  checkBlocked('Vite env override',()=>{
    const p='apps/desktop-ui/.env.production';
    fixtureWrite(p,'VITE_SOURCE=unreviewed\n');return ()=>rmSync(join(temp,p));
  },/Unreviewed shadow npm\/build configuration/); count++;
  checkBlocked('TypeScript build config drift',()=>{
    const p='apps/desktop-ui/tsconfig.json',before=read(p);
    fixtureWrite(p,before+' \n');return ()=>fixtureWrite(p,before);
  },/Unreviewed build manifest\/lock\/Tauri contract/); count++;
  checkBlocked('new PostCSS build config',()=>{
    const p='apps/desktop-ui/postcss.config.js';
    fixtureWrite(p,'export default {};\n');return ()=>rmSync(join(temp,p));
  },/New unreviewed build configuration file/); count++;

  console.log(JSON.stringify({test:'Agent11 trusted pre-Vite production-gate acceptance',controls:count,
    verdict:'PASS: clean source accepted, hostile pattern never reached compiler gate, config and lockfile changes rejected',
    bound:'Trusted script + first-party source/config only, NOT third-party plugin patterns or parser remediation'},null,2));
} finally {rmSync(temp,{recursive:true,force:true});}
