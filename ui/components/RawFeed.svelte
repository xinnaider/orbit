<script lang="ts">
  import { onMount, tick } from 'svelte';

  export let lines: string[] = [];

  let container: HTMLDivElement;
  let atBottom = true;

  onMount(() => {
    scrollToBottom();
  });

  $: if (lines.length > 0) {
    tick().then(() => {
      if (atBottom) scrollToBottom();
    });
  }

  function scrollToBottom() {
    if (container) container.scrollTop = container.scrollHeight;
  }

  function onScroll() {
    if (!container) return;
    const threshold = 40;
    atBottom = container.scrollHeight - container.scrollTop - container.clientHeight < threshold;
  }
  function escapeHtml(s: string): string {
    return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
  }

  function highlight(line: string): string {
    const safe = escapeHtml(line);
    return safe
      .replace(/(\{|\}|\[|\]|,)/g, '<span class="punct">$1</span>')
      .replace(/"([^"]+)":/g, '<span class="key">"$1"</span>:')
      .replace(/: "([^"]+)"/g, ': <span class="str">"$1"</span>')
      .replace(/: (-?\d+\.?\d*)/g, ': <span class="num">$1</span>')
      .replace(/: (true|false|null)/g, ': <span class="bool">$1</span>');
  }
</script>

<div class="raw-feed" bind:this={container} on:scroll={onScroll}>
  {#if lines.length === 0}
    <div class="empty">no output recorded</div>
  {:else}
    {#each lines as line, i}
      <pre class="raw-line" class:alt={i % 2 === 1}><code>{@html highlight(line)}</code></pre>
    {/each}
  {/if}
</div>

<style>
  .raw-feed {
    flex: 1;
    overflow-y: auto;
    padding: var(--sp-4) 0;
    font-family: var(--mono);
    font-size: 10px;
    line-height: 1.5;
  }
  .raw-line {
    margin: 0;
    padding: 1px var(--sp-6);
    white-space: pre-wrap;
    word-break: break-all;
  }
  .alt {
    background: rgba(255, 255, 255, 0.015);
  }
  .empty {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    font-size: var(--sm);
    color: var(--t3);
  }
  :global(.raw-line .punct) { color: var(--t2); }
  :global(.raw-line .key) { color: var(--ac); }
  :global(.raw-line .str) { color: var(--user-fg); }
  :global(.raw-line .num) { color: var(--warning, #f5a623); }
  :global(.raw-line .bool) { color: var(--s-input); }
</style>
