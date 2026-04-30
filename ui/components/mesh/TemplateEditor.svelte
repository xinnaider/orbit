<script lang="ts">
  import { addTemplate, editTemplate } from '../../lib/stores/mesh/templates';
  import { MESH_DEFAULT_PROVIDER } from '../../lib/stores/mesh/constants';
  import { addToast } from '../../lib/stores/toasts';
  import Modal from '../shared/Modal.svelte';

  export let initial: {
    id?: number;
    name: string;
    prePrompt: string;
    model: string | null;
    useWorktree: boolean;
    kind: 'agent' | 'browser' | 'skill';
  };
  export let floorId: number;
  export let onClose: () => void;

  let name = initial.name;
  let prePrompt = initial.prePrompt;
  let model = initial.model ?? '';
  let useWorktree = initial.useWorktree;
  const kind = initial.kind;
  let saving = false;

  $: title = initial.id
    ? 'edit template'
    : kind === 'browser'
      ? 'new browser panel'
      : 'new agent template';

  async function onSave() {
    if (!name.trim()) {
      addToast({ type: 'error', message: 'name is required', autoDismiss: true });
      return;
    }
    if (kind === 'agent' && !prePrompt.trim()) {
      addToast({ type: 'error', message: 'pre-prompt is required', autoDismiss: true });
      return;
    }
    saving = true;
    try {
      const trimmedModel = model.trim();
      if (initial.id) {
        await editTemplate(
          initial.id,
          name.trim(),
          prePrompt.trim() || (kind === 'browser' ? 'https://' : ''),
          trimmedModel || null,
          useWorktree
        );
      } else {
        await addTemplate(
          floorId,
          name.trim(),
          prePrompt.trim() || (kind === 'browser' ? 'https://' : ''),
          trimmedModel || null,
          kind === 'agent' ? useWorktree : false,
          kind === 'browser' ? 'browser' : kind === 'skill' ? 'skill' : MESH_DEFAULT_PROVIDER
        );
      }
      onClose();
    } catch (e) {
      addToast({ type: 'error', message: String(e), autoDismiss: true });
    } finally {
      saving = false;
    }
  }
</script>

<Modal {title} width="560px" zIndex={200} on:close={onClose}>
  <div class="field">
    <label class="label" for="tpl-name">
      {kind === 'browser' ? 'panel name' : 'role name'}
    </label>
    <input
      id="tpl-name"
      class="input"
      bind:value={name}
      placeholder={kind === 'browser' ? 'e.g. Docs' : 'e.g. Reader, Planner, Executor…'}
    />
  </div>

  {#if kind === 'agent'}
    <div class="field">
      <label class="label" for="tpl-prompt">pre-prompt (role instruction)</label>
      <textarea
        id="tpl-prompt"
        class="input textarea"
        bind:value={prePrompt}
        rows="10"
        placeholder="you are responsible for reading the project code and explaining the architecture..."
      ></textarea>
    </div>

    <div class="field">
      <label class="label" for="tpl-model">model (optional)</label>
      <input id="tpl-model" class="input" bind:value={model} placeholder="claude-sonnet-4-6" />
    </div>

    <label class="checkbox">
      <input type="checkbox" bind:checked={useWorktree} />
      <span>use isolated worktree</span>
    </label>
  {:else}
    <div class="field">
      <label class="label" for="tpl-url">initial url</label>
      <input
        id="tpl-url"
        class="input"
        bind:value={prePrompt}
        placeholder="https://docs.anthropic.com"
      />
      <span class="hint"> the panel opens this url when the node is added. </span>
    </div>
  {/if}

  <div class="actions">
    <button class="btn ghost" on:click={onClose} disabled={saving}>cancel</button>
    <button class="btn primary" on:click={onSave} disabled={saving}>
      {saving ? 'saving…' : 'save'}
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
    font-size: var(--sm);
    color: var(--t1);
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }
  .input {
    background: var(--bg);
    color: var(--t0);
    border: 1px solid var(--bd1);
    border-radius: var(--radius-sm);
    padding: var(--sp-4) var(--sp-5);
    font-size: var(--base);
    font-family: inherit;
    outline: none;
  }
  .input:focus {
    border-color: var(--ac);
  }
  .textarea {
    resize: vertical;
    min-height: 120px;
    line-height: 1.5;
  }
  .hint {
    font-size: var(--xs);
    color: var(--t2);
    line-height: 1.4;
  }
  .checkbox {
    display: flex;
    align-items: center;
    gap: var(--sp-3);
    font-size: var(--sm);
    color: var(--t0);
    cursor: pointer;
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: var(--sp-4);
    margin-top: var(--sp-3);
  }
  .btn {
    padding: var(--sp-3) var(--sp-7);
    border-radius: var(--radius-sm);
    font-size: var(--sm);
    border: 1px solid var(--bd1);
    text-transform: lowercase;
    letter-spacing: 0.3px;
    cursor: pointer;
    font-family: inherit;
  }
  .btn.ghost {
    background: transparent;
    color: var(--t0);
  }
  .btn.ghost:hover {
    background: var(--bg3);
  }
  .btn.primary {
    background: var(--ac);
    color: #000;
    border-color: var(--ac);
  }
  .btn.primary:hover {
    background: transparent;
    color: var(--ac);
  }
  .btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }
</style>
