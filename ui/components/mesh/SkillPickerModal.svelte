<script lang="ts">
  import { createEventDispatcher, onMount } from 'svelte';
  import { availableSkills, ensureSkillsLoaded } from '../../lib/stores/mesh/skills';
  import Modal from '../shared/Modal.svelte';

  const dispatch = createEventDispatcher<{
    submit: { slug: string; name: string };
    cancel: void;
  }>();

  let filter = '';

  onMount(ensureSkillsLoaded);

  function focusOnMount(node: HTMLInputElement) {
    node.focus();
    return { destroy() {} };
  }

  $: filtered = $availableSkills.filter((s) => {
    if (!filter.trim()) return true;
    const q = filter.toLowerCase();
    return s.name.toLowerCase().includes(q) || s.description.toLowerCase().includes(q);
  });

  function pick(slug: string, name: string) {
    dispatch('submit', { slug, name });
  }
</script>

<Modal title="pick a skill" width="600px" zIndex={200} on:close={() => dispatch('cancel')}>
  <input class="search" placeholder="search skills…" bind:value={filter} use:focusOnMount />

  <div class="list">
    {#each filtered as s (s.slug)}
      <button class="skill-item" type="button" on:click={() => pick(s.slug, s.name)}>
        <div class="name">{s.name}</div>
        <div class="desc">{s.description}</div>
        <div class="slug">{s.slug}</div>
      </button>
    {/each}
    {#if filtered.length === 0}
      <div class="empty">no skills found in ~/.claude/skills/</div>
    {/if}
  </div>
</Modal>

<style>
  .search {
    background: var(--bg);
    color: var(--t0);
    border: 1px solid var(--bd1);
    border-radius: var(--radius-sm);
    padding: var(--sp-4) var(--sp-5);
    font-size: var(--base);
    font-family: inherit;
    outline: none;
  }
  .search:focus {
    border-color: var(--ac);
  }

  .list {
    display: flex;
    flex-direction: column;
    gap: var(--sp-3);
    max-height: 420px;
    overflow-y: auto;
  }

  .skill-item {
    text-align: left;
    background: var(--bg);
    border: 1px solid var(--bd1);
    border-radius: var(--radius-sm);
    padding: var(--sp-5);
    font-family: inherit;
    color: var(--t0);
    cursor: pointer;
    display: flex;
    flex-direction: column;
    gap: var(--sp-2);
  }
  .skill-item:hover {
    border-color: var(--ac);
    background: var(--bg3);
  }
  .name {
    font-size: var(--base);
    font-weight: 600;
  }
  .desc {
    font-size: var(--sm);
    color: var(--t1);
    line-height: 1.4;
  }
  .slug {
    font-size: var(--xs);
    color: var(--t2);
    font-family: var(--mono);
  }
  .empty {
    color: var(--t2);
    font-size: var(--sm);
    text-align: center;
    padding: var(--sp-9);
    font-style: italic;
  }
</style>
