<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { open } from '@tauri-apps/plugin-dialog';
  import { createSession, diagnoseSpawn, testSsh } from '../lib/tauri';
  import type { SpawnDiagnostic, SshTestResult } from '../lib/tauri';
  import { generateAgentName } from '../lib/android-names';

  const dispatch = createEventDispatcher();

  let path = '';
  let prompt = '';
  let model = 'auto';
  let loading = false;
  let error = '';
  let diagRunning = false;
  let diag: SpawnDiagnostic | null = null;
  let agentName = '';
  let projectSuffix = '';
  let generatedAgent = '';
  let generatedProject = '';
  let useWorktree = false;

  $: if (path) {
    const p = path.split(/[/\\]/).filter(Boolean).pop() ?? '';
    if (!generatedProject) generatedProject = p;
    if (!generatedAgent) generatedAgent = generateAgentName();
  }

  $: resolvedAgent = agentName.trim() || generatedAgent;
  $: resolvedProject = projectSuffix.trim() || generatedProject;
  $: namePreview =
    resolvedAgent && resolvedProject
      ? `${resolvedAgent} · ${resolvedProject}`
      : resolvedAgent || resolvedProject;

  let sshMode = false;
  let sshHost = '';
  let sshUser = 'ubuntu';
  let sshPassword = '';
  let showPassword = false;
  let sshTesting = false;
  let sshTestResult: SshTestResult | null = null;

  async function testConnection() {
    if (!sshHost.trim() || !sshUser.trim()) return;
    sshTesting = true;
    sshTestResult = null;
    try {
      sshTestResult = await testSsh(
        sshHost.trim(),
        sshUser.trim(),
        sshPassword.trim() || undefined
      );
    } catch (e: any) {
      sshTestResult = { ok: false, error: e?.message ?? String(e) };
    } finally {
      sshTesting = false;
    }
  }

  $: if (sshHost || sshUser || sshPassword) sshTestResult = null;

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
      error = sshMode ? 'remote path required' : 'project path required';
      return;
    }
    if (sshMode && !sshHost.trim()) {
      error = 'ssh host required';
      return;
    }
    if (sshMode && !sshUser.trim()) {
      error = 'ssh user required';
      return;
    }
    const agent = agentName.trim() || generatedAgent || generateAgentName();
    const project =
      projectSuffix.trim() || generatedProject || path.split(/[/\\]/).filter(Boolean).pop() || '';
    const finalName = project ? `${agent} · ${project}` : agent;
    loading = true;
    error = '';
    try {
      await createSession({
        projectPath: path.trim(),
        prompt: prompt.trim() || 'Hello',
        model: model === 'auto' ? undefined : model,
        permissionMode: 'ignore',
        sessionName: finalName,
        useWorktree,
        sshHost: sshMode ? sshHost.trim() : undefined,
        sshUser: sshMode ? sshUser.trim() : undefined,
        sshPassword: sshMode && sshPassword.trim() ? sshPassword.trim() : undefined,
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

    <div class="mode-toggle">
      <button
        class="mode-btn"
        class:active={!sshMode}
        on:click={() => (sshMode = false)}
        disabled={loading}>local</button
      >
      <button
        class="mode-btn"
        class:active={sshMode}
        on:click={() => (sshMode = true)}
        disabled={loading}>ssh remote</button
      >
    </div>

    {#if sshMode}
      <div class="row">
        <div class="field" style="flex:2">
          <label class="label" for="ns-ssh-host">host</label>
          <input
            id="ns-ssh-host"
            class="input"
            bind:value={sshHost}
            placeholder="vps.example.com"
            disabled={loading}
          />
        </div>
        <div class="field" style="flex:1">
          <label class="label" for="ns-ssh-user">user</label>
          <input
            id="ns-ssh-user"
            class="input"
            bind:value={sshUser}
            placeholder="ubuntu"
            disabled={loading}
          />
        </div>
      </div>

      <div class="row" style="align-items:flex-end">
        <div class="field" style="flex:1">
          <label class="label" for="ns-ssh-pw"
            >password <span class="label-opt">(optional)</span></label
          >
          <div class="pw-row">
            <input
              id="ns-ssh-pw"
              class="input"
              type={showPassword ? 'text' : 'password'}
              bind:value={sshPassword}
              placeholder="leave empty for key-based auth"
              disabled={loading}
              autocomplete="off"
            />
            <button
              class="pw-toggle"
              type="button"
              on:click={() => (showPassword = !showPassword)}
              title={showPassword ? 'hide' : 'show'}
              tabindex="-1"
            >
              {#if showPassword}
                <svg
                  width="13"
                  height="13"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="2"
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  ><path
                    d="M17.94 17.94A10.07 10.07 0 0 1 12 20c-7 0-11-8-11-8a18.45 18.45 0 0 1 5.06-5.94"
                  /><path
                    d="M9.9 4.24A9.12 9.12 0 0 1 12 4c7 0 11 8 11 8a18.5 18.5 0 0 1-2.16 3.19"
                  /><line x1="1" y1="1" x2="23" y2="23" /></svg
                >
              {:else}
                <svg
                  width="13"
                  height="13"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="2"
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  ><path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z" /><circle
                    cx="12"
                    cy="12"
                    r="3"
                  /></svg
                >
              {/if}
            </button>
          </div>
        </div>

        <div class="field test-btn-field">
          <button
            class="btn-test"
            type="button"
            on:click={testConnection}
            disabled={loading || sshTesting || !sshHost.trim() || !sshUser.trim()}
          >
            {#if sshTesting}
              <span class="spin">◌</span> testing…
            {:else}
              test connection
            {/if}
          </button>
          {#if sshTestResult}
            {#if sshTestResult.ok}
              <span class="test-ok">✓ connected · {sshTestResult.latencyMs}ms</span>
            {:else}
              <span class="test-fail">✗ {sshTestResult.error}</span>
            {/if}
          {/if}
        </div>
      </div>
    {/if}

    <div class="field">
      <label class="label" for="ns-path">{sshMode ? 'remote path' : 'path'}</label>
      <div class="path-row">
        <input
          id="ns-path"
          class="input"
          bind:value={path}
          placeholder={sshMode ? '/home/ubuntu/project' : '/home/user/project'}
          disabled={loading}
          on:keydown={(e) => e.key === 'Enter' && prompt && submit()}
        />
        {#if !sshMode}
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
        placeholder="what should claude work on? (optional)"
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
      <label class="label" for="ns-agent">apelido</label>
      <div class="nickname-row">
        <input
          id="ns-agent"
          class="input"
          bind:value={agentName}
          placeholder={generatedAgent || '—'}
          title="nome do agente"
          disabled={loading}
        />
        <span class="nick-sep">·</span>
        <input
          id="ns-project"
          class="input"
          bind:value={projectSuffix}
          placeholder={generatedProject || 'projeto'}
          title="sufixo do projeto"
          disabled={loading}
        />
      </div>
      {#if namePreview}
        <span class="name-preview">{namePreview}</span>
      {/if}
    </div>

    {#if !sshMode}
      <label class="toggle-row">
        <input type="checkbox" bind:checked={useWorktree} disabled={loading} />
        <span class="toggle-label">criar git worktree</span>
      </label>
    {/if}

    {#if sshMode}
      <p class="ssh-hint">
        requires: key-based auth (no passphrase or ssh-agent active) · <code>claude</code> installed on
        remote · first connection auto-accepts host key
      </p>
    {/if}

    {#if error}
      <p class="error">! {error}</p>
    {/if}

    {#if diag && !sshMode}
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
      {#if !sshMode}
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
  .mode-toggle {
    display: flex;
    gap: 4px;
    background: var(--bg2);
    border: 1px solid var(--bd1);
    border-radius: 3px;
    padding: 3px;
  }
  .mode-btn {
    flex: 1;
    background: none;
    border: none;
    color: var(--t2);
    font-size: var(--xs);
    padding: 4px 8px;
    border-radius: 2px;
    letter-spacing: 0.04em;
    transition: all 0.15s;
  }
  .mode-btn.active {
    background: var(--bg3);
    color: var(--t0);
  }
  .mode-btn:hover:not(.active) {
    color: var(--t1);
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
  .pw-row {
    display: flex;
    gap: 4px;
  }
  .pw-row .input {
    flex: 1;
    min-width: 0;
  }
  .pw-toggle {
    background: var(--bg3);
    border: 1px solid var(--bd1);
    border-radius: 3px;
    color: var(--t2);
    cursor: pointer;
    padding: 0 6px;
    display: flex;
    align-items: center;
    flex-shrink: 0;
  }
  .pw-toggle:hover {
    border-color: var(--bd2);
    color: var(--t0);
  }
  .label-opt {
    color: var(--t3);
    font-size: var(--xs);
  }
  .test-btn-field {
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
    align-items: flex-start;
  }
  .btn-test {
    background: var(--bg3);
    border: 1px solid var(--bd1);
    border-radius: 3px;
    color: var(--t1);
    cursor: pointer;
    font-size: var(--sm);
    padding: 5px 10px;
    white-space: nowrap;
    transition: border-color 0.1s;
  }
  .btn-test:hover:not(:disabled) {
    border-color: var(--ac);
    color: var(--ac);
  }
  .btn-test:disabled {
    opacity: 0.45;
    cursor: default;
  }
  .test-ok {
    font-size: var(--xs);
    color: var(--ac);
  }
  .test-fail {
    font-size: var(--xs);
    color: var(--s-error);
    max-width: 200px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .spin {
    display: inline-block;
    animation: spin 1s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  .ssh-hint {
    font-size: var(--xs);
    color: var(--t3);
    line-height: 1.5;
    margin: 0;
  }
  .ssh-hint code {
    color: var(--t2);
    background: var(--bg3);
    padding: 0 3px;
    border-radius: 2px;
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

  .nickname-row {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .nickname-row .input {
    flex: 1;
  }
  .nick-sep {
    color: var(--t3);
    font-size: var(--md);
    flex-shrink: 0;
  }
  .name-preview {
    font-size: var(--xs);
    color: var(--t3);
    letter-spacing: 0.03em;
  }

  .toggle-row {
    display: flex;
    align-items: center;
    gap: 8px;
    cursor: pointer;
    user-select: none;
  }
  .toggle-row input[type='checkbox'] {
    accent-color: var(--ac);
    width: 14px;
    height: 14px;
    cursor: pointer;
  }
  .toggle-label {
    font-size: var(--sm);
    color: var(--t1);
  }
</style>
