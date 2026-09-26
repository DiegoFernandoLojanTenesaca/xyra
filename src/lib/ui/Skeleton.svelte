<script lang="ts">
  let { label, rows = 6 }: { label: string; rows?: number } = $props();
</script>

<div role="status" aria-label={label}>
  {#each Array.from({ length: rows }, (_, i) => i) as i (i)}
    <span style="--i:{i}"></span>
  {/each}
</div>

<style>
  div {
    display: grid;
    gap: var(--space-3);
  }
  span {
    height: var(--size-thumb);
    background: linear-gradient(90deg, var(--color-panel), var(--color-panelRaised), var(--color-panel));
    background-size: 200% 100%;
    animation: shimmer 1.4s ease-in-out infinite;
    animation-delay: calc(var(--i) * 80ms);
  }
  span:nth-child(3n + 2) {
    width: 85%;
  }
  span:nth-child(3n) {
    width: 70%;
  }
  @keyframes shimmer {
    from {
      background-position: 100% 0;
    }
    to {
      background-position: -100% 0;
    }
  }
</style>
