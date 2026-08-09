<script lang="ts">
  export let title = 'Something went wrong';
  export let message: string;
  export let onRetry: (() => void) | undefined = undefined;
</script>
<section class="state-panel error-state" role="alert" data-testid="error-state">
  <div class="state-icon">!</div>
  <div>
    <h2>{title}</h2>
    <p>{message}</p>
    {#if onRetry}<button class="button secondary" on:click={onRetry}>Try again</button>{/if}
  </div>
</section>
