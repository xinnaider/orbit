<script lang="ts">
  import { sendKeystroke, sendMessage } from '../lib/tauri';
  import { pendingMessages } from '../lib/stores/journal';

  export let sessionId: string;
  export let agentName: string;

  let inputText = '';

  let textareaEl: HTMLTextAreaElement;

  async function handleSend() {
    if (!inputText.trim()) return;
    const text = inputText;
    inputText = '';
    if (textareaEl) textareaEl.style.height = 'auto';
    pendingMessages.add(text);
    await sendMessage(sessionId, text);
  }

  async function handleQuickAction(key: string) {
    const display = key === '\x03' ? 'Ctrl+C' : key;
    pendingMessages.add(display);
    await sendKeystroke(sessionId, key);
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSend();
    }
  }
</script>

<div class="command-input">
  <div class="input-row">
    <div class="input-wrapper">
      <span class="prompt">$</span>
      <textarea
        bind:this={textareaEl}
        bind:value={inputText}
        onkeydown={handleKeydown}
        placeholder="Send command to {agentName}... (Shift+Enter for new line)"
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
</style>
