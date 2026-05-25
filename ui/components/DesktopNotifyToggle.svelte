<script lang="ts">
  import { Bell, BellOff } from 'lucide-svelte';
  import { notificationsEnabled } from '../lib/stores/preferences';
  import { HAS_TAURI } from '../lib/tauri/invoke';
  import { setDesktopNotificationsEnabled } from '../lib/tauri/desktop';

  async function onToggle() {
    const next = !$notificationsEnabled;
    notificationsEnabled.set(next);
    if (HAS_TAURI) {
      try {
        await setDesktopNotificationsEnabled(next);
      } catch (e) {
        console.warn('Failed to sync notification preference', e);
      }
    }
  }

  $: enabled = $notificationsEnabled;
  $: title = enabled
    ? 'Desktop notifications on — click to mute'
    : 'Desktop notifications muted — click to unmute';
</script>

{#if HAS_TAURI}
  <button
    type="button"
    class="notify-btn"
    class:muted={!enabled}
    {title}
    aria-label={title}
    aria-pressed={enabled}
    onclick={onToggle}
  >
    {#if enabled}
      <Bell size={13} />
    {:else}
      <BellOff size={13} />
    {/if}
  </button>
{/if}

<style>
  .notify-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    width: 26px;
    height: 26px;
    background: none;
    border: none;
    border-radius: var(--radius-sm);
    color: var(--ac);
    cursor: pointer;
    transition:
      color 0.15s,
      background 0.15s;
  }
  .notify-btn:hover {
    color: var(--t0);
    background: color-mix(in srgb, var(--t0), transparent 94%);
  }
  .notify-btn.muted {
    color: var(--t3);
  }
  .notify-btn.muted:hover {
    color: var(--t1);
  }
</style>
