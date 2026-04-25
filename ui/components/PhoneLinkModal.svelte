<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import Modal from './shared/Modal.svelte';

  export let baseUrl = '';
  export let accessUrl = '';
  export let apiKey = '';
  export let qrSvg = '';
  export let generating = false;

  const dispatch = createEventDispatcher<{ close: void; rotate: void }>();

  let copiedKey = false;
  let copiedLink = false;

  async function copyKey() {
    if (!apiKey) return;
    await navigator.clipboard.writeText(apiKey);
    copiedKey = true;
    setTimeout(() => (copiedKey = false), 2000);
  }

  async function copyLink() {
    if (!accessUrl) return;
    await navigator.clipboard.writeText(accessUrl);
    copiedLink = true;
    setTimeout(() => (copiedLink = false), 2000);
  }
</script>

<Modal title="Phone Link" width="560px" zIndex={220} on:close={() => dispatch('close')}>
  <div class="beta-banner">
    <span class="beta-pill">mobile beta</span>
    <p>Phone access is in testing. Some screens and actions may not work as expected yet.</p>
  </div>

  <div class="hero">
    <div>
      <h2>Scan on your phone</h2>
      <p>Use the QR code on the same Wi-Fi network, or copy the access link and key manually.</p>
    </div>
    <code class="base-url">{baseUrl}</code>
  </div>

  <div class="qr-shell">
    <div class="qr-code">
      {@html qrSvg}
    </div>

    <div class="qr-copy">
      <span class="qr-label">same Wi-Fi network required</span>
      <div class="link-card">
        <span class="card-label">access link</span>
        <code>{accessUrl}</code>
      </div>
      <div class="link-card">
        <span class="card-label">access key</span>
        <code>{apiKey}</code>
      </div>
    </div>
  </div>

  <div class="actions">
    <button class="btn small" on:click={copyLink}>
      {copiedLink ? 'link copied' : 'copy link'}
    </button>
    <button class="btn small" on:click={copyKey}>
      {copiedKey ? 'key copied' : 'copy key'}
    </button>
    <button class="btn small ghost" on:click={() => dispatch('rotate')} disabled={generating}>
      {generating ? 'rotating...' : 'rotate link'}
    </button>
  </div>
</Modal>

<style>
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
  .hero {
    display: flex;
    justify-content: space-between;
    gap: var(--sp-4);
    align-items: flex-start;
  }
  .hero h2 {
    margin: 0 0 6px;
    font-size: 24px;
    line-height: 1.1;
    color: var(--t0);
  }
  .hero p {
    margin: 0;
    font-size: var(--sm);
    line-height: 1.5;
    color: var(--t2);
    max-width: 40ch;
  }
  .base-url {
    padding: var(--sp-2) var(--sp-3);
    border-radius: var(--radius-sm);
    background: var(--bg2);
    color: var(--ac);
    font-family: var(--mono);
    font-size: var(--xs);
    word-break: break-all;
  }
  .qr-shell {
    display: flex;
    gap: var(--sp-6);
    align-items: flex-start;
    padding: var(--sp-5);
    background: var(--bg2);
    border: 1px solid var(--bd1);
    border-radius: var(--radius-md);
  }
  .qr-code {
    flex-shrink: 0;
    width: 220px;
    height: 220px;
    padding: 4px;
    border-radius: var(--radius-sm);
    background: #fff;
    overflow: hidden;
  }
  .qr-copy {
    display: flex;
    flex-direction: column;
    gap: var(--sp-3);
    min-width: 0;
  }
  .qr-label {
    font-size: var(--xs);
    color: var(--t3);
  }
  .link-card {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: var(--sp-3);
    border-radius: var(--radius-sm);
    background: var(--bg1);
    border: 1px solid var(--bd1);
  }
  .card-label {
    font-size: var(--xs);
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--t3);
  }
  .link-card code {
    font-family: var(--mono);
    font-size: var(--xs);
    color: var(--t0);
    word-break: break-all;
  }
  .actions {
    display: flex;
    gap: var(--sp-2);
    flex-wrap: wrap;
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
  }
  @media (max-width: 720px) {
    .hero,
    .qr-shell {
      flex-direction: column;
    }
    .qr-code {
      width: 180px;
      height: 180px;
    }
  }
</style>
