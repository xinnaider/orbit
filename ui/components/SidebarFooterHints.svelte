<script lang="ts">
  import { onDestroy, onMount } from 'svelte';

  const HINTS = [
    'Close → tray',
    'Tray · reopen app',
    '⌘I · inspector',
    '⌘\\ · split pane',
    'Drag · assign pane',
    'Ctx menu · mute',
  ] as const;

  const INTERVAL_MS = 4500;

  let index = 0;
  let hint = HINTS[0];
  let timer: ReturnType<typeof setInterval> | undefined;

  onMount(() => {
    timer = setInterval(() => {
      index = (index + 1) % HINTS.length;
      hint = HINTS[index];
    }, INTERVAL_MS);
  });

  onDestroy(() => {
    if (timer) clearInterval(timer);
  });
</script>

<span class="footer-hint" title={HINTS.join(' · ')}>{hint}</span>

<style>
  .footer-hint {
    display: block;
    min-width: 0;
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 9px;
    color: var(--t3);
    line-height: 1.35;
    letter-spacing: 0.02em;
  }
</style>
