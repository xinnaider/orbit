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
  import { sessionEffort } from '../lib/stores/ui';
  import type { SlashCommand } from '../lib/types';
  import type { JournalEntry } from '../lib/types';
  import { providerCaps, getCaps } from '../lib/stores/providers';
  import SlashCommandPicker from './shared/SlashCommandPicker.svelte';

  export let sessionId: number;
  export let cwd: string = '';
  export let sessionStatus: string = '';
  export let provider: string = 'claude-code';
  export let providerModels: string[] = [];

  $: caps = getCaps($providerCaps, provider);

  let text = '';
  let textarea: HTMLTextAreaElement;
  let commands: SlashCommand[] = [];
  let files: string[] = [];
  let sendError = '';
  let picker: SlashCommandPicker;

  // Commands that require an interactive TTY — sending them kills the session.
  const INTERACTIVE_CMDS = new Set(['/mcp', '/login', '/logout', '/init', '/doctor']);

  // Model aliases per backend
  const CLAUDE_MODELS = ['opus', 'opus-1m', 'sonnet', 'haiku'];
  const CODEX_MODELS = ['gpt-5.4', 'gpt-5.4-mini', 'gpt-5.3-codex', 'gpt-5.2'];
  $: MODEL_OPTIONS =
    provider === 'claude-code'
      ? CLAUDE_MODELS
      : provider === 'codex'
        ? CODEX_MODELS
        : providerModels;
  const EFFORT_LEVELS = ['low', 'medium', 'high', 'max'];

  // Orbit-native commands — provider-aware
  $: modelHint =
    provider === 'claude-code'
      ? 'Switch model (opus, sonnet, haiku)'
      : provider === 'codex'
        ? 'Switch model (gpt-5.4, gpt-5.4-mini, ...)'
        : 'Switch model (type model ID)';

  $: effectiveCommands = (() => {
    const cmds: SlashCommand[] = [{ cmd: '/model', desc: modelHint, category: 'orbit' }];
    if (caps.supportsEffort) {
      cmds.push({
        cmd: '/effort',
        desc: 'Set thinking effort (low, medium, high, max)',
        category: 'orbit',
      });
    }
    return cmds;
  })();

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
    'Orbit keeps all your agents in sync — one dashboard, infinite sessions',
    'Each agent runs in its own orbit — isolated, parallel, always tracked',
    'Real-time log streaming — watch your agents compute at the speed of light',
    'Token usage and cost tracked per session — every bit accounted for',
    'Use @ to attach files directly to your message',
    'Use / to trigger slash commands inside any session',
    'Sessions persist — your agents remember where they left off',
    'Switch between agents without losing orbital momentum',
    'Multiple agents, one control center — mission control for AI',
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
      const remote = await getSlashCommands(provider);
      const blocked = new Set([...INTERACTIVE_CMDS, '/model', '/effort']);
      commands = [...effectiveCommands, ...remote.filter((c) => !blocked.has(c.cmd))];
    } catch (_e) {
      commands = [...effectiveCommands];
    }
  });

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

  // atQuery: compute the @ file query from current cursor position
  function computeAtQuery(): string | null {
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
    return computeAtQuery();
  })();

  // Whether any picker dropdown is active (for keyboard handling)
  $: pickerVisible = text.startsWith('/') || aq !== null;

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
      const arg = msg.slice(6).trim();
      if (!arg) {
        const hint = MODEL_OPTIONS.length > 0 ? ` (${MODEL_OPTIONS.join(', ')})` : '';
        sendError = `Usage: /model <name>${hint}`;
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
    if (cmd === '/effort' && caps.supportsEffort) {
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

  function handlePickerSelect(
    e: CustomEvent<{ type: 'cmd' | 'subOption' | 'file'; value: string }>
  ) {
    const { type, value } = e.detail;
    if (type === 'cmd') {
      text = value + ' ';
      textarea?.focus();
    } else if (type === 'subOption') {
      const cmd = text.toLowerCase().startsWith('/model') ? '/model' : '/effort';
      text = cmd + ' ' + value;
      send();
    } else if (type === 'file') {
      if (!textarea) return;
      const pos = textarea.selectionStart;
      const before = text.slice(0, pos);
      const atPos = before.lastIndexOf('@');
      if (atPos === -1) return;
      text = text.slice(0, atPos) + '@' + value + ' ' + text.slice(pos);
      tick().then(() => {
        const np = atPos + 1 + value.length + 1;
        textarea.selectionStart = textarea.selectionEnd = np;
        textarea.focus();
      });
    }
  }

  function onKey(e: KeyboardEvent) {
    if (picker?.handleKey(e)) return;
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
  <SlashCommandPicker
    bind:this={picker}
    {commands}
    {text}
    visible={pickerVisible}
    {providerModels}
    modelOptions={MODEL_OPTIONS}
    supportsEffort={caps.supportsEffort}
    {files}
    atQuery={aq}
    on:select={handlePickerSelect}
    on:close={() => {
      text = text + '';
    }}
  />

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
    padding: var(--sp-3) var(--sp-6);
    font-size: var(--xs);
    color: var(--s-error);
    border-bottom: 1px solid rgba(224, 72, 72, 0.2);
    background: rgba(224, 72, 72, 0.05);
  }

  .input-row {
    display: flex;
    align-items: flex-end;
    gap: 0;
    padding: var(--sp-4) var(--sp-5) var(--sp-3);
  }
  .prompt-char {
    color: var(--t2);
    font-size: var(--lg);
    line-height: 1;
    margin-bottom: var(--sp-3);
    margin-right: var(--sp-4);
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
    padding: var(--sp-2) 0;
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
    padding: var(--sp-1) var(--sp-2);
    margin-bottom: var(--sp-2);
    flex-shrink: 0;
  }
  .send-btn:hover:not(:disabled) {
    color: var(--ac);
  }
  .send-btn:disabled {
    opacity: 0.3;
  }

  .hint-bar {
    padding: 0 var(--sp-5) var(--sp-3);
    font-size: var(--xs);
    color: var(--t3);
    opacity: 1;
    transition: opacity 0.3s ease;
    display: flex;
    align-items: center;
    gap: var(--sp-3);
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
