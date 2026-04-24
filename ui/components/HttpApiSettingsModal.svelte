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
</script>

<Modal title="Connect Phone" width="680px" zIndex={200} on:close={() => dispatch('close')}>
  <div class="hero">
    <div class="hero-copy">
      <span class="hero-kicker">mobile access</span>
      <h2>Open Orbit on your phone with one guided flow.</h2>
      <p>
        Turn on web access, scan the QR code on the same Wi-Fi network, and keep API management in
        the advanced section below.
      </p>
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
  </div>

  <div class="beta-banner">
    <span class="beta-pill">mobile beta</span>
    <p>Phone access is still being tested. Some screens and actions may not work as expected yet.</p>
  </div>

  <div class="flow-grid">
    <section class="flow-card">
      <div class="card-head">
        <span class="step-number">1</span>
        <div>
          <div class="card-title">Enable web access</div>
          <p class="card-copy">Orbit needs its web server enabled before another device can join.</p>
        </div>
      </div>

      <div class="toggle-row">
        <div>
          <label class="label" for="http-enabled">allow Orbit on other devices</label>
          <div class="subtle">
            {#if baseUrl}
              current address: <code>{baseUrl}</code>
            {:else}
              default address uses your local network IP on port {port}
            {/if}
          </div>
        </div>
        <label class="toggle-wrap">
          <input id="http-enabled" type="checkbox" bind:checked={enabled} on:change={markChanged} />
          <span class="toggle-track"><span class="toggle-thumb"></span></span>
        </label>
      </div>

      {#if settingsChanged}
        <div class="action-row">
          <button class="btn primary" on:click={saveSettings} disabled={saving}>
            {saving ? 'saving...' : 'save and continue'}
          </button>
          <span class="subtle">Save once after changing web access, host, or port.</span>
        </div>
      {/if}

      {#if restartNeeded}
        <div class="info">
          Restart Orbit before scanning on your phone. The web server changes only apply after
          restart.
        </div>
      {/if}
    </section>

    <section class="flow-card">
      <div class="card-head">
        <span class="step-number">2</span>
        <div>
          <div class="card-title">Generate a phone link</div>
          <p class="card-copy">Create a fresh access link, then scan the QR code on the same network.</p>
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
          <span>After restart, reopen this screen and generate a fresh phone link.</span>
        </div>
      {:else}
        <div class="quick-actions">
          <button class="btn primary" on:click={createPhoneLink} disabled={generatingKey}>
            {generatingKey ? 'preparing...' : justCreatedKey ? 'generate another link' : 'prepare phone link'}
          </button>
          <span class="subtle">QR code and access key open in a separate modal after generation.</span>
        </div>

        {#if justCreatedKey}
          <div class="link-ready">
            <div class="link-ready-copy">
              <strong>Phone link ready.</strong>
              <span>Open the QR modal to scan, copy the key, or rotate the link.</span>
            </div>
            <button class="btn small" on:click={() => (showPhoneLinkModal = true)}>open QR modal</button>
          </div>
        {/if}
      {/if}
    </section>
  </div>

  <details class="advanced">
    <summary>Advanced settings and API keys</summary>

    <div class="advanced-body">
      <div class="section">
        <span class="section-title">server settings</span>

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
        <span class="section-title">manual API keys</span>

        <div class="create-key-row">
          <input
            class="input"
            bind:value={advancedLabel}
            placeholder="key label (e.g. laptop, ssh-server)"
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
    </div>
  </details>

  {#if showPhoneLinkModal && justCreatedKey && accessUrl}
    <PhoneLinkModal
      baseUrl={baseUrl}
      accessUrl={accessUrl}
      apiKey={justCreatedKey.key}
      {qrSvg}
      generating={generatingKey}
      on:close={() => (showPhoneLinkModal = false)}
      on:rotate={createPhoneLink}
    />
  {/if}
</Modal>

<style>
  .hero {
    display: flex;
    justify-content: space-between;
    gap: var(--sp-6);
    align-items: flex-start;
  }
  .hero-copy {
    display: flex;
    flex-direction: column;
    gap: var(--sp-2);
  }
  .hero-kicker {
    font-size: var(--xs);
    color: var(--t3);
    letter-spacing: 0.12em;
    text-transform: uppercase;
  }
  .hero h2 {
    margin: 0;
    font-size: 24px;
    line-height: 1.1;
    color: var(--t0);
  }
  .hero p {
    margin: 0;
    font-size: var(--sm);
    line-height: 1.6;
    color: var(--t2);
    max-width: 52ch;
  }
  .status-badge {
    flex-shrink: 0;
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
    font-size: var(--sm);
    color: var(--t1);
    line-height: 1.5;
  }
  .beta-pill {
    flex-shrink: 0;
    padding: 5px 8px;
    border-radius: 999px;
    font-size: 10px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    background: rgba(245, 166, 35, 0.14);
    color: var(--warning, #f5a623);
  }
  .flow-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: var(--sp-5);
  }
  .flow-card {
    display: flex;
    flex-direction: column;
    gap: var(--sp-5);
    padding: var(--sp-6);
    border-radius: var(--radius-md);
    background: linear-gradient(180deg, rgba(255, 255, 255, 0.02), rgba(255, 255, 255, 0))
      var(--bg2);
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
  .toggle-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--sp-4);
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
    box-sizing: border-box;
  }
  .input:focus {
    border-color: var(--bd2);
  }
  .input:disabled {
    opacity: 0.5;
  }
  .divider {
    border-top: 1px solid var(--bd1);
    margin: var(--sp-1) 0;
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
  .action-row {
    display: flex;
    gap: var(--sp-3);
    align-items: center;
    flex-wrap: wrap;
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
  .quick-actions {
    display: flex;
    flex-direction: column;
    gap: var(--sp-3);
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
  .link-ready {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--sp-3);
    padding: var(--sp-4);
    background: var(--bg1);
    border: 1px solid var(--bd1);
    border-radius: var(--radius-sm);
  }
  .link-ready-copy {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .link-ready-copy strong {
    font-size: var(--sm);
    color: var(--t0);
  }
  .link-ready-copy span {
    font-size: var(--xs);
    color: var(--t2);
    line-height: 1.5;
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
  .advanced {
    border-top: 1px solid var(--bd1);
    padding-top: var(--sp-5);
  }
  .advanced summary {
    cursor: pointer;
    list-style: none;
    font-size: var(--sm);
    color: var(--t1);
  }
  .advanced summary::-webkit-details-marker {
    display: none;
  }
  .advanced-body {
    display: flex;
    flex-direction: column;
    gap: var(--sp-5);
    margin-top: var(--sp-5);
  }
  @media (max-width: 780px) {
    .hero {
      flex-direction: column;
    }
    .status-badge {
      align-self: flex-start;
    }
    .flow-grid {
      grid-template-columns: 1fr;
    }
    .qr-code {
      width: 180px;
      height: 180px;
    }
    .field-row,
    .create-key-row {
      flex-direction: column;
    }
    .field-port {
      flex: 1;
    }
    .link-ready {
      flex-direction: column;
      align-items: flex-start;
    }
  }
</style>
