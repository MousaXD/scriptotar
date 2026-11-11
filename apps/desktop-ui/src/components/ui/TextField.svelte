<script lang="ts">
  import type { HTMLInputAttributes } from 'svelte/elements';
  export let label: string;
  export let value = '';
  export let type: 'text' | 'search' | 'password' | 'url' | 'email' = 'text';
  export let name: string | undefined = undefined;
  export let id: string | undefined = undefined;
  export let placeholder = '';
  export let description: string | undefined = undefined;
  export let autocomplete: HTMLInputAttributes['autocomplete'] = undefined;
  export let disabled = false;
  export let required = false;
  export let invalid = false;
  export let onInput: ((value: string) => void) | undefined = undefined;
  const uid = 'scriptotar-input-' + Math.random().toString(36).slice(2);
  $: controlId = id || uid;
  function input(event: Event) { value = (event.currentTarget as HTMLInputElement).value; onInput?.(value); }
</script>

<div class="field">
  <label for={controlId}>{label}</label>
  <input id={controlId} {type} {name} {placeholder} {disabled} {required} {autocomplete}
    bind:value aria-invalid={invalid || undefined}
    aria-describedby={description ? controlId + '-help' : undefined} on:input={input} />
  {#if description}<small id={controlId + '-help'}>{description}</small>{/if}
</div>

<style>
  .field { display: grid; gap: 6px; min-width: 0; }
  label { color: var(--text); font-size: var(--text-sm); font-weight: 600; }
  input { display: block; width: 100%; min-width: 0; min-height: var(--control-height); padding: 8px 12px; border: 1px solid var(--border-strong); border-radius: var(--radius-sm); background: var(--surface-2); color: var(--text); font-size: var(--text-sm); }
  input[aria-invalid='true'] { border-color: var(--danger); }
  /* URLs, email addresses and credentials remain legible in an Arabic interface. */
  input[type='url'], input[type='email'], input[type='password'] { direction: ltr; text-align: start; }
  input:focus-visible { outline: 2px solid var(--accent-strong); outline-offset: 2px; }
  small { font-size: var(--text-xs); color: var(--muted); }
</style>
