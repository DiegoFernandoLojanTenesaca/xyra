<script lang="ts">
  import type { Component } from 'svelte';

  let { icon: IconComponent, text }: { icon: Component<{ size?: number }>; text: string } = $props();
</script>

<div>
  <IconComponent size={32} />
  <p>{text}</p>
</div>

<style>
  div {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-8) var(--space-5);
    color: var(--color-textFaint);
    text-align: center;
  }
  p {
    margin: 0;
    color: var(--color-textMuted);
  }
</style>
