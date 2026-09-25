<script lang="ts">
  import { Search } from '@lucide/svelte';
  import type { Snippet } from 'svelte';
  import type { HTMLInputAttributes } from 'svelte/elements';

  let { value = $bindable(''), children, ...rest }: { value?: string; children?: Snippet } & HTMLInputAttributes = $props();
</script>

<div>
  <Search size={16} />
  <input type="search" bind:value {...rest} />
  {@render children?.()}
</div>

<style>
  div {
    position: relative;
    display: flex;
    align-items: center;
    flex: 1;
    min-width: var(--size-searchMin);
    max-width: var(--size-searchMax);
  }
  div > :global(svg) {
    position: absolute;
    left: var(--space-3);
    color: var(--color-textMuted);
  }
  input {
    width: 100%;
    padding-left: var(--space-8);
  }
</style>
