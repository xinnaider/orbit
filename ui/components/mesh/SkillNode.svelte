<script lang="ts">
  import { Handle, Position, NodeResizer } from '@xyflow/svelte';
  import { onMount } from 'svelte';
  import { get } from 'svelte/store';
  import { meshReadSkill } from '../../lib/tauri/mesh';
  import { resizeNode as persistResize } from '../../lib/stores/mesh/graph';
  import { availableSkills, ensureSkillsLoaded } from '../../lib/stores/mesh/skills';
  import { addToast } from '../../lib/stores/toasts';

  type SkillNodeData = {
    label: string;
    templateName: string;
    prePrompt: string; // holds the skill slug
  };

  export let id: string;
  export let data: SkillNodeData;

  let description = '';
  let preview = '';

  onMount(async () => {
    // Cache-first: ensureSkillsLoaded triggers one shared IPC for all skill nodes.
    try {
      await ensureSkillsLoaded();
      const cached = get(availableSkills).find((s) => s.slug === data.prePrompt);
      const skill = cached ?? (await meshReadSkill(data.prePrompt));
      description = skill.description;
      preview = skill.content.split('\n').slice(0, 15).join('\n');
    } catch (e) {
      preview = '(failed to read SKILL.md)';
      addToast({ type: 'error', message: `skill '${data.prePrompt}': ${e}`, autoDismiss: true });
    }
  });

  async function onResizeEnd(_e: unknown, params: { width: number; height: number }) {
    try {
      await persistResize(Number(id), params.width, params.height);
    } catch (e) {
      addToast({ type: 'error', message: `failed to persist size: ${e}`, autoDismiss: true });
    }
  }
</script>

<div class="skill-node">
  <NodeResizer
    minWidth={240}
    minHeight={180}
    lineClass="resize-line"
    handleClass="resize-handle"
    {onResizeEnd}
  />
  <!-- Loose mode + per-side source handles; onConnect normalises agent↔skill. -->
  <Handle id="top" type="source" position={Position.Top} />
  <Handle id="left" type="source" position={Position.Left} />
  <Handle id="right" type="source" position={Position.Right} />
  <Handle id="bottom" type="source" position={Position.Bottom} />

  <header class="sk-header">
    <span class="dot"></span>
    <strong>{data.label}</strong>
    <span class="kind-badge">skill</span>
  </header>

  <div class="sk-slug">~/.claude/skills/{data.prePrompt}/</div>

  {#if description}
    <div class="sk-desc">{description}</div>
  {/if}

  <pre class="sk-preview">{preview}</pre>
</div>

<style>
  .skill-node {
    position: relative;
    width: 100%;
    height: 100%;
    min-width: 240px;
    min-height: 180px;
    background: var(--bg);
    border: 1px solid var(--bd1);
    border-left: 3px solid var(--think-fg);
    border-radius: var(--radius-md);
    display: flex;
    flex-direction: column;
    overflow: hidden;
    color: var(--t0);
    font-family: var(--mono);
  }

  .sk-header {
    display: flex;
    align-items: center;
    gap: var(--sp-3);
    padding: var(--sp-3) var(--sp-5);
    background: var(--bg1);
    border-bottom: 1px solid var(--bd);
    flex-shrink: 0;
    font-size: var(--sm);
  }
  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--think-fg);
  }
  .kind-badge {
    font-size: var(--xs);
    color: var(--think-fg);
    background: var(--think-bg);
    padding: 1px 6px;
    border-radius: 10px;
    text-transform: lowercase;
  }

  .sk-slug {
    font-size: var(--xs);
    color: var(--t2);
    padding: var(--sp-3) var(--sp-5) 0;
    font-family: var(--mono);
  }

  .sk-desc {
    font-size: var(--sm);
    color: var(--t1);
    padding: var(--sp-3) var(--sp-5);
    line-height: 1.4;
  }

  .sk-preview {
    flex: 1;
    margin: 0;
    padding: var(--sp-3) var(--sp-5);
    font-size: var(--xs);
    color: var(--t2);
    background: var(--bg1);
    overflow: auto;
    white-space: pre-wrap;
    border-top: 1px solid var(--bd);
    font-family: var(--mono);
  }

  :global(.svelte-flow__node-skill .svelte-flow__handle) {
    background: var(--think-fg);
    width: 24px;
    height: 24px;
    border: 2px solid var(--bg);
    z-index: 10;
  }
  :global(.svelte-flow__node-skill .svelte-flow__handle:hover) {
    background: var(--ac);
  }
  :global(.svelte-flow__node-skill .resize-handle) {
    width: 16px;
    height: 16px;
    background: var(--think-fg);
    border: 2px solid var(--bg);
    border-radius: 3px;
  }
  :global(.svelte-flow__node-skill .resize-line) {
    border-color: var(--think-fg);
    border-width: 3px;
  }
</style>
