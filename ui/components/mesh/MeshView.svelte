<script lang="ts">
  import { onMount } from 'svelte';
  import { get } from 'svelte/store';
  import {
    floorsStore,
    activeFloorId,
    loadFloors,
    addFloor,
    removeFloor,
  } from '../../lib/stores/mesh/floors';
  import {
    addTemplate,
    getBrowserUrl,
    getSkillSlug,
    isBrowserTemplate,
    isSkillTemplate,
    loadTemplates,
    removeTemplate,
    templatesStore,
  } from '../../lib/stores/mesh/templates';
  import { graphsStore, loadGraphs, addGraph, removeGraph } from '../../lib/stores/mesh/graphs';
  import { activeGraphId } from '../../lib/stores/mesh/graph';
  import { addToast } from '../../lib/stores/toasts';
  import GraphCanvas from './GraphCanvas.svelte';
  import TemplateEditor from './TemplateEditor.svelte';
  import InputPromptModal from './InputPromptModal.svelte';
  import NewGraphModal from './NewGraphModal.svelte';
  import ConfirmModal from './ConfirmModal.svelte';
  import SkillPickerModal from './SkillPickerModal.svelte';
  import KindIcon from './KindIcon.svelte';

  type TemplateForm = {
    id?: number;
    name: string;
    prePrompt: string;
    model: string | null;
    useWorktree: boolean;
    kind: 'agent' | 'browser' | 'skill';
  };

  let showTemplateEditor = false;
  let editingTemplate: TemplateForm | null = null;
  let showSkillPicker = false;
  let showNewGraphModal = false;

  $: agentTemplates = $templatesStore.filter(
    (t) => t.provider !== 'browser' && t.provider !== 'skill' && t.provider !== 'note'
  );
  $: browserTemplates = $templatesStore.filter((t) => t.provider === 'browser');
  $: skillTemplates = $templatesStore.filter((t) => t.provider === 'skill');

  let prompt: {
    title: string;
    label: string;
    placeholder: string;
    confirmLabel: string;
    onSubmit: (v: string) => void;
  } | null = null;

  let confirmDlg: {
    title: string;
    message: string;
    onConfirm: () => void;
  } | null = null;

  onMount(async () => {
    try {
      await loadFloors();
    } catch (e) {
      addToast({ type: 'error', message: `failed to load floors: ${e}`, autoDismiss: true });
    }
  });

  function openPrompt(opts: {
    title: string;
    label: string;
    placeholder?: string;
    confirmLabel?: string;
    onSubmit: (v: string) => void;
  }) {
    prompt = {
      title: opts.title,
      label: opts.label,
      placeholder: opts.placeholder ?? '',
      confirmLabel: opts.confirmLabel ?? 'ok',
      onSubmit: opts.onSubmit,
    };
  }

  function openConfirm(title: string, message: string, onConfirm: () => void) {
    confirmDlg = { title, message, onConfirm };
  }

  function onCreateFloor() {
    openPrompt({
      title: 'new floor',
      label: 'floor name',
      placeholder: 'e.g. auth workspace',
      confirmLabel: 'create',
      onSubmit: async (name) => {
        prompt = null;
        try {
          const f = await addFloor(name);
          activeFloorId.set(f.id);
          await loadTemplates(f.id);
          await loadGraphs(f.id);
        } catch (e) {
          addToast({ type: 'error', message: `failed to create floor: ${e}`, autoDismiss: true });
        }
      },
    });
  }

  async function onSelectFloor(id: number) {
    activeFloorId.set(id);
    activeGraphId.set(null);
    try {
      await loadTemplates(id);
      await loadGraphs(id);
    } catch (e) {
      addToast({ type: 'error', message: `failed to open floor: ${e}`, autoDismiss: true });
    }
  }

  function onDeleteFloor(id: number, e: Event) {
    e.stopPropagation();
    openConfirm('delete floor', 'everything inside this floor will be removed.', async () => {
      confirmDlg = null;
      try {
        await removeFloor(id);
        if (get(activeFloorId) === id) {
          activeFloorId.set(null);
          templatesStore.set([]);
          graphsStore.set([]);
          activeGraphId.set(null);
        }
      } catch (err) {
        addToast({ type: 'error', message: `failed to delete: ${err}`, autoDismiss: true });
      }
    });
  }

  function openNewTemplate(kind: 'agent' | 'browser') {
    if (kind === 'browser') {
      editingTemplate = {
        name: 'Browser',
        prePrompt: 'https://',
        model: null,
        useWorktree: false,
        kind: 'browser',
      };
    } else {
      editingTemplate = {
        name: '',
        prePrompt: '',
        model: null,
        useWorktree: true,
        kind: 'agent',
      };
    }
    showTemplateEditor = true;
  }

  function openSkillPicker() {
    showSkillPicker = true;
  }

  async function onSkillPicked(e: CustomEvent<{ slug: string; name: string }>) {
    showSkillPicker = false;
    const fid = get(activeFloorId);
    if (fid === null) return;
    try {
      // prePrompt stores the skill slug so the pipeline can re-read SKILL.md at spawn time.
      await addTemplate(fid, e.detail.name, e.detail.slug, null, false, 'skill');
    } catch (err) {
      addToast({ type: 'error', message: `failed to create skill: ${err}`, autoDismiss: true });
    }
  }

  function openEditTemplate(t: {
    id: number;
    name: string;
    prePrompt: string;
    model: string | null;
    useWorktree: boolean;
    provider: string;
  }) {
    editingTemplate = {
      id: t.id,
      name: t.name,
      prePrompt: t.prePrompt,
      model: t.model,
      useWorktree: t.useWorktree,
      kind: t.provider === 'browser' ? 'browser' : 'agent',
    };
    showTemplateEditor = true;
  }

  function onCreateGraph() {
    if (get(activeFloorId) === null) return;
    showNewGraphModal = true;
  }

  async function onNewGraphSubmit(e: CustomEvent<{ name: string; provider: string }>) {
    showNewGraphModal = false;
    const fid = get(activeFloorId);
    if (fid === null) return;
    try {
      const g = await addGraph(fid, e.detail.name, e.detail.provider);
      activeGraphId.set(g.id);
    } catch (err) {
      addToast({ type: 'error', message: `failed to create graph: ${err}`, autoDismiss: true });
    }
  }

  async function onSelectGraph(id: number) {
    activeGraphId.set(id);
  }

  function onDeleteGraph(id: number, e: Event) {
    e.stopPropagation();
    openConfirm('delete graph', 'the graph and all its nodes/edges will be removed.', async () => {
      confirmDlg = null;
      try {
        await removeGraph(id);
        if (get(activeGraphId) === id) activeGraphId.set(null);
      } catch (err) {
        addToast({ type: 'error', message: `failed to delete: ${err}`, autoDismiss: true });
      }
    });
  }

  function onDeleteTemplate(id: number, e: Event) {
    e.stopPropagation();
    openConfirm('delete template', 'the template will be removed from this floor.', async () => {
      confirmDlg = null;
      try {
        await removeTemplate(id);
      } catch (err) {
        addToast({ type: 'error', message: `failed to delete: ${err}`, autoDismiss: true });
      }
    });
  }
</script>

<div class="mesh-view">
  <aside class="floors-pane">
    <header>
      <h3>floors</h3>
      <button on:click={onCreateFloor} class="icon-btn" title="new floor">+</button>
    </header>
    <ul>
      {#each $floorsStore as f (f.id)}
        <li class:active={$activeFloorId === f.id}>
          <button class="floor-item" on:click={() => onSelectFloor(f.id)} type="button">
            {f.name}
          </button>
          <button class="icon-btn tiny" on:click={(e) => onDeleteFloor(f.id, e)} title="delete"
            >×</button
          >
        </li>
      {/each}
      {#if $floorsStore.length === 0}
        <li class="empty">no floors yet. click + to create one.</li>
      {/if}
    </ul>
  </aside>

  <main class="main-pane">
    {#if $activeFloorId === null}
      <div class="empty-state">
        <h2>select or create a floor</h2>
        <p>a floor is an isolated workspace of graphs and templates.</p>
      </div>
    {:else if $activeGraphId === null}
      <div class="floor-overview">
        <section>
          <header>
            <h3>templates</h3>
            <div class="header-actions">
              <button on:click={() => openNewTemplate('agent')} class="btn-primary">
                + agent
              </button>
              <button on:click={() => openNewTemplate('browser')} class="btn-primary ghost">
                + browser
              </button>
              <button on:click={openSkillPicker} class="btn-primary ghost"> + skill </button>
            </div>
          </header>

          {#if $templatesStore.length === 0}
            <div class="empty">no templates yet.</div>
          {/if}

          {#if agentTemplates.length > 0}
            <div class="group">
              <div class="group-title">
                <KindIcon kind="agent" /> agents · {agentTemplates.length}
              </div>
              <div class="cards">
                {#each agentTemplates as t (t.id)}
                  <div
                    role="button"
                    tabindex="0"
                    class="card"
                    on:click={() => openEditTemplate(t)}
                    on:keydown={(e) => e.key === 'Enter' && openEditTemplate(t)}
                  >
                    <div class="card-title">{t.name}</div>
                    <div class="card-sub">
                      {t.prePrompt.slice(0, 80)}{t.prePrompt.length > 80 ? '…' : ''}
                    </div>
                    <div class="card-meta">
                      {t.model ?? 'default model'} · {t.useWorktree ? 'worktree' : 'no worktree'}
                    </div>
                    <button
                      class="icon-btn tiny card-del"
                      on:click={(e) => onDeleteTemplate(t.id, e)}
                      title="delete">×</button
                    >
                  </div>
                {/each}
              </div>
            </div>
          {/if}

          {#if browserTemplates.length > 0}
            <div class="group">
              <div class="group-title">
                <KindIcon kind="browser" /> browsers · {browserTemplates.length}
              </div>
              <div class="cards">
                {#each browserTemplates as t (t.id)}
                  {@const url = isBrowserTemplate(t) ? getBrowserUrl(t) : ''}
                  <div
                    role="button"
                    tabindex="0"
                    class="card card-browser"
                    on:click={() => openEditTemplate(t)}
                    on:keydown={(e) => e.key === 'Enter' && openEditTemplate(t)}
                  >
                    <div class="card-title">{t.name}</div>
                    <div class="card-sub">
                      {url.slice(0, 80)}{url.length > 80 ? '…' : ''}
                    </div>
                    <div class="card-meta">browser panel</div>
                    <button
                      class="icon-btn tiny card-del"
                      on:click={(e) => onDeleteTemplate(t.id, e)}
                      title="delete">×</button
                    >
                  </div>
                {/each}
              </div>
            </div>
          {/if}

          {#if skillTemplates.length > 0}
            <div class="group">
              <div class="group-title">
                <KindIcon kind="skill" /> skills · {skillTemplates.length}
              </div>
              <div class="cards">
                {#each skillTemplates as t (t.id)}
                  {@const slug = isSkillTemplate(t) ? getSkillSlug(t) : ''}
                  <div class="card card-skill" role="presentation">
                    <div class="card-title">{t.name}</div>
                    <div class="card-sub">
                      {slug.slice(0, 80)}{slug.length > 80 ? '…' : ''}
                    </div>
                    <div class="card-meta">claude skill · {slug}</div>
                    <button
                      class="icon-btn tiny card-del"
                      on:click={(e) => onDeleteTemplate(t.id, e)}
                      title="delete">×</button
                    >
                  </div>
                {/each}
              </div>
            </div>
          {/if}
        </section>

        <section>
          <header>
            <h3>graphs</h3>
            <button on:click={onCreateGraph} class="btn-primary">+ graph</button>
          </header>
          <div class="cards">
            {#each $graphsStore as g (g.id)}
              <div
                role="button"
                tabindex="0"
                class="card"
                on:click={() => onSelectGraph(g.id)}
                on:keydown={(e) => e.key === 'Enter' && onSelectGraph(g.id)}
              >
                <div class="card-title">{g.name}</div>
                <button
                  class="icon-btn tiny card-del"
                  on:click={(e) => onDeleteGraph(g.id, e)}
                  title="delete">×</button
                >
              </div>
            {/each}
            {#if $graphsStore.length === 0}
              <div class="empty">no graphs yet.</div>
            {/if}
          </div>
        </section>
      </div>
    {:else}
      <GraphCanvas />
    {/if}
  </main>
</div>

{#if showTemplateEditor && editingTemplate !== null && $activeFloorId !== null}
  <TemplateEditor
    initial={editingTemplate}
    floorId={$activeFloorId}
    onClose={() => {
      showTemplateEditor = false;
      editingTemplate = null;
    }}
  />
{/if}

{#if prompt}
  <InputPromptModal
    title={prompt.title}
    label={prompt.label}
    placeholder={prompt.placeholder}
    confirmLabel={prompt.confirmLabel}
    on:submit={(e) => prompt?.onSubmit(e.detail)}
    on:cancel={() => (prompt = null)}
  />
{/if}

{#if confirmDlg}
  <ConfirmModal
    title={confirmDlg.title}
    message={confirmDlg.message}
    on:confirm={() => confirmDlg?.onConfirm()}
    on:cancel={() => (confirmDlg = null)}
  />
{/if}

{#if showSkillPicker}
  <SkillPickerModal on:submit={onSkillPicked} on:cancel={() => (showSkillPicker = false)} />
{/if}

{#if showNewGraphModal}
  <NewGraphModal on:submit={onNewGraphSubmit} on:cancel={() => (showNewGraphModal = false)} />
{/if}

<style>
  .mesh-view {
    display: flex;
    flex: 1;
    height: 100%;
    overflow: hidden;
    background: var(--bg);
    color: var(--t0);
  }

  .floors-pane {
    width: 220px;
    border-right: 1px solid var(--bd);
    display: flex;
    flex-direction: column;
    background: var(--bg1);
  }

  .floors-pane header,
  .main-pane header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: var(--sp-5) var(--sp-7);
    border-bottom: 1px solid var(--bd);
  }

  .floors-pane h3,
  .main-pane h3 {
    margin: 0;
    font-size: var(--sm);
    font-weight: 600;
    letter-spacing: 0.3px;
    text-transform: lowercase;
    color: var(--t1);
  }

  .floors-pane ul {
    list-style: none;
    padding: var(--sp-2) 0;
    overflow-y: auto;
    flex: 1;
  }

  .floors-pane li {
    display: flex;
    align-items: center;
    justify-content: space-between;
    font-size: var(--base);
  }

  .floor-item {
    flex: 1;
    text-align: left;
    background: transparent;
    border: none;
    color: var(--t0);
    padding: var(--sp-4) var(--sp-7);
    font-size: var(--base);
    font-family: inherit;
    cursor: pointer;
  }
  .floors-pane li:hover {
    background: var(--bg3);
  }
  .floors-pane li.active {
    background: var(--ac-d);
    border-left: 2px solid var(--ac);
  }
  .floors-pane li.active .floor-item {
    color: var(--ac);
  }
  .floors-pane li.empty {
    color: var(--t2);
    font-size: var(--sm);
    font-style: italic;
    padding: var(--sp-7);
    cursor: default;
  }

  .main-pane {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .empty-state {
    margin: auto;
    text-align: center;
    color: var(--t2);
    font-size: var(--base);
  }
  .empty-state h2 {
    font-size: var(--lg);
    font-weight: 600;
    color: var(--t0);
    margin-bottom: var(--sp-4);
  }

  .floor-overview {
    padding: var(--sp-9);
    overflow-y: auto;
    flex: 1;
  }
  .floor-overview section {
    margin-bottom: var(--sp-9);
  }

  .header-actions {
    display: flex;
    gap: var(--sp-3);
  }

  .group {
    margin-top: var(--sp-7);
  }
  .group:first-of-type {
    margin-top: var(--sp-5);
  }
  .group-title {
    display: inline-flex;
    align-items: center;
    gap: var(--sp-2);
    font-size: var(--xs);
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--t2);
    margin-bottom: var(--sp-3);
    font-weight: 600;
  }

  .cards {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(260px, 1fr));
    gap: var(--sp-6);
    margin-top: var(--sp-3);
  }

  .card {
    position: relative;
    padding: var(--sp-7);
    border-radius: var(--radius-md);
    background: var(--bg2);
    border: 1px solid var(--bd1);
    cursor: pointer;
    transition:
      border-color 0.1s,
      background 0.1s;
    text-align: left;
    font-family: inherit;
    color: var(--t0);
  }
  .card:hover {
    background: var(--bg3);
    border-color: var(--bd2);
  }
  .card.card-browser {
    border-left: 3px solid var(--s-init);
  }
  .card.card-skill {
    border-left: 3px solid var(--think-fg);
  }

  .card-title {
    font-weight: 600;
    font-size: var(--base);
    margin-bottom: var(--sp-3);
    display: flex;
    align-items: center;
    gap: var(--sp-3);
  }
  .card-sub {
    font-size: var(--sm);
    color: var(--t1);
    margin-bottom: var(--sp-4);
    line-height: 1.4;
  }
  .card-meta {
    font-size: var(--xs);
    color: var(--t2);
    text-transform: lowercase;
  }
  .card-del {
    position: absolute;
    top: var(--sp-3);
    right: var(--sp-3);
  }

  .empty {
    color: var(--t2);
    font-size: var(--sm);
    grid-column: 1 / -1;
    text-align: center;
    padding: var(--sp-9);
    font-style: italic;
  }

  .icon-btn {
    background: transparent;
    border: 1px solid var(--bd1);
    color: var(--t0);
    cursor: pointer;
    border-radius: var(--radius-sm);
    width: 24px;
    height: 24px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    font-size: var(--md);
  }
  .icon-btn.tiny {
    width: 18px;
    height: 18px;
    font-size: var(--xs);
  }
  .icon-btn:hover {
    background: var(--bg3);
    border-color: var(--ac);
    color: var(--ac);
  }

  .btn-primary {
    background: var(--ac);
    color: #000;
    border: 1px solid var(--ac);
    border-radius: var(--radius-sm);
    padding: var(--sp-3) var(--sp-6);
    font-size: var(--sm);
    cursor: pointer;
    font-family: inherit;
    text-transform: lowercase;
    letter-spacing: 0.3px;
  }
  .btn-primary:hover {
    background: transparent;
    color: var(--ac);
  }
  .btn-primary.ghost {
    background: transparent;
    color: var(--t0);
    border-color: var(--bd1);
  }
  .btn-primary.ghost:hover {
    border-color: var(--s-init);
    color: var(--s-init);
  }
</style>
