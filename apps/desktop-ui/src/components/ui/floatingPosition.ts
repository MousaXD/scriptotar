/** Position a small popover against its trigger without being clipped by scroll containers.
 * The element stays in the Svelte DOM; only its CSS position changes.
 * Keyboard focus and pointer containment therefore remain stable.
 */
export type FloatingOptions = {
  anchor: HTMLElement;
  align?: 'start' | 'end';
  matchWidth?: boolean;
  maxHeight?: number;
};

const MARGIN = 8;
const GAP = 4;

export function floatingPosition(node: HTMLElement, initial: FloatingOptions) {
  let options = initial;
  function position() {
    const { anchor, align = 'start', matchWidth = false, maxHeight = 320 } = options;
    if (!anchor?.isConnected) return;
    const rect = anchor.getBoundingClientRect();
    const viewportWidth = window.innerWidth;
    const viewportHeight = window.innerHeight;
    const maxWidth = Math.max(0, viewportWidth - MARGIN * 2);
    const intrinsicWidth = Math.max(node.scrollWidth, rect.width, 160);
    const width = Math.min(matchWidth ? (rect.width || intrinsicWidth) : intrinsicWidth, maxWidth);
    const availableBelow = Math.max(0, viewportHeight - rect.bottom - GAP - MARGIN);
    const availableAbove = Math.max(0, rect.top - GAP - MARGIN);
    const flip = availableBelow < Math.min(160, node.scrollHeight || 160) && availableAbove > availableBelow;
    const available = flip ? availableAbove : availableBelow;
    const height = Math.min(Math.max(0, available), maxHeight);
    const effectiveHeight = Math.min(node.scrollHeight || height, height);
    const rtl = getComputedStyle(anchor).direction === 'rtl';
    const start = rtl ? rect.right - width : rect.left;
    const end = rtl ? rect.left : rect.right - width;
    const x = align === 'start' ? start : end;

    node.style.position = 'fixed';
    node.style.inset = 'auto';
    node.style.left = Math.max(MARGIN, Math.min(x, viewportWidth - width - MARGIN)) + 'px';
    node.style.top = (flip
      ? Math.max(MARGIN, rect.top - GAP - effectiveHeight)
      : Math.max(MARGIN, rect.bottom + GAP)) + 'px';
    node.style.width = width + 'px';
    node.style.maxHeight = height + 'px';
    node.style.overflowY = 'auto';
  }

  position();
  window.addEventListener('resize', position);
  // Capture scrolls from any nested workspace pane, not only from the page.
  window.addEventListener('scroll', position, true);
  return {
    update(next: FloatingOptions) { options = next; position(); },
    destroy() {
      window.removeEventListener('resize', position);
      window.removeEventListener('scroll', position, true);
    }
  };
}
