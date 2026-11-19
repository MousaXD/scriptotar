#!/usr/bin/env node
/**
 * Agent 11 PRE-VITE source/config review guard.
 * Run BEFORE npm ci, vite build, or the Tauri frontend build.
 * Narrow scope: reviewed desktop-ui/src, index.html, Vite config and Tauri
 * build contract. It cannot establish safety of third-party/plugin-generated
 * glob patterns, other workspaces, or a malicious CI workflow edit.
 * Failures require human review; this is NOT a picomatch parser fix.
 */
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFileSync, readdirSync, existsSync } from 'node:fs';
import { dirname, extname, join, relative, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';

const argv = process.argv.slice(2);
const trustedDefaultRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const isWorkspaceAudit = argv.length === 2 && argv[0] === '--workspace';
const root = isWorkspaceAudit ? resolve(process.cwd(), argv[1]) : trustedDefaultRoot;
const repo = resolve(root, '..', '..');
const trustedGitBlobShas = Object.freeze({
  'apps/desktop-ui/package.json': '19edb804b2c442be96311e2f39f7788dfb286dda',
  'apps/desktop-ui/package-lock.json': 'eebc9228dd404f0b9f9bd55f2665ab3e00fe9dfa',
  'apps/desktop-ui/tsconfig.json': 'aab6eff7f7e345f6fb93b86e16d69aa328d8e140',
  'apps/desktop-ui/index.html': '250619c2983d104116bbc976222b59888b99ad80',
  'apps/desktop/src-tauri/tauri.conf.json': 'b28cc6a5057032479f94d5b94ed01f2ea23fa03e'
});
const trustedConfigSha256 = '252cb3b99585c239741cc3590c9636da49f087cc6e6ed07a5c6fcef7b89e4300';
// Vite/esbuild also compiles .mts and .cts TypeScript modules. Omitting these
// lets hostile first-party glob macros evade the reviewed src scanner.
const extensions = new Set(['.js','.jsx','.mjs','.cjs','.ts','.tsx','.mts','.cts','.svelte']);
const macro = /\bimport(?:\s|\/\*[\s\S]*?\*\/)*\.(?:\s|\/\*[\s\S]*?\*\/)*meta(?:\s|\/\*[\s\S]*?\*\/)*(?:\.(?:\s|\/\*[\s\S]*?\*\/)*glob(?:Eager)?\b|\[(?:\s|\/\*[\s\S]*?\*\/)*(['"\x60])glob(?:Eager)?\1(?:\s|\/\*[\s\S]*?\*\/)*\])/i;
const modules = /\b(?:picomatch|micromatch|tinyglobby|fdir|fast-glob|globby)\b/i;
const calls = /\b(?:glob|globSync|globStream|globIterate|fastGlob)\s*\(/i;

function inspectText(text) {
  const hits = [];
  if (macro.test(text)) hits.push('Vite import.meta.glob macro');
  if (modules.test(text)) hits.push('glob matcher/build-tool reference');
  if (calls.test(text)) hits.push('direct glob API call');
  return hits;
}
function selfTest() {
  const hostile = [
    "const x = import.meta.glob('./+(*a)')",
    "const x = import /* a */ . meta /* b */ . glob('./+(*a)')",
    "const x = import.meta['glob']('./+(*a)')",
    'const x = import.meta["glob"]("./+(*a)")',
    'const x = import.meta[\x60glob\x60]("./+(*a)")',
    "const x = import.meta.globEager('./*(*a)')",
    "const x = globSync(userPattern)",
    "import matcher from 'picomatch'",
    "const tool = require('tinyglobby')"
  ];
  const clean = [
    'const title = "Scriptotar";',
    'const filename = "a".repeat(36) + "b";',
    'const queued = await invoke("enqueue_url", { url });',
    'const paths = ["normal.js", "other.js"];'
  ];
  for (const input of hostile) assert.ok(inspectText(input).length, 'missed hostile fixture: '+input);
  for (const input of clean) assert.deepEqual(inspectText(input), [], 'blocked clean control: '+input);
  console.log('SYNTHETIC pre-build guard controls '+(hostile.length+clean.length)+'/'+(hostile.length+clean.length)+' PASS; not production or parser certification.');
}

// P0-149-07: examine effective JS module paths, not raw regex matches.
// Quote/escape decoding is mandatory; no arbitrary module-literal length cap.
// This bounded scanner excludes comments and ordinary non-module strings.
// Unrecognized dynamic imports are REVIEW REQUIRED, not silent success.
function tokenizeModuleSource(source) {
  const out = [];
  function code(at, inTemplateExpression = false) {
    let braces = 0, i = at;
    while (i < source.length) {
      const ch = source[i], next = source[i+1];
      if (/\s/.test(ch)) { i++; continue; }
      if (ch === '/' && next === '/') {
        i += 2;
        while (i < source.length && source[i] !== '\n' && source[i] !== '\r') i++;
        continue;
      }
      if (ch === '/' && next === '*') {
        const end = source.indexOf('*/',i+2);
        if (end < 0) { out.push({kind:'invalid',value:'unclosed comment'}); return source.length; }
        i = end+2; continue;
      }
      if (ch === "'" || ch === '"' || ch === '\x60') {
        const token = {kind:'literal',quote:ch,raw:'',closed:false,interpolated:false};
        out.push(token);
        const begin = ++i;
        while (i < source.length) {
          if (source[i] === '\\') { i += 2; continue; }
          if (source[i] === ch) {
            token.raw = source.slice(begin,i);
            token.closed = true;
            i++; break;
          }
          if (ch === '\x60' && source[i] === '$' && source[i+1] === '{') {
            token.interpolated = true;
            i = code(i+2,true);
            continue;
          }
          if (ch !== '\x60' && (source[i] === '\r' || source[i] === '\n')) break;
          i++;
        }
        if (!token.closed) token.raw = source.slice(begin,i);
        continue;
      }
      if (/[A-Za-z_$]/.test(ch)) {
        const begin = i++;
        while (i < source.length && /[A-Za-z0-9_$]/.test(source[i])) i++;
        out.push({kind:'word',value:source.slice(begin,i)});
        continue;
      }
      if (ch === '{') braces++;
      if (ch === '}') {
        if (inTemplateExpression && braces === 0) return i+1;
        braces--;
      }
      out.push({kind:'punct',value:ch});
      i++;
    }
    if (inTemplateExpression) out.push({kind:'invalid',value:'unclosed template expression'});
    return i;
  }
  code(0);
  return out;
}
function decodedModuleString(t) {
  if (!t || t.kind !== 'literal' || !t.closed || t.interpolated)
    throw Error('unsupported computed or unterminated module literal');
  const raw=t.raw;
  let value='';
  for (let i=0;i<raw.length;i++) {
    if (raw[i] !== '\\') {value+=raw[i];continue;}
    const next=raw[++i];
    if (next === undefined) throw Error('incomplete JS string escape');
    if (next === 'u' && raw[i+1] === '{') {
      const close=raw.indexOf('}',i+2),hex=close<0?'':raw.slice(i+2,close);
      if (!/^[0-9a-fA-F]{1,6}$/.test(hex) || parseInt(hex,16)>0x10ffff)
        throw Error('invalid Unicode brace escape');
      value+=String.fromCodePoint(parseInt(hex,16));
      i=close;continue;
    }
    if (next === 'x' || next === 'u') {
      const digits=next==='x'?2:4,hex=raw.slice(i+1,i+1+digits);
      if (!new RegExp('^[a-fA-F0-9]{'+digits+'}$').test(hex))
        throw Error('invalid JS hex/unicode escape');
      value+=String.fromCharCode(parseInt(hex,16));
      i+=digits;continue;
    }
    if (next === '\n' || next === '\r' || next === '\u2028' || next === '\u2029') {
      if (next === '\r' && raw[i+1] === '\n') i++;
      continue;
    }
    if (next === '0') {
      if (/[0-9]/.test(raw[i+1]||'')) throw Error('legacy octal escape');
      value+='\0';continue;
    }
    if (/[1-9]/.test(next)) throw Error('legacy octal/decimal escape');
    const known={n:'\n',r:'\r',t:'\t',b:'\b',f:'\f',v:'\v'};
    value+=Object.prototype.hasOwnProperty.call(known,next)?known[next]:next;
  }
  return value;
}
function inspectEffectiveModules(sourceText,path,reviewedSource,findings) {
  const ts=tokenizeModuleSource(sourceText),location=relative(root,path);
  const checked=new Set();
  function check(index) {
    if (checked.has(index)) return;
    checked.add(index);
    let effective;
    try { effective=decodedModuleString(ts[index]); }
    catch(err) {
      findings.push('Ambiguous/unsupported module specifier: '+location+' -> '+String(err.message).slice(0,90));
      return;
    }
    // Windows interprets backslashes as path separators; reject ambiguous
    // controls/drives everywhere instead of allowing OS-dependent verdicts.
    if (/[\\\x00-\x1f\x7f]/.test(effective)||/^[A-Za-z]:[\\/]/.test(effective)) {
      findings.push('Unreviewed out-of-scope relative source reference: '+location+' -> ambiguous path');
      return;
    }
    for (const hit of inspectText(effective))
      findings.push(hit+' in decoded module path: '+location);
    if (effective.startsWith('/'))
      findings.push('Unreviewed absolute/root-relative module import: '+location+' -> '+effective.slice(0,240));
    else if (effective.startsWith('./')||effective.startsWith('../')) {
      const target=resolve(dirname(path),effective);
      if (target!==reviewedSource&&!target.startsWith(reviewedSource+sep))
        findings.push('Unreviewed out-of-scope relative source reference: '+location+' -> '+effective.slice(0,240));
    }
  }
  function fromClause(start) {
    let depth=0;
    for (let j=start;j<ts.length && j<start+2048;j++) {
      const t=ts[j],v=t.value;
      if (v==='{'||v==='('||v==='[') depth++;
      else if (v==='}'||v===')'||v===']') depth=Math.max(0,depth-1);
      if (depth===0 && v==='from') {check(j+1);return;}
      if (depth===0&&(v===';'||(j>start&&(v==='import'||v==='export')))) break;
    }
  }
  for (let i=0;i<ts.length;i++) {
    const t=ts[i],n=ts[i+1];
    if (t.kind!=='word') continue;
    if (t.value==='import' && ts[i-1]?.value!=='.') {
      if (n?.value==='.') continue;
      if (n?.value==='(') {
        check(i+2);
        if (ts[i+3]?.value!==')' && ts[i+3]?.value!==',')
          findings.push('Ambiguous/unsupported dynamic module specifier: '+location);
      } else if (n?.kind==='literal') check(i+1);
      else if (n?.value!==':'&&n?.value!=='='&&n?.value!==')') fromClause(i+1);
    } else if (t.value==='export' && (n?.value==='*'||n?.value==='{'||n?.value==='type')) {
      fromClause(i+1);
    } else if (t.value==='require' && ts[i-1]?.value!=='.' && n?.value==='(') {
      check(i+2);
    }
  }
}

function collect(dir, problems, result) {
  if (!existsSync(dir)) { problems.push('Missing reviewed source root: '+dir); return; }
  for (const item of readdirSync(dir, { withFileTypes: true })) {
    const path = join(dir, item.name);
    if (item.isSymbolicLink()) problems.push('Source symlink requires review: '+relative(root,path));
    else if (item.isDirectory()) collect(path, problems, result);
    else if (item.isFile() && extensions.has(extname(path))) result.push(path);
  }
}
function gitBlobShaCanonical(file, findings) {
  const content = readFileSync(file, 'utf8');
  if (/\r(?!\n)/.test(content)) findings.push('Bare carriage return in reviewed build contract: '+relative(repo,file));
  const canonical = Buffer.from(content.replace(/\r\n/g, '\n'), 'utf8');
  return createHash('sha1').update('blob '+canonical.byteLength+'\0').update(canonical).digest('hex');
}
function audit() {
  const findings = [];
  const configFile = join(root,'vite.config.ts');
  const packageFile = join(root,'package.json');
  const packageLockFile = join(root,'package-lock.json');
  const tsconfigFile = join(root,'tsconfig.json');
  const tauriFile = join(repo,'apps','desktop','src-tauri','tauri.conf.json');
  for (const path of [configFile,packageFile,packageLockFile,tsconfigFile,tauriFile]) {
    if (!existsSync(path)) findings.push('Missing build contract: '+relative(repo,path));
  }
  if (!findings.length) {
    // GitHub Windows checkout may convert Git LF to CRLF. Compare canonical
    // line endings, but preserve fail-closed review for all other bytes.
    const configText = readFileSync(configFile, 'utf8');
    if (/\r(?!\n)/.test(configText))
      findings.push('Unexpected bare-CR Vite config line ending: manual review required');
    const sha256 = createHash('sha256').update(configText.replace(/\r\n/g, '\n'), 'utf8').digest('hex');
    if (sha256 !== trustedConfigSha256)
      findings.push('Unreviewed Vite config SHA-256 change: inspect all plugin and PostCSS glob sinks BEFORE build');
    for (const [rel,expected] of Object.entries(trustedGitBlobShas)) {
      const path = join(repo,rel);
      const actual = gitBlobShaCanonical(path,findings);
      if (actual !== expected) findings.push('Unreviewed build manifest/lock/Tauri contract '+rel+'; expected Git blob '+expected+' observed '+actual);
    }
    const pkg = JSON.parse(readFileSync(packageFile,'utf8'));
    const tauri = JSON.parse(readFileSync(tauriFile,'utf8'));
    if (pkg.scripts?.build !== 'vite build') findings.push('Unreviewed npm build command');
    if (tauri.build?.beforeBuildCommand !== 'cd ../desktop-ui && npm ci && npm run build')
      findings.push('Unreviewed Tauri beforeBuildCommand');
    if (tauri.build?.frontendDist !== '../../desktop-ui/dist')
      findings.push('Unreviewed Tauri frontendDist');
  }
  // npm-shrinkwrap.json overrides package-lock.json. A PR adding .npmrc or a
  // parent workspace manifest may also change the install graph despite a
  // correctly pinned package-lock. Reject these BEFORE npm ci in every job.
  // Unreviewed Vite/rollup/PostCSS configs or Vite .env files also change
  // compiler behavior before the ordinary frontend import scanner can help.
  const shadowInstall = /^(?:npm-shrinkwrap\.json|\.npmrc|\.yarnrc(?:\.yml)?|pnpm-lock\.yaml|pnpm-workspace\.yaml|yarn\.lock|bun\.lock(?:b)?|\.pnp\.(?:cjs|js)|\.env(?:\..*)?)$/i;
  // PostCSS's cosmiconfig loader accepts .postcssrc[.cjs|.js|.json|.yaml|...]
  // in the workspace or any searched ancestor; unreviewed dotfiles can
  // execute plugins during Vite before any first-party glob scanner runs.
  const postcssRc = /^\.postcssrc(?:\..+)?$/i;
  // Reject EVERY alternate build-config suffix (including JSON/YAML), not
  // only JS/TS configs, at all checked parent/workspace levels.
  const extraBuildConfig = /^(?:vite|svelte|postcss|tailwind|rollup|webpack|esbuild)\.config(?:\..+)?$/i;
  for (const dir of [repo,join(repo,'apps'),root]) {
    if (!existsSync(dir)) { findings.push('Missing reviewed source parent: '+dir); continue; }
    for (const entry of readdirSync(dir,{withFileTypes:true})) {
      const file = join(dir,entry.name);
      if (shadowInstall.test(entry.name) || postcssRc.test(entry.name) ||
          (extraBuildConfig.test(entry.name) && file !== configFile) ||
          (dir !== root && (entry.name === 'package.json' || entry.name === 'package-lock.json'))) {
        findings.push('Unreviewed shadow npm/build configuration: '+relative(repo,file));
      }
    }
  }
  const files = [];
  collect(join(root,'src'),findings,files);
  const html = join(root,'index.html');
  if (!existsSync(html)) findings.push('Missing reviewed HTML entrypoint');
  else files.push(html);
  for (const item of readdirSync(root, { withFileTypes: true })) {
    if (item.isFile() && /^(?:postcss|tailwind|svelte|rollup|vite)\.config\./i.test(item.name) && item.name !== 'vite.config.ts')
      findings.push('New unreviewed build configuration file: '+item.name);
  }
  if (files.length < 40) findings.push('Unexpectedly few reviewed frontend sources: '+files.length);
  const reviewedSource = resolve(root, 'src');
  // Vite can compile ../qa/evil.ts imported by a reviewed module even though
  // qa/ is outside this scanner's source root. Fail closed on escape literals.
  // Conservative including comments/strings; deliberate manual review required.
  // Scan the entire literal (source files already have a 1 MB ceiling).
  // A 400-character regex cap silently omitted valid Vite imports that resolve
  // outside src after redundant ./ path segments are normalized.
  const relativeLiteral = /(["'\x60])(\.{1,2}\/[^"'\x60\r\n]+)\1/g;
  // Root-relative Vite specifiers can compile unreviewed workspace modules.
  // Restrict known import/from forms; computed/alias/plugin imports still
  // require independent review rather than a blanket safety claim.
  const absoluteModuleLiteral = /\b(?:from|import)(?:\s|\/\*[\s\S]*?\*\/)*(?:\((?:\s|\/\*[\s\S]*?\*\/)*)?(["'\x60])\/[^"'\x60\r\n]+\1/g;
  for (const path of files.sort()) {
    const bytes = readFileSync(path);
    if (bytes.length > 1000000) { findings.push('Large source requires review: '+relative(root,path)); continue; }
    const sourceText = bytes.toString('utf8');
    for (const hit of inspectText(sourceText)) findings.push(hit+': '+relative(root,path));
    if (path !== html) {
      inspectEffectiveModules(sourceText,path,reviewedSource,findings);
      absoluteModuleLiteral.lastIndex = 0;
      for (const match of sourceText.matchAll(absoluteModuleLiteral)) {
        findings.push('Unreviewed absolute/root-relative module import: '+relative(root,path)+' -> '+match[0].slice(0,240));
      }
      relativeLiteral.lastIndex = 0;
      for (const match of sourceText.matchAll(relativeLiteral)) {
        const resolved = resolve(dirname(path), match[2]);
        if (resolved !== reviewedSource && !resolved.startsWith(reviewedSource + sep)) {
          findings.push('Unreviewed out-of-scope relative source reference: '+relative(root,path)+' -> '+match[2].slice(0,240));
        }
      }
    }
  }
  console.log(JSON.stringify({
    check:'Agent11 PRE-BUILD first-party Vite source-pattern review guard',
    reviewedViteConfigSha256:trustedConfigSha256,
    sourceContractBlobPins:trustedGitBlobShas,
    scannedFirstPartyFiles:files.length, findings,
    scope:'desktop-ui/src, index.html, Vite config, build contracts; excludes third-party/plugin-generated glob patterns',
    result: findings.length ? 'REVIEW REQUIRED BEFORE VITE' : 'NO FIRST-PARTY GLOB MARKERS DETECTED; NOT FULL SECURITY CERTIFICATION'
  },null,2));
  if (findings.length) process.exitCode = 1;
}
if (argv.length === 1 && argv[0] === '--self-test') selfTest();
else if (argv.length === 0 || isWorkspaceAudit) audit();
else { console.error('Usage: node scripts/guard-vite-build-patterns.mjs [--self-test | --workspace TARGET_PATH]'); process.exitCode=2; }
