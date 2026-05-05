<script lang="ts">
  import { createEventDispatcher, onDestroy, onMount } from 'svelte';

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
  let editor: import('monaco-editor').editor.IStandaloneDiffEditor | null = null;
  let monaco: typeof import('monaco-editor') | null = null;
  let _dirty = false;
  let _contentListener: (() => void) | null = null;
  let _autoSaveTimer: ReturnType<typeof setTimeout> | null = null;

  function setModel() {
    if (!editor || !monaco) return;
    cancelAutoSave();
    if (_contentListener) _contentListener();
    _contentListener = null;

    const current = editor.getModel();
    current?.original.dispose();
    current?.modified.dispose();
    editor.setModel({
      original: monaco.editor.createModel(original, language),
      modified: monaco.editor.createModel(modified, language),
    });
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
    }).dispose;
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
    if (!editor || !monaco) return;
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

  /** Public: mark as saved (clear dirty indicator + cancel pending auto-save). */
  export function markSaved() {
    cancelAutoSave();
    _dirty = false;
    dispatch('dirty', { dirty: false });
  }

  onMount(async () => {
    monaco = await import('monaco-editor');
    monaco.editor.setTheme('vs-dark');
    editor = monaco.editor.createDiffEditor(host, {
      automaticLayout: true,
      readOnly: !editable,
      minimap: { enabled: false },
      scrollBeyondLastLine: false,
    });
    setModel();
    setupKeybinding();
  });

  // Recreate models only when content actually changes
  let _prevOriginal: string | undefined;
  let _prevModified: string | undefined;
  $: if (editor && monaco && (original !== _prevOriginal || modified !== _prevModified)) {
    _prevOriginal = original;
    _prevModified = modified;
    cancelAutoSave();
    setModel();
  }

  // Toggle readOnly on editable change — do NOT recreate models
  $: if (editor && monaco) {
    editor.updateOptions({ readOnly: !editable });
    if (!editable && _dirty) {
      cancelAutoSave();
      _dirty = false;
      dispatch('dirty', { dirty: false });
    }
  }

  // Focus the modified editor when edit mode is enabled
  $: if (editor && monaco && editable) {
    requestAnimationFrame(() => {
      editor?.getModifiedEditor()?.focus();
    });
  }

  $: if (autoSave && !editable) {
    cancelAutoSave();
  }

  onDestroy(() => {
    cancelAutoSave();
    if (_contentListener) _contentListener();
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
