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
  const CODEX_MODELS = ['gpt-5.5', 'gpt-5.4', 'gpt-5.4-mini', 'gpt-5.3-codex', 'gpt-5.2'];
  $: MODEL_OPTIONS =
    provider === 'claude-code'
      ? CLAUDE_MODELS
      : provider === 'codex'
        ? CODEX_MODELS
        : providerModels;

  // Effort levels from provider (model-aware) — falls back to global default
  $: currentModel = $sessions.find((s) => s.id === sessionId)?.model ?? 'auto';
  $: effortLevels = caps.effortLevels[currentModel] ??
    caps.effortLevels['auto'] ?? ['low', 'medium', 'high', 'max'];

  // Orbit-native commands — provider-aware
  $: modelHint =
    provider === 'claude-code'
      ? 'Switch model (opus, sonnet, haiku)'
      : provider === 'codex'
        ? 'Switch model (gpt-5.5, gpt-5.4, ...)'
        : 'Switch model (type model ID)';

  $: effectiveCommands = (() => {
    const cmds: SlashCommand[] = [{ cmd: '/model', desc: modelHint, category: 'orbit' }];
    if (caps.supportsEffort) {
      cmds.push({
        cmd: '/effort',
        desc: `Set thinking effort (${effortLevels.join(', ')})`,
        category: 'orbit',
      });
    }
    cmds.push({
      cmd: '/orchestrate',
      desc: '/orchestrate [provider] [task] — delegate to another agent',
      category: 'orbit',
    });
    cmds.push({
      cmd: '/create-agent',
      desc: '/create-agent [provider] [role/task] - create a named agent session',
      category: 'orbit',
    });
    return cmds;
  })();

  function parseProviderPrefix(rest: string): { chosenProvider: string; userGoal: string } {
    const knownAliases: Record<string, string> = {};
    const aliasCounts: Record<string, number> = {};
    const addAlias = (alias: string, providerId: string) => {
      const key = alias.toLowerCase();
      if (!key) return;
      if (knownAliases[key] === providerId) return;
      knownAliases[key] = providerId;
      aliasCounts[key] = (aliasCounts[key] ?? 0) + 1;
    };

    for (const b of $backendsStore) {
      addAlias(b.cliName, b.id);
      addAlias(b.id, b.id);
      for (const sub of b.subProviders ?? []) {
        addAlias(sub.id, sub.id);
        addAlias(sub.name, sub.id);
        addAlias(sub.id.split(/[-_]/)[0], sub.id);
      }
    }

    const firstWord = rest.split(/\s/)[0]?.toLowerCase() ?? '';
    if (firstWord && knownAliases[firstWord] && aliasCounts[firstWord] === 1) {
      return {
        chosenProvider: knownAliases[firstWord],
        userGoal: rest.slice(firstWord.length).trim(),
      };
    }

    return { chosenProvider: '', userGoal: rest };
  }

  function providerIsUnavailable(providerId: string): boolean {
    if (!providerId) return false;
    const backend = $backendsStore.find((b) => b.id === providerId);
    if (backend) return !backend.cliAvailable;
    const parent = $backendsStore.find((b) =>
      (b.subProviders ?? []).some((sub) => sub.id === providerId)
    );
    return parent ? !parent.cliAvailable : true;
  }

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
2. \`orbit_create_agent(provider="ollama-cloud", model="ollama-cloud/glm-5.1", cwd="/project", prompt="Task")\`

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

  function buildCreateAgentPrompt(userGoal: string, preferredProvider?: string): string {
    const installed = $backendsStore
      .filter((b) => b.cliAvailable)
      .map((b) => `${b.id} (${b.cliName})`)
      .join(', ');
    const providerList = installed || 'none detected';
    const workspacePath = cwd || '(current workspace path unavailable)';
    const currentSessionId = sessionId;
    const providerHint = preferredProvider
      ? `\n- Use provider "${preferredProvider}" if orbit_list_providers reports it as a provider id or OpenCode subProviders[].id.`
      : `\n- Prefer the current provider "${provider}" if it is available; otherwise pick the best installed provider.`;

    const intro = `You are Orbit's agent factory. Your job is to create a new named agent session in the current workspace, not to do the requested work yourself.

## Current workspace
Use this exact cwd for the new agent:
\`${workspacePath}\`

## Parent session
This factory prompt is running inside Orbit session ${currentSessionId}. Every agent you create must be a child of this session.
Always pass parentSessionId: ${currentSessionId} to orbit_create_agent so the new agent appears under the current session in the Orbit tree.

## Provider discovery
Always call orbit_list_providers first. Do not guess provider IDs or model IDs.
Installed providers snapshot from Orbit UI: ${providerList}${providerHint}

## OpenCode model rules
- orbit_list_providers returns OpenCode options under subProviders[].
- If the user names a subprovider such as "ollama", choose the subProviders[].id/name that matches it, for example "ollama-cloud".
- For OpenCode agents, pass provider as the matching subProviders[].id, not "opencode", when a subprovider is known.
- Match user model words against exact model IDs/names from that subprovider. For example, "kimi 2.6" should match a model like "kimi-k2.6:cloud" if listed.

## Creation rule
Create exactly one agent unless the user explicitly asks for multiple agents.
Use orbit_create_agent with:
- name: the explicit agent name from the user. If no name is explicit, infer a short role name such as "CEO", "Backend", "Frontend", "Developer", "Reviewer", or "QA".
- cwd: the current workspace path above.
- prompt: a complete role prompt built from the requested agent type and mission.
- parentSessionId: ${currentSessionId}.
- wait: false, unless the user explicitly asks you to wait for the result.

After creating the agent, report the sessionId, name, provider, and the mission you gave it.

## Agent prompt template
The prompt you pass to orbit_create_agent must include:

1. Identity
You are <agent name>, an Orbit agent working in this repository.

2. Mission
A concise mission derived from the user's request.

3. Operating rules
- Read the relevant code/context before acting.
- Keep scope tight and preserve unrelated user changes.
- Follow existing repo patterns and local instructions.
- Ask only when blocked by missing information that cannot be inferred safely.
- If editing code, verify with the narrowest useful test/check and report what ran.
- Final answer must include changed files, verification, blockers, and next recommended handoff.

4. Role-specific rules
- CEO / Orchestrator: define strategy, priorities, ownership, milestones, and delegation. Do not edit code unless explicitly asked.
- Developer: implement end-to-end, keep changes cohesive, and verify behavior.
- Backend: focus APIs, data models, services, migrations, security, and tests.
- Frontend: focus UI behavior, accessibility, responsive polish, and state flow.
- Reviewer: inspect risks, regressions, missing tests, and concrete file/line findings.
- QA: build a focused test plan, run checks when possible, and document reproduction steps.
- Research: map the problem, sources, tradeoffs, and recommended execution path.

## Naming rules
- If the user says "agent CEO", use "CEO" unless they provide a more specific name.
- If the user says "chamado X", "nome X", or "agent X", use X as the session name.
- Keep names under 32 characters.
- Do not add project suffixes; Orbit already knows the workspace.

## If the request is incomplete
If the user provides a role/name but no detailed mission, create a bootstrap agent for that role. For example, "agent CEO" should create a CEO agent whose mission is to understand the workspace, define operating strategy, propose the first org chart, and recommend the next agents to create.
If the user provides neither role nor name nor mission, ask one concise question instead of creating a generic session.`;

    if (userGoal) {
      return `${intro}\n\n## User request\n${userGoal}`;
    }
    return `${intro}\n\nNo agent request was provided. Ask the user what role/name and mission this new agent should have.`;
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

    // Intercept /effort for providers that expose reasoning effort.
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
      const { chosenProvider, userGoal } = parseProviderPrefix(rest);

      if (providerIsUnavailable(chosenProvider)) {
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

    // Intercept /create-agent [provider] [role/task]
    if (cmd === '/create-agent') {
      const rest = msg.slice('/create-agent'.length).trim();
      const { chosenProvider, userGoal } = parseProviderPrefix(rest);

      if (providerIsUnavailable(chosenProvider)) {
        sendError = `${chosenProvider} is not installed`;
        setTimeout(() => (sendError = ''), 5000);
        return;
      }

      text = '';
      if (textarea) textarea.style.height = 'auto';

      const createAgentPrompt = buildCreateAgentPrompt(userGoal, chosenProvider);
      const label = chosenProvider
        ? `Agent factory ready with ${chosenProvider}`
        : 'Agent factory ready';
      emitSystemEntry(label);
      pendingMessages.add(createAgentPrompt);
      try {
        await sendSessionMessage(sessionId, createAgentPrompt);
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

  @media (max-width: 768px) {
    .input-row {
      padding: var(--sp-5) var(--sp-5) var(--sp-4);
    }
    textarea {
      font-size: 16px;
      max-height: 160px;
    }
    .send-btn {
      font-size: 20px;
      padding: var(--sp-3) var(--sp-4);
    }
    .hint-bar {
      display: none;
    }
  }
</style>
