<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { Terminal } from '@xterm/xterm';
  import { FitAddon } from '@xterm/addon-fit';
  import { ptyWrite, ptyResize, onPtyOutput } from '../lib/tauri/terminal';
  import '@xterm/xterm/css/xterm.css';

  let {
    sessionId,
  }: {
    sessionId: number;
  } = $props();

  let container: HTMLDivElement | undefined = $state();
  let terminal: Terminal | undefined = $state();
  let fitAddon: FitAddon | undefined = $state();
  let unlisten: (() => void) | undefined = $state();
  let resizeObserver: ResizeObserver | undefined = $state();

  onMount(async () => {
    if (!container) return;

    const term = new Terminal({
      cursorBlink: true,
      fontSize: 13,
      fontFamily: 'Consolas, "Courier New", monospace',
      scrollback: 5000,
      theme: {
        background: '#1a1a1a',
        foreground: '#d4d4d4',
        cursor: '#d4d4d4',
      },
    });

    const fit = new FitAddon();
    term.loadAddon(fit);

    term.open(container);

    // Delay fit to ensure container has dimensions
    requestAnimationFrame(() => fit.fit());

    term.onData(async (data) => {
      try {
        await ptyWrite(sessionId, data);
      } catch (e) {
        console.error('pty write error:', e);
      }
    });

    term.onResize(async ({ cols, rows }) => {
      try {
        await ptyResize(sessionId, rows, cols);
      } catch (e) {
        console.error('pty resize error:', e);
      }
    });

    unlisten = await onPtyOutput(({ sessionId: sid, data, eof }) => {
      if (sid !== sessionId) return;
      if (eof) {
        term?.writeln('\r\n[process exited]');
        return;
      }
      term?.write(data);
    });

    terminal = term;
    fitAddon = fit;

    let resizeTimer: ReturnType<typeof setTimeout>;
    resizeObserver = new ResizeObserver(() => {
      clearTimeout(resizeTimer);
      resizeTimer = setTimeout(() => fit?.fit(), 50);
    });
    resizeObserver.observe(container);
  });

  onDestroy(() => {
    resizeObserver?.disconnect();
    unlisten?.();
    terminal?.dispose();
  });
</script>

<div class="terminal-panel" bind:this={container}></div>

<style>
  .terminal-panel {
    width: 100%;
    height: 100%;
    min-height: 200px;
  }

  .terminal-panel :global(.xterm) {
    height: 100%;
  }
</style>
