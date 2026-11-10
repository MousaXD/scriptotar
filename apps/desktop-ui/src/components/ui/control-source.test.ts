import { describe, expect, it } from 'vitest';
import palette from '../../design/tokens.css?raw';
import base from '../../design/base.css?raw';
import app from '../../app.css?raw';
import theme from '../../theme.css?raw';
import selectSource from './Select.svelte?raw';
import sliderSource from './Slider.svelte?raw';
import checkboxSource from './Checkbox.svelte?raw';
import switchSource from './Switch.svelte?raw';

describe('visual and semantic regression contract', () => {
  it('keeps the foundational app surfaces flat and token-driven', () => {
    expect(app + theme).not.toMatch(/(?:radial|linear)-gradient\(/);
    expect(palette).toContain('--control-height: 40px');
    expect(palette).toContain('--font-arabic:');
    expect(palette).toContain('--color-surface:');
    expect(theme).toContain('prefers-color-scheme: light');
  });

  it('never leaves a visually native closed select in migrated or legacy controls', () => {
    expect(base).toContain('select:not([multiple])');
    expect(base).toContain('appearance: none');
    expect(base).toContain("dir='rtl'");
    expect(selectSource).toContain('role="listbox"');
    expect(selectSource).toContain('aria-activedescendant');
    expect(selectSource).toContain('function listKeys(');
  });

  it('brands the slider across WebKit and Firefox while keeping real inputs', () => {
    expect(sliderSource).toContain('type="range"');
    expect(sliderSource).toContain('::-webkit-slider-thumb');
    expect(sliderSource).toContain('::-moz-range-thumb');
    expect(checkboxSource).toContain('type="checkbox"');
    expect(switchSource).toContain('role="switch"');
    expect(app).toContain("input:not([type='checkbox']):not([type='radio']):not([type='range']):not([type='hidden'])");
    expect(sliderSource).toContain('border: 0; border-radius: 0');
  });
});
