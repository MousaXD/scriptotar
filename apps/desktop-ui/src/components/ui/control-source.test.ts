import { describe, expect, it } from 'vitest';
import palette from '../../design/tokens.css?raw';
import base from '../../design/base.css?raw';
import app from '../../app.css?raw';
import theme from '../../theme.css?raw';
import selectSource from './Select.svelte?raw';
import sliderSource from './Slider.svelte?raw';
import checkboxSource from './Checkbox.svelte?raw';
import switchSource from './Switch.svelte?raw';
import popoverSource from './Popover.svelte?raw';
import textFieldSource from './TextField.svelte?raw';

describe('visual and semantic regression contract', () => {
  it('keeps the foundational app surfaces flat and token-driven', () => {
    expect(app + theme).not.toMatch(/(?:radial|linear)-gradient\(/);
    expect(palette).toContain('--control-height: 40px');
    expect(palette).toContain('--font-arabic:');
    expect(palette).toContain('--color-surface:');
    expect(theme).toContain('prefers-color-scheme: light');
    // Light-theme text-field rules must not paint over native slider or toggle controls.
    expect(theme).toContain("input:where(:not([type='checkbox']):not([type='radio']):not([type='range']):not([type='hidden']))");
    expect(theme).toContain("stroke='%23586b76'");
    expect(app).toContain('inset-inline-start: -13px');
    expect(app).toContain('inset-inline-end: 42px');
    // All workspaces must align text to the inline start in both LTR and Arabic RTL.
    expect(app).not.toContain('text-align: left;');
    expect(app).toContain('text-align: start;');
    expect(app).toContain('margin-inline-end: 7px;');
    expect(app).toContain('border-inline-end: 0;');
    // Search inputs deliberately clear their inner ring; the wrapper must provide it.
    expect(app).toContain('outline: 0;');
    expect(app).toContain('.global-search:focus-within, .search-field:focus-within { outline: 2px solid var(--accent-strong); outline-offset: 2px; }');
  });

  it('never leaves a visually native closed select in migrated or legacy controls', () => {
    expect(base).toContain('select:not([multiple])');
    expect(base).toContain('appearance: none');
    expect(base).toContain("dir='rtl'");
    expect(selectSource).toContain('role="listbox"');
    expect(selectSource).toContain('aria-activedescendant');
    expect(selectSource).toContain('function listKeys(');
  });

  it('keeps popover width limits consistent with viewport-aware placement', () => {
    expect(popoverSource).toContain('min-width: min(200px, 90vw)');
  });

  it('keeps invalid text-field borders visible in the system-light theme', () => {
    expect(theme).toContain(":root[data-theme='system'] input:where(");
    // Global theme selectors include :root[data-theme], so TextField needs an
    // additional local class to win regardless of CSS extraction order.
    expect(textFieldSource).toContain(".field input[aria-invalid='true'] { border-color: var(--danger); }");
    expect(textFieldSource).not.toContain("!important");
  });

  it('brands the slider across WebKit and Firefox while keeping real inputs', () => {
    expect(sliderSource).toContain('type="range"');
    expect(sliderSource).toContain('::-webkit-slider-thumb');
    expect(sliderSource).toContain('::-moz-range-thumb');
    expect(checkboxSource).toContain('type="checkbox"');
    expect(switchSource).toContain('role="switch"');
    expect(app).toContain("input:where(:not([type='checkbox']):not([type='radio']):not([type='range']):not([type='hidden']))");
    expect(sliderSource).toContain('border: 0; border-radius: 0');
    expect(base).toContain("input:where(:not([type='checkbox']):not([type='radio']):not([type='range']):not([type='hidden']))");
    // :where does not add specificity, so component error borders keep precedence.
    expect(app + base + theme).not.toContain("input:not([type='checkbox']):not([type='radio']):not([type='range']):not([type='hidden'])");
  });
});
