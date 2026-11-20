'use strict';
/*
 * AGENT11 RESEARCH ONLY. NOT A PRODUCTION DROP-IN OR RELEASE REMEDIATION.
 * An experimental Node-specific RegExp subclass that uses vm.Script
 * synchronous deadlines for regex.exec. The original locked picomatch@4.0.7
 * remains unchanged. This demonstrates an execution-time mechanism only.
 *
 * Explicit incomplete surfaces: regex compilation; picomatch parser/scan;
 * RegExp.prototype.exec.call bypass; direct user-supplied native RegExp;
 * module loaded before this experimental hook; alternate imports; callbacks;
 * global flags/lastIndex cross-realm; other JS engines/browsers; package
 * authenticity; installed builds; CPU resource budgets/performance.
 *
 * On timeout THROW a typed error (fail closed). NEVER pretend "no match"
 * because that could silently exclude a required source file from the build.
 */
const vm = require('node:vm');
const { createRequire } = require('node:module');
const { resolve } = require('node:path');
const { performance } = require('node:perf_hooks');

const code = new vm.Script('out = RegExp.prototype.exec.call(regex, input)', {
  filename: 'agent11-bounded-picomatch-regexp.cjs'
});
const contexts = new WeakMap();

function cloneNativeMatch(m) {
  if (m === null) return null;
  const out = Array.from(m);
  out.index = m.index;
  out.input = m.input;
  out.groups = m.groups === undefined ? undefined : Object.assign(Object.create(null), m.groups);
  if (m.indices !== undefined) {
    out.indices = Array.from(m.indices, x => x == null ? x : Array.from(x));
    out.indices.groups = m.indices.groups === undefined ? undefined :
      Object.fromEntries(Object.entries(m.indices.groups).map(([key, v]) =>
        [key, v == null ? v : Array.from(v)]));
  }
  return out;
}

function install(options = {}) {
  const timeoutMs = options.timeoutMs ?? 80;
  if (!Number.isSafeInteger(timeoutMs) || timeoutMs < 5 || timeoutMs > 1000) {
    throw new TypeError('Experiment timeout must be integer 5-1000 ms');
  }
  const r = createRequire(resolve('apps/desktop-ui/package.json'));
  const pm = r('picomatch');
  const inner = r('picomatch/lib/picomatch');
  const manifest = r('picomatch/package.json');
  if (manifest.version !== '4.0.7') throw new Error('Only original pinned published 4.0.7 research supported');
  const originalToRegex = inner.toRegex;
  if (inner.toRegex.__agent11Protected) throw new Error('Experimental bounded hook already installed');

  class GuardedRegExp extends RegExp {
    exec(input) {
      const text = String(input);
      let ctx = contexts.get(this);
      if (!ctx) {
        ctx = vm.createContext({regex:this,input:null,out:null});
        contexts.set(this,ctx);
      }
      ctx.input=text;
      ctx.out=null;
      const started=performance.now();
      try {
        code.runInContext(ctx,{timeout:timeoutMs});
        return cloneNativeMatch(ctx.out);
      } catch(e) {
        if (e && e.code==='ERR_SCRIPT_EXECUTION_TIMEOUT') {
          contexts.delete(this);
          this.lastIndex = 0;
          const error=new Error('Agent11 research: regex execution exceeded '+timeoutMs+' ms budget');
          error.code='ERR_AGENT11_PICOMATCH_DEADLINE';
          error.elapsedMs=Math.round(performance.now()-started);
          throw error;
        }
        throw e;
      } finally {
        ctx.input=null;
        ctx.out=null;
      }
    }
  }
  function experimentalToRegex(source, opts) {
    const original=originalToRegex(source,opts);
    if (!(original instanceof RegExp)) return original;
    return new GuardedRegExp(original.source,original.flags);
  }
  Object.defineProperty(experimentalToRegex,'__agent11Protected',{value:true});
  // Internal picomatch.compileRe() uses the inner lexical picomatch.toRegex;
  // exports/index.js copies methods by value and therefore needs both hooks.
  inner.toRegex = experimentalToRegex;
  pm.toRegex = experimentalToRegex;

  return {
    GuardedRegExp,
    uninstall() {
      inner.toRegex = originalToRegex;
      pm.toRegex = originalToRegex;
    },
    marker:'AGENT11_EXPERIMENT_ONLY',
    timeoutMs
  };
}
module.exports={install};
