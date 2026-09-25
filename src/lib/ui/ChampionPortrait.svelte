<script lang="ts">
  import { Lock } from '@lucide/svelte';
  import type { Snippet } from 'svelte';
  import { app } from '../app.svelte';
  import type { ChampionInfo } from '../types';

  let { champion, size, badge }: { champion: ChampionInfo; size: string; badge?: Snippet } = $props();
</script>

<span class="portrait" class:locked={champion.locked} style="--portrait-size:{size}" title={champion.locked ? app.t('common:locked') : champion.name}>
  <img src={champion.icon} alt={champion.name} loading="lazy" />
  <span class="badge">
    {#if champion.locked}<Lock size={14} />{:else if badge}{@render badge()}{/if}
  </span>
</span>

<style>
  .portrait {
    position: relative;
    display: block;
  }
  img {
    width: var(--portrait-size);
    height: var(--portrait-size);
  }
  .locked img {
    filter: grayscale(1);
    opacity: 0.55;
  }
  .badge {
    position: absolute;
    color: var(--color-textMuted);
    right: calc(-1 * var(--space-2));
    bottom: calc(-1 * var(--space-2));
  }
</style>
