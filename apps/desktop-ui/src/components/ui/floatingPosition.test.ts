import { describe, expect, it, vi } from 'vitest';
import { floatingPosition } from './floatingPosition';

describe('floating control placement', () => {
  it('escapes ancestor overflow and opens above the trigger when bottom space is insufficient', () => {
    const anchor = document.createElement('button');
    const panel = document.createElement('div');
    document.body.append(anchor, panel);
    vi.spyOn(anchor, 'getBoundingClientRect').mockReturnValue({
      top: 710, bottom: 750, left: 40, right: 260, width: 220, height: 40, x: 40, y: 710,
      toJSON: () => ({})
    });
    Object.defineProperty(panel, 'scrollHeight', { configurable: true, value: 200 });
    const action = floatingPosition(panel, { anchor, matchWidth: true });
    expect(panel.style.position).toBe('fixed');
    expect(panel.style.width).toBe('220px');
    expect(Number.parseFloat(panel.style.top)).toBeLessThan(710);
    action.destroy();
    anchor.remove(); panel.remove();
  });

  it('uses the logical inline-start anchor in RTL without exceeding the viewport', () => {
    const anchor = document.createElement('button');
    const panel = document.createElement('div');
    anchor.style.direction = 'rtl';
    document.body.append(anchor, panel);
    vi.spyOn(anchor, 'getBoundingClientRect').mockReturnValue({
      top: 20, bottom: 60, left: 100, right: 300, width: 200, height: 40, x: 100, y: 20,
      toJSON: () => ({})
    });
    const action = floatingPosition(panel, { anchor, matchWidth: true });
    expect(panel.style.left).toBe('100px');
    action.destroy();
    anchor.remove(); panel.remove();
  });
});
