<script lang="ts">
  import { ChartColumn } from '@lucide/svelte';
  import { app, percent } from '../app.svelte';
  import EmptyState from '../components/EmptyState.svelte';
  import PageHeader from '../components/PageHeader.svelte';
  import StatTile from '../components/StatTile.svelte';
  import type { StatRow } from '../types';

  const TOP_CHAMPIONS = 12;
  const HIGH_WIN_RATE = 55;

  const t = $derived(app.t);
  const stats = $derived(app.stats);
  const augmentsUsed = $derived(new Set((stats?.recent ?? []).flatMap((g) => g.augments.map((a) => a.id))).size);
</script>

{#snippet statRows(rows: StatRow[], open?: (id: number) => void)}
  <div class="panel cut list">
    {#each rows as row (row.id)}
      {@const winRate = percent(row.wins, row.games)}
      <button class="row" disabled={!open} onclick={() => open?.(row.id)}>
        {#if row.icon}<img src={row.icon} alt="" />{/if}
        <span class="name">{row.name}</span>
        <small class="muted">{row.games}</small>
        <div class="bar"><i class:high={winRate >= HIGH_WIN_RATE} style="width:{winRate}%"></i></div>
        <b class:accent={winRate >= HIGH_WIN_RATE}>{winRate}%</b>
      </button>
    {/each}
  </div>
{/snippet}

<PageHeader title={t('common:nav.stats')} subtitle={t('stats:subtitle')} />

{#if stats && stats.games}
  <div class="tiles">
    <StatTile label={t('stats:games')} value={stats.games} />
    <StatTile label={t('stats:wins')} value={stats.wins} />
    <StatTile label={t('stats:winRate')} value="{percent(stats.wins, stats.games)}%" accent />
    <StatTile label={t('stats:augmentsUsed')} value={augmentsUsed} />
  </div>

  <div class="pair">
    <section>
      <h3 class="section-title">{t('stats:yourChampions')}</h3>
      {@render statRows(stats.champions.slice(0, TOP_CHAMPIONS), app.openAugments)}
    </section>
    <section>
      <h3 class="section-title">{t('stats:bestAugments')} <small>({t('stats:minGames')})</small></h3>
      {@render statRows(stats.augments)}
    </section>
  </div>

  <h3 class="section-title">{t('stats:recentGames')}</h3>
  <div class="recent">
    {#each stats.recent as game (game.game_id)}
      <div class="game panel" class:won={game.win}>
        {#if game.icon}<img src={game.icon} alt="" />{/if}
        <div class="column">
          <b class="result condensed">{game.win ? t('stats:victory') : t('stats:defeat')}</b>
          <small class="muted">{game.champion} · {t(`common:modes.${game.mode}`)} · {game.date}</small>
        </div>
        <div class="augments">
          {#each game.augments as augment (augment.id)}
            {#if augment.icon}<img src={augment.icon} alt={augment.name} title={augment.name} />{/if}
          {/each}
        </div>
      </div>
    {/each}
  </div>
{:else if stats}
  <div class="panel cut"><EmptyState icon={ChartColumn} text={t('stats:empty')} /></div>
{/if}

<style>
  .tiles {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: var(--space-4);
    margin-bottom: var(--space-6);
  }
  .pair {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
    gap: var(--space-6);
    margin-bottom: var(--space-6);
  }
  .list {
    padding: var(--space-2) 0;
  }
  .row {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    width: 100%;
    padding: var(--space-2) var(--space-4);
    border: none;
    background: none;
    text-align: left;
  }
  .row:disabled {
    cursor: default;
  }
  .row:hover:not(:disabled) {
    background: color-mix(in srgb, var(--color-accent) 8%, transparent);
  }
  .row img {
    width: var(--size-thumb);
    height: var(--size-thumb);
  }
  .name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .bar {
    width: 88px;
    height: var(--space-1);
    background: var(--color-line);
  }
  .bar i {
    display: block;
    height: 100%;
    background: var(--quality-fair);
  }
  .bar i.high {
    background: var(--color-accent);
  }
  .row b {
    width: var(--space-10);
    text-align: right;
    font-variant-numeric: tabular-nums;
  }
  .recent {
    display: grid;
    gap: var(--space-2);
  }
  .game {
    display: flex;
    align-items: center;
    gap: var(--space-4);
    padding: var(--space-2) var(--space-4);
    border-left: 3px solid var(--color-textFaint);
  }
  .game.won {
    border-left-color: var(--color-accent);
    background: linear-gradient(90deg, color-mix(in srgb, var(--color-accent) 10%, transparent), transparent 40%), var(--color-panel);
  }
  .game > img {
    width: var(--size-thumbMd);
    height: var(--size-thumbMd);
  }
  .column {
    display: flex;
    flex-direction: column;
    gap: 2px;
    flex: 1;
  }
  .result {
    font-size: var(--text-lg);
    color: var(--color-textMuted);
  }
  .won .result {
    color: var(--color-accentBright);
  }
  .augments {
    display: flex;
    gap: var(--space-1);
  }
  .augments img {
    width: var(--size-thumb);
    height: var(--size-thumb);
  }
</style>
