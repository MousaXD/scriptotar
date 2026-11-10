<script lang="ts">
  export type RadioOption = { value: string; label: string; disabled?: boolean };
  export let label: string;
  export let options: RadioOption[] = [];
  export let value = '';
  export let disabled = false;
  export let name: string | undefined = undefined;
  export let onChange: ((value: string) => void) | undefined = undefined;
  function change(event: Event) { value = (event.currentTarget as HTMLInputElement).value; onChange?.(value); }
</script>

<fieldset {disabled}>
  <legend>{label}</legend>
  <div class="group">
    {#each options as option (option.value)}
      <label class:unavailable={option.disabled}>
        <input type="radio" {name} value={option.value} bind:group={value}
          disabled={disabled || option.disabled} on:change={change} />
        <span>{option.label}</span>
      </label>
    {/each}
  </div>
</fieldset>

<style>
  fieldset { padding: 0; margin: 0; border: 0; min-width: 0; }
  legend { color: var(--text); font-size: var(--text-sm); font-weight: 600; margin-bottom: 8px; }
  .group { display: flex; flex-wrap: wrap; gap: 8px 16px; }
  label { min-height: 40px; display: flex; gap: 8px; align-items: center; cursor: pointer; font-size: var(--text-sm); }
  label.unavailable, fieldset:disabled label { opacity: .55; cursor: not-allowed; }
  input { appearance: none; -webkit-appearance: none; flex: 0 0 20px; width: 20px; height: 20px; margin: 0; background: var(--surface-2); border: 1.5px solid var(--border-strong); border-radius: 50%; cursor: inherit; }
  input:checked { border: 6px solid var(--accent-strong); background: var(--accent-ink); }
  input:focus-visible { outline: 2px solid var(--accent-strong); outline-offset: 3px; }
</style>
