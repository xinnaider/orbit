<script lang="ts">
  export let label: string = 'Thinking';
</script>

<div class="typing">
  <span class="label">{label}</span>
  <span class="dots">
    <span class="dot"></span>
    <span class="dot"></span>
    <span class="dot"></span>
  </span>
</div>

<style>
  .typing {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 14px;
  }
  .label {
    font-size: 12px;
    color: var(--text-dim);
    font-style: italic;
  }
  .dots {
    display: flex;
    gap: 3px;
    align-items: center;
  }
  .dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--text-dim);
    animation: bounce 1.4s ease-in-out infinite;
  }
  .dot:nth-child(1) {
    animation-delay: 0s;
  }
  .dot:nth-child(2) {
    animation-delay: 0.2s;
  }
  .dot:nth-child(3) {
    animation-delay: 0.4s;
  }

  @keyframes bounce {
    0%,
    60%,
    100% {
      transform: translateY(0);
      opacity: 0.4;
    }
    30% {
      transform: translateY(-4px);
      opacity: 1;
    }
  }
</style>
