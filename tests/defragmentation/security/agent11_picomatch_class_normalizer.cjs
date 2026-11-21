'use strict';

/**
 * PR264 follow-up EXPERIMENT ONLY: bounded, language-equivalent rewrites of
 * generated regex subexpressions which express repeated overlapping classes.
 *
 * Exact upstream source: micromatch/picomatch#264
 *   8144a1a22a8fa1766951411f220cc8c6228cb041
 *
 * Not a general complexity analyzer, published package, or production fix.
 * Only handles ASCII class/literal shapes shown by the generated source.
 * All unrecognized patterns remain untouched and potentially unsafe.
 */
module.exports = function normalizeRepeatedClasses(source, opts = {}) {
  // Keep regex-capture and unusual-input modes out of this narrow experiment.
  if (typeof source !== 'string' || source.length > 4096 ||
      opts.capture === true || opts.maxExtglobRecursion === false ||
      opts.regex === true || opts.contains === true) return source;

  // If a repeated group offers both the picomatch generated literal "[ab]"
  // and [ab] class, then "b" is redundant: it is already covered by [ab].
  // Dropping that alternative preserves the accepted language exactly.
  source = source.replace(
    /\(\?:\(\?:\\\[([a-z]{1,16})\\\]\|\[([a-z]{1,16})\]\)\|([a-z])\)\+/g,
    (whole, literal, klass, letter) =>
      literal === klass && klass.includes(letter)
        ? '(?:\\[' + literal + '\\]|[' + klass + '])+' : whole
  );

  // A nested + of a repeated atom accepts exactly the same strings as one
  // repeated atom. Original generated regexes may contain a capturing wrapper.
  // Our bounded differential checks cover capture-output parity on the
  // recognized form, but this remains exploratory and needs independent review.
  const classArm = /\(\?:\\\[([a-z]{1,16})\\\]\|\[([a-z]{1,16})\]\)/g;
  for (const found of source.matchAll(classArm)) {
    if (found[1] !== found[2]) continue;
    const atom = found[0];
    const nested = '(?:(' + '(?:' + atom + ')+' + '))+'; 
    const flat = '((?:' + atom + ')+)';
    if (source.includes(nested)) source = source.split(nested).join(flat);
  }

  // A repeated union of 1-character literal branches already contains every
  // concatenation of those letters. Remove explicit multi-character branches
  // when each character has a 1-character alternative, preserving the language.
  source = source.replace(/\(\?:([a-z]+(?:\|[a-z]+)+)\)\+/g, (whole, body) => {
    const parts = body.split('|');
    const singles = new Set(parts.filter(p => p.length === 1));
    const kept = parts.filter(p => p.length === 1 || ![...p].every(c => singles.has(c)));
    return kept.length === parts.length ? whole : '(?:' + kept.join('|') + ')+';
  });

  return source;
};
