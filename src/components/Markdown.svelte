<script lang="ts">
  import { marked } from 'marked';

  export let content: string;

  // Configure marked for inline-friendly rendering
  marked.setOptions({
    breaks: false,
    gfm: true,
  });

  $: html = marked.parse(content || '') as string;
</script>

<div class="md">{@html html}</div>

<style>
  .md :global(p) { margin: 0 0 4px 0; }
  .md :global(p:last-child) { margin-bottom: 0; }
  .md :global(p:only-child) { margin: 0; }
  .md :global(strong) { font-weight: 700; color: var(--text-primary); }
  .md :global(em) { font-style: italic; }
  .md :global(code) {
    font-family: 'Cascadia Code', 'Fira Code', monospace;
    background: var(--bg-code);
    padding: 1px 5px;
    border-radius: 3px;
    font-size: 0.9em;
  }
  .md :global(pre) {
    background: var(--bg-code);
    padding: 8px 10px;
    border-radius: 6px;
    overflow-x: auto;
    margin: 6px 0;
  }
  .md :global(pre code) {
    background: none;
    padding: 0;
  }
  .md :global(ul), .md :global(ol) {
    margin: 4px 0;
    padding-left: 20px;
  }
  .md :global(li) { margin: 2px 0; }
  .md :global(h1), .md :global(h2), .md :global(h3) {
    font-weight: 700;
    margin: 8px 0 4px 0;
  }
  .md :global(h1) { font-size: 1.2em; }
  .md :global(h2) { font-size: 1.1em; }
  .md :global(h3) { font-size: 1em; }
  .md :global(blockquote) {
    border-left: 3px solid var(--border);
    padding-left: 10px;
    margin: 4px 0;
    color: var(--text-secondary);
  }
  .md :global(a) { color: var(--blue); text-decoration: none; }
  .md :global(hr) { border: none; border-top: 1px solid var(--border); margin: 8px 0; }
  .md :global(table) { border-collapse: collapse; width: 100%; margin: 6px 0; }
  .md :global(th), .md :global(td) {
    border: 1px solid var(--border);
    padding: 4px 8px;
    font-size: 0.9em;
  }
  .md :global(th) { background: var(--bg-overlay); font-weight: 600; }
</style>
