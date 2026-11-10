<script lang="ts">
  import { translator } from '../i18n/translate';
</script>

<!-- This is a workspace placeholder, not a full-screen splash. Keep the chrome interactive. -->
<section class="workspace-skeleton" role="status" aria-label={$translator('boot.loading')} aria-busy="true">
  <span class="sr-only">{$translator('boot.loading')}</span>
  <div class="heading-skeleton" aria-hidden="true">
    <div class="skeleton title"></div>
    <div class="skeleton subtitle"></div>
  </div>
  <div class="metric-skeletons" aria-hidden="true">
    {#each [0, 1, 2, 3] as item (item)}
      <div class="metric-skeleton"><div class="skeleton label"></div><div class="skeleton value"></div></div>
    {/each}
  </div>
  <div class="panel-skeletons" aria-hidden="true">
    <div class="panel-skeleton"><div class="skeleton label"></div><div class="skeleton row"></div><div class="skeleton row short"></div></div>
    <div class="panel-skeleton"><div class="skeleton label"></div><div class="skeleton row"></div><div class="skeleton row short"></div></div>
  </div>
</section>

<style>
  .workspace-skeleton { display: grid; gap: var(--space-4, 16px); width: 100%; }
  .heading-skeleton { display: grid; gap: 12px; margin-block-end: 8px; }
  .skeleton { border-radius: var(--radius-sm, 6px); background: var(--color-surface-raised, #17222d); }
  .title { height: 30px; width: min(260px, 70%); }
  .subtitle { height: 12px; width: min(370px, 85%); }
  .metric-skeletons { display: grid; grid-template-columns: repeat(4, minmax(0, 1fr)); gap: 12px; }
  .metric-skeleton, .panel-skeleton { display: grid; align-content: start; gap: 18px; min-width: 0; padding: 20px; border: 1px solid var(--color-border, #22303c); border-radius: var(--radius-md, 10px); background: var(--color-surface, #0e151c); }
  .metric-skeleton { min-height: 118px; }
  .label { height: 11px; width: 42%; }
  .value { height: 27px; width: 65%; }
  .panel-skeletons { display: grid; grid-template-columns: 1.3fr 1fr; gap: 12px; }
  .panel-skeleton { min-height: 200px; }
  .row { height: 15px; width: 94%; }
  .row.short { width: 62%; }
  .sr-only { position: absolute; width: 1px; height: 1px; padding: 0; margin: -1px; overflow: hidden; clip: rect(0, 0, 0, 0); white-space: nowrap; border: 0; }
  @media (max-width: 900px) { .metric-skeletons { grid-template-columns: repeat(2, minmax(0, 1fr)); } .panel-skeletons { grid-template-columns: 1fr; } }
  @media (max-width: 540px) { .metric-skeletons { grid-template-columns: 1fr; } }
</style>
