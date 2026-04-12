<script lang="ts">
  import { onMount, createEventDispatcher } from 'svelte';
  import { open } from '@tauri-apps/plugin-dialog';
  import {
    createSession,
    diagnoseSpawn,
    setSessionApiKey,
    getProviders,
    checkEnvVar,
  } from '../lib/tauri';
  import type { SpawnDiagnostic, ProviderInfo } from '../lib/tauri';
  import { generateAgentName } from '../lib/android-names';

  const dispatch = createEventDispatcher();

  let path = '';
  let prompt = '';
  let model = 'auto';
  let providerId = 'claude-code';
  let apiKeyOverride = '';
  let loading = false;
  let error = '';
  let diagRunning = false;
  let diag: SpawnDiagnostic | null = null;
  let agentName = '';
  let projectSuffix = '';
  let generatedAgent = '';
  let generatedProject = '';
  let useWorktree = false;

  // Provider data from backend
  let allProviders: ProviderInfo[] = [];
  let providerSearch = '';

  // Favorites shown at top of selector
  const FAVORITE_IDS = [
    'claude-code',
    'codex',
    'openrouter',
    'anthropic',
    'openai',
    'google',
    'deepseek',
  ];

  $: selectedProvider = allProviders.find((p) => p.id === providerId) ?? null;
  $: providerModels = selectedProvider?.models ?? [];
  $: isClaude = providerId === 'claude-code';
  $: needsApiKey = selectedProvider && selectedProvider.env.length > 0;
  $: envVarName = selectedProvider?.env?.[0] ?? '';

  // Filter providers for the search dropdown
  $: favoriteProviders = allProviders.filter((p) => FAVORITE_IDS.includes(p.id));
  $: otherProviders = allProviders.filter(
    (p) =>
      !FAVORITE_IDS.includes(p.id) &&
      (providerSearch === '' || p.name.toLowerCase().includes(providerSearch.toLowerCase()))
  );

  onMount(async () => {
    try {
      allProviders = await getProviders();
    } catch (e) {
      console.warn('[NewSessionModal] getProviders failed:', e);
    }
  });

  // Reset model when provider changes
  $: {
    if (selectedProvider) {
      const first = selectedProvider.models[0];
      model = first?.id ?? '';
    }
  }

  // Check API key configured status when provider changes
  let keyConfigured = false;
  $: if (envVarName) {
    checkEnvVar(envVarName)
      .then((v) => (keyConfigured = v))
      .catch(() => (keyConfigured = false));
  } else {
    keyConfigured = false;
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
    try {
      diag = await diagnoseSpawn();
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

  async function submit() {
    if (!path.trim()) {
      error = 'project path required';
      return;
    }
    if (!selectedProvider?.cliAvailable) {
      error = `${selectedProvider?.name ?? providerId} CLI not found`;
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
        model: model === 'auto' ? undefined : model,
        permissionMode: 'ignore',
        sessionName: finalName,
        useWorktree: isClaude ? useWorktree : false,
        provider: providerId,
      });
      // Pass API key override if provided
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

  function statusIcon(p: ProviderInfo): string {
    if (!p.cliAvailable) return '○';
    if (p.configured || p.env.length === 0) return '●';
    return '◐';
  }

  function statusColor(p: ProviderInfo): string {
    if (!p.cliAvailable) return 'var(--t3)';
    if (p.configured || p.env.length === 0) return 'var(--s-working)';
    return 'var(--s-input)';
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
      <label class="label" for="ns-path">path</label>
      <div class="path-row">
        <input
          id="ns-path"
          class="input"
          bind:value={path}
          placeholder="/home/user/project"
          disabled={loading}
          on:keydown={(e) => e.key === 'Enter' && prompt && submit()}
        />
        <button class="browse" on:click={browse} disabled={loading} title="browse">⌘</button>
      </div>
    </div>

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

    <!-- Provider selector -->
    <div class="field">
      <label class="label" for="ns-provider">provider</label>
      <div class="provider-grid">
        {#each favoriteProviders as p}
          <button
            class="provider-chip"
            class:active={providerId === p.id}
            class:unavailable={!p.cliAvailable}
            disabled={loading}
            on:click={() => (providerId = p.id)}
            title={p.cliAvailable ? p.name : `${p.name} (CLI not installed)`}
          >
            <span class="chip-dot" style="color:{statusColor(p)}">{statusIcon(p)}</span>
            <span class="chip-name">{p.name}</span>
          </button>
        {/each}
      </div>
      {#if allProviders.length > FAVORITE_IDS.length}
        <input
          class="input provider-search"
          bind:value={providerSearch}
          placeholder="search providers..."
          disabled={loading}
        />
        {#if providerSearch}
          <div class="provider-results">
            {#each otherProviders.slice(0, 12) as p}
              <button
                class="provider-result"
                class:active={providerId === p.id}
                disabled={loading || !p.cliAvailable}
                on:click={() => {
                  providerId = p.id;
                  providerSearch = '';
                }}
              >
                <span class="chip-dot" style="color:{statusColor(p)}">{statusIcon(p)}</span>
                <span>{p.name}</span>
                <span class="result-count">{p.models.length} models</span>
              </button>
            {/each}
            {#if otherProviders.length === 0}
              <div class="no-results">no providers match "{providerSearch}"</div>
            {/if}
          </div>
        {/if}
      {/if}
    </div>

    <!-- Model selector -->
    <div class="field">
      <label class="label" for="ns-model">model</label>
      {#if providerModels.length <= 10}
        <select id="ns-model" class="input select" bind:value={model} disabled={loading}>
          {#each providerModels as m}
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
          {#each providerModels as m}
            <option value={m.id}>{m.name}</option>
          {/each}
        </datalist>
      {/if}
    </div>

    <!-- API Key -->
    {#if needsApiKey}
      <div class="field">
        <label class="label" for="ns-apikey">
          API Key
          {#if keyConfigured}
            <span class="key-hint configured">(configured via {envVarName})</span>
          {:else}
            <span class="key-hint">{envVarName} not set</span>
          {/if}
        </label>
        <input
          id="ns-apikey"
          class="input"
          type="password"
          bind:value={apiKeyOverride}
          placeholder={keyConfigured ? 'override (optional)' : `paste ${envVarName}...`}
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

    {#if isClaude}
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
        <div class="diag-row" class:ok={diag.claudeFound} class:fail={!diag.claudeFound}>
          claude: {diag.claudeFound ? `✓ ${diag.claudePath ?? diag.whereOutput}` : '✗ not found'}
        </div>
        {#if diag.versionOutput}
          <div class="diag-row ok">version: {diag.versionOutput.slice(0, 60)}</div>
        {/if}
        {#if !diag.claudeFound}
          <div class="diag-row fail">install: npm install -g @anthropic-ai/claude-code</div>
        {/if}
      </div>
    {/if}

    <div class="actions">
      {#if isClaude}
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

  /* Provider chips */
  .provider-grid {
    display: flex;
    flex-wrap: wrap;
    gap: 5px;
  }
  .provider-chip {
    display: flex;
    align-items: center;
    gap: 4px;
    background: var(--bg2);
    border: 1px solid var(--bd1);
    border-radius: 3px;
    padding: 4px 8px;
    font-size: var(--xs);
    color: var(--t1);
    cursor: pointer;
    transition: all 0.15s;
  }
  .provider-chip:hover {
    border-color: var(--bd2);
    color: var(--t0);
  }
  .provider-chip.active {
    border-color: var(--ac);
    color: var(--ac);
    background: rgba(0, 212, 126, 0.08);
  }
  .provider-chip.unavailable {
    opacity: 0.4;
  }
  .chip-dot {
    font-size: 8px;
    line-height: 1;
  }
  .chip-name {
    white-space: nowrap;
  }

  .provider-search {
    margin-top: 6px;
    font-size: var(--xs);
    padding: 4px 8px;
  }
  .provider-results {
    display: flex;
    flex-direction: column;
    max-height: 180px;
    overflow-y: auto;
    border: 1px solid var(--bd1);
    border-radius: 3px;
    background: var(--bg2);
  }
  .provider-result {
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
  .provider-result:hover,
  .provider-result.active {
    background: var(--bg3);
    color: var(--t0);
  }
  .provider-result:disabled {
    opacity: 0.3;
    cursor: not-allowed;
  }
  .result-count {
    margin-left: auto;
    color: var(--t3);
    font-size: 10px;
  }
  .no-results {
    padding: 8px;
    font-size: var(--xs);
    color: var(--t3);
    text-align: center;
  }

  .key-hint {
    font-weight: normal;
    color: var(--s-input);
    font-size: 10px;
  }
  .key-hint.configured {
    color: var(--s-working);
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
