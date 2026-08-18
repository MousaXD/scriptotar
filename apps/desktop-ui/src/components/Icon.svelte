<script lang="ts">
  export type IconName = 'home' | 'research' | 'queue' | 'transcript' | 'sparkles' | 'library' | 'settings' | 'search' | 'globe' | 'panel' | 'command' | 'folder' | 'plus' | 'activity';
  export let name: IconName;
  export let size = 18;
  export let strokeWidth = 1.8;
</script>

<svg class="icon" width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width={strokeWidth} stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
  {#if name === 'home'}
    <path d="M3 11.5 12 4l9 7.5"/><path d="M5.5 10.5V20h13v-9.5"/><path d="M9.5 20v-5h5v5"/>
  {:else if name === 'research'}
    <circle cx="11" cy="11" r="6"/><path d="m16 16 4.5 4.5"/><path d="M8.5 11h5M11 8.5v5"/>
  {:else if name === 'queue'}
    <path d="M5 6h14M5 12h10M5 18h6"/><path d="m16 15 3 3-3 3"/>
  {:else if name === 'transcript'}
    <path d="M6 3h9l3 3v15H6z"/><path d="M15 3v4h4M9 11h6M9 15h6M9 19h4"/>
  {:else if name === 'sparkles'}
    <path d="m12 3 1.2 3.2L16.5 7.5l-3.3 1.3L12 12l-1.2-3.2-3.3-1.3 3.3-1.3z"/><path d="m18.5 13 .8 2.1 2.2.9-2.2.8-.8 2.2-.8-2.2-2.2-.8 2.2-.9zM5.5 14l.7 1.7 1.8.7-1.8.7-.7 1.8-.7-1.8-1.8-.7 1.8-.7z"/>
  {:else if name === 'library'}
    <path d="M4 5.5h4v14H4zM10 5.5h4v14h-4zM16 5.5h4v14h-4z"/><path d="M4 9h4M10 15h4M16 11h4"/>
  {:else if name === 'settings'}
    <circle cx="12" cy="12" r="3"/><path d="M19 12a7 7 0 0 0-.1-1l2-1.5-2-3.5-2.4 1a7 7 0 0 0-1.7-1L14.5 3h-5L9.2 6a7 7 0 0 0-1.7 1L5.1 6 3 9.5 5.1 11a7 7 0 0 0 0 2L3 14.5 5.1 18l2.4-1a7 7 0 0 0 1.7 1l.3 3h5l.3-3a7 7 0 0 0 1.7-1l2.4 1 2.1-3.5-2.1-1.5c.1-.3.1-.7.1-1z"/>
  {:else if name === 'search'}
    <circle cx="10.5" cy="10.5" r="6.5"/><path d="m15.5 15.5 5 5"/>
  {:else if name === 'globe'}
    <circle cx="12" cy="12" r="9"/><path d="M3 12h18M12 3a14 14 0 0 1 0 18M12 3a14 14 0 0 0 0 18"/>
  {:else if name === 'panel'}
    <rect x="3" y="4" width="18" height="16" rx="2"/><path d="M9 4v16"/>
  {:else if name === 'command'}
    <path d="M9 7V5a2 2 0 1 0-2 2h10a2 2 0 1 0-2-2v14a2 2 0 1 0 2-2H7a2 2 0 1 0 2 2z"/>
  {:else if name === 'folder'}
    <path d="M3.5 7.5h6l2-2h9v13h-17z"/>
  {:else if name === 'plus'}
    <path d="M12 5v14M5 12h14"/>
  {:else if name === 'activity'}
    <path d="M3 12h4l2.2-5 4.2 10 2.2-5H21"/>
  {/if}
</svg>

<style>
  .icon { display: block; flex: 0 0 auto; }
</style>
