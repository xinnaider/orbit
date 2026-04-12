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
  import { addToast } from '../lib/stores/toasts';
  import { FileText } from 'lucide-svelte';
  import { pendingMessages } from '../lib/stores/journal';
  import type { SlashCommand } from '../lib/types';

  export let sessionId: number;
  export let agentName: string;
  export let agentCwd: string = '';

  let inputText = '';
  let textareaEl: HTMLTextAreaElement;
  let selectedIdx = 0;
  let showSuggestions = false;
  let commands: SlashCommand[] = [];
  let suggestionEls: HTMLButtonElement[] = [];
  let suggestionsContainer: HTMLDivElement | null = null;

  let projectFiles: string[] = [];
  let showFilePicker = false;
  let fileSelectedIdx = 0;
  let filePickerContainer: HTMLDivElement | null = null;

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
      commands = await getSlashCommands();
    } catch (_e) {
      // Fallback: empty
    }
  });

  let prevSessionId = sessionId;
  let prevCwd = '';
  $: {
    if (sessionId !== prevSessionId) {
      inputText = '';
      showSuggestions = false;
      showFilePicker = false;
      if (textareaEl) textareaEl.style.height = 'auto';
      prevSessionId = sessionId;
    }
    if (agentCwd && agentCwd !== prevCwd) {
      prevCwd = agentCwd;
      listProjectFiles(agentCwd)
        .then((files) => (projectFiles = files))
        .catch(() => {});
    }
  }

  $: query = inputText.startsWith('/') ? inputText.toLowerCase() : '';
  $: filtered = query ? commands.filter((c) => c.cmd.toLowerCase().includes(query)) : [];
  $: {
    showSuggestions = filtered.length > 0 && inputText.startsWith('/');
    if (selectedIdx >= filtered.length) selectedIdx = 0;
  }

  function getAtQuery(): string | null {
    if (!textareaEl) return null;
    const pos = textareaEl.selectionStart;
    const textBefore = inputText.slice(0, pos);
    // Find the last @ that isn't preceded by a non-space char
    const lastAt = textBefore.lastIndexOf('@');
    if (lastAt === -1) return null;
    // @ must be at start or preceded by whitespace
    if (lastAt > 0 && inputText[lastAt - 1] !== ' ' && inputText[lastAt - 1] !== '\n') return null;
    const queryStr = textBefore.slice(lastAt + 1);
    // If there's a space after the query, the user finished typing the path
    if (queryStr.includes(' ')) return null;
    return queryStr;
  }

  $: atQuery = (() => {
    // Re-run when inputText changes
    void inputText;
    return getAtQuery();
  })();

  $: filteredFiles = (() => {
    if (atQuery === null) return [];
    if (atQuery === '') return projectFiles.slice(0, 15);
    const q = atQuery.toLowerCase();
    const nameMatches: string[] = [];
    const pathMatches: string[] = [];
    for (const f of projectFiles) {
      const fl = f.toLowerCase();
      const fileName = fl.split('/').pop() ?? '';
      if (fileName.includes(q)) {
        nameMatches.push(f);
      } else if (fl.includes(q)) {
        pathMatches.push(f);
      }
      if (nameMatches.length + pathMatches.length >= 30) break;
    }
    return [...nameMatches, ...pathMatches].slice(0, 15);
  })();

  $: {
    showFilePicker = filteredFiles.length > 0 && atQuery !== null;
    if (fileSelectedIdx >= filteredFiles.length) fileSelectedIdx = 0;
  }

  function scrollSelectedIntoView() {
    tick().then(() => {
      if (!suggestionsContainer) return;
      const buttons = suggestionsContainer.querySelectorAll<HTMLElement>('.suggestion');
      const el = buttons[selectedIdx];
      if (!el) return;
      const top = el.offsetTop;
      const bottom = top + el.offsetHeight;
      if (bottom > suggestionsContainer.scrollTop + suggestionsContainer.clientHeight) {
        suggestionsContainer.scrollTop = bottom - suggestionsContainer.clientHeight;
      } else if (top < suggestionsContainer.scrollTop) {
        suggestionsContainer.scrollTop = top;
      }
    });
  }

  function scrollFileSelectedIntoView() {
    tick().then(() => {
      if (!filePickerContainer) return;
      const buttons = filePickerContainer.querySelectorAll<HTMLElement>('.suggestion');
      const el = buttons[fileSelectedIdx];
      if (!el) return;
      const top = el.offsetTop;
      const bottom = top + el.offsetHeight;
      if (bottom > filePickerContainer.scrollTop + filePickerContainer.clientHeight) {
        filePickerContainer.scrollTop = bottom - filePickerContainer.clientHeight;
      } else if (top < filePickerContainer.scrollTop) {
        filePickerContainer.scrollTop = top;
      }
    });
  }

  const MODEL_ALIASES: Record<string, string> = {
    opus: 'claude-opus-4-6',
    sonnet: 'claude-sonnet-4-6',
    haiku: 'claude-haiku-4-5-20251001',
    'opus-4': 'claude-opus-4-20250514',
    'sonnet-4': 'claude-sonnet-4-20250514',
    'sonnet-4.5': 'claude-sonnet-4-5-20250514',
  };

  const EFFORT_LEVELS = ['low', 'medium', 'high', 'max'];

  async function handleSend() {
    if (!inputText.trim()) return;
    const text = inputText.trim();
    inputText = '';
    showSuggestions = false;
    showFilePicker = false;
    if (textareaEl) textareaEl.style.height = 'auto';

    // Intercept /model
    const modelMatch = text.match(/^\/model\s+(.+)$/i);
    if (modelMatch) {
      const arg = modelMatch[1].trim().toLowerCase();
      const resolved = MODEL_ALIASES[arg] ?? arg;
      await updateSessionModel(sessionId, resolved);
      sessions.update((l) => updateSessionState(l, sessionId, { model: resolved }));
      addToast({ type: 'info', message: `Model set to ${resolved}`, autoDismiss: true });
      return;
    }

    // Intercept /effort
    const effortMatch = text.match(/^\/effort\s+(.+)$/i);
    if (effortMatch) {
      const level = effortMatch[1].trim().toLowerCase();
      if (!EFFORT_LEVELS.includes(level)) {
        addToast({
          type: 'error',
          message: `Invalid effort: "${level}". Use: ${EFFORT_LEVELS.join(', ')}`,
          autoDismiss: true,
        });
        return;
      }
      await updateSessionEffort(sessionId, level);
      addToast({ type: 'info', message: `Effort set to ${level}`, autoDismiss: true });
      return;
    }

    pendingMessages.add(text);
    await sendSessionMessage(sessionId, text);
  }

  async function handleQuickAction(key: string) {
    const display = key === '\x03' ? 'Ctrl+C' : key;
    pendingMessages.add(display);
    await sendSessionMessage(sessionId, key);
  }

  function selectCommand(cmd: string) {
    inputText = cmd + ' ';
    showSuggestions = false;
    textareaEl?.focus();
  }

  function selectFile(filePath: string) {
    if (!textareaEl) return;
    const pos = textareaEl.selectionStart;
    const textBefore = inputText.slice(0, pos);
    const lastAt = textBefore.lastIndexOf('@');
    if (lastAt === -1) return;
    const before = inputText.slice(0, lastAt);
    const after = inputText.slice(pos);
    inputText = before + '@' + filePath + ' ' + after;
    showFilePicker = false;
    tick().then(() => {
      if (textareaEl) {
        const newPos = lastAt + 1 + filePath.length + 1;
        textareaEl.selectionStart = newPos;
        textareaEl.selectionEnd = newPos;
        textareaEl.focus();
      }
    });
  }

  function handleKeydown(e: KeyboardEvent) {
    // @ file picker navigation
    if (showFilePicker) {
      if (e.key === 'ArrowDown') {
        e.preventDefault();
        fileSelectedIdx = (fileSelectedIdx + 1) % filteredFiles.length;
        scrollFileSelectedIntoView();
        return;
      }
      if (e.key === 'ArrowUp') {
        e.preventDefault();
        fileSelectedIdx = (fileSelectedIdx - 1 + filteredFiles.length) % filteredFiles.length;
        scrollFileSelectedIntoView();
        return;
      }
      if (e.key === 'Tab') {
        e.preventDefault();
        selectFile(filteredFiles[fileSelectedIdx]);
        return;
      }
      if (e.key === 'Escape') {
        e.preventDefault();
        showFilePicker = false;
        return;
      }
      if (e.key === 'Enter' && !e.shiftKey) {
        e.preventDefault();
        selectFile(filteredFiles[fileSelectedIdx]);
        return;
      }
    }

    if (showSuggestions) {
      if (e.key === 'ArrowDown') {
        e.preventDefault();
        selectedIdx = (selectedIdx + 1) % filtered.length;
        scrollSelectedIntoView();
        return;
      }
      if (e.key === 'ArrowUp') {
        e.preventDefault();
        selectedIdx = (selectedIdx - 1 + filtered.length) % filtered.length;
        scrollSelectedIntoView();
        return;
      }
      if (e.key === 'Tab') {
        e.preventDefault();
        selectCommand(filtered[selectedIdx].cmd);
        return;
      }
      if (e.key === 'Escape') {
        e.preventDefault();
        showSuggestions = false;
        return;
      }
      if (e.key === 'Enter' && !e.shiftKey) {
        const exactMatch = filtered.find((c) => c.cmd === inputText.trim());
        if (exactMatch) {
          e.preventDefault();
          handleSend();
          return;
        }
        if (filtered.length > 0 && inputText.trim() !== filtered[selectedIdx].cmd) {
          e.preventDefault();
          selectCommand(filtered[selectedIdx].cmd);
          return;
        }
      }
    }
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSend();
    }
  }
</script>

<div class="command-input">
  {#if showFilePicker}
    <div class="suggestions" bind:this={filePickerContainer}>
      {#each filteredFiles as filePath, i}
        <button
          class="suggestion"
          class:selected={i === fileSelectedIdx}
          onclick={() => selectFile(filePath)}
          onmouseenter={() => (fileSelectedIdx = i)}
        >
          <span class="file-icon"><FileText size={11} /></span>
          <span class="cmd">{filePath.split('/').pop()}</span>
          <span class="desc">{filePath}</span>
        </button>
      {/each}
    </div>
  {:else if showSuggestions}
    <div class="suggestions" bind:this={suggestionsContainer}>
      {#each filtered as item, i}
        <button
          class="suggestion"
          class:selected={i === selectedIdx}
          onclick={() => selectCommand(item.cmd)}
          onmouseenter={() => (selectedIdx = i)}
        >
          <span class="cmd">{item.cmd}</span>
          <span class="desc">{item.desc}</span>
          <span class="cat {item.category}">{item.category}</span>
        </button>
      {/each}
    </div>
  {/if}
  <div class="input-row">
    <div class="input-wrapper">
      <span class="prompt">$</span>
      <textarea
        bind:this={textareaEl}
        bind:value={inputText}
        onkeydown={handleKeydown}
        placeholder="Send command to {agentName}... (/ for commands, @ for files)"
        rows="1"
        oninput={(e) => {
          const t = e.currentTarget;
          t.style.height = 'auto';
          t.style.height = Math.min(t.scrollHeight, 120) + 'px';
        }}
      ></textarea>
    </div>
    <button class="send-btn" onclick={handleSend}>Send</button>
  </div>
  <div class="hint-bar" class:fade-out={!hintVisible}>
    <span class="hint-icon">◎</span>
    {currentHint}
  </div>
</div>

<style>
  .command-input {
    padding: 8px 14px;
    border-top: 1px solid var(--border);
    background: var(--bg-subtle);
    position: relative;
  }
  .input-row {
    display: flex;
    gap: 8px;
    align-items: flex-end;
  }
  .input-wrapper {
    flex: 1;
    display: flex;
    align-items: flex-start;
    background: var(--bg-overlay);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 0 10px;
  }
  .prompt {
    color: var(--text-dim);
    font-size: 13px;
    margin-right: 6px;
    margin-top: 8px;
  }
  textarea {
    background: transparent;
    border: none;
    color: var(--text-primary);
    font-size: 13px;
    font-family: 'Cascadia Code', monospace;
    padding: 7px 0;
    width: 100%;
    outline: none;
    resize: none;
    line-height: 1.4;
    overflow-y: auto;
    max-height: 120px;
  }
  .send-btn {
    background: var(--blue-dim);
    color: var(--blue);
    border: 1px solid var(--border-send);
    padding: 5px 14px;
    border-radius: 6px;
    font-size: 13px;
    cursor: pointer;
  }
  .hint-bar {
    margin-top: 6px;
    font-size: 11px;
    color: var(--text-dim);
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
    color: var(--blue);
    font-size: 10px;
    flex-shrink: 0;
  }

  /* Suggestions dropdown */
  .suggestions {
    position: absolute;
    bottom: 100%;
    left: 14px;
    right: 14px;
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 4px;
    margin-bottom: 4px;
    max-height: 240px;
    overflow-y: auto;
    z-index: 50;
    box-shadow: 0 -4px 16px rgba(0, 0, 0, 0.2);
  }
  .suggestion {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 6px 10px;
    border: none;
    background: none;
    border-radius: 6px;
    cursor: pointer;
    text-align: left;
    font-size: 12px;
    color: var(--text-primary);
  }
  .suggestion:hover,
  .suggestion.selected {
    background: var(--bg-hover);
  }
  .suggestion .cmd {
    font-family: 'Cascadia Code', monospace;
    font-weight: 600;
    color: var(--blue);
    flex-shrink: 0;
  }
  .suggestion .file-icon {
    display: flex;
    align-items: center;
    color: var(--text-dim);
    flex-shrink: 0;
  }
  .suggestion .desc {
    color: var(--text-muted);
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .suggestion .cat {
    font-size: 9px;
    padding: 1px 5px;
    border-radius: 3px;
    flex-shrink: 0;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.3px;
  }
  .cat.built-in {
    background: var(--bg-overlay);
    color: var(--text-dim);
  }
  .cat.skill {
    background: var(--purple-dim);
    color: var(--purple);
  }
  .cat.command {
    background: var(--green-dim);
    color: var(--green);
  }
  .cat.agent {
    background: var(--amber-dim);
    color: var(--amber);
  }
</style>
