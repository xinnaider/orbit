<script lang="ts">
  import { createEventDispatcher, onDestroy, onMount } from 'svelte';
  import * as monaco from 'monaco-editor';
  import EditorWorker from 'monaco-editor/esm/vs/editor/editor.worker?worker';

  export let original: string;
  export let modified: string;
  export let language = 'plaintext';
  export let editable = false;
  export let autoSave = false;

  const dispatch = createEventDispatcher<{
    save: { content: string; auto: boolean };
    dirty: { dirty: boolean };
  }>();

  let host: HTMLDivElement;
  let editor: monaco.editor.IStandaloneDiffEditor | null = null;
  let editorReady = false;
  let _dirty = false;
  let _contentListener: monaco.IDisposable | null = null;
  let _autoSaveTimer: ReturnType<typeof setTimeout> | null = null;
  let _initFrame: ReturnType<typeof requestAnimationFrame> | null = null;

  function ensureMonacoWorkers() {
    const globalScope = self as unknown as {
      MonacoEnvironment?: {
        getWorker(_moduleId: string, _label: string): Worker;
      };
    };

    globalScope.MonacoEnvironment ??= {
      getWorker: () => new EditorWorker(),
    };
  }

  function disposeCurrentModels() {
    const current = editor?.getModel();
    current?.original.dispose();
    current?.modified.dispose();
  }

  function setModel() {
    if (!editor) return;
    cancelAutoSave();
    _contentListener?.dispose();
    _contentListener = null;
    _prevOriginal = original;
    _prevModified = modified;

    const previous = editor.getModel();
    editor.setModel({
      original: monaco.editor.createModel(original, language),
      modified: monaco.editor.createModel(modified, language),
    });
    previous?.original.dispose();
    previous?.modified.dispose();

    _dirty = false;
    dispatch('dirty', { dirty: false });

    // Track dirty state + trigger auto-save
    const mod = editor.getModifiedEditor();
    _contentListener = mod.onDidChangeModelContent(() => {
      const modModel = editor?.getModel()?.modified;
      if (!modModel) return;
      const isDirty = modModel.getValue() !== original;
      if (isDirty !== _dirty) {
        _dirty = isDirty;
        dispatch('dirty', { dirty: isDirty });
      }
      if (autoSave && editable && isDirty) {
        scheduleAutoSave();
      }
    });
  }

  function scheduleAutoSave() {
    cancelAutoSave();
    _autoSaveTimer = setTimeout(() => {
      const model = editor?.getModel()?.modified;
      if (model && _dirty) {
        dispatch('save', { content: model.getValue(), auto: true });
      }
    }, 1500);
  }

  function cancelAutoSave() {
    if (_autoSaveTimer !== null) {
      clearTimeout(_autoSaveTimer);
      _autoSaveTimer = null;
    }
  }

  function setupKeybinding() {
    if (!editor) return;
    const mod = editor.getModifiedEditor();
    mod.addAction({
      id: 'orbit-save-file',
      label: 'Save File',
      keybindings: [monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyS],
      run: () => {
        if (!editable) return;
        cancelAutoSave();
        const model = editor?.getModel()?.modified;
        if (model) {
          dispatch('save', { content: model.getValue(), auto: false });
        }
      },
    });
  }

  export function markSaved() {
    cancelAutoSave();
    _dirty = false;
    dispatch('dirty', { dirty: false });
  }

  onMount(() => {
    ensureMonacoWorkers();

    _initFrame = requestAnimationFrame(() => {
      _initFrame = null;
      if (!host) return;

      monaco.editor.setTheme('vs-dark');
      editor = monaco.editor.createDiffEditor(host, {
        automaticLayout: true,
        readOnly: !editable,
        minimap: { enabled: false },
        scrollBeyondLastLine: false,
        renderSideBySide: false,
        compactMode: true,
        renderOverviewRuler: false,
        renderIndicators: true,
        diffWordWrap: 'on',
      });
      setModel();
      setupKeybinding();
      editorReady = true;
    });
  });

  let _prevOriginal: string | undefined;
  let _prevModified: string | undefined;
  $: if (editor && (original !== _prevOriginal || modified !== _prevModified)) {
    _prevOriginal = original;
    _prevModified = modified;
    setModel();
  }

  $: if (editor) {
    editor.updateOptions({ readOnly: !editable });
    if (!editable && _dirty) {
      cancelAutoSave();
      _dirty = false;
      dispatch('dirty', { dirty: false });
    }
  }

  $: if (editor && editable) {
    requestAnimationFrame(() => {
      editor?.getModifiedEditor()?.focus();
    });
  }

  $: if (autoSave && !editable) {
    cancelAutoSave();
  }

  onDestroy(() => {
    if (_initFrame !== null) {
      cancelAnimationFrame(_initFrame);
      _initFrame = null;
    }
    cancelAutoSave();
    _contentListener?.dispose();
    _contentListener = null;
    disposeCurrentModels();
    editor?.dispose();
    editor = null;
  });
</script>

<div class="monaco-diff-shell">
  <div class="monaco-diff-host" bind:this={host}></div>
  {#if !editorReady}
    <div class="monaco-diff-loading">Loading diff editor...</div>
  {/if}
</div>

<style>
  .monaco-diff-shell {
    position: relative;
    width: 100%;
    height: 100%;
    min-height: 0;
  }

  .monaco-diff-host {
    width: 100%;
    height: 100%;
    min-height: 0;
  }

  .monaco-diff-loading {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    color: var(--muted, #9ca3af);
    background: var(--bg, #0f1115);
    font-size: 12px;
  }
</style>
