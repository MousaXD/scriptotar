<script lang="ts">
  export let label: string;
  export let checked = false;
  export let disabled = false;
  export let name: string | undefined = undefined;
  export let description: string | undefined = undefined;
  export let onChange: ((checked: boolean) => void) | undefined = undefined;
  function change(event: Event) { checked = (event.currentTarget as HTMLInputElement).checked; onChange?.(checked); }
</script>

<label class="control">
  <span class="copy"><span class="label">{label}</span>{#if description}<small>{description}</small>{/if}</span>
  <input type="checkbox" role="switch" {name} {disabled} bind:checked on:change={change} />
</label>

<style>
  .control { display: flex; align-items: center; justify-content: space-between; gap: 16px; min-height: 40px; cursor: pointer; }
  .control:has(input:disabled) { cursor: not-allowed; opacity: .55; }
  .copy { display: grid; gap: 3px; min-width: 0; }
  .label { color: var(--text); font-size: var(--text-sm); font-weight: 600; }
  small { color: var(--muted); font-size: var(--text-xs); line-height: var(--line-compact); }
  input { appearance: none; -webkit-appearance: none; position: relative; flex: 0 0 42px; width: 42px; height: 24px; margin: 0; border: 1px solid var(--border-strong); border-radius: var(--radius-pill); background: var(--surface-3); cursor: inherit; transition: background-color var(--motion-fast) var(--ease-standard); }
  input::after { content: ''; position: absolute; width: 18px; height: 18px; inset-inline-start: 2px; inset-block-start: 2px; border-radius: 50%; background: var(--text); transition: transform var(--motion-fast) var(--ease-standard); }
  input:checked { background: var(--accent-strong); border-color: var(--accent-strong); }
  input:checked::after { background: var(--accent-ink); transform: translateX(18px); }
  :global([dir='rtl']) input:checked::after { transform: translateX(-18px); }
  input:focus-visible { outline: 2px solid var(--accent-strong); outline-offset: 3px; }
  @media (prefers-reduced-motion: reduce) { input, input::after { transition: none; } }
</style>
