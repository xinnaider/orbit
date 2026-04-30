<script lang="ts">
  import { onDestroy } from 'svelte';
  import { Handle, Position, NodeResizer } from '@xyflow/svelte';
  import { WebviewWindow } from '@tauri-apps/api/webviewWindow';
  import { resizeNode as persistResize } from '../../lib/stores/mesh/graph';
  import { addToast } from '../../lib/stores/toasts';

  type BrowserNodeData = {
    label: string;
    templateName: string;
    prePrompt: string; // used as initial URL
  };

  export let id: string;
  export let data: BrowserNodeData;

  let url = data.prePrompt && data.prePrompt.startsWith('http') ? data.prePrompt : 'https://';
  let iframeUrl = url;
  let inputUrl = url;

  let popup: WebviewWindow | null = null;
  let popupOpen = false;

  async function onResizeEnd(_e: unknown, params: { width: number; height: number }) {
    try {
      await persistResize(Number(id), params.width, params.height);
    } catch (e) {
      addToast({ type: 'error', message: `failed to persist size: ${e}`, autoDismiss: true });
    }
  }

  function normalizeUrl(raw: string): string {
    const trimmed = raw.trim();
    if (!trimmed) return '';
    if (/^https?:\/\//i.test(trimmed)) return trimmed;
    return `https://${trimmed}`;
  }

  async function openPopupAt(target: string) {
    // WebviewWindow has no navigate() API; tear down + rebuild for new URL.
    if (popup) {
      try {
        await popup.close();
      } catch {
        /* ignore */
      }
      popup = null;
    }
    const label = `mesh-browser-${id}-${Date.now()}`;
    try {
      popup = new WebviewWindow(label, {
        url: target,
        title: data.label || 'Browser',
        width: 1024,
        height: 768,
      });
      popupOpen = true;
      popup.once('tauri://destroyed', () => {
        popup = null;
        popupOpen = false;
      });
      popup.once('tauri://error', (e) => {
        addToast({ type: 'error', message: `popup error: ${e.payload}`, autoDismiss: true });
        popup = null;
        popupOpen = false;
      });
    } catch (e) {
      addToast({
        type: 'error',
        message: `failed to open browser window: ${e}`,
        autoDismiss: true,
      });
    }
  }

  async function go() {
    const u = normalizeUrl(inputUrl);
    if (!u) return;
    iframeUrl = u;
    url = u;
    if (popupOpen) await openPopupAt(u);
  }

  async function popOut() {
    const u = normalizeUrl(inputUrl);
    if (!u || u === 'https://') {
      addToast({ type: 'error', message: 'enter a url first', autoDismiss: true });
      return;
    }
    await openPopupAt(u);
  }

  async function popIn() {
    if (popup) {
      try {
        await popup.close();
      } catch {
        /* ignore — `tauri://destroyed` listener will reset state */
      }
    }
    popup = null;
    popupOpen = false;
  }

  onDestroy(() => {
    if (popup) {
      void popup.close();
      popup = null;
    }
  });
</script>

<div class="browser-node">
  <NodeResizer
    minWidth={320}
    minHeight={240}
    lineClass="resize-line"
    handleClass="resize-handle"
    {onResizeEnd}
  />
  <Handle id="top" type="source" position={Position.Top} />
  <Handle id="left" type="source" position={Position.Left} />
  <Handle id="right" type="source" position={Position.Right} />
  <Handle id="bottom" type="source" position={Position.Bottom} />
  <header class="bn-header">
    <div class="bn-title">
      <span class="dot"></span>
      <strong>{data.label}</strong>
      <span class="kind-badge">browser</span>
    </div>
  </header>

  <div class="bn-address">
    <input
      class="bn-url"
      bind:value={inputUrl}
      on:keydown={(e) => e.key === 'Enter' && (e.preventDefault(), go())}
      placeholder="https://…"
    />
    <button class="bn-go" on:click={go} title="go">↵</button>
    {#if popupOpen}
      <button class="bn-popup" on:click={popIn} title="close floating window">⊟</button>
    {:else}
      <button
        class="bn-popup"
        on:click={popOut}
        title="open in floating window (works for sites that block iframes)">⊞</button
      >
    {/if}
  </div>

  <div class="bn-frame">
    {#if popupOpen}
      <div class="placeholder popup-active">
        <div>this browser is showing in a floating window.</div>
        <button class="popup-action" on:click={popIn}>close window</button>
      </div>
    {:else if iframeUrl && iframeUrl !== 'https://'}
      <iframe
        src={iframeUrl}
        title={data.label}
        sandbox="allow-same-origin allow-scripts allow-forms allow-popups allow-popups-to-escape-sandbox"
        referrerpolicy="no-referrer"
      ></iframe>
    {:else}
      <div class="placeholder">type a url above and press enter</div>
    {/if}
  </div>
</div>

<style>
  .browser-node {
    width: 100%;
    height: 100%;
    min-width: 320px;
    min-height: 240px;
    background: var(--bg);
    border: 1px solid var(--bd1);
    border-left: 3px solid var(--s-init);
    border-radius: var(--radius-md);
    display: flex;
    flex-direction: column;
    overflow: hidden;
    color: var(--t0);
    font-family: var(--mono);
  }

  .bn-header {
    display: flex;
    align-items: center;
    padding: var(--sp-3) var(--sp-5);
    background: var(--bg1);
    border-bottom: 1px solid var(--bd);
    flex-shrink: 0;
  }
  .bn-title {
    display: flex;
    align-items: center;
    gap: var(--sp-3);
    font-size: var(--sm);
  }
  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--s-init);
  }
  .kind-badge {
    font-size: var(--xs);
    color: var(--s-init);
    background: rgba(72, 136, 224, 0.12);
    padding: 1px 6px;
    border-radius: 10px;
    text-transform: lowercase;
    letter-spacing: 0.5px;
  }

  .bn-address {
    display: flex;
    gap: var(--sp-2);
    padding: var(--sp-3);
    background: var(--bg1);
    border-bottom: 1px solid var(--bd);
    flex-shrink: 0;
  }
  .bn-url {
    flex: 1;
    background: var(--bg);
    border: 1px solid var(--bd1);
    color: var(--t0);
    padding: var(--sp-2) var(--sp-4);
    font-size: var(--sm);
    border-radius: var(--radius-sm);
    font-family: inherit;
    outline: none;
  }
  .bn-url:focus {
    border-color: var(--ac);
  }
  .bn-go {
    background: var(--ac);
    color: #000;
    border: none;
    border-radius: var(--radius-sm);
    padding: 0 var(--sp-5);
    font-size: var(--sm);
    cursor: pointer;
    font-family: inherit;
  }
  .bn-popup {
    background: transparent;
    color: var(--t1);
    border: 1px solid var(--bd1);
    border-radius: var(--radius-sm);
    padding: 0 var(--sp-4);
    font-size: var(--sm);
    cursor: pointer;
    font-family: inherit;
  }
  .bn-popup:hover {
    border-color: var(--s-init);
    color: var(--s-init);
  }
  .placeholder.popup-active {
    flex-direction: column;
    gap: var(--sp-4);
    color: var(--t1);
    font-style: normal;
    background: var(--bg1);
  }
  .popup-action {
    background: transparent;
    color: var(--s-init);
    border: 1px solid var(--s-init);
    border-radius: var(--radius-sm);
    padding: var(--sp-2) var(--sp-5);
    font-size: var(--sm);
    cursor: pointer;
    font-family: inherit;
  }
  .popup-action:hover {
    background: rgba(72, 136, 224, 0.1);
  }

  .bn-frame {
    flex: 1;
    /* iframe surface stays white so light-themed embedded pages render right. */
    background: #fff;
    position: relative;
    overflow: hidden;
  }
  .bn-frame iframe {
    width: 100%;
    height: 100%;
    border: 0;
    display: block;
  }
  .placeholder {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--t2);
    font-size: var(--sm);
    font-style: italic;
    background: var(--bg2);
  }

  :global(.svelte-flow__node-browser .svelte-flow__handle) {
    background: var(--s-init);
    width: 24px;
    height: 24px;
    border: 2px solid var(--bg);
    z-index: 10;
  }
  :global(.svelte-flow__node-browser .svelte-flow__handle:hover) {
    background: var(--ac);
  }

  :global(.svelte-flow__node-browser .resize-handle) {
    width: 16px;
    height: 16px;
    background: var(--s-init);
    border: 2px solid var(--bg);
    border-radius: 3px;
  }
  :global(.svelte-flow__node-browser .resize-line) {
    border-color: var(--s-init);
    border-width: 3px;
  }
</style>
