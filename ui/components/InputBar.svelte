<script lang="ts">
  import { onMount, onDestroy, tick } from 'svelte';
  import {
    sendSessionMessage,
    getSlashCommands,
    listProjectFiles,
    updateSessionModel,
    updateSessionEffort,
  } from '../lib/tauri';
  import { sessions, updateSessionState } from '../lib/stores/sessions';
  import { journal } from '../lib/stores/journal';
  import { pendingMessages } from '../lib/stores/journal';
  import { modelDisplayName } from '../lib/status';
  import { sessionEffort } from '../lib/stores/ui';
  import type { SlashCommand } from '../lib/types';
  import type { JournalEntry } from '../lib/types';

  export let sessionId: number;
  export let cwd: string = '';
  export let sessionStatus: string = '';
  export let provider: string = 'claude-code';

  let text = '';
  let textarea: HTMLTextAreaElement;
  let commands: SlashCommand[] = [];
  let files: string[] = [];
  let suggestions: SlashCommand[] = [];
  let fileSuggestions: string[] = [];
  let selIdx = 0;
  let fileSelIdx = 0;
  let sendError = '';

  // Commands that require an interactive TTY — sending them kills the session.
  const INTERACTIVE_CMDS = new Set(['/mcp', '/login', '/logout', '/init', '/doctor']);

  // Aliases passed directly to claude --model (CLI resolves them)
  const MODEL_OPTIONS = ['opus', 'opus-1m', 'sonnet', 'haiku'];
  const EFFORT_LEVELS = ['low', 'medium', 'high', 'max'];

  // Orbit-native commands added to suggestions
  const ORBIT_COMMANDS: SlashCommand[] = [
    { cmd: '/model', desc: 'Switch model (opus, opus-1m, sonnet, haiku)', category: 'orbit' },
  ];

  $: effectiveCommands =
    provider === 'claude-code'
      ? [
          ...ORBIT_COMMANDS,
          {
            cmd: '/effort',
            desc: 'Set thinking effort (low, medium, high, max)',
            category: 'orbit',
          },
        ]
      : ORBIT_COMMANDS;

  function emitSystemEntry(msg: string) {
    const entry: JournalEntry = {
      sessionId: String(sessionId),
      timestamp: new Date().toISOString(),
      entryType: 'system',
      text: msg,
      thinking: null,
      thinkingDuration: null,
      tool: null,
      toolInput: null,
      output: null,
      exitCode: null,
      linesChanged: null,
    };
    journal.update((map) => {
      const next = new Map(map);
      next.set(sessionId, [...(next.get(sessionId) ?? []), entry]);
      return next;
    });
  }

  const hints = [
    'Orbit keeps all your Claude agents in sync — one dashboard, infinite sessions',
    'Each agent runs in its own orbit — isolated, parallel, always tracked',
    'Real-time log streaming — watch your agents compute at the speed of light',
    'Token usage and cost tracked per session — every bit accounted for',
    'Use @ to attach files directly to your message',
    'Use / to trigger slash commands inside any session',
    'Sessions persist — your agents remember where they left off',
    'Switch between agents without losing orbital momentum',
    'Multiple Claude agents, one control center — mission control for AI',
    'Your agents orbit the same codebase, each on their own trajectory',
  ];
  let hintIdx = 0;
  let currentHint = hints[0];
  let hintVisible = true;

  const hintTimer = setInterval(() => {
    hintVisible = false;
    setTimeout(() => {
      hintIdx = (hintIdx + 1) % hints.length;
      currentHint = hints[hintIdx];
      hintVisible = true;
    }, 300);
  }, 5000);
  onDestroy(() => clearInterval(hintTimer));

  onMount(async () => {
    try {
      const remote = await getSlashCommands();
      const blocked = new Set([...INTERACTIVE_CMDS, '/model']);
      commands = [...effectiveCommands, ...remote.filter((c) => !blocked.has(c.cmd))];
    } catch (_e) {
      commands = [...effectiveCommands];
    }
  });

  $: {
    const remoteBlocked = new Set([...INTERACTIVE_CMDS, '/model']);
    if (commands.length > 0) {
      commands = commands.filter((c) => {
        const isOrbit = c.category === 'orbit';
        if (isOrbit) {
          return provider === 'claude-code' || c.cmd !== '/effort';
        }
        return true;
      });
    }
  }

  let prevId = sessionId;
  $: if (sessionId !== prevId) {
    text = '';
    prevId = sessionId;
    if (textarea) textarea.style.height = 'auto';
  }

  let prevCwd = '';
  $: if (cwd && cwd !== prevCwd) {
    prevCwd = cwd;
    listProjectFiles(cwd)
      .then((f) => (files = f))
      .catch((e) => console.warn('[InputBar] listProjectFiles failed:', e));
  }

  // Sub-options for /model and /effort
  let subOptions: string[] = [];
  $: {
    const lower = text.toLowerCase();
    if (lower.startsWith('/model ')) {
      const arg = lower.slice(7);
      subOptions = MODEL_OPTIONS.filter((o) => o.startsWith(arg));
    } else if (lower.startsWith('/effort ')) {
      const arg = lower.slice(8);
      subOptions = EFFORT_LEVELS.filter((o) => o.startsWith(arg));
    } else {
      subOptions = [];
    }
  }
  $: showSubOptions = subOptions.length > 0;

  $: suggestions =
    subOptions.length > 0
      ? []
      : text.startsWith('/')
        ? text.length === 1
          ? commands.slice(0, 8)
          : commands.filter((c) => c.cmd.toLowerCase().includes(text.toLowerCase())).slice(0, 8)
        : [];
  $: showSuggestions = suggestions.length > 0;
  $: if (selIdx >= suggestions.length) selIdx = 0;

  let subSelIdx = 0;
  $: if (subSelIdx >= subOptions.length) subSelIdx = 0;

  function atQuery(): string | null {
    if (!textarea) return null;
    const before = text.slice(0, textarea.selectionStart);
    const atPos = before.lastIndexOf('@');
    if (atPos === -1) return null;
    if (atPos > 0 && text[atPos - 1] !== ' ' && text[atPos - 1] !== '\n') return null;
    const q = before.slice(atPos + 1);
    if (q.includes(' ')) return null;
    return q;
  }
  let aq: string | null = null;
  $: aq = (() => {
    void text;
    return atQuery();
  })();
  $: fileSuggestions =
    aq === null
      ? []
      : aq === ''
        ? files.slice(0, 10)
        : (() => {
            const q = (aq as string).toLowerCase();
            const name = files.filter((f) => f.split('/').pop()!.toLowerCase().includes(q));
            const path = files.filter((f) => !name.includes(f) && f.toLowerCase().includes(q));
            return [...name, ...path].slice(0, 10);
          })();
  $: showFiles = fileSuggestions.length > 0;
  $: if (fileSelIdx >= fileSuggestions.length) fileSelIdx = 0;

  async function send() {
    const msg = text.trim();
    if (!msg) return;

    const cmd = msg.split(/\s/)[0].toLowerCase();

    if (INTERACTIVE_CMDS.has(cmd)) {
      sendError = `${cmd} requires interactive input and is not supported inside Orbit`;
      setTimeout(() => (sendError = ''), 5000);
      return;
    }

    // Intercept /model
    if (cmd === '/model') {
      const arg = msg.slice(6).trim().toLowerCase();
      if (!arg) {
        sendError = `Usage: /model <name> (${MODEL_OPTIONS.join(', ')})`;
        setTimeout(() => (sendError = ''), 5000);
        return;
      }
      text = '';
      if (textarea) textarea.style.height = 'auto';
      await updateSessionModel(sessionId, arg);
      sessions.update((l) => updateSessionState(l, sessionId, { model: arg, contextWindow: null }));
      emitSystemEntry(`Model changed to ${arg}`);
      return;
    }

    // Intercept /effort (Claude Code only)
    if (cmd === '/effort' && provider === 'claude-code') {
      const arg = msg.slice(7).trim().toLowerCase();
      if (!arg || !EFFORT_LEVELS.includes(arg)) {
        sendError = `Usage: /effort <level> (${EFFORT_LEVELS.join(', ')})`;
        setTimeout(() => (sendError = ''), 5000);
        return;
      }
      text = '';
      if (textarea) textarea.style.height = 'auto';
      await updateSessionEffort(sessionId, arg);
      sessionEffort.set(String(sessionId), arg);
      emitSystemEntry(`Effort level changed to ${arg}`);
      return;
    }

    text = '';
    sendError = '';
    if (textarea) textarea.style.height = 'auto';
    pendingMessages.add(msg);
    try {
      await sendSessionMessage(sessionId, msg);
    } catch (e: any) {
      sendError = e?.message ?? String(e);
      setTimeout(() => (sendError = ''), 4000);
    }
  }

  function selectCmd(cmd: string) {
    text = cmd + ' ';
    suggestions = [];
    textarea?.focus();
  }

  function selectSubOption(opt: string) {
    const cmd = text.toLowerCase().startsWith('/model') ? '/model' : '/effort';
    text = cmd + ' ' + opt;
    subOptions = [];
    send();
  }

  function selectFile(f: string) {
    if (!textarea) return;
    const pos = textarea.selectionStart;
    const before = text.slice(0, pos);
    const atPos = before.lastIndexOf('@');
    if (atPos === -1) return;
    text = text.slice(0, atPos) + '@' + f + ' ' + text.slice(pos);
    fileSuggestions = [];
    tick().then(() => {
      const np = atPos + 1 + f.length + 1;
      textarea.selectionStart = textarea.selectionEnd = np;
      textarea.focus();
    });
  }

  function onKey(e: KeyboardEvent) {
    if (showSubOptions) {
      if (e.key === 'ArrowDown') {
        e.preventDefault();
        subSelIdx = (subSelIdx + 1) % subOptions.length;
        return;
      }
      if (e.key === 'ArrowUp') {
        e.preventDefault();
        subSelIdx = (subSelIdx - 1 + subOptions.length) % subOptions.length;
        return;
      }
      if (e.key === 'Tab' || (e.key === 'Enter' && !e.shiftKey)) {
        e.preventDefault();
        selectSubOption(subOptions[subSelIdx]);
        return;
      }
      if (e.key === 'Escape') {
        subOptions = [];
        return;
      }
    }
    if (showFiles) {
      if (e.key === 'ArrowDown') {
        e.preventDefault();
        fileSelIdx = (fileSelIdx + 1) % fileSuggestions.length;
        return;
      }
      if (e.key === 'ArrowUp') {
        e.preventDefault();
        fileSelIdx = (fileSelIdx - 1 + fileSuggestions.length) % fileSuggestions.length;
        return;
      }
      if (e.key === 'Tab' || (e.key === 'Enter' && !e.shiftKey)) {
        e.preventDefault();
        selectFile(fileSuggestions[fileSelIdx]);
        return;
      }
      if (e.key === 'Escape') {
        fileSuggestions = [];
        return;
      }
    }
    if (showSuggestions) {
      if (e.key === 'ArrowDown') {
        e.preventDefault();
        selIdx = (selIdx + 1) % suggestions.length;
        return;
      }
      if (e.key === 'ArrowUp') {
        e.preventDefault();
        selIdx = (selIdx - 1 + suggestions.length) % suggestions.length;
        return;
      }
      if (e.key === 'Tab' || (e.key === 'Enter' && !e.shiftKey)) {
        e.preventDefault();
        selectCmd(suggestions[selIdx].cmd);
        return;
      }
      if (e.key === 'Escape') {
        suggestions = [];
        return;
      }
    }
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      send();
    }
  }

  function autoResize(e: Event) {
    const t = e.target as HTMLTextAreaElement;
    t.style.height = 'auto';
    t.style.height = Math.min(t.scrollHeight, 120) + 'px';
  }

  async function quickAction(msg: string) {
    pendingMessages.add(msg);
    try {
      await sendSessionMessage(sessionId, msg);
    } catch (e: any) {
      sendError = e?.message ?? String(e);
      setTimeout(() => (sendError = ''), 4000);
    }
  }
</script>

<div class="input-area">
  {#if sendError}
    <div class="send-error">! {sendError}</div>
  {/if}
  <!-- Autocomplete dropdowns -->
  {#if showSubOptions}
    <div class="dropdown">
      {#each subOptions as opt, i}
        <button class="drop-item" class:sel={i === subSelIdx} on:click={() => selectSubOption(opt)}>
          <span class="drop-main">{opt}</span>
        </button>
      {/each}
    </div>
  {:else if showFiles || showSuggestions}
    <div class="dropdown">
      {#if showFiles}
        {#each fileSuggestions as f, i}
          <button class="drop-item" class:sel={i === fileSelIdx} on:click={() => selectFile(f)}>
            <span class="drop-icon">@</span>
            <span class="drop-main">{f.split('/').pop()}</span>
            <span class="drop-sub">{f}</span>
          </button>
        {/each}
      {:else}
        {#each suggestions as s, i}
          <button class="drop-item" class:sel={i === selIdx} on:click={() => selectCmd(s.cmd)}>
            <span class="drop-main">{s.cmd}</span>
            <span class="drop-sub">{s.desc}</span>
          </button>
        {/each}
      {/if}
    </div>
  {/if}

  <div class="input-row">
    <span class="prompt-char" class:dim={sessionStatus === 'initializing'}>›</span>
    <textarea
      bind:this={textarea}
      bind:value={text}
      on:keydown={onKey}
      on:input={autoResize}
      placeholder={sessionStatus === 'initializing'
        ? 'waiting for session to start...'
        : sessionStatus === 'stopped'
          ? 'session stopped — type to resume...'
          : 'message... (/ for commands, @ for files)'}
      rows="1"
      disabled={sessionStatus === 'initializing'}
    ></textarea>
    <button
      class="send-btn"
      on:click={send}
      disabled={!text.trim() || sessionStatus === 'initializing'}
      title="Enter">⏎</button
    >
  </div>

  <div class="hint-bar" class:fade-out={!hintVisible}>
    <span class="hint-icon">◎</span>
    {currentHint}
  </div>
</div>

<style>
  .input-area {
    border-top: 1px solid var(--bd);
    background: var(--bg1);
    position: relative;
    flex-shrink: 0;
  }
  .send-error {
    padding: 5px 12px;
    font-size: var(--xs);
    color: var(--s-error);
    border-bottom: 1px solid rgba(224, 72, 72, 0.2);
    background: rgba(224, 72, 72, 0.05);
  }

  .dropdown {
    position: absolute;
    bottom: 100%;
    left: 0;
    right: 0;
    background: var(--bg2);
    border: 1px solid var(--bd1);
    border-bottom: none;
    border-radius: 4px 4px 0 0;
    max-height: 200px;
    overflow-y: auto;
  }
  .drop-item {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    text-align: left;
    background: none;
    border: none;
    padding: 6px 12px;
    cursor: pointer;
    border-bottom: 1px solid var(--bd);
  }
  .drop-item:hover,
  .drop-item.sel {
    background: var(--bg3);
  }
  .drop-icon {
    color: var(--ac);
    font-size: var(--xs);
    width: 14px;
    flex-shrink: 0;
  }
  .drop-main {
    font-size: var(--md);
    color: var(--t0);
    font-weight: 500;
    flex-shrink: 0;
  }
  .drop-sub {
    font-size: var(--xs);
    color: var(--t2);
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .input-row {
    display: flex;
    align-items: flex-end;
    gap: 0;
    padding: 8px 10px 6px;
  }
  .prompt-char {
    color: var(--t2);
    font-size: var(--lg);
    line-height: 1;
    margin-bottom: 6px;
    margin-right: 8px;
    flex-shrink: 0;
    transition: color 0.2s;
  }
  .prompt-char.dim {
    color: var(--t3);
  }
  textarea {
    flex: 1;
    background: none;
    border: none;
    color: var(--t0);
    font-size: var(--base);
    font-family: var(--mono);
    padding: 4px 0;
    resize: none;
    outline: none;
    line-height: 1.5;
    overflow-y: auto;
    max-height: 120px;
  }
  textarea::placeholder {
    color: var(--t3);
  }
  .send-btn {
    background: none;
    border: none;
    color: var(--t2);
    font-size: var(--lg);
    padding: 2px 4px;
    margin-bottom: 4px;
    flex-shrink: 0;
  }
  .send-btn:hover:not(:disabled) {
    color: var(--ac);
  }
  .send-btn:disabled {
    opacity: 0.3;
  }

  .hint-bar {
    padding: 0 10px 7px;
    font-size: var(--xs);
    color: var(--t3);
    opacity: 1;
    transition: opacity 0.3s ease;
    display: flex;
    align-items: center;
    gap: 5px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .hint-bar.fade-out {
    opacity: 0;
  }
  .hint-icon {
    color: var(--ac);
    font-size: 10px;
    flex-shrink: 0;
  }
</style>
