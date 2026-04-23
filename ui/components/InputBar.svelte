<script lang="ts">
  import { onMount, onDestroy, tick } from 'svelte';
  import {
    sendSessionMessage,
    getSlashCommands,
    listProjectFiles,
    updateSessionModel,
    updateSessionEffort,
    stopSession,
  } from '../lib/tauri';
  import { messageHistory } from '../lib/stores/history';
  import { sessions, updateSessionState } from '../lib/stores/sessions';
  import { journal } from '../lib/stores/journal';
  import { pendingMessages } from '../lib/stores/journal';
  import { sessionEffort } from '../lib/stores/ui';
  import type { SlashCommand } from '../lib/types';
  import type { JournalEntry } from '../lib/types';
  import { providerCaps, getCaps, backends as backendsStore } from '../lib/stores/providers';
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
  // Display names shown in the picker; resolved to real IDs before sending
  const CLAUDE_MODELS = ['Opus 4.7', 'Opus 4.7 (1M)', 'Opus 4.6', 'Sonnet 4.6', 'Haiku 4.5'];
  const CLAUDE_MODEL_ALIASES: Record<string, string> = {
    // Versioned short aliases (also accepted if user types them)
    'opus-4.7': 'claude-opus-4-7',
    'opus-4.7-1m': 'claude-opus-4-7[1m]',
    'opus-4.6': 'claude-opus-4-6',
    'sonnet-4.6': 'claude-sonnet-4-6',
    'haiku-4.5': 'claude-haiku-4-5-20251001',
    // Display-name aliases (shown in picker)
    'Opus 4.7': 'claude-opus-4-7',
    'Opus 4.7 (1M)': 'claude-opus-4-7[1m]',
    'Opus 4.6': 'claude-opus-4-6',
    'Sonnet 4.6': 'claude-sonnet-4-6',
    'Haiku 4.5': 'claude-haiku-4-5-20251001',
  };
  const CODEX_MODELS = ['gpt-5.4', 'gpt-5.4-mini', 'gpt-5.3-codex', 'gpt-5.2'];
  $: MODEL_OPTIONS =
    provider === 'claude-code'
      ? CLAUDE_MODELS
      : provider === 'codex'
        ? CODEX_MODELS
        : providerModels;

  // Effort levels from provider (model-aware) — falls back to global default
  $: currentModel = $sessions.find((s) => s.id === sessionId)?.model ?? '';
  $: effortLevels = caps.effortLevels[currentModel] ??
    caps.effortLevels['auto'] ?? ['low', 'medium', 'high', 'max'];

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
    cmds.push({
      cmd: '/orchestrate',
      desc: '/orchestrate [provider] [task] — delegate to another agent',
      category: 'orbit',
    });
    return cmds;
  })();

  function buildOrchestratePrompt(userGoal: string, preferredProvider?: string): string {
    const installed = $backendsStore
      .filter((b) => b.cliAvailable)
      .map((b) => `${b.id} (${b.cliName})`)
      .join(', ');
    const notInstalled = $backendsStore
      .filter((b) => !b.cliAvailable)
      .map((b) => `${b.id}`)
      .join(', ');
    const providerList = installed || 'none detected';
    const unavailable = notInstalled ? `\nNot installed: ${notInstalled}` : '';

    const intro = `You have access to Orbit's multi-agent orchestration tools via MCP. Use them to delegate tasks to other AI agents.

## IMPORTANT — Always discover providers first

Before creating any agent, call **orbit_list_providers** to get the exact provider IDs, model IDs, and capabilities. Do NOT guess model names.

## Quick reference: installed providers
${providerList}${unavailable}

## Tools

### orbit_list_providers
Returns all providers with models[], subProviders[], effortLevels, and capabilities. **Call this first.**

### orbit_create_agent
Spawn a new agent session visible in the Orbit dashboard.
- **name**: display name in sidebar (optional, e.g. "test runner")
- **provider**: provider ID from orbit_list_providers (default: claude-code)
- **model**: exact model ID from orbit_list_providers (optional)
- **cwd**: working directory (required)
- **prompt**: task for the agent (required)
- **wait**: true = block until done and return output (default). false = return sessionId immediately, then you MUST poll orbit_get_status until status is "completed", "stopped", or "error"
- **timeoutSecs**: max wait seconds (default: 300)

### orbit_get_status
Get session status, tokens, output, context %. Use to poll wait=false sessions — keep calling until terminal status.

### orbit_send_message
Send a follow-up message to an existing session (uses --resume).

### orbit_cancel_agent
Kill a running agent.

### orbit_list_sessions
List all dashboard sessions, optionally filtered by status.

### orbit_get_subagents
Get subagent tree for a session.

## Example workflows

**Delegate to another Claude:**
\`orbit_create_agent(name="auth tests", cwd="/project", prompt="Write tests for auth module")\`

**Use a specific model from another provider:**
1. \`orbit_list_providers()\` → find opencode has "ollama-cloud/glm-5.1"
2. \`orbit_create_agent(provider="opencode", model="ollama-cloud/glm-5.1", cwd="/project", prompt="Task")\`

**Fan-out pattern (parallel agents):**
1. \`orbit_create_agent(wait=false, name="task-a", prompt="Task A")\` → sessionId 1
2. \`orbit_create_agent(wait=false, name="task-b", prompt="Task B")\` → sessionId 2
3. Poll both with \`orbit_get_status\` until done, then combine results

**Worker + reviewer loop:**
1. Create worker agent → get output
2. Create reviewer agent with worker's output → get feedback
3. If feedback has issues, orbit_send_message to worker with corrections`;

    const providerHint = preferredProvider
      ? `\n\n**Use provider "${preferredProvider}" for this task.**`
      : '';

    if (userGoal) {
      return `${intro}\n\n## Your task${providerHint}\n\n${userGoal}`;
    }
    if (preferredProvider) {
      return `${intro}${providerHint}\n\nReady to orchestrate with ${preferredProvider}. What should I delegate?`;
    }
    return `${intro}\n\nReady to orchestrate. What would you like me to delegate?`;
  }

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
      seq: 0,
      epoch: '',
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
      // Resolve alias (e.g. "opus") to real model ID ("claude-opus-4-7")
      const resolved = provider === 'claude-code' ? (CLAUDE_MODEL_ALIASES[arg] ?? arg) : arg;
      await updateSessionModel(sessionId, resolved);
      sessions.update((l) =>
        updateSessionState(l, sessionId, { model: resolved, contextWindow: null })
      );
      emitSystemEntry(`Model changed to ${resolved}`);
      return;
    }

    // Intercept /effort (Claude Code only)
    if (cmd === '/effort' && caps.supportsEffort) {
      const arg = msg.slice(7).trim().toLowerCase();
      if (!arg || !effortLevels.includes(arg)) {
        sendError = `Usage: /effort <level> (${effortLevels.join(', ')})`;
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

    // Intercept /orchestrate [provider] [prompt]
    if (cmd === '/orchestrate') {
      const rest = msg.slice('/orchestrate'.length).trim();
      const knownIds = $backendsStore.map((b) => b.id);
      const knownAliases: Record<string, string> = {};
      for (const b of $backendsStore) {
        knownAliases[b.cliName] = b.id;
        knownAliases[b.id] = b.id;
      }

      let chosenProvider = '';
      let userGoal = rest;

      // Check if first word is a provider name/alias
      const firstWord = rest.split(/\s/)[0]?.toLowerCase() ?? '';
      if (firstWord && knownAliases[firstWord]) {
        chosenProvider = knownAliases[firstWord];
        userGoal = rest.slice(firstWord.length).trim();
      }

      if (chosenProvider && !$backendsStore.find((b) => b.id === chosenProvider)?.cliAvailable) {
        sendError = `${chosenProvider} is not installed`;
        setTimeout(() => (sendError = ''), 5000);
        return;
      }

      text = '';
      if (textarea) textarea.style.height = 'auto';

      const orchestratePrompt = buildOrchestratePrompt(userGoal, chosenProvider);
      const label = chosenProvider
        ? `Orchestrating with ${chosenProvider}`
        : 'Multi-agent orchestration enabled';
      emitSystemEntry(label);
      pendingMessages.add(orchestratePrompt);
      try {
        await sendSessionMessage(sessionId, orchestratePrompt);
      } catch (e: any) {
        sendError = e?.message ?? String(e);
        setTimeout(() => (sendError = ''), 4000);
      }
      return;
    }

    text = '';
    sendError = '';
    if (textarea) textarea.style.height = 'auto';
    messageHistory.push(String(sessionId), msg);
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

    if (e.ctrlKey && e.key === 'c' && text === '') {
      e.preventDefault();
      stopSession(sessionId);
      journal.update((m) => {
        const entries = m.get(sessionId) ?? [];
        entries.push({
          sessionId: String(sessionId),
          timestamp: new Date().toISOString(),
          entryType: 'system',
          text: 'Sent interrupt signal (Ctrl+C)',
          thinking: null,
          thinkingDuration: null,
          tool: null,
          toolInput: null,
          output: null,
          exitCode: null,
          linesChanged: null,
          seq: entries.length,
          epoch: '',
        });
        m.set(sessionId, entries);
        return new Map(m);
      });
      return;
    }

    const cursor = textarea?.selectionStart ?? 0;
    const len = text.length;
    const atStart = cursor === 0;
    const atEnd = cursor === len;

    if (e.key === 'ArrowUp' && atStart) {
      e.preventDefault();
      const prev = messageHistory.up(String(sessionId), text);
      if (prev !== null) {
        text = prev;
        tick().then(() => {
          if (textarea) textarea.selectionStart = textarea.selectionEnd = 0;
        });
      }
      return;
    }

    if (e.key === 'ArrowDown' && atEnd) {
      e.preventDefault();
      const next = messageHistory.down(String(sessionId));
      if (next !== null) {
        text = next;
        tick().then(() => {
          if (textarea) textarea.selectionStart = textarea.selectionEnd = text.length;
        });
      }
      return;
    }

    if (e.key !== 'ArrowUp' && e.key !== 'ArrowDown') {
      messageHistory.resetCursor(String(sessionId));
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
  <SlashCommandPicker
    bind:this={picker}
    {commands}
    {text}
    visible={pickerVisible}
    {providerModels}
    modelOptions={MODEL_OPTIONS}
    supportsEffort={caps.supportsEffort}
    {effortLevels}
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
