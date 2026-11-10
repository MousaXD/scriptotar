<script lang="ts">
  import { tick } from 'svelte';
  export let label: string;
  export let triggerLabel = 'More options';
  export let disabled = false;
  let root: HTMLDivElement;
  let trigger: HTMLButtonElement;
  let panel: HTMLDivElement;
  let open = false;
  function close(restore = false) {
    open = false;
    if (restore) void tick().then(() => trigger?.focus());
  }
  async function toggle() {
    if (disabled) return;
    if (open) { close(true); return; }
    open = true;
    await tick();
    panel?.focus();
  }
  function key(event: KeyboardEvent) {
    if (event.key === 'Escape' && open) { event.preventDefault(); close(true); }
  }
  function outside(event: PointerEvent) {
    if (open && root && event.target instanceof Node && !root.contains(event.target)) close();
  }
  function blur(event: FocusEvent) {
    if (open && (!event.relatedTarget || !root.contains(event.relatedTarget as Node))) close();
  }
</script>

<svelte:window on:pointerdown={outside} on:keydown={key} />
<div class="popover" bind:this={root} on:focusout={blur}>
  <button type="button" bind:this={trigger} {disabled} aria-haspopup="dialog"
    aria-expanded={open} on:click={toggle}><slot name="trigger">{triggerLabel}</slot></button>
  {#if open}
    <div class="panel" role="dialog" aria-label={label} tabindex="-1" bind:this={panel}>
      <slot />
    </div>
  {/if}
</div>

<style>
  .popover { position: relative; display: inline-block; }
  button { min-height: var(--control-height); padding: 0 12px; border: 1px solid var(--border-strong); border-radius: var(--radius-sm); color: var(--text); background: var(--surface-2); font-size: var(--text-sm); cursor: pointer; }
  button:hover:not(:disabled) { background: var(--surface-3); }
  button:disabled { opacity: .55; cursor: not-allowed; }
  button:focus-visible { outline: 2px solid var(--accent-strong); outline-offset: 2px; }
  .panel { position: absolute; z-index: 45; inset-block-start: calc(100% + 4px); inset-inline-end: 0; width: max-content; max-width: min(440px, 90vw); min-width: 200px; padding: 12px; border: 1px solid var(--border-strong); border-radius: var(--radius-md); background: var(--surface); box-shadow: var(--shadow-overlay); color: var(--text); }
  .panel:focus-visible { outline: 2px solid var(--accent-strong); outline-offset: 2px; }
</style>
