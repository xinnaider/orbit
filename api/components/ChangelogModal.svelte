<script lang="ts">
  import Markdown from './Markdown.svelte';

  export let changelogContent: string;
  export let currentVersion: string;
  export let onClose: () => void;

  function handleOverlayClick(e: MouseEvent) {
    if (e.target === e.currentTarget) onClose();
  }
</script>

<div
  class="overlay"
  role="dialog"
  aria-modal="true"
  tabindex="-1"
  on:click={handleOverlayClick}
  on:keydown={(e) => e.key === 'Escape' && onClose()}
>
  <div class="modal">
    <div class="modal-header">
      <div class="modal-title">
        <span class="title-text">novidades do orbit</span>
        <span class="version-badge">v{currentVersion}</span>
      </div>
      <button class="close-btn" on:click={onClose} aria-label="Fechar">✕</button>
    </div>
    <div class="modal-body">
      <div class="current-badge">● versão atual — v{currentVersion}</div>
      <Markdown content={changelogContent} />
    </div>
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.6);
    z-index: 600;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .modal {
    width: 480px;
    max-height: 520px;
    background: var(--bg2);
    border: 1px solid var(--bd2);
    border-radius: 6px;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.6);
  }
  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 16px;
    border-bottom: 1px solid var(--bd);
    flex-shrink: 0;
  }
  .modal-title {
    display: flex;
    align-items: center;
    gap: 8px;
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
    border-radius: 3px;
    padding: 2px 7px;
  }
  .close-btn {
    background: none;
    border: none;
    color: var(--t2);
    font-size: 13px;
    cursor: pointer;
    padding: 2px 4px;
    line-height: 1;
    transition: color 0.15s;
  }
  .close-btn:hover {
    color: var(--t1);
  }
  .modal-body {
    flex: 1;
    overflow-y: auto;
    padding: 16px 20px;
  }
  .modal-body::-webkit-scrollbar {
    width: 4px;
  }
  .modal-body::-webkit-scrollbar-track {
    background: transparent;
  }
  .modal-body::-webkit-scrollbar-thumb {
    background: var(--bd2);
    border-radius: 2px;
  }
  .current-badge {
    font-size: 10px;
    color: var(--ac);
    margin-bottom: 14px;
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
    margin: 16px 0 10px;
    font-weight: 500;
  }
  .modal-body :global(h3) {
    font-size: var(--sm);
    color: var(--t0);
    font-weight: 500;
    margin: 10px 0 4px;
  }
  .modal-body :global(p) {
    font-size: var(--sm);
    color: var(--t1);
    line-height: 1.6;
    margin-bottom: 8px;
  }
  .modal-body :global(hr) {
    border: none;
    border-top: 1px solid var(--bd);
    margin: 12px 0;
  }
</style>
