<script lang="ts">
  import type { Snippet } from 'svelte';

  let { icon, name, highlighted = false, children }: { icon: string; name: string; highlighted?: boolean; children?: Snippet } = $props();
</script>

<div class="chip" class:highlighted title={name}>
  {#if icon}<img src={icon} alt="" />{/if}
  <span>{name}</span>
  {#if children}<span class="extra">{@render children()}</span>{/if}
</div>

<style>
  .chip {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    min-width: 0;
    padding: var(--space-2) var(--space-3);
    background: var(--color-panelRaised);
    border: var(--border-hairline) solid var(--color-line);
  }
  .chip.highlighted {
    border-color: var(--color-accent);
    background: color-mix(in srgb, var(--color-accent) 14%, var(--color-panelRaised));
  }
  img {
    width: var(--size-thumb);
    height: var(--size-thumb);
    flex: none;
  }
  span {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: var(--text-md);
  }
  .extra {
    flex: none;
    display: flex;
    align-items: center;
    color: var(--color-accentBright);
  }
</style>
