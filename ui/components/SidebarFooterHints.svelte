<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { formatModChord, splitPaneHint } from '../lib/shortcuts';

  function buildHints(): string[] {
    return [
      'Close → tray',
      'Tray · reopen app',
      `${formatModChord('I')} · inspector`,
      `${splitPaneHint()} · split pane`,
      'Drag · assign pane',
      'Ctx menu · mute',
    ];
  }

  const INTERVAL_MS = 4500;

  let hints = buildHints();
  let index = 0;
  let hint = hints[0];
  let timer: ReturnType<typeof setInterval> | undefined;

  onMount(() => {
    hints = buildHints();
    hint = hints[0];
    timer = setInterval(() => {
      index = (index + 1) % hints.length;
      hint = hints[index];
    }, INTERVAL_MS);
  });

  onDestroy(() => {
    if (timer) clearInterval(timer);
  });
</script>

<span class="footer-hint" title={hints.join(' · ')}>{hint}</span>

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
