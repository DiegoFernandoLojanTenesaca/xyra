<script lang="ts">
  import type { Snippet } from 'svelte';

  let { title, subtitle = '', eyebrow = '', children }: { title: string; subtitle?: string; eyebrow?: string; children?: Snippet } = $props();
</script>

<header>
  <div>
    {#if eyebrow}<span class="eyebrow">{eyebrow}</span>{/if}
    <h1 class="condensed">{title}<span class="accent">.</span></h1>
    {#if subtitle}<p class="muted">{subtitle}</p>{/if}
  </div>
  {#if children}<div class="actions">{@render children()}</div>{/if}
</header>

<style>
  header {
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    gap: var(--space-5);
    margin-bottom: var(--space-6);
  }
  h1 {
    margin: var(--space-1) 0 0;
    font-size: var(--text-4xl);
  }
  p {
    margin: var(--space-2) 0 0;
    font-size: var(--text-md);
    letter-spacing: var(--tracking-tight);
  }
  .actions {
    display: flex;
    gap: var(--space-3);
  }
</style>
