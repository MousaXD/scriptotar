#!/usr/bin/env node
/**
 * Agent11 SEPARATE follow-up adversarial corpus for PR264+experimental patch.
 * Never change, replace, or weaken the immutable original 19-case contract.
 * Inputs are fixed short negative near-misses, run in killable child processes.
 * A candidate that fails even one remains NOT REMEDIATED / RELEASE NO-GO.
 */
import { spawnSync } from 'node:child_process';
import { createRequire } from 'node:module';
import { resolve } from 'node:path';
const moduleFile = createRequire(resolve('apps/desktop-ui/package.json')).resolve('picomatch');
const cases = [
  { label: 'uppercase overlapping class', pattern: '+([AB]|B)', input: 'B'.repeat(36) + '/', expected: false },
  { label: 'uppercase nested repeated class', pattern: '+(@(+([AB])))', input: 'A'.repeat(36) + '/', expected: false },
  { label: 'range overlapping class', pattern: '+([a-c]|c)', input: 'c'.repeat(36) + '/', expected: false },
  { label: 'range nested repeated class', pattern: '+(@(+([a-c])))', input: 'a'.repeat(36) + '/', expected: false },
  { label: 'case-folded overlapping class', pattern: '+([ab]|B)', opts: {nocase:true}, input: 'b'.repeat(36) + '/', expected: false },
  { label: 'capture-option overlapping class', pattern: '+([ab]|b)', opts: {capture:true}, input: 'b'.repeat(36) + '/', expected: false },
  { label: 'capture-option nested repeated class', pattern: '+(@(+([ab])))', opts: {capture:true}, input: 'a'.repeat(36) + '/', expected: false },
  { label: 'disabled recursion overlapping class', pattern: '+([ab]|b)', opts: {maxExtglobRecursion:false}, input: 'b'.repeat(36) + '/', expected: false },
  { label: 'uppercase redundant literals', pattern: '+(A|B|AB)', input: 'AB'.repeat(22) + '/', expected: false },
  { label: 'lowercase alternate redundant order', pattern: '+(a|b|ba)', input: 'ba'.repeat(22) + '/', expected: false },
  { label: 'uppercase safe control', pattern: '+(A|B)', input: 'ABAB', expected: true },
  { label: 'uppercase small matching control', pattern: '+([AB]|B)', input: 'B', expected: true }
];
const childSrc = [
  "const pm = require(process.argv[1]);",
  "const x = JSON.parse(process.argv[2]);",
  "const actual = pm.isMatch(x.input, x.pattern, x.opts || {});",
  "process.stdout.write(JSON.stringify({ actual }));",
  "if (actual !== x.expected) process.exitCode = 3;"
].join('\n');
let failed = 0;
for (const x of cases) {
  const child = spawnSync(
    process.execPath, ['-e', childSrc, moduleFile, JSON.stringify(x)],
    {cwd: process.cwd(), encoding:'utf8', timeout:1500, maxBuffer:65536}
  );
  let actual;
  try {actual = JSON.parse(child.stdout.trim()).actual;} catch { /* invalid output */ }
  const timeout = child.error?.code === 'ETIMEDOUT';
  const good = !timeout && child.status === 0 && actual === x.expected;
  const result = good ? 'PASS' : timeout ? 'TIMEOUT' : 'MISMATCH/ERROR';
  console.log('EXTENDED: ' + x.label + ': ' + result +
    '; expected=' + x.expected + '; actual=' + String(actual));
  if (!good) {
    failed++;
    if (child.stderr.trim()) console.error(child.stderr.trim());
  }
}
console.log('EXTENDED SUMMARY: ' + (cases.length - failed) + '/' + cases.length + ' passing');
if (failed) {
  console.error('NO-GO: additional original-parser-risk variants remain unbounded or changed behavior.');
  process.exitCode = 1;
}
