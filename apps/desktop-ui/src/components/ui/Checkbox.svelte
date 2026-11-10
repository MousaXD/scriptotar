<script lang="ts">
  export let label: string;
  export let checked = false;
  export let disabled = false;
  export let name: string | undefined = undefined;
  export let value = 'on';
  export let description: string | undefined = undefined;
  export let onChange: ((checked: boolean) => void) | undefined = undefined;
  function change(event: Event) { checked = (event.currentTarget as HTMLInputElement).checked; onChange?.(checked); }
</script>

<label class="control">
  <input type="checkbox" {name} {value} {disabled} bind:checked on:change={change} />
  <span class="copy"><span class="label">{label}</span>{#if description}<small>{description}</small>{/if}</span>
</label>

<style>
  .control { display: flex; align-items: flex-start; gap: 10px; min-height: 40px; padding-block: 7px; cursor: pointer; }
  .control:has(input:disabled) { cursor: not-allowed; opacity: .55; }
  input { appearance: none; -webkit-appearance: none; position: relative; flex: 0 0 20px; width: 20px; height: 20px; margin: 0; border: 1.5px solid var(--border-strong); border-radius: 5px; background: var(--surface-2); cursor: inherit; }
  input:checked { background: var(--accent); border-color: var(--accent-strong); }
  input:checked::after { content: ''; position: absolute; width: 5px; height: 9px; left: 6px; top: 2px; border: solid var(--accent-ink); border-width: 0 2px 2px 0; transform: rotate(45deg); }
  input:focus-visible { outline: 2px solid var(--accent-strong); outline-offset: 3px; }
  .copy { display: grid; gap: 3px; }
  .label { font-size: var(--text-sm); color: var(--text); font-weight: 600; }
  small { color: var(--muted); font-size: var(--text-xs); line-height: var(--line-compact); }
</style>
