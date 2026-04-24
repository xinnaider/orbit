<script lang="ts">
  import type { CliBackend, SubProvider } from '../../lib/tauri';
  import { loadProviderKey } from '../../lib/tauri/providers';
  import SearchSelect from './SearchSelect.svelte';

  export let backends: CliBackend[];
  export let backendId: string;
  export let subProviderId: string;
  export let model: string;
  export let apiKeyOverride: string;
  export let sshMode: boolean;
  export let loading: boolean;

  let subProviderSearch = '';
  let savedKeyLoaded = false;
  let hasSavedKey = false;
  let showCustomProviderSteps = false;

  $: selectedBackend = backends.find((b) => b.id === backendId) ?? null;
  $: hasSubProviders = selectedBackend?.hasSubProviders ?? false;

  // SSH mode: reset to claude-code if current backend doesn't support SSH
  $: if (sshMode && !(backends.find((b) => b.id === backendId)?.supportsSsh ?? false))
    backendId = 'claude-code';
  $: sshBackends = sshMode ? backends.filter((b) => b.supportsSsh) : backends;

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
  $: currentModels = hasSubProviders
    ? (selectedSubProvider?.models ?? [])
    : (selectedBackend?.models ?? []);

  // API key needed? (only for sub-provider backends like OpenCode)
  $: envVars = selectedSubProvider?.env ?? [];
  $: needsApiKey = hasSubProviders && envVars.length > 0;

  // Reset model and load saved key when backend or sub-provider changes
  let prevBackendId = backendId;
  let prevSubProviderId = subProviderId;
  $: if (backendId !== prevBackendId || subProviderId !== prevSubProviderId) {
    prevBackendId = backendId;
    prevSubProviderId = subProviderId;
    const first = currentModels[0];
    model = first?.id ?? '';
    // Load saved API key for this provider
    savedKeyLoaded = false;
    hasSavedKey = false;
    if (needsApiKey && subProviderId) {
      loadProviderKey(subProviderId)
        .then((result) => {
          if (result) {
            hasSavedKey = true;
            if (!apiKeyOverride) {
              apiKeyOverride = result.apiKey;
            }
          }
          savedKeyLoaded = true;
        })
        .catch(() => {
          savedKeyLoaded = true;
        });
    }
  }

  function selectSubProvider(p: SubProvider) {
    subProviderId = p.id;
    subProviderSearch = '';
  }

  function isOpenCodeBackend(b: CliBackend | null | undefined): boolean {
    if (!b) return false;
    return (
      b.id.toLowerCase().includes('opencode') ||
      b.name.toLowerCase().includes('opencode') ||
      b.cliName.toLowerCase().includes('opencode') ||
      b.hasSubProviders
    );
  }

  function selectBackend(b: CliBackend) {
    backendId = b.id;
  }
</script>

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

<!-- CLI Backend selector -->
<div class="field">
  <!-- svelte-ignore a11y_label_has_associated_control -->
  <label class="label">backend</label>
  <div class="backend-row">
    {#each sshBackends as b}
      <button
        class="backend-chip"
        class:active={backendId === b.id}
        class:unavailable={!b.cliAvailable}
        disabled={loading || !b.cliAvailable}
        on:click={() => selectBackend(b)}
        title={b.cliAvailable ? b.name : `${b.name} (not installed — ${b.installHint})`}
      >
        <span class="chip-dot" style="color:{b.cliAvailable ? 'var(--s-working)' : 'var(--t3)'}"
          >{b.cliAvailable ? '●' : '○'}</span
        >
        <span>{b.name}</span>
      </button>
    {/each}
  </div>
</div>

{#if isOpenCodeBackend(selectedBackend)}
  <div class="opencode-help">
    <div class="help-title">Custom providers</div>
    <div class="help-body">
      <p>
        Alem dos providers padrao do OpenCode, voce tambem pode selecionar um custom provider aqui,
        se ele estiver configurado.
      </p>
      <button
        type="button"
        class="help-link"
        on:click={() => (showCustomProviderSteps = !showCustomProviderSteps)}
      >
        {showCustomProviderSteps ? 'Ocultar passo a passo' : 'Ver como configurar'}
      </button>
      {#if showCustomProviderSteps}
        <div class="help-steps">
          <p>
            1. Edite <code>~/.config/opencode/opencode.jsonc</code> ou
            <code>~/.config/opencode/opencode.json</code>.
          </p>
          <p>2. Adicione seu provider na secao <code>provider</code>.</p>
          <p>3. Defina pelo menos <code>name</code> e os <code>models</code> que quer expor.</p>
          <p>4. Reinicie o Orbit para o provider aparecer no seletor.</p>
        </div>
        <pre>{`{
  "provider": {
    "my-provider": {
      "name": "My Provider",
      "options": { "apiKey": "sk-..." },
      "models": {
        "my-model": {
          "name": "My Model",
          "limit": { "context": 128000, "output": 4096 }
        }
      }
    }
  }
}`}</pre>
      {/if}
    </div>
  </div>
{/if}

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
    <SearchSelect
      items={currentModels}
      bind:value={model}
      placeholder="select model..."
      disabled={loading}
    />
  </div>
{/if}

<!-- API Key (OpenCode sub-providers only) -->
{#if needsApiKey}
  <div class="field">
    <label class="label" for="ns-apikey"
      >API Key
      {#if hasSavedKey}
        <span class="key-saved">✓ saved</span>
      {/if}</label
    >
    <input
      id="ns-apikey"
      class="input"
      type="password"
      bind:value={apiKeyOverride}
      placeholder={hasSavedKey ? 'using saved key — paste to override' : 'paste API key...'}
      disabled={loading}
    />
  </div>
{/if}

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
  /* Backend chips */
  .backend-row {
    display: flex;
    gap: var(--sp-3);
  }
  .backend-chip {
    display: flex;
    align-items: center;
    gap: var(--sp-3);
    flex: 1;
    justify-content: center;
    background: var(--bg2);
    border: 1px solid var(--bd1);
    border-radius: var(--radius-sm);
    padding: var(--sp-3) var(--sp-5);
    font-size: var(--sm);
    color: var(--t1);
    cursor: pointer;
    transition:
      border-color 0.15s,
      color 0.15s,
      background 0.15s;
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

  .opencode-help {
    border: 1px solid var(--bd1);
    border-radius: var(--radius-sm);
    background: var(--bg2);
    color: var(--t1);
    font-size: var(--xs);
    overflow: hidden;
  }
  .help-title {
    padding: var(--sp-3) var(--sp-4);
    color: var(--ac);
    font-weight: 600;
    border-bottom: 1px solid var(--bd);
  }
  .help-body {
    display: flex;
    flex-direction: column;
    gap: var(--sp-3);
    padding: 0 var(--sp-4) var(--sp-4);
  }
  .help-body p {
    margin: var(--sp-3) 0 0;
    line-height: 1.5;
  }
  .help-link {
    align-self: flex-start;
    padding: 0;
    border: none;
    background: none;
    color: var(--ac);
    font: inherit;
    cursor: pointer;
  }
  .help-link:hover {
    text-decoration: underline;
  }
  .help-steps {
    display: flex;
    flex-direction: column;
    gap: var(--sp-2);
  }
  .help-steps p {
    margin: 0;
  }
  .help-body code {
    color: var(--t0);
    background: var(--bg3);
    border: 1px solid var(--bd);
    border-radius: 4px;
    padding: 1px 4px;
  }
  .help-body pre {
    margin: 0;
    overflow-x: auto;
    color: var(--t0);
    background: var(--bg1);
    border: 1px solid var(--bd);
    border-radius: var(--radius-sm);
    padding: var(--sp-3);
    line-height: 1.45;
    white-space: pre;
  }

  /* Sub-provider list */
  .sub-search {
    font-size: var(--xs);
    padding: var(--sp-2) var(--sp-4);
  }
  .sub-list {
    display: flex;
    flex-direction: column;
    max-height: 160px;
    overflow-y: auto;
    border: 1px solid var(--bd1);
    border-radius: var(--radius-sm);
    background: var(--bg2);
  }
  .sub-item {
    display: flex;
    align-items: center;
    gap: var(--sp-3);
    padding: var(--sp-3) var(--sp-4);
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
    padding: var(--sp-4);
    font-size: var(--xs);
    color: var(--t3);
    text-align: center;
  }

  .key-saved {
    color: var(--s-working);
    font-weight: normal;
    font-size: 10px;
  }
</style>
