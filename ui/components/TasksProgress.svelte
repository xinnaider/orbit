<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import type { TaskItem } from '../lib/types';
  import { getSessionTasks } from '../lib/tauri';

  export let sessionId: string;

  let tasks: TaskItem[] = [];
  let pollInterval: ReturnType<typeof setInterval>;

  async function loadTasks() {
    try {
      tasks = await getSessionTasks(sessionId);
    } catch (_e) {
      // ignore
    }
  }

  onMount(() => {
    loadTasks();
    pollInterval = setInterval(loadTasks, 3000);
  });

  onDestroy(() => {
    if (pollInterval) clearInterval(pollInterval);
  });

  $: if (sessionId) loadTasks();

  $: completedCount = tasks.filter((t) => t.status === 'completed').length;
  $: inProgressCount = tasks.filter((t) => t.status === 'in_progress').length;
  $: pendingCount = tasks.filter((t) => t.status === 'pending').length;
  $: totalCount = tasks.length;
  $: progressPercent = totalCount > 0 ? (completedCount / totalCount) * 100 : 0;

  function statusDot(status: string): string {
    if (status === 'completed') return 'dot-completed';
    if (status === 'in_progress') return 'dot-progress';
    return 'dot-pending';
  }

  function statusIcon(status: string): string {
    if (status === 'completed') return '✓';
    if (status === 'in_progress') return '▸';
    return '○';
  }
</script>

<div class="tasks-panel">
  {#if totalCount === 0}
    <p class="empty">No tasks in this session</p>
  {:else}
    <!-- Progress bar -->
    <div class="progress-section">
      <div class="progress-header">
        <span class="progress-label">{completedCount}/{totalCount} completed</span>
        <span class="progress-pct">{Math.round(progressPercent)}%</span>
      </div>
      <div class="progress-bar">
        <div class="progress-fill" style="width: {progressPercent}%"></div>
      </div>
      <div class="status-counts">
        {#if inProgressCount > 0}
          <span class="count-item"
            ><span class="dot dot-progress"></span>{inProgressCount} in progress</span
          >
        {/if}
        {#if pendingCount > 0}
          <span class="count-item"><span class="dot dot-pending"></span>{pendingCount} pending</span
          >
        {/if}
      </div>
    </div>

    <!-- Task list -->
    <div class="task-list">
      {#each tasks as task (task.id)}
        <div
          class="task-item"
          class:completed={task.status === 'completed'}
          class:in-progress={task.status === 'in_progress'}
        >
          <div class="task-header">
            <span class="task-icon {statusDot(task.status)}">{statusIcon(task.status)}</span>
            <span class="task-subject">
              {#if task.status === 'in_progress' && task.activeForm}
                {task.activeForm}
              {:else}
                {task.subject}
              {/if}
            </span>
          </div>

          <!-- Dependency indicators -->
          {#if task.blockedBy.length > 0}
            <div class="deps">
              <span class="dep-label">blocked by:</span>
              {#each task.blockedBy as depId}
                {@const depTask = tasks.find((t) => t.id === depId)}
                {#if depTask}
                  <span class="dep-chip" class:dep-done={depTask.status === 'completed'}>
                    #{depId}
                  </span>
                {/if}
              {/each}
            </div>
          {/if}
          {#if task.blocks.length > 0}
            <div class="deps">
              <span class="dep-label">blocks:</span>
              {#each task.blocks as depId}
                <span class="dep-chip dep-blocks">#{depId}</span>
              {/each}
            </div>
          {/if}
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .tasks-panel {
    padding: 10px 12px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .empty {
    color: var(--text-dim);
    font-size: 13px;
    text-align: center;
    padding: 20px;
  }

  /* Progress section */
  .progress-section {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .progress-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  .progress-label {
    font-size: 12px;
    font-weight: 600;
    color: var(--text-primary);
  }
  .progress-pct {
    font-size: 11px;
    color: var(--text-dim);
  }
  .progress-bar {
    height: 6px;
    background: var(--bg-overlay);
    border-radius: 3px;
    overflow: hidden;
  }
  .progress-fill {
    height: 100%;
    background: var(--green);
    border-radius: 3px;
    transition: width 0.3s ease;
  }
  .status-counts {
    display: flex;
    gap: 10px;
  }
  .count-item {
    display: flex;
    align-items: center;
    gap: 4px;
    font-size: 11px;
    color: var(--text-muted);
  }
  .dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    flex-shrink: 0;
  }
  .dot-completed {
    background: var(--green);
  }
  .dot-progress {
    background: var(--blue);
  }
  .dot-pending {
    background: var(--text-dim);
  }

  /* Task list */
  .task-list {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .task-item {
    padding: 6px 8px;
    border-radius: 6px;
    border: 1px solid transparent;
  }
  .task-item.in-progress {
    background: color-mix(in srgb, var(--blue) 8%, transparent);
    border-color: color-mix(in srgb, var(--blue) 20%, transparent);
  }
  .task-item.completed {
    opacity: 0.5;
  }
  .task-header {
    display: flex;
    align-items: flex-start;
    gap: 6px;
  }
  .task-icon {
    font-size: 11px;
    flex-shrink: 0;
    width: 16px;
    text-align: center;
    margin-top: 1px;
  }
  .task-icon.dot-completed {
    color: var(--green);
  }
  .task-icon.dot-progress {
    color: var(--blue);
  }
  .task-icon.dot-pending {
    color: var(--text-dim);
  }
  .task-subject {
    font-size: 12px;
    color: var(--text-primary);
    line-height: 1.3;
  }

  /* Dependencies */
  .deps {
    display: flex;
    align-items: center;
    gap: 4px;
    margin-left: 22px;
    margin-top: 3px;
    flex-wrap: wrap;
  }
  .dep-label {
    font-size: 10px;
    color: var(--text-dim);
  }
  .dep-chip {
    font-size: 10px;
    padding: 0 5px;
    border-radius: 3px;
    background: var(--bg-overlay);
    color: var(--text-muted);
    font-family: 'Cascadia Code', monospace;
  }
  .dep-chip.dep-done {
    background: var(--green-dim);
    color: var(--green);
    text-decoration: line-through;
  }
  .dep-chip.dep-blocks {
    background: var(--amber-dim);
    color: var(--amber);
  }
</style>
