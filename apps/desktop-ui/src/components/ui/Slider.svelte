<script lang="ts">
  export let label: string;
  export let value = 0;
  export let min = 0;
  export let max = 100;
  export let step = 1;
  export let disabled = false;
  export let name: string | undefined = undefined;
  export let id: string | undefined = undefined;
  export let format: (value: number) => string = (number) => String(number);
  export let onChange: ((value: number) => void) | undefined = undefined;
  const uid = 'scriptotar-range-' + Math.random().toString(36).slice(2);
  $: controlId = id || uid;
  $: percent = max > min ? Math.max(0, Math.min(100, (value - min) * 100 / (max - min))) : 0;

  function update(event: Event) {
    value = Number((event.currentTarget as HTMLInputElement).value);
    onChange?.(value);
  }
</script>

<div class="range-field">
  <div class="heading"><label for={controlId}>{label}</label><output for={controlId}>{format(value)}</output></div>
  <div class="track-wrapper">
    <div class="track" aria-hidden="true"><div class="fill" style:width={percent + '%'}></div></div>
    <input id={controlId} type="range" {name} {min} {max} {step} {disabled}
      bind:value on:input={update} aria-valuetext={format(value)} />
  </div>
</div>

<style>
  .range-field { display: grid; gap: 10px; min-width: 0; }
  .heading { display: flex; align-items: center; justify-content: space-between; gap: 12px; }
  label { font-size: var(--text-sm); color: var(--text); font-weight: 600; }
  output { font: 600 var(--text-sm) var(--font-technical); color: var(--muted); }
  .track-wrapper { position: relative; height: 40px; display: flex; align-items: center; }
  .track { position: absolute; inset-inline: 0; height: 5px; border-radius: var(--radius-pill); background: var(--surface-3); overflow: hidden; }
  .fill { height: 100%; background: var(--accent-strong); }
  input { position: absolute; inset: 0; width: 100%; height: 40px; min-height: 0; margin: 0; padding: 0; border: 0; border-radius: 0; background: transparent; cursor: pointer; appearance: none; -webkit-appearance: none; }
  input::-webkit-slider-runnable-track { background: transparent; height: 5px; }
  input::-moz-range-track { background: transparent; height: 5px; }
  input::-webkit-slider-thumb { appearance: none; -webkit-appearance: none; height: 17px; width: 17px; margin-top: -6px; border: 3px solid var(--surface); border-radius: 50%; background: var(--accent-strong); box-shadow: 0 0 0 1px var(--border-strong); }
  input::-moz-range-thumb { height: 11px; width: 11px; border: 3px solid var(--surface); border-radius: 50%; background: var(--accent-strong); box-shadow: 0 0 0 1px var(--border-strong); }
  input:disabled { cursor: not-allowed; opacity: .55; }
  input:focus-visible { outline: 2px solid var(--accent-strong); outline-offset: 3px; }
</style>
