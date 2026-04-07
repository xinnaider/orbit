<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { open } from '@tauri-apps/plugin-dialog';
  import { createSession, diagnoseSpawn } from '../lib/tauri';
  import type { SpawnDiagnostic } from '../lib/tauri';

  const dispatch = createEventDispatcher();

  let path = '';
  let prompt = '';
  let model = 'auto';
  let loading = false;
  let error = '';
  let diagRunning = false;
  let diag: SpawnDiagnostic | null = null;

  let isRemote = false;
  let sshHost = '';
  let sshUser = 'ubuntu';

  async function runDiag() {
    diagRunning = true;
    try {
      diag = await diagnoseSpawn();
    } catch (e: any) {
      error = e?.message ?? String(e);
    } finally {
      diagRunning = false;
    }
  }

  const models = [
    { v: 'auto', l: 'auto' },
    { v: 'claude-sonnet-4-6', l: 'sonnet-4.6' },
    { v: 'claude-opus-4-6', l: 'opus-4.6' },
    { v: 'claude-haiku-4-5-20251001', l: 'haiku-4.5' },
  ];

  async function browse() {
    const sel = await open({ directory: true, multiple: false });
    if (sel && typeof sel === 'string') path = sel;
  }

  async function submit() {
    if (!path.trim()) {
      error = isRemote ? 'remote path required' : 'project path required';
      return;
    }
    if (isRemote && (!sshHost.trim() || !sshUser.trim())) {
      error = 'ssh host and user required';
      return;
    }
    loading = true;
    error = '';
    try {
      await createSession({
        projectPath: path.trim(),
        prompt: prompt.trim() || 'Hello',
        model: model === 'auto' ? undefined : model,
        permissionMode: 'ignore',
        ...(isRemote ? { sshHost: sshHost.trim(), sshUser: sshUser.trim() } : {}),
      });
      dispatch('done');
    } catch (e: any) {
      error = e?.message ?? String(e);
    } finally {
      loading = false;
    }
  }

  function onKey(e: KeyboardEvent) {
    if (e.key === 'Escape') dispatch('cancel');
  }
</script>

<svelte:window on:keydown={onKey} />

<div
  class="overlay"
  role="dialog"
  aria-modal="true"
  tabindex="-1"
  on:click|self={() => dispatch('cancel')}
  on:keydown={onKey}
>
  <div class="modal">
    <div class="modal-header">
      <span class="modal-title">new session</span>
      <button class="close" on:click={() => dispatch('cancel')}>✕</button>
    </div>

    <div class="field">
      <label class="label" for="ns-path">{isRemote ? 'remote path' : 'path'}</label>
      <div class="path-row">
        <input
          id="ns-path"
          class="input"
          bind:value={path}
          placeholder={isRemote ? '/home/ubuntu/project' : '/home/user/project'}
          disabled={loading}
          on:keydown={(e) => e.key === 'Enter' && prompt && submit()}
        />
        {#if !isRemote}
          <button class="browse" on:click={browse} disabled={loading} title="browse">⌘</button>
        {/if}
      </div>
    </div>

    <div class="field">
      <label class="label" for="ns-prompt">prompt</label>
      <textarea
        id="ns-prompt"
        class="input textarea"
        bind:value={prompt}
        placeholder="what should claude work on? (optional — leave blank to start interactively)"
        rows="3"
        disabled={loading}
        on:keydown={(e) => {
          if (e.key === 'Enter' && e.metaKey) submit();
        }}
      ></textarea>
    </div>

    <div class="row">
      <div class="field half">
        <label class="label" for="ns-model">model</label>
        <select id="ns-model" class="input select" bind:value={model} disabled={loading}>
          {#each models as m}
            <option value={m.v}>{m.l}</option>
          {/each}
        </select>
      </div>
    </div>

    <div class="field">
      <label
        class="label"
        for="ns-remote"
        style="flex-direction:row;align-items:center;gap:6px;cursor:pointer"
      >
        <input id="ns-remote" type="checkbox" bind:checked={isRemote} disabled={loading} />
        remote session (ssh)
      </label>
    </div>

    {#if isRemote}
      <div class="row">
        <div class="field half">
          <label class="label" for="ns-ssh-host">ssh host</label>
          <input
            id="ns-ssh-host"
            class="input"
            bind:value={sshHost}
            placeholder="vps.example.com"
            disabled={loading}
          />
        </div>
        <div class="field half">
          <label class="label" for="ns-ssh-user">ssh user</label>
          <input
            id="ns-ssh-user"
            class="input"
            bind:value={sshUser}
            placeholder="ubuntu"
            disabled={loading}
          />
        </div>
      </div>
    {/if}

    {#if error}
      <p class="error">! {error}</p>
    {/if}

    {#if diag}
      <div class="diag">
        <div class="diag-row" class:ok={diag.claudeFound} class:fail={!diag.claudeFound}>
          claude: {diag.claudeFound ? `✓ ${diag.claudePath ?? diag.whereOutput}` : '✗ not found'}
        </div>
        {#if diag.versionOutput}
          <div class="diag-row ok">version: {diag.versionOutput.slice(0, 60)}</div>
        {/if}
        {#if !diag.claudeFound}
          <div class="diag-row fail">install: npm install -g @anthropic-ai/claude-code</div>
          <div class="diag-row" style="font-size:9px;color:var(--t3)">
            PATH: {diag.augmentedPath.slice(0, 120)}
          </div>
        {/if}
      </div>
    {/if}

    <div class="actions">
      {#if !isRemote}
        <button class="btn ghost" on:click={runDiag} disabled={diagRunning || loading}>
          {diagRunning ? 'testing...' : '⚙ diagnose'}
        </button>
      {/if}
      <button class="btn ghost" on:click={() => dispatch('cancel')} disabled={loading}
        >cancel</button
      >
      <button class="btn primary" on:click={submit} disabled={loading || !path}>
        {loading ? 'spawning...' : 'start session'}
      </button>
    </div>
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    z-index: 100;
    background: rgba(0, 0, 0, 0.7);
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .modal {
    background: var(--bg1);
    border: 1px solid var(--bd1);
    border-radius: 4px;
    width: 480px;
    max-width: 94vw;
    display: flex;
    flex-direction: column;
    gap: 14px;
    padding: 20px;
  }
  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .modal-title {
    font-size: var(--md);
    color: var(--t1);
    letter-spacing: 0.06em;
  }
  .close {
    background: none;
    border: none;
    color: var(--t2);
    font-size: 12px;
    padding: 2px 4px;
  }
  .close:hover {
    color: var(--t0);
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 5px;
  }
  .label {
    font-size: var(--xs);
    color: var(--t2);
    letter-spacing: 0.06em;
  }
  .input {
    background: var(--bg2);
    border: 1px solid var(--bd1);
    border-radius: 3px;
    color: var(--t0);
    font-size: var(--md);
    padding: 6px 8px;
    outline: none;
    width: 100%;
    transition: border-color 0.15s;
  }
  .input:focus {
    border-color: var(--bd2);
  }
  .input:disabled {
    opacity: 0.5;
  }
  .textarea {
    resize: none;
    line-height: 1.5;
  }
  .select {
    appearance: none;
    cursor: pointer;
  }

  .path-row {
    display: flex;
    gap: 6px;
  }
  .path-row .input {
    flex: 1;
  }
  .browse {
    background: var(--bg2);
    border: 1px solid var(--bd1);
    color: var(--t1);
    border-radius: 3px;
    padding: 0 10px;
    font-size: var(--base);
    flex-shrink: 0;
  }
  .browse:hover {
    border-color: var(--bd2);
    color: var(--t0);
  }

  .row {
    display: flex;
    gap: 12px;
  }
  .half {
    flex: 1;
  }
  .error {
    font-size: var(--sm);
    color: var(--s-error);
  }
  .diag {
    background: var(--bg3);
    border: 1px solid var(--bd1);
    border-radius: 3px;
    padding: 8px 10px;
    display: flex;
    flex-direction: column;
    gap: 3px;
  }
  .diag-row {
    font-size: var(--xs);
    color: var(--t1);
  }
  .diag-row.ok {
    color: var(--s-working);
  }
  .diag-row.fail {
    color: var(--s-error);
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }
  .btn {
    background: none;
    border: 1px solid var(--bd1);
    border-radius: 3px;
    color: var(--t1);
    font-size: var(--sm);
    padding: 5px 14px;
    transition: all 0.15s;
  }
  .btn:hover {
    border-color: var(--bd2);
    color: var(--t0);
  }
  .btn.primary {
    background: var(--ac-d);
    border-color: var(--ac);
    color: var(--ac);
  }
  .btn.primary:hover {
    background: rgba(0, 212, 126, 0.18);
  }
  .btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }
</style>
