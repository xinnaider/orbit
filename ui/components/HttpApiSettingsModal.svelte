<script lang="ts">
  import { createEventDispatcher, onMount } from 'svelte';
  import {
    generateApiKey,
    listApiKeys,
    revokeApiKey,
    getHttpSettings,
    setHttpSettings,
    getLanIp,
  } from '../lib/tauri';
  import type { ApiKeyCreated, ApiKeyInfo, HttpSettings } from '../lib/tauri';
  import Modal from './shared/Modal.svelte';
  import { generateQrSvg } from '../lib/qr';

  const dispatch = createEventDispatcher<{ close: void }>();

  let enabled = false;
  let host = '127.0.0.1';
  let port = 9999;
  let saving = false;
  let settingsChanged = false;
  let restartNeeded = false;

  let keys: ApiKeyInfo[] = [];
  let newKeyLabel = '';
  let generatingKey = false;
  let justCreatedKey: ApiKeyCreated | null = null;
  let copied = false;
  let lanIp = '';
  let qrSvg = '';

  $: accessUrl =
    enabled && lanIp && keys.length > 0
      ? `http://${lanIp}:${port}?token=${justCreatedKey?.key ?? 'YOUR_KEY'}`
      : '';

  $: if (accessUrl && justCreatedKey) {
    qrSvg = generateQrSvg(accessUrl, 180);
  }

  onMount(async () => {
    const settings = await getHttpSettings();
    enabled = settings.enabled;
    host = settings.host;
    port = settings.port;

    keys = await listApiKeys();
    lanIp = await getLanIp();
  });

  async function saveSettings() {
    saving = true;
    try {
      await setHttpSettings(enabled, host, port);
      settingsChanged = false;
      restartNeeded = true;
    } finally {
      saving = false;
    }
  }

  function markChanged() {
    settingsChanged = true;
  }

  async function createKey() {
    if (!newKeyLabel.trim()) return;
    generatingKey = true;
    try {
      justCreatedKey = await generateApiKey(newKeyLabel.trim());
      newKeyLabel = '';
      keys = await listApiKeys();
    } finally {
      generatingKey = false;
    }
  }

  async function deleteKey(id: string) {
    await revokeApiKey(id);
    keys = await listApiKeys();
    if (justCreatedKey?.id === id) justCreatedKey = null;
  }

  async function copyKey() {
    if (!justCreatedKey) return;
    await navigator.clipboard.writeText(justCreatedKey.key);
    copied = true;
    setTimeout(() => (copied = false), 2000);
  }
</script>

<Modal title="HTTP API settings" width="520px" zIndex={200} on:close={() => dispatch('close')}>
  <div class="section">
    <div class="row">
      <label class="label" for="http-enabled">enable HTTP API server</label>
      <label class="toggle-wrap">
        <input id="http-enabled" type="checkbox" bind:checked={enabled} on:change={markChanged} />
        <span class="toggle-track"><span class="toggle-thumb"></span></span>
      </label>
    </div>

    {#if enabled}
      <div class="field-row">
        <div class="field">
          <label class="label" for="http-host">host</label>
          <input
            id="http-host"
            class="input"
            bind:value={host}
            placeholder="127.0.0.1"
            on:input={markChanged}
          />
        </div>
        <div class="field field-port">
          <label class="label" for="http-port">port</label>
          <input
            id="http-port"
            class="input"
            type="number"
            min="1024"
            max="65535"
            bind:value={port}
            on:input={markChanged}
          />
        </div>
      </div>

      {#if host !== '127.0.0.1' && host !== 'localhost'}
        <div class="warn">binding to {host} exposes the API to the network</div>
      {/if}
    {/if}

    {#if settingsChanged}
      <button class="btn primary" on:click={saveSettings} disabled={saving}>
        {saving ? 'saving...' : 'save settings'}
      </button>
    {/if}

    {#if restartNeeded}
      <div class="info">restart Orbit for changes to take effect</div>
    {/if}
  </div>

  <div class="divider"></div>

  <div class="section">
    <span class="section-title">API keys</span>

    {#if justCreatedKey}
      <div class="new-key-banner">
        <span class="new-key-label">copy this key now — it won't be shown again</span>
        <div class="new-key-row">
          <code class="key-value">{justCreatedKey.key}</code>
          <button class="btn small" on:click={copyKey}>
            {copied ? 'copied' : 'copy'}
          </button>
        </div>
      </div>
    {/if}

    <div class="create-key-row">
      <input
        class="input"
        bind:value={newKeyLabel}
        placeholder="key label (e.g. laptop, ssh-server)"
        disabled={generatingKey}
        on:keydown={(e) => e.key === 'Enter' && createKey()}
      />
      <button
        class="btn primary"
        on:click={createKey}
        disabled={generatingKey || !newKeyLabel.trim()}
      >
        {generatingKey ? 'generating...' : 'generate'}
      </button>
    </div>

    {#if keys.length > 0}
      <div class="key-list">
        {#each keys as key (key.id)}
          <div class="key-item">
            <div class="key-info">
              <span class="key-label">{key.label}</span>
              <span class="key-date">{key.createdAt}</span>
            </div>
            <button class="btn ghost small" on:click={() => deleteKey(key.id)}>revoke</button>
          </div>
        {/each}
      </div>
    {:else}
      <span class="empty">no API keys yet</span>
    {/if}
  </div>

  {#if enabled && justCreatedKey && lanIp}
    <div class="divider"></div>

    <div class="section">
      <span class="section-title">mobile access</span>

      <div class="qr-section">
        <div class="qr-code">
          {@html qrSvg}
        </div>
        <div class="qr-info">
          <span class="qr-label">scan to open Orbit on your phone</span>
          <code class="qr-url">http://{lanIp}:{port}</code>
          <span class="qr-hint">same Wi-Fi network required</span>
        </div>
      </div>
    </div>
  {/if}
</Modal>

<style>
  .section {
    display: flex;
    flex-direction: column;
    gap: var(--sp-4);
  }
  .section-title {
    font-size: var(--xs);
    color: var(--t2);
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }
  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .field-row {
    display: flex;
    gap: var(--sp-4);
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: var(--sp-2);
    flex: 1;
  }
  .field-port {
    flex: 0 0 100px;
  }
  .label {
    font-size: var(--xs);
    color: var(--t2);
    letter-spacing: 0.06em;
  }
  .input {
    background: var(--bg2);
    border: 1px solid var(--bd1);
    border-radius: var(--radius-sm);
    color: var(--t0);
    font-size: var(--sm);
    padding: var(--sp-3) var(--sp-4);
    outline: none;
    width: 100%;
    transition: border-color 0.15s;
    font-family: var(--mono);
  }
  .input:focus {
    border-color: var(--bd2);
  }
  .input:disabled {
    opacity: 0.5;
  }
  .divider {
    border-top: 1px solid var(--bd1);
    margin: var(--sp-4) 0;
  }
  .toggle-wrap {
    position: relative;
    display: inline-block;
    cursor: pointer;
  }
  .toggle-wrap input {
    position: absolute;
    opacity: 0;
    width: 0;
    height: 0;
  }
  .toggle-track {
    display: block;
    width: 36px;
    height: 20px;
    background: var(--bg3);
    border-radius: 10px;
    transition: background 0.2s;
    position: relative;
  }
  .toggle-wrap input:checked + .toggle-track {
    background: var(--ac);
  }
  .toggle-thumb {
    position: absolute;
    top: 2px;
    left: 2px;
    width: 16px;
    height: 16px;
    background: var(--t0);
    border-radius: 50%;
    transition: transform 0.2s;
  }
  .toggle-wrap input:checked + .toggle-track .toggle-thumb {
    transform: translateX(16px);
  }
  .warn {
    font-size: var(--xs);
    color: var(--warning, #f5a623);
    padding: var(--sp-2) var(--sp-3);
    background: rgba(245, 166, 35, 0.08);
    border-radius: var(--radius-sm);
  }
  .info {
    font-size: var(--xs);
    color: var(--ac);
    padding: var(--sp-2) var(--sp-3);
    background: var(--ac-d);
    border-radius: var(--radius-sm);
  }
  .create-key-row {
    display: flex;
    gap: var(--sp-3);
  }
  .create-key-row .input {
    flex: 1;
  }
  .key-list {
    display: flex;
    flex-direction: column;
    gap: var(--sp-2);
  }
  .key-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--sp-3) var(--sp-4);
    background: var(--bg2);
    border-radius: var(--radius-sm);
    border: 1px solid var(--bd1);
  }
  .key-info {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .key-label {
    font-size: var(--sm);
    color: var(--t0);
  }
  .key-date {
    font-size: var(--xs);
    color: var(--t3);
  }
  .new-key-banner {
    background: var(--ac-d);
    border: 1px solid var(--ac);
    border-radius: var(--radius-sm);
    padding: var(--sp-4);
    display: flex;
    flex-direction: column;
    gap: var(--sp-3);
  }
  .new-key-label {
    font-size: var(--xs);
    color: var(--ac);
  }
  .new-key-row {
    display: flex;
    align-items: center;
    gap: var(--sp-3);
  }
  .key-value {
    font-family: var(--mono);
    font-size: var(--xs);
    color: var(--t0);
    background: var(--bg1);
    padding: var(--sp-2) var(--sp-3);
    border-radius: var(--radius-sm);
    flex: 1;
    word-break: break-all;
    user-select: all;
  }
  .empty {
    font-size: var(--xs);
    color: var(--t3);
    text-align: center;
    padding: var(--sp-4);
  }
  .btn {
    background: none;
    border: 1px solid var(--bd1);
    border-radius: var(--radius-sm);
    color: var(--t1);
    font-size: var(--sm);
    padding: var(--sp-3) var(--sp-7);
    transition: all 0.15s;
    font-family: var(--mono);
    cursor: pointer;
    white-space: nowrap;
  }
  .btn:hover {
    border-color: var(--bd2);
    color: var(--t0);
  }
  .btn.primary {
    background: var(--ac-d);
    border-color: var(--ac);
    color: var(--ac);
  }
  .btn.primary:hover {
    background: rgba(0, 212, 126, 0.18);
  }
  .btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }
  .btn.small {
    padding: var(--sp-2) var(--sp-4);
    font-size: var(--xs);
  }
  .btn.ghost {
    border-color: transparent;
  }
  .btn.ghost:hover {
    border-color: var(--bd1);
    color: var(--error, #ef4444);
  }
  .qr-section {
    display: flex;
    gap: var(--sp-6);
    align-items: center;
  }
  .qr-code {
    flex-shrink: 0;
    border-radius: var(--radius-sm);
    overflow: hidden;
    border: 1px solid var(--bd1);
  }
  .qr-info {
    display: flex;
    flex-direction: column;
    gap: var(--sp-2);
  }
  .qr-label {
    font-size: var(--sm);
    color: var(--t1);
  }
  .qr-url {
    font-family: var(--mono);
    font-size: var(--xs);
    color: var(--ac);
    background: var(--bg2);
    padding: var(--sp-2) var(--sp-3);
    border-radius: var(--radius-sm);
    word-break: break-all;
  }
  .qr-hint {
    font-size: var(--xs);
    color: var(--t3);
  }
</style>
