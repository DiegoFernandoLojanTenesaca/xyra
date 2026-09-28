<script lang="ts">
  import { ChartColumn } from '@lucide/svelte';
  import { resource } from '$shared/services/resource.svelte';
  import type { Challenges, MasteryProgress, MatchSummary, StatsSummary } from '$shared/types';
  import EmptyState from '$shared/ui/EmptyState.svelte';
  import SegmentedControl from '$shared/ui/SegmentedControl.svelte';
  import Skeleton from '$shared/ui/Skeleton.svelte';
  import { link } from '../link.svelte';
  import { mobile } from '../mobile.svelte';

  const TOP_CHAMPIONS = 10;
  const VIEWS = ['augments', 'history', 'mastery', 'challenges'] as const;
  type View = (typeof VIEWS)[number];
  const CLOSEST_MASTERIES = 10;
  const CLOSEST_CHALLENGES = 8;
  const NO_LEVEL = 'NONE';

  const t = $derived(mobile.t);
  const format = $derived(mobile.format);
  const stats = resource(
    () => (link.status === 'online' ? (link.state?.account ?? true) : null),
    () => link.get<StatsSummary>('/api/stats'),
    (failure) => failure,
  );
  const summary = $derived(stats.value);
  const rate = (wins: number, games: number) => format.percent(games ? (100 * wins) / games : 0);
  let view = $state<View>('augments');
  const online = (wanted: View) => (link.status === 'online' && view === wanted ? (link.state?.account ?? true) : null);
  const matches = resource(
    () => online('history'),
    () => link.call<MatchSummary[]>('GET', '/api/matches'),
    (failure) => failure,
  );
  const masteries = resource(
    () => online('mastery'),
    () => link.call<MasteryProgress[]>('GET', '/api/masteries'),
    (failure) => failure,
  );
  const challenges = resource(
    () => online('challenges'),
    () => link.call<Challenges>('GET', '/api/challenges'),
    (failure) => failure,
  );
  const closestMasteries = $derived(
    [...(masteries.value ?? [])]
      .filter((m) => m.until_next > 0)
      .sort((a, b) => a.until_next - b.until_next)
      .slice(0, CLOSEST_MASTERIES),
  );
  const levelName = (level: string) => (level === NO_LEVEL || !level ? t('challenges:noLevel') : t(`common:leagues.${level.toLowerCase()}`));
</script>

{#snippet bar(done: number)}
  <span class="bar"><i style="width:{done}%"></i></span>
{/snippet}

<div class="views">
  <SegmentedControl options={VIEWS.map((id) => [id, t(`mobile:stats.${id}`)] as [View, string])} bind:value={view} />
</div>

{#if view === 'history'}
  {#if matches.error}
    <EmptyState icon={ChartColumn} text={mobile.errorText(matches.error)} />
  {:else if !matches.value}
    <Skeleton label={t('history:loading')} rows={6} />
  {:else}
    <section class="block panel cut">
      <h2 class="section-title">{t('common:nav.history')}</h2>
      {#each matches.value as game (game.game_id)}
        <div class="row">
          {#if game.champion.icon}<img src={game.champion.icon} alt="" />{/if}
          <span class="grow stack">
            {game.champion.name}
            <small class="muted">{t('home:kda', { kills: game.kills, deaths: game.deaths, assists: game.assists })} · {t(`common:modes.${game.mode}`)}</small>
          </span>
          <b class:accent={game.win}>{game.win ? t('stats:victory') : t('stats:defeat')}</b>
        </div>
      {:else}
        <p class="muted">{t('history:empty')}</p>
      {/each}
    </section>
  {/if}
{:else if view === 'mastery'}
  {#if masteries.error}
    <EmptyState icon={ChartColumn} text={mobile.errorText(masteries.error)} />
  {:else if !masteries.value}
    <Skeleton label={t('mastery:loading')} rows={6} />
  {:else}
    <section class="block panel cut">
      <h2 class="section-title">{t('mastery:sort.next')}</h2>
      {#each closestMasteries as mastery (mastery.champion.id)}
        <button class="row entry" onclick={() => mobile.openBuild(mastery.champion.id)}>
          {#if mastery.champion.icon}<img src={mastery.champion.icon} alt="" />{/if}
          <span class="grow stack">
            {mastery.champion.name}
            {@render bar((100 * mastery.since_level) / Math.max(1, mastery.since_level + mastery.until_next))}
            <small class="muted">{t('mastery:untilNext', { points: format.number(mastery.until_next) })}</small>
          </span>
          <b class="accent">{t('mastery:level', { level: mastery.level })}</b>
        </button>
      {/each}
    </section>
  {/if}
{:else if view === 'challenges'}
  {#if challenges.error}
    <EmptyState icon={ChartColumn} text={mobile.errorText(challenges.error)} />
  {:else if !challenges.value}
    <Skeleton label={t('challenges:loading')} rows={6} />
  {:else}
    {@const summary = challenges.value}
    <div class="tiles">
      <div class="panel cut"><small class="muted">{t('challenges:points')}</small><b class="accent">{format.number(summary.points)}</b></div>
      <div class="panel cut"><small class="muted">{t('challenges:top')}</small><b>{format.percent(summary.top_percent, 1)}</b></div>
    </div>
    <section class="block panel cut">
      <h2 class="section-title">{t('challenges:overall', { level: levelName(summary.level) })}</h2>
      {#each summary.closest.slice(0, CLOSEST_CHALLENGES) as challenge (challenge.id)}
        <div class="row">
          {#if challenge.icon}<img src={challenge.icon} alt="" />{/if}
          <span class="grow stack">
            {challenge.name}
            {@render bar(challenge.progress)}
            <small class="muted">{format.number(challenge.value)} / {format.number(challenge.next_value)}</small>
          </span>
          <small class="accent">{levelName(challenge.next_level)}</small>
        </div>
      {/each}
    </section>
  {/if}
{:else if stats.error}
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

  {@const follow = summary.following}
  {#if follow.followed + follow.ignored}
    <section class="block panel cut">
      <h2 class="section-title">{t('stats:following.title')}</h2>
      <div class="row">
        <span class="grow">{t('stats:following.taken')}</span><b class="accent">{rate(follow.followed, follow.followed + follow.ignored)}</b>
      </div>
      <div class="row"><span class="grow">{t('stats:following.withIt')}</span><b>{follow.followed ? rate(follow.followed_wins, follow.followed) : '—'}</b></div>
      <div class="row"><span class="grow">{t('stats:following.without')}</span><b>{follow.ignored ? rate(follow.ignored_wins, follow.ignored) : '—'}</b></div>
      <small class="muted">{t('stats:following.summary', { followed: follow.followed, total: follow.followed + follow.ignored })}</small>
    </section>
  {/if}

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
  .views {
    margin-bottom: var(--space-4);
  }
  .stack {
    display: grid;
    gap: var(--space-1);
  }
  .stack small {
    font-size: var(--text-xs);
  }
  .bar {
    display: block;
    height: var(--space-1);
    background: var(--color-line);
  }
  .bar i {
    display: block;
    height: 100%;
    background: var(--color-accent);
  }
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
