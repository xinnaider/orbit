<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import type { TaskItem } from '../lib/types';
  import { getSessionTasks } from '../lib/tauri';
  import { taskUpdateTrigger } from '../lib/stores/tasks';

  export let sessionId: string;

  let tasks: TaskItem[] = [];
  let timer: ReturnType<typeof setInterval>;

  async function load() {
    try {
      tasks = await getSessionTasks(sessionId);
    } catch (_e) {
      /* no-op */
    }
  }

  onMount(() => {
    load();
    timer = setInterval(load, 3000);
  });
  onDestroy(() => clearInterval(timer));

  const unsub = taskUpdateTrigger.subscribe((id) => {
    if (id === Number(sessionId)) load();
  });
  onDestroy(() => unsub());

  $: if (sessionId) load();

  $: done = tasks.filter((t) => t.status === 'completed').length;
  $: total = tasks.length;
  $: pct = total > 0 ? (done / total) * 100 : 0;

  function icon(s: string) {
    if (s === 'completed') return '✓';
    if (s === 'in_progress') return '▸';
    return '○';
  }
  function cls(s: string) {
    if (s === 'completed') return 'done';
    if (s === 'in_progress') return 'active';
    return 'pending';
  }
</script>

<div class="tasks">
  {#if tasks.length === 0}
    <p class="empty">no tasks</p>
  {:else}
    <div class="progress-row">
      <div class="bar"><div class="fill" style="width:{pct}%"></div></div>
      <span class="count">{done}/{total}</span>
    </div>
    {#each tasks as t}
      <div class="task {cls(t.status)}">
        <span class="task-icon">{icon(t.status)}</span>
        <span class="task-name">{t.subject}</span>
      </div>
    {/each}
  {/if}
</div>

<style>
  .tasks {
    padding: var(--sp-5) var(--sp-6);
    display: flex;
    flex-direction: column;
    gap: var(--sp-2);
  }
  .empty {
    font-size: var(--xs);
    color: var(--t3);
    padding: var(--sp-2) 0;
  }

  .progress-row {
    display: flex;
    align-items: center;
    gap: var(--sp-4);
    margin-bottom: var(--sp-3);
  }
  .bar {
    flex: 1;
    height: 2px;
    background: var(--bg3);
    border-radius: 1px;
    overflow: hidden;
  }
  .fill {
    height: 100%;
    background: var(--ac);
    transition: width 0.3s;
  }
  .count {
    font-size: var(--xs);
    color: var(--t2);
    flex-shrink: 0;
  }

  .task {
    display: flex;
    align-items: flex-start;
    gap: var(--sp-3);
    padding: var(--sp-2) 0;
  }
  .task-icon {
    font-size: var(--xs);
    flex-shrink: 0;
    margin-top: 1px;
  }
  .done .task-icon {
    color: var(--s-working);
  }
  .active .task-icon {
    color: var(--s-input);
  }
  .pending .task-icon {
    color: var(--t3);
  }
  .task-name {
    font-size: var(--xs);
    color: var(--t1);
    line-height: 1.4;
  }
  .done .task-name {
    color: var(--t2);
    text-decoration: line-through;
  }
  .active .task-name {
    color: var(--t0);
  }
</style>
