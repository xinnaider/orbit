<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { Terminal } from '@xterm/xterm';
  import { FitAddon } from '@xterm/addon-fit';
  import { ptyCreate, ptyWrite, ptyResize, ptyKill, onPtyOutput } from '../lib/tauri/terminal';
  import '@xterm/xterm/css/xterm.css';

  let {
    sessionId = 0,
    terminalId = '',
  }: {
    sessionId?: number;
    terminalId?: string;
  } = $props();

  let container: HTMLDivElement | undefined = $state();
  let terminal: Terminal | undefined = $state();
  let fitAddon: FitAddon | undefined = $state();
  let unlisten: (() => void) | undefined = $state();
  let resizeObserver: ResizeObserver | undefined = $state();

  let loading = $state(false);
  let error = $state('');
  // The numeric PTY id used for all pty* calls.
  let numericId = $state(0);
  // Whether we spawned the PTY ourselves (and must kill it on destroy).
  let ownedPty = $state(false);

  /** Derive a stable numeric id from a string by summing char codes. */
  function hashString(s: string): number {
    let h = 0;
    for (let i = 0; i < s.length; i++) {
      h = (Math.imul(31, h) + s.charCodeAt(i)) | 0;
    }
    // Ensure positive, non-zero
    return Math.abs(h) || Date.now();
  }

  function resolveNumericId(): number {
    if (sessionId > 0) return sessionId;
    if (terminalId) return hashString(terminalId);
    return Date.now();
  }

  async function spawnPty(term: Terminal, fit: FitAddon): Promise<void> {
    const isWindows = navigator.platform.startsWith('Win');
    const shell = isWindows ? 'powershell.exe' : '/bin/bash';
    const cwd = '.';

    // Get current terminal dimensions before spawning
    fit.fit();
    const rows = term.rows || 24;
    const cols = term.cols || 80;

    await ptyCreate(numericId, shell, [], cwd, [], rows, cols);
  }

  async function initTerminal(): Promise<void> {
    if (!container) return;

    loading = true;
    error = '';

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

    numericId = resolveNumericId();

    // Auto-spawn a shell when no external sessionId drives the PTY
    if (sessionId <= 0) {
      try {
        await spawnPty(term, fit);
        ownedPty = true;
      } catch (e) {
        error = e instanceof Error ? e.message : String(e);
        loading = false;
        term.dispose();
        return;
      }
    }

    loading = false;

    term.onData(async (data) => {
      try {
        await ptyWrite(numericId, data);
      } catch (e) {
        console.error('pty write error:', e);
      }
    });

    term.onResize(async ({ cols, rows }) => {
      try {
        await ptyResize(numericId, rows, cols);
      } catch (e) {
        console.error('pty resize error:', e);
      }
    });

    unlisten = await onPtyOutput(({ sessionId: sid, data, eof }) => {
      if (sid !== numericId) return;
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
  }

  onMount(async () => {
    await initTerminal();
  });

  onDestroy(() => {
    resizeObserver?.disconnect();
    unlisten?.();
    terminal?.dispose();
    if (ownedPty) {
      ptyKill(numericId).catch((e) => console.error('pty kill error:', e));
    }
  });
</script>

{#if loading}
  <div class="terminal-overlay">
    <span class="terminal-status">Starting shell…</span>
  </div>
{:else if error}
  <div class="terminal-overlay">
    <span class="terminal-status error">{error}</span>
    <button
      class="retry-btn"
      onclick={() => {
        terminal?.dispose();
        terminal = undefined;
        initTerminal();
      }}>Retry</button
    >
  </div>
{/if}

<div class="terminal-panel" bind:this={container} class:hidden={!!error || loading}></div>

<style>
  .terminal-panel {
    width: 100%;
    height: 100%;
    min-height: 200px;
  }

  .terminal-panel.hidden {
    display: none;
  }

  .terminal-panel :global(.xterm) {
    height: 100%;
  }

  .terminal-overlay {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    width: 100%;
    height: 100%;
    min-height: 200px;
    gap: 12px;
    background: #1a1a1a;
    color: #d4d4d4;
  }

  .terminal-status {
    font-family: Consolas, 'Courier New', monospace;
    font-size: 13px;
    opacity: 0.7;
  }

  .terminal-status.error {
    color: #f48771;
    opacity: 1;
  }

  .retry-btn {
    padding: 4px 14px;
    background: #2a2a2a;
    border: 1px solid #444;
    border-radius: 4px;
    color: #d4d4d4;
    font-size: 12px;
    cursor: pointer;
  }

  .retry-btn:hover {
    background: #333;
  }
</style>
