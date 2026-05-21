<script lang="ts">
  import { journal, pendingMessages } from './journal';
  import type { JournalEntry } from '../types';

  export let session: { id: number } | null = null;

  // Exato espelho dos reactive blocks do CentralPanel
  $: entries = session?.id == null ? [] : ($journal.get(session.id) ?? []);

  let prevEntryCount = 0;
  $: {
    const count = entries.length;
    if (count > prevEntryCount) {
      const last = entries[count - 1];
      if (
        last &&
        (last.entryType === 'user' ||
          last.entryType === 'assistant' ||
          last.entryType === 'toolCall')
      ) {
        pendingMessages.clear();
      }
    }
    prevEntryCount = count;
  }

  // Estado derivado pro teste
  $: feedEmpty = entries.length === 0 && $pendingMessages.length === 0;
  $: entryCount = entries.length;
  $: pendingCount = $pendingMessages.length;
</script>

<div data-testid="harness">
  {#if feedEmpty}
    <div class="feed-empty" data-testid="feed-empty">empty</div>
  {:else}
    <div class="feed" data-testid="feed">
      {#each entries as entry (entry.seq)}
        <div class="entry" data-testid="entry-{entry.seq}">{entry.text}</div>
      {/each}
    </div>
  {/if}
  {#if pendingCount > 0}
    <div class="pending" data-testid="pending">
      {#each $pendingMessages as msg (msg.id)}
        <div>{msg.text}</div>
      {/each}
    </div>
  {/if}
  <div data-testid="counts" data-entries={entryCount} data-pending={pendingCount}></div>
</div>
