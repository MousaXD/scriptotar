<script lang="ts">
  import type { JobState } from '../types';
  import { translator } from '../i18n/translate';
  import type { ShellMessageKey } from '../i18n/messages';

  export let state: JobState;

  // Keep the semantic state key unchanged for CSS and automation, localize only the visible badge.
  const stateKeys = {
    queued: 'job.queued',
    preparing: 'job.preparing',
    downloading: 'job.downloading',
    transcribing: 'job.transcribing',
    processing: 'job.processing',
    completed: 'job.completed',
    failed: 'job.failed',
    cancelled: 'job.cancelled',
    interrupted: 'job.interrupted'
  } satisfies Record<JobState, ShellMessageKey>;
</script>

<span class="state-badge state-{state}" data-i18n-ignore>{$translator(stateKeys[state])}</span>
