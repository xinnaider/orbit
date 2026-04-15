<script lang="ts">
  import Markdown from './Markdown.svelte';
  import Modal from './shared/Modal.svelte';

  export let changelogContent: string;
  export let currentVersion: string;
  export let onClose: () => void;
</script>

<Modal
  width="480px"
  zIndex={600}
  overlayBg="rgba(0, 0, 0, 0.6)"
  modalStyle="max-height: 520px; background: var(--bg2); border-color: var(--bd2); overflow: hidden; box-shadow: 0 8px 32px rgba(0,0,0,0.6); gap: 0; padding: 0;"
  on:close={onClose}
>
  <div class="modal-header">
    <div class="modal-title">
      <span class="title-text">what's new in orbit</span>
      <span class="version-badge">v{currentVersion}</span>
    </div>
    <button class="close-btn" on:click={onClose} aria-label="Close">✕</button>
  </div>
  <div class="modal-body">
    <div class="current-badge">● current version — v{currentVersion}</div>
    <Markdown content={changelogContent} />
  </div>
</Modal>

<style>
  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--sp-6) var(--sp-8);
    border-bottom: 1px solid var(--bd);
    flex-shrink: 0;
  }
  .modal-title {
    display: flex;
    align-items: center;
    gap: var(--sp-4);
  }
  .title-text {
    font-size: var(--md);
    color: var(--t0);
    font-weight: 500;
  }
  .version-badge {
    font-size: 10px;
    color: var(--ac);
    background: var(--ac-d);
    border: 1px solid rgba(0, 212, 126, 0.2);
    border-radius: var(--radius-sm);
    padding: var(--sp-1) 7px;
  }
  .close-btn {
    background: none;
    border: none;
    color: var(--t2);
    font-size: 13px;
    cursor: pointer;
    padding: var(--sp-1) var(--sp-2);
    line-height: 1;
    transition: color 0.15s;
  }
  .close-btn:hover {
    color: var(--t1);
  }
  .modal-body {
    flex: 1;
    overflow-y: auto;
    padding: var(--sp-8) var(--sp-9);
  }
  .modal-body::-webkit-scrollbar {
    width: 4px;
  }
  .modal-body::-webkit-scrollbar-track {
    background: transparent;
  }
  .modal-body::-webkit-scrollbar-thumb {
    background: var(--bd2);
    border-radius: var(--radius-sm);
  }
  .current-badge {
    font-size: 10px;
    color: var(--ac);
    margin-bottom: var(--sp-7);
  }
  /* Sobrescreve estilos do Markdown.svelte dentro do modal */
  .modal-body :global(h1) {
    display: none; /* esconde "# Changelog" do cabeçalho do arquivo */
  }
  .modal-body :global(h2) {
    font-size: 10px;
    color: var(--t2);
    text-transform: uppercase;
    letter-spacing: 0.08em;
    margin: var(--sp-8) 0 var(--sp-5);
    font-weight: 500;
  }
  .modal-body :global(h3) {
    font-size: var(--sm);
    color: var(--t0);
    font-weight: 500;
    margin: var(--sp-5) 0 var(--sp-2);
  }
  .modal-body :global(p) {
    font-size: var(--sm);
    color: var(--t1);
    line-height: 1.6;
    margin-bottom: var(--sp-4);
  }
  .modal-body :global(hr) {
    border: none;
    border-top: 1px solid var(--bd);
    margin: var(--sp-6) 0;
  }
</style>
