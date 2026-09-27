<script lang="ts">
  import { ArrowDown, ArrowUp, ArrowUpDown } from '@lucide/svelte';
  import type { Component } from 'svelte';
  import type { ChampionChange, ChangeVerdict } from '../types';

  const ICONS: Record<ChangeVerdict, Component<{ size?: number }>> = { buff: ArrowUp, nerf: ArrowDown, adjusted: ArrowUpDown };

  let { change, label }: { change: ChampionChange; label: string } = $props();
  const Icon = $derived(ICONS[change.verdict]);
</script>

<details class="panel cut {change.verdict}">
  <summary>
    {#if change.champion.icon}<img src={change.champion.icon} alt="" />{/if}
    <span class="name">
      <b>{change.champion.name}</b>
      {#if change.context}<small class="muted">{change.context}</small>{/if}
    </span>
    <span class="verdict"><Icon size={14} />{label}</span>
  </summary>
  {#each change.groups as group, i (i)}
    {#if group.title}<h4>{group.title}</h4>{/if}
    <ul>
      {#each group.lines as line, j (j)}<li>{line}</li>{/each}
    </ul>
  {/each}
</details>

<style>
  details {
    --verdict-color: var(--color-warning);
    padding: 0 var(--space-4);
  }
  .buff {
    --verdict-color: var(--color-success);
  }
  .nerf {
    --verdict-color: var(--color-accentBright);
  }
  summary {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-3) 0;
    cursor: pointer;
    list-style: none;
  }
  img {
    width: var(--size-thumb);
    height: var(--size-thumb);
    flex: none;
  }
  .name {
    display: grid;
    flex: 1;
    min-width: 0;
  }
  .name small {
    display: -webkit-box;
    overflow: hidden;
    font-size: var(--text-sm);
    -webkit-box-orient: vertical;
    -webkit-line-clamp: 2;
    line-clamp: 2;
  }
  details[open] .name small {
    display: block;
  }
  .verdict {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    flex: none;
    color: var(--verdict-color);
    font-size: var(--text-xs);
    font-weight: 800;
    letter-spacing: var(--tracking-wide);
    text-transform: uppercase;
  }
  h4 {
    margin: 0 0 var(--space-1);
    font-size: var(--text-sm);
  }
  ul {
    margin: 0 0 var(--space-3);
    padding-left: var(--space-5);
    font-size: var(--text-sm);
    line-height: 1.5;
  }
</style>
