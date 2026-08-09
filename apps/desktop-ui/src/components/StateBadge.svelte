<script lang="ts">
  import type { JobState } from '../types';
  export let state: JobState;
  const label: Record<JobState, string> = {
    queued: 'Queued', preparing: 'Preparing', downloading: 'Downloading', transcribing: 'Transcribing',
    processing: 'Processing', completed: 'Completed', failed: 'Failed', cancelled: 'Cancelled', interrupted: 'Interrupted'
  };
</script>
<span class="state-badge state-{state}">{label[state]}</span>
