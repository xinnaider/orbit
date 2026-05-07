<script lang="ts">
  import { createEventDispatcher, onMount } from 'svelte';
  import {
    generateApiKey,
    listApiKeys,
    revokeApiKey,
    getHttpSettings,
    setHttpSettings,
    getLanIp,
    resetSessions,
  } from '../lib/tauri';
  import type { ApiKeyCreated, ApiKeyInfo } from '../lib/tauri';
  import Modal from './shared/Modal.svelte';
  import PhoneLinkModal from './PhoneLinkModal.svelte';
  import { generateQrSvg } from '../lib/qr';

  const dispatch = createEventDispatcher<{ close: void }>();
  const PHONE_LABEL_PREFIX = 'phone';

  let enabled = false;
  let host = '127.0.0.1';
  let port = 9999;
  let saving = false;
  let settingsChanged = false;
  let restartNeeded = false;

  let keys: ApiKeyInfo[] = [];
  let generatingKey = false;
  let justCreatedKey: ApiKeyCreated | null = null;
  let lanIp = '';
  let qrSvg = '';
  let advancedLabel = '';
  let showPhoneLinkModal = false;

  let resetting = false;
  let confirmReset = false;
  let activeTab: 'mobile' | 'server' | 'danger' = 'mobile';

  $: connectHost =
    host === '127.0.0.1' || host === 'localhost' || host === '0.0.0.0' ? lanIp : host;
  $: baseUrl = enabled && connectHost ? `http://${connectHost}:${port}` : '';
  $: accessUrl = baseUrl && justCreatedKey ? `${baseUrl}?token=${justCreatedKey.key}` : '';
  $: connectionState = !enabled
    ? 'disabled'
    : settingsChanged
      ? 'pending'
      : restartNeeded
        ? 'restart'
        : 'ready';

  $: if (accessUrl && connectionState === 'ready') {
    generateQrSvg(accessUrl, 200).then((svg) => (qrSvg = svg));
  } else {
    qrSvg = '';
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
      showPhoneLinkModal = false;
    } finally {
      saving = false;
    }
  }

  function markChanged() {
    settingsChanged = true;
    showPhoneLinkModal = false;
  }

  async function createKey(label: string) {
    if (!label.trim()) return;
    generatingKey = true;
    try {
      justCreatedKey = await generateApiKey(label.trim());
      advancedLabel = '';
      showPhoneLinkModal = true;
      keys = await listApiKeys();
    } finally {
      generatingKey = false;
    }
  }

  function nextPhoneLabel() {
    const phoneKeys = keys.filter((key) =>
      key.label.toLowerCase().startsWith(PHONE_LABEL_PREFIX)
    ).length;
    return `${PHONE_LABEL_PREFIX}-${phoneKeys + 1}`;
  }

  async function createPhoneLink() {
    await createKey(nextPhoneLabel());
  }

  async function deleteKey(id: string) {
    await revokeApiKey(id);
    keys = await listApiKeys();
    if (justCreatedKey?.id === id) {
      justCreatedKey = null;
      showPhoneLinkModal = false;
    }
  }

  async function handleReset() {
    resetting = true;
    try {
      await resetSessions();
      confirmReset = false;
      dispatch('close');
    } finally {
      resetting = false;
    }
  }
</script>

<Modal title="Settings" width="620px" zIndex={200} on:close={() => dispatch('close')}>
  <div class="tabs">
    <button
      class="tab"
      class:active={activeTab === 'mobile'}
      on:click={() => (activeTab = 'mobile')}>mobile access</button
    >
    <button
      class="tab"
      class:active={activeTab === 'server'}
      on:click={() => (activeTab = 'server')}>server & keys</button
    >
    <button
      class="tab"
      class:active={activeTab === 'danger'}
      on:click={() => (activeTab = 'danger')}>danger zone</button
    >
  </div>

  {#if activeTab === 'mobile'}
    <div class="tab-content">
      <div class="beta-banner">
        <span class="beta-pill">beta</span>
        <p>
          Phone access is still being tested. Some screens and actions may not work as expected yet.
        </p>
      </div>

      <div class="phone-grid">
        <div class="phone-card">
          <div class="card-head">
            <span class="step-number">1</span>
            <div>
              <div class="card-title">Enable web access</div>
              <p class="card-copy">Turn on the web server so your phone can connect.</p>
            </div>
          </div>

          <div class="status-badge status-{connectionState}">
            {#if connectionState === 'disabled'}
              web access off
            {:else if connectionState === 'pending'}
              save settings
            {:else if connectionState === 'restart'}
              restart required
            {:else}
              ready to connect
            {/if}
          </div>

          <div class="toggle-row">
            <div>
              <label class="label" for="http-enabled">allow other devices</label>
              <div class="subtle">
                {#if baseUrl}
                  current address: <code>{baseUrl}</code>
                {:else}
                  default address uses your local network IP on port {port}
                {/if}
              </div>
            </div>
            <label class="toggle-wrap">
              <input
                id="http-enabled"
                type="checkbox"
                bind:checked={enabled}
                on:change={markChanged}
              />
              <span class="toggle-track"><span class="toggle-thumb"></span></span>
            </label>
          </div>

          {#if settingsChanged}
            <button class="btn primary" on:click={saveSettings} disabled={saving}>
              {saving ? 'saving...' : 'save'}
            </button>
          {/if}
        </div>

        <div class="phone-card">
          <div class="card-head">
            <span class="step-number">2</span>
            <div>
              <div class="card-title">Generate phone link</div>
              <p class="card-copy">Create a link and scan the QR code on the same network.</p>
            </div>
          </div>

          {#if connectionState === 'disabled'}
            <div class="state-panel">
              <strong>Web access is off.</strong>
              <span>Enable it above first.</span>
            </div>
          {:else if connectionState === 'pending'}
            <div class="state-panel">
              <strong>Settings are waiting to be saved.</strong>
              <span>Save the server changes, then generate a phone link.</span>
            </div>
          {:else if connectionState === 'restart'}
            <div class="state-panel">
              <strong>Restart needed.</strong>
              <span>After restart, reopen Settings and generate a fresh phone link.</span>
            </div>
          {:else}
            <button class="btn primary" on:click={createPhoneLink} disabled={generatingKey}>
              {generatingKey ? 'preparing...' : 'prepare phone link'}
            </button>

            {#if justCreatedKey}
              <div class="link-ready">
                <strong>Phone link ready.</strong>
                <button class="btn small" on:click={() => (showPhoneLinkModal = true)}
                  >open QR modal</button
                >
              </div>
            {/if}
          {/if}
        </div>
      </div>
    </div>
  {/if}

  {#if activeTab === 'server'}
    <div class="tab-content">
      <div class="section">
        <span class="section-title">server</span>
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
      </div>

      <div class="divider"></div>

      <div class="section">
        <span class="section-title">API keys</span>
        <div class="create-key-row">
          <input
            class="input"
            bind:value={advancedLabel}
            placeholder="key label (e.g. laptop)"
            disabled={generatingKey}
            on:keydown={(e) => e.key === 'Enter' && createKey(advancedLabel)}
          />
          <button
            class="btn primary"
            on:click={() => createKey(advancedLabel)}
            disabled={generatingKey || !advancedLabel.trim()}
          >
            {generatingKey ? 'generating...' : 'generate'}
          </button>
        </div>
        {#if keys.length > 0}
          <div class="key-list">
            {#each keys as key (key.id)}
              <div class="key-item">
                <span class="key-label">{key.label}</span>
                <button class="btn ghost small" on:click={() => deleteKey(key.id)}>revoke</button>
              </div>
            {/each}
          </div>
        {:else}
          <span class="empty">no API keys</span>
        {/if}
      </div>
    </div>
  {/if}

  {#if activeTab === 'danger'}
    <div class="tab-content">
      <div class="section">
        <span class="section-title">danger zone</span>
        <p class="reset-copy">
          Reset wipes all sessions and outputs from the database. Running sessions are killed. It
          does not touch projects, API keys, or HTTP settings.
        </p>
        <button class="btn danger" on:click={() => (confirmReset = true)}>reset all sessions</button
        >
      </div>
    </div>
  {/if}
</Modal>

{#if confirmReset}
  <Modal
    title="Reset all sessions?"
    width="440px"
    zIndex={300}
    on:close={() => (confirmReset = false)}
  >
    <p class="reset-warn">
      All sessions and their outputs will be permanently deleted. Running sessions will be killed.
      This cannot be undone.
    </p>
    <div class="reset-actions">
      <button class="btn danger" on:click={handleReset} disabled={resetting}>
        {resetting ? 'resetting...' : 'yes, reset everything'}
      </button>
      <button class="btn" on:click={() => (confirmReset = false)}>cancel</button>
    </div>
  </Modal>
{/if}

{#if showPhoneLinkModal && justCreatedKey && accessUrl}
  <PhoneLinkModal
    {baseUrl}
    {accessUrl}
    apiKey={justCreatedKey.key}
    {qrSvg}
    generating={generatingKey}
    on:close={() => (showPhoneLinkModal = false)}
    on:rotate={createPhoneLink}
  />
{/if}

<style>
  .tabs {
    display: flex;
    gap: 0;
    border-bottom: 1px solid var(--bd);
  }
  .tab {
    flex: 1;
    background: none;
    border: none;
    border-bottom: 2px solid transparent;
    color: var(--t2);
    font-size: var(--xs);
    letter-spacing: 0.08em;
    text-transform: uppercase;
    padding: var(--sp-4) var(--sp-5);
    cursor: pointer;
    transition: all 0.12s;
    font-family: inherit;
  }
  .tab:hover {
    color: var(--t1);
  }
  .tab.active {
    color: var(--ac);
    border-bottom-color: var(--ac);
  }

  .tab-content {
    display: flex;
    flex-direction: column;
    gap: var(--sp-6);
  }

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

  .status-badge {
    align-self: flex-start;
    padding: var(--sp-2) var(--sp-4);
    border-radius: 999px;
    font-size: var(--xs);
    letter-spacing: 0.08em;
    text-transform: uppercase;
    border: 1px solid var(--bd1);
    background: var(--bg2);
    color: var(--t1);
  }
  .status-disabled {
    color: var(--t2);
  }
  .status-pending,
  .status-restart {
    border-color: rgba(245, 166, 35, 0.28);
    background: rgba(245, 166, 35, 0.08);
    color: var(--warning, #f5a623);
  }
  .status-ready {
    border-color: var(--ac);
    background: var(--ac-d);
    color: var(--ac);
  }

  .beta-banner {
    display: flex;
    gap: var(--sp-4);
    align-items: flex-start;
    padding: var(--sp-4) var(--sp-5);
    border-radius: var(--radius-md);
    border: 1px solid rgba(245, 166, 35, 0.24);
    background: rgba(245, 166, 35, 0.08);
  }
  .beta-banner p {
    margin: 0;
    font-size: var(--xs);
    color: var(--t1);
    line-height: 1.5;
  }
  .beta-pill {
    flex-shrink: 0;
    padding: 3px 8px;
    border-radius: 999px;
    font-size: 9px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    background: rgba(245, 166, 35, 0.14);
    color: var(--warning, #f5a623);
  }

  .phone-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--sp-5);
  }
  .phone-card {
    display: flex;
    flex-direction: column;
    gap: var(--sp-5);
    padding: var(--sp-6);
    border-radius: var(--radius-md);
    background: linear-gradient(180deg, rgba(255, 255, 255, 0.02), transparent) var(--bg2);
    border: 1px solid var(--bd1);
  }
  .card-head {
    display: flex;
    gap: var(--sp-4);
    align-items: flex-start;
  }
  .step-number {
    width: 28px;
    height: 28px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border-radius: 999px;
    background: var(--bg3);
    color: var(--t0);
    font-size: var(--sm);
    flex-shrink: 0;
  }
  .card-title {
    font-size: var(--md);
    color: var(--t0);
    margin-bottom: 4px;
  }
  .card-copy {
    margin: 0;
    font-size: var(--xs);
    line-height: 1.5;
    color: var(--t2);
  }

  .state-panel {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: var(--sp-4);
    background: var(--bg1);
    border: 1px dashed var(--bd2);
    border-radius: var(--radius-sm);
  }
  .state-panel strong {
    font-size: var(--sm);
    color: var(--t0);
  }
  .state-panel span {
    font-size: var(--xs);
    color: var(--t2);
    line-height: 1.5;
  }

  .toggle-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--sp-4);
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

  .subtle {
    margin-top: 6px;
    font-size: var(--xs);
    color: var(--t3);
    line-height: 1.5;
  }
  .subtle code {
    color: var(--t1);
    font-family: var(--mono);
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
    font-family: var(--mono);
    box-sizing: border-box;
  }
  .input:focus {
    border-color: var(--bd2);
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

  .divider {
    border-top: 1px solid var(--bd1);
  }

  .warn {
    font-size: var(--xs);
    color: var(--warning, #f5a623);
    padding: var(--sp-2) var(--sp-3);
    background: rgba(245, 166, 35, 0.08);
    border-radius: var(--radius-sm);
  }

  .btn {
    background: none;
    border: 1px solid var(--bd1);
    border-radius: var(--radius-sm);
    color: var(--t1);
    font-size: var(--sm);
    padding: var(--sp-3) var(--sp-7);
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
  .btn.danger {
    border-color: rgba(239, 68, 68, 0.3);
    color: var(--error, #ef4444);
  }
  .btn.danger:hover {
    background: rgba(239, 68, 68, 0.1);
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

  .link-ready {
    display: flex;
    align-items: center;
    gap: var(--sp-3);
    font-size: var(--sm);
    color: var(--t1);
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
  .key-label {
    font-size: var(--sm);
    color: var(--t0);
  }
  .empty {
    font-size: var(--xs);
    color: var(--t3);
    text-align: center;
    padding: var(--sp-4);
  }

  .reset-copy {
    margin: 0;
    font-size: var(--xs);
    color: var(--t2);
    line-height: 1.5;
  }
</style>
