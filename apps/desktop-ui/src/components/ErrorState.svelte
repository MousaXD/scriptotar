<script lang="ts">
  import { translator } from '../i18n/translate';
  export let title: string | undefined = undefined;
  export let message: string;
  export let onRetry: (() => void) | undefined = undefined;
</script>
<section class="state-panel error-state" role="alert" data-testid="error-state" data-i18n-ignore>
  <div class="state-icon">!</div>
  <div>
    <h2>{title ?? $translator('error.genericTitle')}</h2>
    <p>{message}</p>
    {#if onRetry}<button class="button secondary" on:click={onRetry}>{$translator('error.tryAgain')}</button>{/if}
  </div>
</section>
