<script lang="ts">
  import { onDestroy, onMount } from 'svelte';

  export let original: string;
  export let modified: string;
  export let language = 'plaintext';

  let host: HTMLDivElement;
  let editor: import('monaco-editor').editor.IStandaloneDiffEditor | null = null;
  let monaco: typeof import('monaco-editor') | null = null;

  function setModel() {
    if (!editor || !monaco) return;
    const current = editor.getModel();
    current?.original.dispose();
    current?.modified.dispose();
    editor.setModel({
      original: monaco.editor.createModel(original, language),
      modified: monaco.editor.createModel(modified, language),
    });
  }

  onMount(async () => {
    monaco = await import('monaco-editor');
    editor = monaco.editor.createDiffEditor(host, {
      automaticLayout: true,
      readOnly: true,
      minimap: { enabled: false },
      scrollBeyondLastLine: false,
    });
    setModel();
  });

  $: if (editor && monaco) setModel();

  onDestroy(() => {
    const current = editor?.getModel();
    current?.original.dispose();
    current?.modified.dispose();
    editor?.dispose();
  });
</script>

<div class="monaco-diff-host" bind:this={host}></div>

<style>
  .monaco-diff-host {
    width: 100%;
    height: 100%;
    min-height: 0;
  }
</style>
