<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { open } from '@tauri-apps/plugin-dialog';
  import { createSession } from '../lib/tauri';
  import type { CreateSessionOptions } from '../lib/tauri';

  const dispatch = createEventDispatcher();

  let projectPath = '';
  let prompt = '';
  let model = 'auto';
  let permissionMode: 'ignore' | 'approve' = 'ignore';
  let sessionName = '';
  let loading = false;
  let error = '';

  const models = [
    { value: 'auto', label: 'Auto (Claude default)' },
    { value: 'claude-sonnet-4-6', label: 'Sonnet 4.6' },
    { value: 'claude-opus-4-6', label: 'Opus 4.6' },
    { value: 'claude-haiku-4-5-20251001', label: 'Haiku 4.5' },
  ];

  async function handleSubmit() {
    if (!projectPath.trim() || !prompt.trim()) {
      error = 'Project path and prompt are required.';
      return;
    }
    loading = true;
    error = '';
    try {
      const opts: CreateSessionOptions = {
        projectPath: projectPath.trim(),
        prompt: prompt.trim(),
        model: model === 'auto' ? undefined : model,
        permissionMode,
        sessionName: sessionName.trim() || undefined,
      };
      await createSession(opts);
      dispatch('created');
    } catch (e: any) {
      error = e?.message ?? String(e);
    } finally {
      loading = false;
    }
  }

  async function pickFolder() {
    const selected = await open({ directory: true, multiple: false });
    if (selected && typeof selected === 'string') {
      projectPath = selected;
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') dispatch('cancel');
  }
</script>

<svelte:window on:keydown={handleKeydown} />

<div class="overlay" on:click|self={() => dispatch('cancel')}>
  <div class="dialog">
    <h2>New Session</h2>

    <label>
      Project Path
      <div class="path-row">
        <input
          type="text"
          bind:value={projectPath}
          placeholder="C:\Users\user\my-project"
          disabled={loading}
        />
        <button class="browse-btn" type="button" on:click={pickFolder} disabled={loading} title="Browse folder">
          📁
        </button>
      </div>
    </label>

    <label>
      Prompt
      <textarea
        bind:value={prompt}
        placeholder="What should Claude work on?"
        rows="4"
        disabled={loading}
      ></textarea>
    </label>

    <label>
      Session Name (optional)
      <input
        type="text"
        bind:value={sessionName}
        placeholder="e.g. Fix auth bug"
        disabled={loading}
      />
    </label>

    <label>
      Model
      <select bind:value={model} disabled={loading}>
        {#each models as m}
          <option value={m.value}>{m.label}</option>
        {/each}
      </select>
    </label>

    <label class="permission-row">
      <span>Approve tool calls</span>
      <input
        type="checkbox"
        checked={permissionMode === 'approve'}
        on:change={e => permissionMode = e.currentTarget.checked ? 'approve' : 'ignore'}
        disabled={loading}
      />
    </label>

    {#if error}
      <p class="error">{error}</p>
    {/if}

    <div class="actions">
      <button on:click={() => dispatch('cancel')} disabled={loading}>Cancel</button>
      <button class="primary" on:click={handleSubmit} disabled={loading}>
        {loading ? 'Starting…' : 'Start Session'}
      </button>
    </div>
  </div>
</div>

<style>
  .overlay {
    position: fixed; inset: 0;
    background: rgba(0,0,0,0.6);
    display: flex; align-items: center; justify-content: center;
    z-index: 100;
  }
  .dialog {
    background: var(--bg-surface, #1e1e1e);
    border: 1px solid var(--border, #333);
    border-radius: 8px;
    padding: 24px;
    width: 480px;
    max-width: 90vw;
    display: flex; flex-direction: column; gap: 14px;
  }
  h2 { margin: 0; font-size: 1rem; font-weight: 600; }
  label { display: flex; flex-direction: column; gap: 4px; font-size: 0.8rem; color: #aaa; }
  input, textarea, select {
    background: var(--bg-input, #2a2a2a);
    border: 1px solid var(--border, #333);
    border-radius: 4px;
    color: inherit;
    font-size: 0.85rem;
    padding: 6px 8px;
  }
  .path-row {
    display: flex;
    gap: 6px;
    align-items: center;
  }
  .path-row input { flex: 1; }
  .browse-btn {
    background: var(--bg-overlay);
    border: 1px solid var(--border);
    border-radius: 4px;
    color: inherit;
    font-size: 14px;
    padding: 5px 8px;
    cursor: pointer;
    flex-shrink: 0;
  }
  .browse-btn:hover { background: var(--bg-hover); }
  .browse-btn:disabled { opacity: 0.5; cursor: not-allowed; }
  .permission-row { flex-direction: row; align-items: center; justify-content: space-between; }
  .error { color: #f87171; font-size: 0.8rem; margin: 0; }
  .actions { display: flex; justify-content: flex-end; gap: 8px; margin-top: 4px; }
  button { padding: 6px 16px; border-radius: 4px; border: 1px solid #444; background: #2a2a2a; color: inherit; cursor: pointer; }
  button.primary { background: #3b82f6; border-color: #3b82f6; color: white; }
  button:disabled { opacity: 0.5; cursor: not-allowed; }
</style>
