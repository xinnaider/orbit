<script lang="ts">
  import { sendKeystroke, sendMessage } from '../lib/tauri';

  export let sessionId: string;
  export let agentName: string;

  let inputText = '';

  async function handleSend() {
    if (!inputText.trim()) return;
    await sendMessage(sessionId, inputText);
    inputText = '';
  }

  async function handleQuickAction(key: string) {
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
      <input
        type="text"
        bind:value={inputText}
        onkeydown={handleKeydown}
        placeholder="Send command to {agentName}..."
      />
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
    background: rgba(255, 255, 255, 0.01);
  }
  .input-row { display: flex; gap: 8px; align-items: center; }
  .input-wrapper {
    flex: 1;
    display: flex;
    align-items: center;
    background: rgba(255, 255, 255, 0.03);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 0 10px;
  }
  .prompt { color: var(--text-dim); font-size: 13px; margin-right: 6px; }
  input {
    background: transparent;
    border: none;
    color: var(--text-primary);
    font-size: 13px;
    font-family: 'Cascadia Code', monospace;
    padding: 7px 0;
    width: 100%;
    outline: none;
  }
  .send-btn {
    background: var(--blue-dim);
    color: var(--blue);
    border: 1px solid rgba(96, 165, 250, 0.2);
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
    background: rgba(255, 255, 255, 0.03);
    border: none;
    padding: 2px 6px;
    border-radius: 4px;
    cursor: pointer;
  }
  .quick-actions button:hover { background: rgba(255, 255, 255, 0.06); }
  .quick-actions button.ctrl-c { color: var(--red); }
</style>
