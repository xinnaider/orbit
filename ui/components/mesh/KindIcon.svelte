<script lang="ts">
  export let kind: 'agent' | 'browser' | 'skill' | 'note';
  export let size = 14;
</script>

{#if kind === 'agent'}
  <svg
    class="kind-icon"
    viewBox="0 0 12 12"
    width={size}
    height={size}
    aria-hidden="true"
    focusable="false"
  >
    <circle cx="6" cy="4" r="2" fill="none" stroke="currentColor" stroke-width="1.5" />
    <path
      d="M2 11 C2 8 4 7 6 7 C8 7 10 8 10 11"
      fill="none"
      stroke="currentColor"
      stroke-width="1.5"
      stroke-linecap="round"
    />
  </svg>
{:else if kind === 'browser'}
  <svg
    class="kind-icon"
    viewBox="0 0 12 12"
    width={size}
    height={size}
    aria-hidden="true"
    focusable="false"
  >
    <rect
      x="1"
      y="2.5"
      width="10"
      height="7.5"
      rx="0.5"
      fill="none"
      stroke="currentColor"
      stroke-width="1.5"
    />
    <line x1="1" y1="5" x2="11" y2="5" stroke="currentColor" stroke-width="1.5" />
  </svg>
{:else if kind === 'skill'}
  <svg
    class="kind-icon"
    viewBox="0 0 12 12"
    width={size}
    height={size}
    aria-hidden="true"
    focusable="false"
  >
    <path
      d="M2.5 2 L9 2 L10 3 V10 H2.5 Z"
      fill="none"
      stroke="currentColor"
      stroke-width="1.5"
      stroke-linejoin="round"
    />
    <line
      x1="4"
      y1="5"
      x2="8"
      y2="5"
      stroke="currentColor"
      stroke-width="1.5"
      stroke-linecap="round"
    />
    <line
      x1="4"
      y1="7"
      x2="7"
      y2="7"
      stroke="currentColor"
      stroke-width="1.5"
      stroke-linecap="round"
    />
  </svg>
{:else if kind === 'note'}
  <svg
    class="kind-icon"
    viewBox="0 0 12 12"
    width={size}
    height={size}
    aria-hidden="true"
    focusable="false"
  >
    <rect
      x="2"
      y="1.5"
      width="8"
      height="9"
      rx="0.5"
      fill="none"
      stroke="currentColor"
      stroke-width="1.5"
    />
    <line
      x1="4"
      y1="4"
      x2="8"
      y2="4"
      stroke="currentColor"
      stroke-width="1.5"
      stroke-linecap="round"
    />
    <line
      x1="4"
      y1="6"
      x2="8"
      y2="6"
      stroke="currentColor"
      stroke-width="1.5"
      stroke-linecap="round"
    />
    <line
      x1="4"
      y1="8"
      x2="6.5"
      y2="8"
      stroke="currentColor"
      stroke-width="1.5"
      stroke-linecap="round"
    />
  </svg>
{/if}

<style>
  .kind-icon {
    display: inline-block;
    vertical-align: text-bottom;
    flex-shrink: 0;
  }
</style>
