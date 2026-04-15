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
  .md :global(p) {
    margin: 0 0 var(--sp-2) 0;
  }
  .md :global(p:last-child) {
    margin-bottom: 0;
  }
  .md :global(p:only-child) {
    margin: 0;
  }
  .md :global(strong) {
    font-weight: 700;
    color: var(--t0);
  }
  .md :global(em) {
    font-style: italic;
  }
  .md :global(code) {
    font-family: 'Cascadia Code', 'Fira Code', monospace;
    background: var(--bg3);
    padding: 1px var(--sp-3);
    border-radius: var(--radius-sm);
    font-size: 0.9em;
  }
  .md :global(pre) {
    background: var(--bg3);
    border: 1px solid var(--bd1);
    padding: var(--sp-4) var(--sp-5);
    border-radius: var(--radius-md);
    overflow-x: auto;
    margin: var(--sp-3) 0;
  }
  .md :global(pre code) {
    background: none;
    padding: 0;
  }
  .md :global(ul),
  .md :global(ol) {
    margin: var(--sp-2) 0;
    padding-left: var(--sp-9);
  }
  .md :global(li) {
    margin: var(--sp-1) 0;
  }
  .md :global(h1),
  .md :global(h2),
  .md :global(h3) {
    font-weight: 700;
    margin: var(--sp-4) 0 var(--sp-2) 0;
    color: var(--t0);
  }
  .md :global(h1) {
    font-size: 1.2em;
  }
  .md :global(h2) {
    font-size: 1.1em;
  }
  .md :global(h3) {
    font-size: 1em;
  }
  .md :global(blockquote) {
    border-left: 3px solid var(--bd2);
    padding-left: var(--sp-5);
    margin: var(--sp-2) 0;
    color: var(--t1);
  }
  .md :global(a) {
    color: var(--ac);
    text-decoration: none;
  }
  .md :global(a:hover) {
    text-decoration: underline;
  }
  .md :global(hr) {
    border: none;
    border-top: 1px solid var(--bd1);
    margin: var(--sp-4) 0;
  }
  .md :global(table) {
    border-collapse: collapse;
    width: 100%;
    margin: var(--sp-3) 0;
    font-size: 0.9em;
  }
  .md :global(th),
  .md :global(td) {
    border: 1px solid var(--bd2);
    padding: var(--sp-3) var(--sp-5);
    text-align: left;
  }
  .md :global(th) {
    background: var(--bg3);
    font-weight: 600;
    color: var(--t0);
  }
  .md :global(tr:nth-child(even) td) {
    background: var(--bg2);
  }
</style>
