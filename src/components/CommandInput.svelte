<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { sendKeystroke, sendMessage, getSlashCommands, listProjectFiles } from '../lib/tauri';
  import { pendingMessages } from '../lib/stores/journal';
  import type { SlashCommand } from '../lib/types';

  export let sessionId: string;
  export let agentName: string;
  export let agentCwd: string = '';

  let inputText = '';
  let textareaEl: HTMLTextAreaElement;
  let selectedIdx = 0;
  let showSuggestions = false;
  let commands: SlashCommand[] = [];
  let suggestionEls: HTMLButtonElement[] = [];

  // @ file picker state
  let projectFiles: string[] = [];
  let showFilePicker = false;
  let fileSelectedIdx = 0;
  let filePickerEls: HTMLButtonElement[] = [];

  onMount(async () => {
    try {
      commands = await getSlashCommands();
    } catch {
      // Fallback: empty
    }
    if (agentCwd) {
      try {
        projectFiles = await listProjectFiles(agentCwd);
      } catch {
        // Fallback: empty
      }
    }
  });

  // Slash command filtering
  $: query = inputText.startsWith('/') ? inputText.toLowerCase() : '';
  $: filtered = query
    ? commands.filter(c => c.cmd.toLowerCase().includes(query))
    : [];
  $: {
    showSuggestions = filtered.length > 0 && inputText.startsWith('/');
    if (selectedIdx >= filtered.length) selectedIdx = 0;
  }

  // @ file picker: detect "@" token at cursor position
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
    const matches: string[] = [];
    for (const f of projectFiles) {
      if (f.toLowerCase().includes(q)) {
        matches.push(f);
        if (matches.length >= 15) break;
      }
    }
    return matches;
  })();

  $: {
    showFilePicker = filteredFiles.length > 0 && atQuery !== null;
    if (fileSelectedIdx >= filteredFiles.length) fileSelectedIdx = 0;
  }

  function scrollSelectedIntoView() {
    tick().then(() => {
      const el = suggestionEls[selectedIdx];
      if (el) el.scrollIntoView({ block: 'nearest' });
    });
  }

  function scrollFileSelectedIntoView() {
    tick().then(() => {
      const el = filePickerEls[fileSelectedIdx];
      if (el) el.scrollIntoView({ block: 'nearest' });
    });
  }

  async function handleSend() {
    if (!inputText.trim()) return;
    const text = inputText;
    inputText = '';
    showSuggestions = false;
    showFilePicker = false;
    if (textareaEl) textareaEl.style.height = 'auto';
    pendingMessages.add(text);
    await sendMessage(sessionId, text);
  }

  async function handleQuickAction(key: string) {
    const display = key === '\x03' ? 'Ctrl+C' : key;
    pendingMessages.add(display);
    await sendKeystroke(sessionId, key);
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

    // Slash command navigation (existing)
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
        const exactMatch = filtered.find(c => c.cmd === inputText.trim());
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
    <div class="suggestions">
      {#each filteredFiles as filePath, i}
        <button
          bind:this={filePickerEls[i]}
          class="suggestion"
          class:selected={i === fileSelectedIdx}
          onclick={() => selectFile(filePath)}
          onmouseenter={() => fileSelectedIdx = i}
        >
          <span class="file-icon">📄</span>
          <span class="cmd">{filePath.split('/').pop()}</span>
          <span class="desc">{filePath}</span>
        </button>
      {/each}
    </div>
  {:else if showSuggestions}
    <div class="suggestions">
      {#each filtered as item, i}
        <button
          bind:this={suggestionEls[i]}
          class="suggestion"
          class:selected={i === selectedIdx}
          onclick={() => selectCommand(item.cmd)}
          onmouseenter={() => selectedIdx = i}
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
        oninput={(e) => { const t = e.currentTarget; t.style.height = 'auto'; t.style.height = Math.min(t.scrollHeight, 120) + 'px'; }}
      ></textarea>
    </div>
    <button class="send-btn" onclick={handleSend}>Send</button>
  </div>
  <div class="quick-actions">
    <button onclick={() => handleQuickAction('y')}>y</button>
    <button onclick={() => handleQuickAction('n')}>n</button>
    <button onclick={() => handleQuickAction('yes, and continue')}>yes, and continue</button>
    <button class="ctrl-c" onclick={() => handleQuickAction('\x03')}>Ctrl+C</button>
  </div>
</div>

<style>
  .command-input {
    padding: 8px 14px;
    border-top: 1px solid var(--border);
    background: var(--bg-subtle);
    position: relative;
  }
  .input-row { display: flex; gap: 8px; align-items: flex-end; }
  .input-wrapper {
    flex: 1;
    display: flex;
    align-items: flex-start;
    background: var(--bg-overlay);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 0 10px;
  }
  .prompt { color: var(--text-dim); font-size: 13px; margin-right: 6px; margin-top: 8px; }
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
  .quick-actions {
    display: flex;
    gap: 6px;
    margin-top: 6px;
  }
  .quick-actions button {
    font-size: 11px;
    color: var(--text-dim);
    background: var(--bg-overlay);
    border: none;
    padding: 2px 6px;
    border-radius: 4px;
    cursor: pointer;
  }
  .quick-actions button:hover { background: var(--bg-hover); }
  .quick-actions button.ctrl-c { color: var(--red); }

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
    font-size: 12px;
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
  .cat.built-in { background: var(--bg-overlay); color: var(--text-dim); }
  .cat.skill { background: var(--purple-dim); color: var(--purple); }
  .cat.command { background: var(--green-dim); color: var(--green); }
  .cat.agent { background: var(--amber-dim); color: var(--amber); }
</style>
