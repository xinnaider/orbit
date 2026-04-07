<script lang="ts">
  import '../app.css';
  import type { Snippet } from 'svelte';

  let { children }: { children: Snippet } = $props();

  // Disable browser native context menu globally — we handle it ourselves per-element
  function preventNativeContextMenu(e: MouseEvent) {
    e.preventDefault();
  }
</script>

<svelte:window on:contextmenu={preventNativeContextMenu} />

<div id="app">
  {@render children()}
</div>
