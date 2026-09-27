<script lang="ts">
  import { History as HistoryIcon } from '@lucide/svelte';
  import { app, HOME_MODES, inHomeMode, percent, type HomeMode } from '../app.svelte';
  import { getRecentMatches } from '../services/league';
  import { resource } from '../services/resource.svelte';
  import type { MatchSummary } from '../types';
  import EmptyState from '../ui/EmptyState.svelte';
  import PageHeader from '../ui/PageHeader.svelte';
  import SegmentedControl from '../ui/SegmentedControl.svelte';
  import Skeleton from '../ui/Skeleton.svelte';
  import StatTile from '../ui/StatTile.svelte';

  const FILTERS = ['all', ...HOME_MODES] as const;
  type Filter = (typeof FILTERS)[number];
  const RATIO_DIGITS = 2;

  const t = $derived(app.t);
  const format = $derived(app.format);
  let filter = $state<Filter>('all');

  const matches = resource(() => (app.state.phase === 'noClient' ? null : app.state.phase), getRecentMatches);
  const shown = $derived((matches.value ?? []).filter((game) => filter === 'all' || inHomeMode(game, filter)));
  const wins = $derived(shown.filter((game) => game.win).length);
  const ratio = (kills: number, deaths: number, assists: number) => (kills + assists) / Math.max(1, deaths);
  const sum = (pick: (game: MatchSummary) => number) => shown.reduce((total, game) => total + pick(game), 0);
  const averageRatio = $derived(
    ratio(
      sum((game) => game.kills),
      sum((game) => game.deaths),
      sum((game) => game.assists),
    ),
  );
  const modeName = (game: MatchSummary) => {
    const mode = HOME_MODES.find((tab: HomeMode) => inHomeMode(game, tab));
    return mode ? t(`home:modes.${mode}`) : t(`common:modes.${game.mode}`);
  };
</script>

<PageHeader title={t('common:nav.history')} subtitle={t('history:subtitle')} />

<div class="tools">
  <SegmentedControl tabs options={FILTERS.map((id) => [id, id === 'all' ? t('history:all') : t(`home:modes.${id}`)] as [Filter, string])} bind:value={filter} />
</div>

{#if matches.error}
  <EmptyState icon={HistoryIcon} text={app.errorText(matches.error)} />
{:else if !matches.value}
  <Skeleton label={t('history:loading')} rows={6} />
{:else if !shown.length}
  <div class="panel cut"><EmptyState icon={HistoryIcon} text={t('history:empty')} /></div>
{:else}
  <div class="tiles">
    <StatTile label={t('stats:games')} value={shown.length} format={format.number} />
    <StatTile label={t('stats:wins')} value={wins} format={format.number} />
    <StatTile label={t('stats:winRate')} value={percent(wins, shown.length)} format={format.percent} accent />
    <StatTile label={t('history:kda')} value={averageRatio} format={(value) => format.number(value, RATIO_DIGITS)} />
  </div>

  <div class="games">
    {#each shown as game (game.game_id)}
      <div class="game panel" class:won={game.win}>
        {#if game.champion.icon}<img class="champion" src={game.champion.icon} alt="" />{/if}
        <div class="who">
          <b class="result condensed">{game.win ? t('stats:victory') : t('stats:defeat')}</b>
          <small class="muted facts"><span>{game.champion.name}</span><span>{modeName(game)}</span><span>{format.date(game.date)}</span></small>
        </div>
        <div class="numbers">
          <b>{t('home:kda', { kills: game.kills, deaths: game.deaths, assists: game.assists })}</b>
          <small class="muted">{t('history:ratio', { value: format.number(ratio(game.kills, game.deaths, game.assists), RATIO_DIGITS) })}</small>
        </div>
        <div class="numbers">
          <small class="muted">{t('history:farm', { count: game.farm })} · {t('history:gold', { value: format.compact(game.gold) })}</small>
          <small class="muted">{t('history:damage', { value: format.compact(game.damage) })} · {format.clock(game.duration)}</small>
        </div>
        <div class="items">
          {#each game.items as item, i (i)}<img src={item.icon} alt={item.name} title={item.name} />{/each}
        </div>
      </div>
    {/each}
  </div>
  <p class="muted note">{t('history:source')}</p>
{/if}

<style>
  .tools {
    margin-bottom: var(--space-5);
  }
  .tiles {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: var(--space-4);
    margin-bottom: var(--space-6);
  }
  .games {
    display: grid;
    gap: var(--space-2);
  }
  .game {
    display: grid;
    grid-template-columns: var(--size-thumbLg) minmax(0, 1.4fr) minmax(0, 0.8fr) minmax(0, 1.2fr) auto;
    align-items: center;
    gap: var(--space-4);
    padding: var(--space-3) var(--space-4);
    border-left: var(--border-accent) solid var(--color-lineStrong);
  }
  .game.won {
    border-left-color: var(--color-accent);
  }
  .champion {
    width: var(--size-thumbLg);
    height: var(--size-thumbLg);
  }
  .who,
  .numbers {
    display: grid;
    min-width: 0;
  }
  .result {
    font-size: var(--text-lg);
    color: var(--color-textMuted);
  }
  .won .result {
    color: var(--color-accentBright);
  }
  .numbers small {
    font-size: var(--text-sm);
  }
  .items {
    display: flex;
    gap: var(--space-1);
  }
  .items img {
    width: var(--size-thumbSm);
    height: var(--size-thumbSm);
  }
  .note {
    font-size: var(--text-sm);
  }
</style>
