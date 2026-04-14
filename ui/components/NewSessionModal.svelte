<script lang="ts">
  import { onMount, createEventDispatcher } from 'svelte';
  import { open } from '@tauri-apps/plugin-dialog';
  import { createSession, setSessionApiKey, getProviders, diagnoseProvider } from '../lib/tauri';
  import type { ProviderDiagnostic, CliBackend, SubProvider } from '../lib/tauri';
  import { generateAgentName } from '../lib/android-names';

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
  let subProviderSearch = '';
  let sshMode = false;
  let sshHost = '';
  let sshUser = 'ubuntu';
  let sshPassword = '';

  let backends: CliBackend[] = [];

  $: selectedBackend = backends.find((b) => b.id === backendId) ?? null;
  $: isClaude = backendId === 'claude-code';
  $: isOpenCode = backendId === 'opencode';
  $: hasSubProviders = isOpenCode && (selectedBackend?.subProviders?.length ?? 0) > 0;

  // Sub-provider selection (OpenCode only)
  $: selectedSubProvider = hasSubProviders
    ? (selectedBackend?.subProviders.find((p) => p.id === subProviderId) ?? null)
    : null;

  // Filtered sub-providers for search
  $: filteredSubProviders = (selectedBackend?.subProviders ?? []).filter(
    (p) =>
      subProviderSearch === '' ||
      p.name.toLowerCase().includes(subProviderSearch.toLowerCase()) ||
      p.id.toLowerCase().includes(subProviderSearch.toLowerCase())
  );

  // Models depend on backend type
  $: currentModels = isOpenCode
    ? (selectedSubProvider?.models ?? [])
    : (selectedBackend?.models ?? []);

  // API key needed?
  $: envVars = selectedSubProvider?.env ?? [];
  $: needsApiKey = isOpenCode && envVars.length > 0;
  $: envVarName = envVars[0] ?? '';

  onMount(async () => {
    try {
      backends = await getProviders();
      // Pre-select first sub-provider if OpenCode
      const oc = backends.find((b) => b.id === 'opencode');
      if (oc?.subProviders?.length) {
        subProviderId = oc.subProviders[0].id;
      }
    } catch (e) {
      console.warn('[NewSessionModal] getProviders failed:', e);
    }
  });

  // Reset model when backend or sub-provider changes
  let prevBackendId = backendId;
  let prevSubProviderId = subProviderId;
  $: if (backendId !== prevBackendId || subProviderId !== prevSubProviderId) {
    prevBackendId = backendId;
    prevSubProviderId = subProviderId;
    const first = currentModels[0];
    model = first?.id ?? '';
    diag = null;
  }

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
      diag = await diagnoseProvider(
        backendId,
        sshMode
          ? {
              sshHost: sshHost.trim() || undefined,
              sshUser: sshUser.trim() || undefined,
              sshPassword: sshPassword.trim() || undefined,
            }
          : undefined
      );
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
    if (isOpenCode && subProviderId) return subProviderId;
    return backendId;
  }

  function resolveModel(): string | undefined {
    if (!model || model === 'auto') return isClaude ? undefined : model;
    // For opencode, model needs "subprovider/model" format
    if (isOpenCode && subProviderId && !model.includes('/')) {
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
      const session = await createSession({
        projectPath: path.trim(),
        prompt: prompt.trim() || 'Hello',
        model: resolveModel(),
        permissionMode: 'ignore',
        sessionName: finalName,
        useWorktree: isClaude && !sshMode ? useWorktree : false,
        provider: resolveProvider(),
        sshHost: sshMode ? sshHost.trim() : undefined,
        sshUser: sshMode ? sshUser.trim() : undefined,
        sshPassword: sshMode && sshPassword.trim() ? sshPassword.trim() : undefined,
      });
      if (needsApiKey && apiKeyOverride.trim()) {
        await setSessionApiKey(session.id, apiKeyOverride.trim());
      }
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

  function selectSubProvider(p: SubProvider) {
    subProviderId = p.id;
    subProviderSearch = '';
  }

</script>

<svelte:window on:keydown={onKey} />

<div
  class="overlay"
  role="dialog"
  aria-modal="true"
  tabindex="-1"
  on:click|self={() => {}}
  on:keydown={onKey}
>
  <div class="modal">
    <div class="modal-header">
      <span class="modal-title">new session</span>
      <button class="close" on:click={() => dispatch('cancel')}>✕</button>
    </div>

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

    <!-- SSH mode toggle -->
    <div class="field">
      <!-- svelte-ignore a11y_label_has_associated_control -->
      <label class="label">connection</label>
      <div class="backend-row">
        <button
          class="backend-chip"
          class:active={!sshMode}
          on:click={() => (sshMode = false)}
          disabled={loading}
        >
          <span class="chip-dot" style="color:{!sshMode ? 'var(--s-working)' : 'var(--t3)'}">
            {!sshMode ? '●' : '○'}
          </span>
          <span>local</span>
        </button>
        <button
          class="backend-chip"
          class:active={sshMode}
          on:click={() => (sshMode = true)}
          disabled={loading}
        >
          <span class="chip-dot" style="color:{sshMode ? 'var(--s-working)' : 'var(--t3)'}">
            {sshMode ? '●' : '○'}
          </span>
          <span>ssh remote</span>
        </button>
      </div>
    </div>

    {#if sshMode}
      <div class="field">
        <label class="label" for="ns-ssh-host">host</label>
        <input
          id="ns-ssh-host"
          class="input"
          type="text"
          bind:value={sshHost}
          placeholder="vps.example.com"
          disabled={loading}
        />
      </div>

      <div class="field">
        <label class="label" for="ns-ssh-user">user</label>
        <input
          id="ns-ssh-user"
          class="input"
          type="text"
          bind:value={sshUser}
          placeholder="ubuntu"
          disabled={loading}
        />
      </div>

      <div class="field">
        <label class="label" for="ns-ssh-pw">password <span class="key-hint">(optional — uses SSH key if empty)</span></label>
        <input
          id="ns-ssh-pw"
          class="input"
          type="password"
          bind:value={sshPassword}
          placeholder="leave empty for key auth"
          disabled={loading}
        />
      </div>
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

    <!-- CLI Backend selector -->
    <div class="field">
      <!-- svelte-ignore a11y_label_has_associated_control -->
      <label class="label">backend</label>
      <div class="backend-row">
        {#each backends as b}
          <button
            class="backend-chip"
            class:active={backendId === b.id}
            class:unavailable={!b.cliAvailable}
            disabled={loading || !b.cliAvailable}
            on:click={() => (backendId = b.id)}
            title={b.cliAvailable ? b.name : `${b.name} (not installed)`}
          >
            <span class="chip-dot" style="color:{b.cliAvailable ? 'var(--s-working)' : 'var(--t3)'}"
              >{b.cliAvailable ? '●' : '○'}</span
            >
            <span>{b.name}</span>
          </button>
        {/each}
      </div>
    </div>

    <!-- OpenCode: sub-provider selector -->
    {#if hasSubProviders}
      <div class="field">
        <!-- svelte-ignore a11y_label_has_associated_control -->
        <label class="label">provider</label>
        <input
          class="input sub-search"
          bind:value={subProviderSearch}
          placeholder="search providers... ({selectedBackend?.subProviders.length ?? 0} available)"
          disabled={loading}
        />
        <div class="sub-list">
          {#each subProviderSearch ? filteredSubProviders : (selectedBackend?.subProviders ?? []).slice(0, 20) as p}
            <button
              class="sub-item"
              class:active={subProviderId === p.id}
              disabled={loading}
              on:click={() => selectSubProvider(p)}
            >
              <span
                class="chip-dot"
                style="color:{p.configured ? 'var(--s-working)' : 'var(--s-input)'}"
                >{p.configured ? '●' : '◐'}</span
              >
              <span class="sub-name">{p.name}</span>
              <span class="sub-count">{p.models.length}</span>
            </button>
          {/each}
          {#if subProviderSearch && filteredSubProviders.length === 0}
            <div class="no-results">no providers match "{subProviderSearch}"</div>
          {/if}
        </div>
      </div>
    {/if}

    <!-- Model selector -->
    {#if currentModels.length > 0}
      <div class="field">
        <label class="label" for="ns-model">model</label>
        {#if currentModels.length <= 15}
          <select id="ns-model" class="input select" bind:value={model} disabled={loading}>
            {#each currentModels as m}
              <option value={m.id}>{m.name}</option>
            {/each}
          </select>
        {:else}
          <input
            id="ns-model"
            class="input"
            list="model-list"
            bind:value={model}
            placeholder="search models..."
            disabled={loading}
          />
          <datalist id="model-list">
            {#each currentModels as m}
              <option value={m.id}>{m.name}</option>
            {/each}
          </datalist>
        {/if}
      </div>
    {/if}

    <!-- API Key (OpenCode sub-providers only) -->
    {#if needsApiKey}
      <div class="field">
        <label class="label" for="ns-apikey"
          >API Key <span class="key-hint">(optional if already configured in CLI)</span></label
        >
        <input
          id="ns-apikey"
          class="input"
          type="password"
          bind:value={apiKeyOverride}
          placeholder="paste API key to override..."
          disabled={loading}
        />
      </div>
    {/if}

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

    {#if isClaude && !sshMode}
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
        {/if}
      </div>
    {/if}

    <div class="actions">
      <button class="btn ghost" on:click={runDiag} disabled={diagRunning || loading || (sshMode && (!sshHost.trim() || !sshUser.trim()))}>
        {diagRunning ? 'testing...' : '⚙ diagnose'}
      </button>
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
    width: 500px;
    max-width: 94vw;
    max-height: 90vh;
    overflow-y: auto;
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
    display: flex;
    align-items: center;
    gap: 6px;
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

  /* Backend chips */
  .backend-row {
    display: flex;
    gap: 6px;
  }
  .backend-chip {
    display: flex;
    align-items: center;
    gap: 5px;
    flex: 1;
    justify-content: center;
    background: var(--bg2);
    border: 1px solid var(--bd1);
    border-radius: 3px;
    padding: 7px 10px;
    font-size: var(--sm);
    color: var(--t1);
    cursor: pointer;
    transition: border-color 0.15s, color 0.15s, background 0.15s;
    white-space: nowrap;
    min-height: 30px;
  }
  .backend-chip:hover {
    border-color: var(--bd2);
    color: var(--t0);
  }
  .backend-chip.active {
    border-color: var(--ac);
    color: var(--ac);
    background: rgba(0, 212, 126, 0.08);
  }
  .backend-chip.unavailable {
    opacity: 0.35;
  }
  .chip-dot {
    font-size: 8px;
    line-height: 1;
  }

  /* Sub-provider list */
  .sub-search {
    font-size: var(--xs);
    padding: 4px 8px;
  }
  .sub-list {
    display: flex;
    flex-direction: column;
    max-height: 160px;
    overflow-y: auto;
    border: 1px solid var(--bd1);
    border-radius: 3px;
    background: var(--bg2);
  }
  .sub-item {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 5px 8px;
    border: none;
    background: none;
    color: var(--t1);
    font-size: var(--xs);
    text-align: left;
    cursor: pointer;
    border-bottom: 1px solid var(--bd);
  }
  .sub-item:hover {
    background: var(--bg3);
    color: var(--t0);
  }
  .sub-item.active {
    background: rgba(0, 212, 126, 0.06);
    color: var(--ac);
  }
  .sub-item:disabled {
    opacity: 0.3;
    cursor: not-allowed;
  }
  .sub-name {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .sub-count {
    color: var(--t3);
    font-size: 10px;
    flex-shrink: 0;
  }
  .no-results {
    padding: 8px;
    font-size: var(--xs);
    color: var(--t3);
    text-align: center;
  }

  .key-hint {
    font-weight: normal;
    color: var(--t3);
    font-size: 10px;
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
