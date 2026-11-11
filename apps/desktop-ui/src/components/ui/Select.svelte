<script lang="ts" context="module">
  export type SelectOption = { value: string; label: string; disabled?: boolean };
</script>

<script lang="ts">
  import { tick } from 'svelte';
  import { floatingPosition } from './floatingPosition';


  export let label: string;
  export let options: SelectOption[] = [];
  export let value = '';
  export let placeholder = 'Choose an option';
  export let disabled = false;
  export let required = false;
  export let invalid = false;
  export let name: string | undefined = undefined;
  export let id: string | undefined = undefined;
  export let description: string | undefined = undefined;
  export let onChange: ((next: string) => void) | undefined = undefined;

  // Each mounted control needs a unique DOM ID for its listbox and description.
  let trigger: HTMLButtonElement;
  let listbox: HTMLDivElement;
  let root: HTMLDivElement;
  let open = false;
  let active = -1;
  const uid = 'scriptotar-select-' + Math.random().toString(36).slice(2);
  $: controlId = id || uid;
  $: selected = options.find((option) => option.value === value);
  $: selectedIndex = options.findIndex((option) => option.value === value);
  $: if (disabled && open) close();

  function enabled(start: number, direction: number): number {
    if (!options.length) return -1;
    for (let offset = 0; offset < options.length; offset++) {
      const index = (start + direction * offset + options.length * (options.length + 1)) % options.length;
      if (!options[index].disabled) return index;
    }
    return -1;
  }

  async function show() {
    if (disabled || open) return;
    active = selectedIndex >= 0 && !options[selectedIndex].disabled ? selectedIndex : enabled(0, 1);
    open = true;
    await tick();
    if (!open) return;
    listbox?.focus();
    scrollActive();
  }

  function close(restoreFocus = false) {
    open = false;
    if (restoreFocus) void tick().then(() => trigger?.focus());
  }

  function choose(index: number) {
    const option = options[index];
    if (!option || option.disabled) return;
    value = option.value;
    onChange?.(value);
    close(true);
  }

  function scrollActive() {
    if (active < 0) return;
    const option = listbox?.querySelectorAll('[role="option"]')[active] as HTMLElement | undefined;
    option?.scrollIntoView?.({ block: 'nearest' });
  }

  function move(direction: number) {
    const index = enabled((active < 0 ? (direction > 0 ? 0 : options.length - 1) : active + direction), direction);
    if (index !== -1) { active = index; scrollActive(); }
  }

  function triggerKey(event: KeyboardEvent) {
    if (disabled) return;
    if (['ArrowDown', 'ArrowUp', 'Home', 'End'].includes(event.key)) {
      event.preventDefault();
      void show().then(() => {
        if (event.key === 'End') active = enabled(options.length - 1, -1);
        else if (event.key === 'Home') active = enabled(0, 1);
        else if (event.key === 'ArrowUp' && selectedIndex < 0) active = enabled(options.length - 1, -1);
        scrollActive();
      });
    }
  }

  function listKeys(event: KeyboardEvent) {
    switch (event.key) {
      case 'ArrowDown': event.preventDefault(); move(1); break;
      case 'ArrowUp': event.preventDefault(); move(-1); break;
      case 'Home': event.preventDefault(); active = enabled(0, 1); scrollActive(); break;
      case 'End': event.preventDefault(); active = enabled(options.length - 1, -1); scrollActive(); break;
      case 'Enter':
      case ' ': event.preventDefault(); choose(active); break;
      case 'Escape': event.preventDefault(); close(true); break;
      case 'Tab': close(); break;
      default: {
        if (event.key.length !== 1 || event.altKey || event.ctrlKey || event.metaKey) return;
        const match = options.findIndex((o) => !o.disabled && o.label.toLocaleLowerCase().startsWith(event.key.toLocaleLowerCase()));
        if (match >= 0) { active = match; scrollActive(); }
      }
    }
  }

  function outside(event: PointerEvent) {
    if (open && root && event.target instanceof Node && !root.contains(event.target)) close();
  }

  function blur(event: FocusEvent) {
    if (open && root && (!event.relatedTarget || !root.contains(event.relatedTarget as Node))) close();
  }
</script>

<svelte:window on:pointerdown={outside} />
<div class="field" bind:this={root} on:focusout={blur}>
  <label id={controlId + '-label'} for={controlId}>{label}{#if required}<span class="required" aria-hidden="true"> *</span>{/if}</label>
  <button type="button" id={controlId} role="combobox" class:invalid class="trigger"
    bind:this={trigger} {disabled} aria-haspopup="listbox" aria-expanded={open}
    aria-controls={open ? controlId + '-options' : undefined}
    aria-required={required} aria-invalid={invalid || undefined}
    aria-describedby={description ? controlId + '-help' : undefined}
    aria-labelledby={controlId + '-label'}
    on:click={() => open ? close(true) : void show()} on:keydown={triggerKey}>
    <span id={controlId + '-value'} class:placeholder={!selected}>{selected?.label || placeholder}</span>
    <svg aria-hidden="true" viewBox="0 0 16 16" width="16" height="16"><path d="m4 6 4 4 4-4" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round"/></svg>
  </button>
  {#if name}<input type="hidden" {name} {value} {disabled} />{/if}
  {#if description}<small id={controlId + '-help'}>{description}</small>{/if}
  {#if open}
    <div id={controlId + '-options'} class="options" role="listbox" tabindex="0"
      aria-label={label} aria-required={required} aria-invalid={invalid || undefined}
      aria-activedescendant={active >= 0 ? controlId + '-option-' + active : undefined}
      bind:this={listbox} use:floatingPosition={{ anchor: trigger, matchWidth: true }} on:keydown={listKeys}>
      {#each options as option, i (option.value)}
        <button id={controlId + '-option-' + i} type="button" role="option" tabindex="-1"
          class:highlighted={i === active} class:selected={option.value === value}
          aria-selected={option.value === value} disabled={option.disabled}
          on:mouseenter={() => { if (!option.disabled) active = i; }}
          on:click={() => choose(i)}>{option.label}</button>
      {/each}
    </div>
  {/if}
</div>

<style>
  .field { position: relative; display: grid; gap: 6px; min-width: 0; font: inherit; }
  label { color: var(--text); font-size: var(--text-sm); font-weight: 600; }
  .required { color: var(--danger); }
  .trigger { width: 100%; min-height: var(--control-height); display: flex; align-items: center; justify-content: space-between; gap: 12px; padding: 0 12px; border: 1px solid var(--border-strong); border-radius: var(--radius-sm); background: var(--surface-2); color: var(--text); text-align: start; font-size: var(--text-sm); }
  .trigger:hover:not(:disabled) { border-color: var(--color-text-muted); }
  .trigger.invalid { border-color: var(--danger); }
  .trigger svg { flex: 0 0 auto; color: var(--muted); }
  .placeholder { color: var(--muted); }
  .options { position: fixed; z-index: 40; min-width: 0; max-height: min(320px, 45vh); overflow-y: auto; display: grid; padding: 4px; margin: 0; border: 1px solid var(--border-strong); border-radius: var(--radius-md); background: var(--surface); box-shadow: var(--shadow-overlay); outline: none; }
  .options:focus-visible { outline: 2px solid var(--accent-strong); outline-offset: 2px; }
  .options button { display: block; min-height: 40px; width: 100%; border: 0; border-radius: var(--radius-sm); background: transparent; color: var(--text); text-align: start; padding: 8px 10px; font-size: var(--text-sm); cursor: pointer; }
  .options button.highlighted, .options button:hover:not(:disabled) { background: var(--surface-3); }
  .options button.selected { color: var(--accent-strong); font-weight: 700; }
  .options button:disabled { opacity: .5; cursor: not-allowed; }
  small { color: var(--muted); font-size: var(--text-xs); }
</style>
