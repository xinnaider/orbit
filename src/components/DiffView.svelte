<script lang="ts">
  export let sessionId: string;

  // Placeholder — will be populated from diff_builder commands
  let diffContent = 'Select an edit in the log to view its diff';
</script>

<div class="diff-view">
  <div class="diff-content mono">
    <p class="placeholder">{diffContent}</p>
  </div>
</div>

<style>
  .diff-view { padding: 8px; font-size: 10px; }
  .diff-content { line-height: 1.7; }
  .placeholder { color: var(--text-dim); padding: 20px; text-align: center; }
</style>
