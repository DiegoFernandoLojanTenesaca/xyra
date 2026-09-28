<script lang="ts">
  import type { TFunction } from 'i18next';
  import type { LiveStats } from '../types';

  /** The player's KDA, farm, gold and level in the running game; shared by the PC and the phone. */
  let { live, t, number }: { live: LiveStats; t: TFunction; number: (value: number) => string } = $props();
</script>

<div class="panel cut strip">
  <span><small>{t('home:live.kda')}</small><b class="condensed">{t('home:kda', { kills: live.kills, deaths: live.deaths, assists: live.assists })}</b></span>
  <span><small>{t('home:live.farm')}</small><b class="condensed">{number(live.farm)}</b></span>
  <span><small>{t('home:live.gold')}</small><b class="condensed accent">{number(live.gold)}</b></span>
  <span><small>{t('home:live.level')}</small><b class="condensed">{live.level}</b></span>
</div>

<style>
  .strip {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: var(--space-3);
    padding: var(--space-4);
  }
  span {
    display: grid;
    gap: var(--space-1);
  }
  small {
    color: var(--color-textMuted);
    font-size: var(--text-xs);
    font-weight: 700;
    letter-spacing: var(--tracking-wide);
    text-transform: uppercase;
  }
  b {
    font-size: var(--text-xl);
  }
</style>
