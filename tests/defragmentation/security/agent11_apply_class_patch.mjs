#!/usr/bin/env node
/**
 * EXPERIMENT ONLY: append an auditable, narrow normalizer to immutable
 * upstream PR264's disposable runner copy. Never modify production lockfiles,
 * vendored packages, original hostile-pattern tests, or the trusted guard.
 */
import { readFileSync, writeFileSync, copyFileSync } from 'node:fs';
import { execFileSync } from 'node:child_process';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { resolve } from 'node:path';

const require = createRequire(import.meta.url);
const normalize = require('./agent11_picomatch_class_normalizer.cjs');
const patchFile = resolve('apps/desktop-ui/node_modules/picomatch/lib/parse.js');
const helper = resolve('apps/desktop-ui/node_modules/picomatch/lib/agent11_class_normalizer.cjs');
const expectedBlob = 'b9eaa6dbc4a52c931e8ec5d7bb216d0eeb8c61a8';
const actualBlob = execFileSync('git', ['hash-object', patchFile], { encoding: 'utf8' }).trim();
assert.equal(actualBlob, expectedBlob, 'candidate parser source must match exact upstream PR264 blob');

// Check exact known grammar and capture behavior using bounded short strings.
// This is a finite compatibility regression, NOT a ReDoS proof.
const corpus = [
  '^(?:(?=.)(?:(?:\\[ab\\]|[ab])|b)+)$',
  '^(?:(?=.)(?:((?:(?:\\[ab\\]|[ab]))+))+)$',
  '^(?:(?=.)(?:a|b|ab)+)$'
];
for (const input of corpus) {
  const output = normalize(input);
  assert.notEqual(output, input, 'missing expected algebra rewrite');
  const before = new RegExp(input);
  const after = new RegExp(output);
  const chars = ['a', 'b', '[', ']'];
  for (let count = 0; count <= 7; count++) {
    const total = 4 ** count;
    for (let n = 0; n < total; n++) {
      let v = n;
      let sample = '';
      for (let i = 0; i < count; i++) {
        sample += chars[v % 4];
        v = Math.floor(v / 4);
      }
      const left = before.exec(sample);
      const right = after.exec(sample);
      assert.deepEqual(right, left, 'short regex/capture semantics differ on: ' + sample);
    }
  }
}
const controls = [
  '^foo$',
  '^(?:(?:a|bc)+)$',
  '^(?:(?=.)(?:a|b|ab))$',
  '^(?:(?=.)a*b)$',
  '^(?:(?:\\[ab\\]|[ab])|c)$'
];
for (const control of controls) assert.equal(normalize(control), control, 'unexpected rewrite');

const code = readFileSync(patchFile, 'utf8');
const anchor = '  return state;\n};\n\n/**\n * Fast paths for creating regular expressions';
assert.equal(code.split(anchor).length, 2, 'unexpected or ambiguous patch anchor');
const newCode = code.replace(
  anchor,
  "  state.output = require('./agent11_class_normalizer.cjs')(state.output, opts);\n" +
  anchor
);
copyFileSync(resolve('tests/defragmentation/security/agent11_picomatch_class_normalizer.cjs'), helper);
writeFileSync(patchFile, newCode);
console.log('Experimental PR264+bounded-normalizer patch applied to disposable node_modules only');
console.log('original blob=' + expectedBlob + ' algebra test=PASS, historical 19/10 probes remain mandatory');
