<script lang="ts">
  import type { Snippet } from 'svelte';
  import { championTierColor } from '../design/theme';
  import type { ChampionInfo } from '../types';
  import TierBadge from './TierBadge.svelte';

  let { champion, details = [], showTier = true, children }: { champion: ChampionInfo; details?: string[]; showTier?: boolean; children?: Snippet } = $props();
</script>

<section class="panel cut">
  <img class="cut" src={champion.icon} alt="" />
  <div class="text">
    <h2 class="condensed">{champion.name}</h2>
    <span class="muted facts">
      {#if showTier}
        <span class="tier">
          <TierBadge label={champion.tier ? `T${champion.tier}` : '—'} color={championTierColor(champion.tier)} size="var(--size-thumbSm)" />
          #{champion.rank ?? '—'}
        </span>
      {/if}
      {#each details as detail (detail)}<span>{detail}</span>{/each}
    </span>
  </div>
  {#if children}<div class="actions">{@render children()}</div>{/if}
</section>

<style>
  section {
    display: flex;
    align-items: center;
    gap: var(--space-4);
    margin-bottom: var(--space-6);
    padding: var(--space-4);
  }
  img {
    width: var(--size-portraitLg);
    height: var(--size-portraitLg);
    box-shadow: 0 0 0 var(--border-thick) var(--color-accent);
  }
  .text {
    flex: 1;
    min-width: 0;
  }
  h2 {
    margin: 0 0 var(--space-2);
    font-size: var(--text-2xl);
  }
  .facts,
  .tier {
    display: inline-flex;
    align-items: center;
  }
  .tier {
    gap: var(--space-2);
  }
  .actions {
    display: flex;
    gap: var(--space-3);
  }
</style>
