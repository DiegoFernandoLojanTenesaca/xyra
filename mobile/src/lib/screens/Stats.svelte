<script lang="ts">
  import { ChartColumn } from '@lucide/svelte';
  import { resource } from '$shared/services/resource.svelte';
  import type { StatsSummary } from '$shared/types';
  import EmptyState from '$shared/ui/EmptyState.svelte';
  import Skeleton from '$shared/ui/Skeleton.svelte';
  import { link } from '../link.svelte';
  import { mobile } from '../mobile.svelte';

  const TOP_CHAMPIONS = 10;

  const t = $derived(mobile.t);
  const format = $derived(mobile.format);
  const stats = resource(
    () => (link.status === 'online' ? (link.state?.account ?? true) : null),
    () => link.get<StatsSummary>('/api/stats'),
    (failure) => failure,
  );
  const summary = $derived(stats.value);
  const rate = (wins: number, games: number) => format.percent(games ? (100 * wins) / games : 0);
</script>

{#if stats.error}
  <EmptyState icon={ChartColumn} text={mobile.errorText(stats.error)} />
{:else if !summary}
  <Skeleton label={t('mobile:status.connecting')} rows={6} />
{:else if !summary.games}
  <EmptyState icon={ChartColumn} text={t('stats:empty')} />
{:else}
  <div class="tiles">
    <div class="panel cut"><small class="muted">{t('stats:games')}</small><b>{format.number(summary.games)}</b></div>
    <div class="panel cut"><small class="muted">{t('stats:winRate')}</small><b class="accent">{rate(summary.wins, summary.games)}</b></div>
  </div>

  <section class="block panel cut">
    <h2 class="section-title">{t('stats:yourChampions')}</h2>
    {#each summary.champions.slice(0, TOP_CHAMPIONS) as row (row.id)}
      <button class="row entry" onclick={() => mobile.openBuild(row.id)}>
        {#if row.icon}<img src={row.icon} alt="" />{/if}
        <span class="grow">{row.name}</span>
        <small class="muted">{format.number(row.games)}</small>
        <b>{rate(row.wins, row.games)}</b>
      </button>
    {/each}
  </section>

  <section class="block panel cut">
    <h2 class="section-title">{t('stats:recentGames')}</h2>
    {#each summary.recent as game (game.game_id)}
      <div class="row">
        {#if game.champion.icon}<img src={game.champion.icon} alt="" />{/if}
        <span class="grow">
          {game.champion.name}
          <span class="chips small">
            {#each game.augments as augment (augment.id)}{#if augment.icon}<img src={augment.icon} alt={augment.name} title={augment.name} />{/if}{/each}
          </span>
        </span>
        <b class:accent={game.win}>{game.win ? t('stats:victory') : t('stats:defeat')}</b>
      </div>
    {/each}
  </section>
{/if}

<style>
  .tiles {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--space-3);
    margin-bottom: var(--space-4);
  }
  .tiles div {
    display: grid;
    padding: var(--space-4);
  }
  .tiles b {
    font-size: var(--text-2xl);
  }
  .entry {
    width: 100%;
    border: none;
    background: none;
    text-align: left;
  }
  .small {
    margin-top: var(--space-1);
  }
  .small img {
    width: var(--size-thumbSm);
    height: var(--size-thumbSm);
  }
</style>
