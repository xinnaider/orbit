<script lang="ts">
  import { onMount, createEventDispatcher } from 'svelte';
  import { open } from '@tauri-apps/plugin-dialog';
  import { createSession, getProviders, diagnoseProvider } from '../lib/tauri';
  import { backends as backendsStore, providerCaps, getCaps } from '../lib/stores/providers';
  import type { ProviderDiagnostic } from '../lib/tauri';
  import { generateAgentName } from '../lib/android-names';
  import Modal from './shared/Modal.svelte';
  import ProviderSelector from './shared/ProviderSelector.svelte';
  import SshFields from './shared/SshFields.svelte';

  const dispatch = createEventDispatcher();

  let path = '';
  let prompt = '';
  let model = 'auto';
  let backendId = 'claude-code';
  let subProviderId = '';
  let apiKeyOverride = '';
  let loading = false;
  let error = '';
  let diagRunning = false;
  let diag: ProviderDiagnostic | null = null;
  let agentName = '';
  let projectSuffix = '';
  let generatedAgent = '';
  let generatedProject = '';
  let useWorktree = false;
  let sshMode = false;
  let sshHost = '';
  let sshUser = 'ubuntu';
  let sshPassword = '';

  $: backends = $backendsStore;
  $: selectedBackend = backends.find((b) => b.id === backendId) ?? null;
  $: caps = getCaps($providerCaps, backendId);
  $: hasSubProviders = selectedBackend?.hasSubProviders ?? false;

  onMount(async () => {
    // Refresh providers if not already loaded
    if (backends.length === 0) {
      try {
        backendsStore.set(await getProviders());
      } catch (e) {
        console.warn('[NewSessionModal] getProviders failed:', e);
      }
    }
    // Pre-select first sub-provider if OpenCode
    const oc = backends.find((b) => b.hasSubProviders);
    if (oc?.subProviders?.length) {
      subProviderId = oc.subProviders[0].id;
    }
  });

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

  async function runDiag() {
    diagRunning = true;
    diag = null;
    error = '';
    try {
      diag = await diagnoseProvider(backendId, {
        projectPath: path.trim() || undefined,
        sshHost: sshMode ? sshHost.trim() || undefined : undefined,
        sshUser: sshMode ? sshUser.trim() || undefined : undefined,
        sshPassword: sshMode ? sshPassword.trim() || undefined : undefined,
      });
    } catch (e: any) {
      error = e?.message ?? String(e);
    } finally {
      diagRunning = false;
    }
  }

  async function browse() {
    const sel = await open({ directory: true, multiple: false });
    if (sel && typeof sel === 'string') path = sel;
  }

  function resolveProvider(): string {
    if (hasSubProviders && subProviderId) return subProviderId;
    return backendId;
  }

  function resolveModel(): string | undefined {
    if (!model || model === 'auto') return caps.supportsEffort ? undefined : model;
    if (hasSubProviders && subProviderId && !model.includes('/')) {
      return `${subProviderId}/${model}`;
    }
    return model;
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
    if (!selectedBackend?.cliAvailable) {
      error = `${selectedBackend?.name ?? backendId} CLI not found`;
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
        model: resolveModel(),
        permissionMode: 'ignore',
        sessionName: finalName,
        useWorktree: caps.supportsEffort && !sshMode ? useWorktree : false,
        provider: resolveProvider(),
        apiKey:
          hasSubProviders &&
          (selectedBackend?.subProviders.find((p) => p.id === subProviderId)?.env ?? []).length >
            0 &&
          apiKeyOverride.trim()
            ? apiKeyOverride.trim()
            : undefined,
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
</script>

<Modal
  title="new session"
  width="500px"
  closeOnOverlayClick={false}
  on:close={() => dispatch('cancel')}
>
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
      <button class="browse" on:click={browse} disabled={loading} title="browse">⌘</button>
    </div>
  </div>

  <ProviderSelector
    {backends}
    bind:backendId
    bind:subProviderId
    bind:model
    bind:apiKeyOverride
    bind:sshMode
    {loading}
  />

  {#if sshMode}
    <SshFields bind:sshHost bind:sshUser bind:sshPassword {loading} />
  {/if}

  <div class="field">
    <label class="label" for="ns-prompt">prompt</label>
    <textarea
      id="ns-prompt"
      class="input textarea"
      bind:value={prompt}
      placeholder="what should the agent work on? (optional)"
      rows="3"
      disabled={loading}
      on:keydown={(e) => {
        if (e.key === 'Enter' && e.metaKey) submit();
      }}
    ></textarea>
  </div>

  <!-- Session name -->
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

  {#if caps.supportsEffort && !sshMode}
    <label class="toggle-row">
      <input type="checkbox" bind:checked={useWorktree} disabled={loading} />
      <span class="toggle-label">criar git worktree</span>
    </label>
  {/if}

  {#if error}
    <p class="error">! {error}</p>
  {/if}

  {#if diag}
    <div class="diag">
      {#if diag.ssh}
        <div class="diag-row" class:ok={diag.ssh.ok} class:fail={!diag.ssh.ok}>
          ssh: {diag.ssh.ok ? `✓ connected (${diag.ssh.latencyMs}ms)` : `✗ ${diag.ssh.error}`}
        </div>
      {/if}
      {#if !diag.ssh || diag.ssh.ok}
        <div class="diag-row" class:ok={diag.found} class:fail={!diag.found}>
          {diag.cliName}: {diag.found ? `✓ ${diag.path ?? ''}` : '✗ not found'}
        </div>
        {#if diag.version}
          <div class="diag-row ok">version: {diag.version.slice(0, 60)}</div>
        {/if}
        {#if !diag.found}
          <div class="diag-row fail">install: {diag.installHint}</div>
        {/if}
        {#if diag.projectDirOk === true}
          <div class="diag-row ok">path: ✓ exists</div>
        {:else if diag.projectDirOk === false}
          <div class="diag-row fail">path: ✗ directory not found</div>
        {/if}
      {/if}
    </div>
  {/if}

  <div class="actions">
    <button
      class="btn ghost"
      on:click={runDiag}
      disabled={diagRunning || loading || (sshMode && (!sshHost.trim() || !sshUser.trim()))}
    >
      {diagRunning ? 'testing...' : '⚙ diagnose'}
    </button>
    <button class="btn ghost" on:click={() => dispatch('cancel')} disabled={loading}>cancel</button>
    <button class="btn primary" on:click={submit} disabled={loading || !path}>
      {loading ? 'spawning...' : 'start session'}
    </button>
  </div>
</Modal>

<style>
  .field {
    display: flex;
    flex-direction: column;
    gap: var(--sp-3);
  }
  .label {
    font-size: var(--xs);
    color: var(--t2);
    letter-spacing: 0.06em;
    display: flex;
    align-items: center;
    gap: var(--sp-3);
  }
  .input {
    background: var(--bg2);
    border: 1px solid var(--bd1);
    border-radius: var(--radius-sm);
    color: var(--t0);
    font-size: var(--md);
    padding: var(--sp-3) var(--sp-4);
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

  .path-row {
    display: flex;
    gap: var(--sp-3);
  }
  .path-row .input {
    flex: 1;
  }
  .browse {
    background: var(--bg2);
    border: 1px solid var(--bd1);
    color: var(--t1);
    border-radius: var(--radius-sm);
    padding: 0 var(--sp-5);
    font-size: var(--base);
    flex-shrink: 0;
  }
  .browse:hover {
    border-color: var(--bd2);
    color: var(--t0);
  }

  .error {
    font-size: var(--sm);
    color: var(--s-error);
  }
  .diag {
    background: var(--bg3);
    border: 1px solid var(--bd1);
    border-radius: var(--radius-sm);
    padding: var(--sp-4) var(--sp-5);
    display: flex;
    flex-direction: column;
    gap: var(--sp-2);
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
    gap: var(--sp-4);
  }
  .btn {
    background: none;
    border: 1px solid var(--bd1);
    border-radius: var(--radius-sm);
    color: var(--t1);
    font-size: var(--sm);
    padding: var(--sp-3) var(--sp-7);
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
    gap: var(--sp-3);
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
    gap: var(--sp-4);
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
