<script lang="ts">
  import {
    ArrowDown,
    ArrowLeftRight,
    ArrowUp,
    ArrowUpDown,
    Check,
    Download,
    Hammer,
    Layers,
    Pause,
    Play,
    Radio,
    RefreshCw,
    Star,
    Swords,
    TrendingUp,
    TriangleAlert,
    X,
  } from '@lucide/svelte';
  import { untrack, type Component } from 'svelte';
  import { app, HOME_GAME_MODES, HOME_MODES, inHomeMode, percent, type HomeMode } from '../app.svelte';
  import { championTierColor, championTierLabel, qualityColor } from '../design/theme';
  import { around } from '../i18n';
  import { setBorderless, testOverlay } from '../services/engine';
  import { getAugments, getMeta, getModeChampions, getPatchChanges, getRecentMatches, takeBenchPick } from '../services/league';
  import { resource } from '../services/resource.svelte';
  import type { ChampionInfo, ChampionOrder, ChangeVerdict } from '../types';
  import AugmentChip from '../ui/AugmentChip.svelte';
  import Button from '../ui/Button.svelte';
  import ChampionPortrait from '../ui/ChampionPortrait.svelte';
  import EmptyState from '../ui/EmptyState.svelte';
  import Logo from '../ui/Logo.svelte';
  import SegmentedControl from '../ui/SegmentedControl.svelte';
  import LiveStrip from '../ui/LiveStrip.svelte';
  import Skeleton from '../ui/Skeleton.svelte';
  import StatTile from '../ui/StatTile.svelte';
  import TierBadge from '../ui/TierBadge.svelte';

  const TOP_CHAMPIONS = 5;
  const RECENT_GAMES = 5;
  const BEST_AUGMENTS = 4;
  const BEST_FOR_CHAMPION = 6;
  const MIN_BAR = 4;
  const WIN_RATE_DIGITS = 1;
  const POSITION_TOP = 3;
  const MODE_TOP = 8;
  const YOUR_CHAMPIONS = 5;
  const PATCH_CHANGES_SHOWN = 6;
  const RIFT_DEFAULT_POSITION = 'mid';
  const VERDICT_ICONS: Record<ChangeVerdict, Component<{ size?: number }>> = { buff: ArrowUp, nerf: ArrowDown, adjusted: ArrowUpDown };

  const t = $derived(app.t);
  const format = $derived(app.format);
  const engine = $derived(app.state);
  const config = $derived(app.config);
  const stats = $derived(app.stats);

  let notice = $state('');
  const order = $derived(config.champion_order);
  const cards = $derived([...engine.cards].sort((a, b) => a.x - b.x));
  const best = $derived(cards.find((c) => c.best) ?? null);
  const select = $derived(engine.champ_select);
  const mine = $derived(select?.champion ?? null);
  const benchPick = $derived(select?.bench_pick ?? null);
  const counter = $derived(select?.counter_picks[0] ?? null);
  const opponent = $derived(select?.lane_opponent ?? null);
  const topChampions = $derived(app.champions.filter((c) => c.recommendable).slice(0, TOP_CHAMPIONS));
  const liveChampion = $derived(engine.phase === 'inGame' ? (engine.game?.champion ?? null) : null);
  const liveMode = $derived(engine.game && app.choices.augment_modes.includes(engine.game.mode) ? engine.game.mode : null);
  const championAugments = resource(
    () => (liveChampion && liveMode ? { champion: liveChampion.id, mode: liveMode } : null),
    ({ champion, mode }) => getAugments(champion, mode),
  );
  const bestForChampion = $derived((championAugments.value ?? []).slice(0, BEST_FOR_CHAMPION));
  const lastGame = $derived(engine.phase === 'inGame' ? null : (stats?.recent.find((game) => game.augments.length) ?? null));
  const followed = $derived(lastGame?.followed.filter((choice) => choice === true).length ?? 0);
  const seen = $derived(lastGame?.followed.filter((choice) => choice !== null).length ?? 0);

  const hero = $derived.by(() => {
    const game = engine.game;
    if (engine.phase === 'inGame' && game) {
      const base = { icon: game.champion?.icon, live: true, eyebrow: t('home:live'), mode: t(`common:modes.${game.mode}`) };
      const champion = game.champion?.name ?? '';
      return best
        ? { ...base, title: t('home:pickCard', { card: best.name }), strong: best.name, text: t('home:bestOf', { count: cards.length, champion }) }
        : { ...base, title: champion || t('common:phases.inGame'), strong: '', text: t('home:inGame') };
    }
    if (engine.phase === 'champSelect' && select) {
      const base = { live: true, eyebrow: t('common:phases.champSelect'), mode: t(`common:modes.${select.mode}`) };
      if (counter && opponent) {
        const position = select.position ? t(`build:positions.${select.position}`) : '';
        const text = t('home:counterText', { value: format.percent(counter.win_rate, WIN_RATE_DIGITS), enemy: opponent.name, position });
        return {
          ...base,
          icon: counter.champion.icon,
          title: t('home:counterPick', { enemy: opponent.name, champion: counter.champion.name }),
          strong: counter.champion.name,
          text,
        };
      }
      if (benchPick) {
        const reason = {
          champion: mine?.name ?? '',
          rank: benchPick.rank,
          tier: benchPick.tier,
          points: format.compact(benchPick.mastery),
          count: benchPick.played,
        };
        const text = t(`home:benchReason.${order}`, reason);
        return { ...base, icon: benchPick.icon, title: t('home:takeFromBench', { champion: benchPick.name }), strong: benchPick.name, text };
      }
      const text = mine?.rank && select.mode !== 'summonersRift' ? t('home:yourChampion', { rank: mine.rank }) : t('home:riftChampion');
      return { ...base, icon: mine?.icon, title: mine?.name ?? t('common:phases.champSelect'), strong: mine?.name ?? '', text };
    }
    const phase = engine.phase === 'inGame' || engine.phase === 'champSelect' ? 'client' : engine.phase;
    return {
      icon: undefined,
      live: false,
      eyebrow: t(`common:phases.${phase}`),
      mode: '',
      title: t(`home:${phase}.title`),
      strong: '',
      text: t(`home:${phase}.text`),
    };
  });
  const [heroBefore, heroAfter] = $derived(hero.strong ? around(hero.title, hero.strong) : [hero.title, '']);

  const mode = $derived(app.homeMode);
  const gameMode = $derived(HOME_GAME_MODES[mode]);
  const rift = $derived(gameMode === 'summonersRift');
  const liveGameMode = $derived(engine.game?.mode ?? select?.mode ?? null);
  const showCards = $derived(liveMode !== null || (engine.phase !== 'inGame' && (mode === 'mayhem' || mode === 'arena')));
  const meta = resource(() => (rift || select?.mode === 'summonersRift' ? true : null), getMeta);
  /** Where each champion stands with the player's criterion: app.champions comes sorted by it. */
  const preference = $derived(new Map(app.champions.map((champion, i) => [champion.id, i])));
  const byPreference = (a: ChampionInfo, b: ChampionInfo) => (preference.get(a.id) ?? Infinity) - (preference.get(b.id) ?? Infinity);
  /** In ARAM, the champion the player has and those on the bench, best first. */
  const options = $derived(
    select && select.mode !== 'summonersRift' ? [select.champion, ...select.bench].filter((c): c is ChampionInfo => !!c).sort(byPreference) : [],
  );
  /** Every other champion the player can play, with the same criterion; in the Rift, those of the position. */
  const allChampions = $derived.by(() => {
    if (!select) return [];
    if (select.mode !== 'summonersRift') {
      const taken = new Set(options.map((c) => c.id));
      return app.champions.filter((c) => c.recommendable && !taken.has(c.id));
    }
    const info = new Map(app.champions.map((c) => [c.id, c]));
    const slot = meta.value?.positions.find((p) => p.position === (select.position ?? RIFT_DEFAULT_POSITION));
    const played = (slot?.champions ?? []).map((entry) => info.get(entry.champion.id)).filter((c): c is ChampionInfo => !!c && !c.locked);
    return order === 'tier' ? played : played.sort(byPreference);
  });
  const modeChampions = resource(() => (mode === 'aram' || mode === 'arena' ? mode : null), getModeChampions);
  const patch = resource(() => app.language, getPatchChanges);
  const matches = resource(() => (engine.phase === 'noClient' ? null : engine.phase), getRecentMatches);
  const yourChanges = $derived((patch.value?.champions ?? []).filter((change) => change.yours).slice(0, PATCH_CHANGES_SHOWN));
  const modeHistory = $derived((matches.value ?? []).filter((game) => inHomeMode(game, mode)));
  const modeMatches = $derived(modeHistory.slice(0, RECENT_GAMES));
  const modeWins = $derived(modeHistory.filter((game) => game.win).length);

  $effect(() => {
    if (liveGameMode) untrack(() => app.followHomeMode(liveGameMode));
  });

  async function importWholeBuild(champion: number) {
    notice = await app.importWholeBuild(champion);
  }

  /** Swaps the player's champion for one of the bench. */
  async function takeFromBench(champion: ChampionInfo) {
    notice = await takeBenchPick(champion.id).then(() => t('home:benchTaken', { champion: champion.name }), app.errorText);
  }

  async function switchToBorderless() {
    notice = await setBorderless().then(() => t('home:borderlessDone'), app.errorText);
  }
</script>

<section class="hero highlight cut" class:live={hero.live}>
  {#if hero.icon}<img class="cut" src={hero.icon} alt="" />{:else}<Logo size={76} phase={engine.phase} />{/if}
  <div class="text">
    <span class="eyebrow">
      {#if hero.live}<Radio size={12} />{/if}
      <span class="facts"
        ><span>{hero.eyebrow}</span>{#if hero.mode}<span>{hero.mode}</span>{/if}</span
      >
    </span>
    <h2 class="condensed">
      {heroBefore}{#if hero.strong}<em>{hero.strong}</em>{/if}{heroAfter}
    </h2>
    <p class="muted">{hero.text}</p>
    {#if notice && engine.phase === 'champSelect'}<p class="notice">{notice}</p>{/if}
  </div>
  <div class="actions">
    {#if engine.phase === 'champSelect' && mine}
      {#if benchPick}
        <Button variant="primary" icon={ArrowLeftRight} onclick={() => takeFromBench(benchPick)}>{t('home:takeBench')}</Button>
      {/if}
      <Button icon={Hammer} onclick={() => app.openBuild(mine.id)}>{t('home:openBuild')}</Button>
      <Button variant={benchPick ? 'default' : 'primary'} icon={Download} onclick={() => importWholeBuild(mine.id)}>{t('build:importAll')}</Button>
    {:else}
      <Button icon={Play} disabled={engine.phase === 'inGame'} onclick={testOverlay}>{t('home:test')}</Button>
      <Button variant="primary" icon={config.paused ? Play : Pause} onclick={() => app.saveConfig({ paused: !config.paused })}>
        {config.paused ? t('home:resume') : t('home:pause')}
      </Button>
    {/if}
  </div>
</section>

<div class="modes">
  <SegmentedControl tabs options={HOME_MODES.map((id) => [id, t(`home:modes.${id}`)] as [HomeMode, string])} bind:value={() => mode, app.setHomeMode} />
</div>

{#snippet choiceDetail(champion: ChampionInfo)}
  {#if order === 'played'}<small class="muted">{t('home:gamesPlayed', { count: champion.played })}</small>
  {:else if order !== 'tier'}<small class="muted">{t('home:masteryPoints', { points: format.compact(champion.mastery) })}</small>{/if}
{/snippet}

<div class="grid">
  <div class="column">
    {#if select}
      {#if options.length}
        <h3 class="section-title">{t('home:yourOptions')} <small>({t(`home:order.${order}`)})</small></h3>
        <div class="options">
          {#each options as champion, i (champion.id)}
            {@const current = champion.id === mine?.id}
            <div class="panel cut option appear" class:best={i === 0} style="--i:{i}">
              <ChampionPortrait {champion} size="var(--size-portrait)">
                {#snippet badge()}
                  <TierBadge label={championTierLabel(champion.tier)} color={championTierColor(champion.tier)} size="var(--size-thumbSm)" />
                {/snippet}
              </ChampionPortrait>
              <b>{champion.name}</b>
              {@render choiceDetail(champion)}
              {#if current}
                <small class="current">{t('home:current')}</small>
              {:else}
                <Button variant={i === 0 ? 'primary' : 'default'} icon={ArrowLeftRight} onclick={() => takeFromBench(champion)}>{t('home:take')}</Button>
              {/if}
            </div>
          {/each}
        </div>
      {/if}
      <h3 class="section-title">
        {select.mode === 'summonersRift'
          ? t('home:allForPosition', { position: t(`build:positions.${select.position ?? RIFT_DEFAULT_POSITION}`) })
          : t('home:allChampions')}
        <small>({allChampions.length})</small>
      </h3>
      <div class="panel cut list all">
        {#each allChampions as champion, i (champion.id)}
          <button
            class="row champion"
            onclick={() =>
              select.mode === 'summonersRift' ? app.openRiftBuild(champion.id, select.position ?? RIFT_DEFAULT_POSITION) : app.openBuild(champion.id)}
          >
            <b class="rank">#{i + 1}</b>
            <img src={champion.icon} alt="" />
            <span>{champion.name}{@render choiceDetail(champion)}</span>
            <TierBadge label={championTierLabel(champion.tier)} color={championTierColor(champion.tier)} size="var(--size-thumbSm)" />
          </button>
        {:else}
          <p class="muted box">{t('meta:loading')}</p>
        {/each}
      </div>
    {/if}

    {#if engine.live}
      <h3 class="section-title">{t('home:live.title')}</h3>
      <LiveStrip live={engine.live} {t} number={format.number} />
    {/if}

    {#if engine.tips}
      {@const next = engine.tips.next_item}
      <h3 class="section-title">{t('home:tips')}</h3>
      <div class="tips">
        <div class="panel cut tip">
          <span class="key" class:idle={!engine.tips.skill}>{engine.tips.skill ?? '—'}</span>
          <span class="tip-text">
            <b>{engine.tips.skill ? t('home:levelSkill', { skill: engine.tips.skill }) : t('home:noSkillPoint')}</b>
            <small class="muted">{t('home:skillHint')}</small>
          </span>
        </div>
        {#if next}
          <div class="panel cut tip">
            {#if next.item.icon}<img src={next.item.icon} alt="" />{/if}
            <span class="tip-text">
              <b>{next.item.name}</b>
              <small class:ready={next.missing === 0} class="muted">
                {next.missing === null
                  ? t('home:nextItem')
                  : next.missing === 0
                    ? t('home:canBuy')
                    : t('home:goldMissing', { gold: format.number(next.missing) })}
              </small>
            </span>
          </div>
        {/if}
      </div>
    {/if}

    {#if showCards}
      <h3 class="section-title">{engine.phase === 'inGame' ? t('home:cardsOnScreen') : t('home:latestCards')}</h3>
      {#if cards.length}
        <div class="cards">
          {#each cards as card, i (card.id)}
            <div class="card panel cut appear" class:best={card.best} style="--i:{i}">
              {#if card.best}<div class="crown"><Star size={12} />{t('home:bestPick')}</div>{/if}
              {#if card.icon}<img src={card.icon} alt="" />{/if}
              <h4>{card.name}</h4>
              <span class="quality" style="color:{qualityColor(card.quality)}">
                <TierBadge label={card.grade} color={qualityColor(card.quality)} />{t(`common:quality.${card.quality}`)}
              </span>
              <div class="bar"><i style="width:{card.tier === null ? 0 : Math.max(MIN_BAR, Math.min(100, card.performance))}%"></i></div>
              {#if card.reroll}<small class="reroll"><RefreshCw size={12} />{t('common:reroll')}</small>{/if}
            </div>
          {/each}
        </div>
      {:else}
        <div class="panel cut"><EmptyState icon={Layers} text={t('home:noCards')} /></div>
      {/if}
    {/if}

    {#if bestForChampion.length && liveChampion}
      <h3 class="section-title">{t('home:bestForChampion', { champion: liveChampion.name })}</h3>
      <div class="chips">
        {#each bestForChampion as augment (augment.id)}
          <AugmentChip icon={augment.icon} name={augment.name}>
            <TierBadge label={augment.grade} color={qualityColor(augment.quality)} />
          </AugmentChip>
        {/each}
      </div>
    {/if}

    {#if engine.rounds.length}
      <h3 class="section-title">{t('home:rounds')}</h3>
      <div class="panel cut rounds">
        {#each engine.rounds as round, i (i)}
          <div class="round">
            <b class="muted">{t('home:round', { number: i + 1 })}</b>
            {#each round as card (card.id)}
              <AugmentChip icon={card.icon} name={card.name} highlighted={card.best}>
                {#if card.best}<Star size={14} />{/if}
                <TierBadge label={card.grade} color={qualityColor(card.quality)} />
              </AugmentChip>
            {/each}
          </div>
        {/each}
      </div>
    {/if}

    {#if mode === 'ranked'}
      <div class="panel cut box rank-card">
        {#if app.profile?.rank}
          {@const rank = app.profile.rank}
          <img src={rank.crest} alt="" />
          <span
            ><small class="muted">{t('home:yourRank')}</small><b
              >{t('settings:rankValue', { tier: t(`common:leagues.${rank.tier}`), division: rank.division, lp: format.number(rank.lp) })}</b
            ></span
          >
        {:else}
          <span><small class="muted">{t('home:yourRank')}</small><b>{t('settings:unranked')}</b></span>
        {/if}
      </div>
    {/if}

    {#if rift}
      <h3 class="section-title">{t('home:bestByPosition')}</h3>
      {#if meta.error}
        <p class="muted">{app.errorText(meta.error)}</p>
      {:else if !meta.value}
        <Skeleton label={t('meta:loading')} rows={3} />
      {:else}
        <div class="panel cut positions">
          {#each meta.value.positions as slot (slot.position)}
            <div class="position">
              <h4>{t(`build:positions.${slot.position}`)}</h4>
              {#each slot.champions.slice(0, POSITION_TOP) as entry (entry.champion.id)}
                <button class="pick" onclick={() => app.openRiftBuild(entry.champion.id, slot.position)} title={t('meta:openBuild')}>
                  {#if entry.champion.icon}<img src={entry.champion.icon} alt="" />{/if}
                  <span class="who"><b>{entry.champion.name}</b><small class="muted">{format.percent(entry.win_rate, WIN_RATE_DIGITS)}</small></span>
                </button>
              {/each}
            </div>
          {/each}
        </div>
        <div><Button icon={TrendingUp} onclick={() => app.openMeta('rift')}>{t('home:openMeta')}</Button></div>
      {/if}
    {/if}

    {#if mode === 'mayhem' && lastGame}
      <h3 class="section-title">
        {t('home:lastGame', { champion: lastGame.champion.name })}
        {#if seen}<small>({t('home:followedSummary', { followed, seen })})</small>{/if}
      </h3>
      <div class="chips">
        {#each lastGame.augments as augment, i (augment.id)}
          {@const choice = lastGame.followed[i]}
          <AugmentChip icon={augment.icon} name={augment.name} highlighted={choice === true}>
            {#if choice === true}<span title={t('home:followedYes')}><Check size={14} /></span>
            {:else if choice === false}<span class="muted" title={t('home:followedNo')}><X size={14} /></span>{/if}
          </AugmentChip>
        {/each}
      </div>
    {/if}

    <div class="pair">
      {#if rift && app.profile?.masteries.length}
        <section>
          <h3 class="section-title">{t('home:yourChampions')} <small>({t('home:byMastery')})</small></h3>
          <div class="panel cut list">
            {#each app.profile.masteries.slice(0, YOUR_CHAMPIONS) as champion, i (champion.id)}
              <button class="row appear champion" style="--i:{i}" onclick={() => app.openModeBuild(champion.id, 'rift')}>
                <img src={champion.icon} alt="" />
                <span>{champion.name}<small class="muted">{t('home:masteryPoints', { points: format.compact(champion.points) })}</small></span>
              </button>
            {/each}
          </div>
        </section>
      {:else if mode === 'aram' || mode === 'arena'}
        <section>
          <h3 class="section-title">{t(`home:modeTop.${mode}`)}</h3>
          <small class="muted order-hint">{t(`home:modeTopHint.${mode}`)}</small>
          {#if modeChampions.error}
            <p class="muted">{app.errorText(modeChampions.error)}</p>
          {:else if !modeChampions.value}
            <Skeleton label={t('meta:loading')} rows={4} />
          {:else}
            <div class="panel cut list">
              {#each modeChampions.value.slice(0, MODE_TOP) as entry, i (entry.champion.id)}
                <button
                  class="row appear champion"
                  style="--i:{i}"
                  onclick={() => (mode === 'arena' ? app.openModeAugments(entry.champion.id, 'arena') : app.openModeBuild(entry.champion.id, 'aram'))}
                >
                  <b class="rank">#{entry.rank}</b>
                  {#if entry.champion.icon}<img src={entry.champion.icon} alt="" />{/if}
                  <span>{entry.champion.name}<small class="muted">{format.percent(entry.win_rate, WIN_RATE_DIGITS)}</small></span>
                  <TierBadge label={championTierLabel(entry.tier)} color={championTierColor(entry.tier)} size="var(--size-thumbSm)" />
                </button>
              {/each}
            </div>
            <div class="more"><Button icon={TrendingUp} onclick={() => app.openMeta(mode === 'arena' ? 'arena' : 'aram')}>{t('home:openMeta')}</Button></div>
          {/if}
        </section>
      {:else if mode === 'mayhem' && topChampions.length}
        <section>
          <h3 class="section-title">{t('home:topMayhem')} <small>({t('home:ownedOnly')})</small></h3>
          <SegmentedControl
            options={app.choices.champion_orders.map((id) => [id, t(`home:order.${id}`)] as [ChampionOrder, string])}
            bind:value={() => order, (next) => app.saveConfig({ champion_order: next }).then(app.reloadData)}
          />
          <small class="muted order-hint">{t(`home:orderHint.${order}`)}</small>
          <div class="panel cut list">
            {#each topChampions as champion, i (champion.id)}
              <button class="row appear champion" style="--i:{i}" onclick={() => app.openBuild(champion.id)}>
                <b class="rank">#{champion.rank}</b>
                <img src={champion.icon} alt="" />
                <span
                  >{champion.name}
                  {#if order === 'played'}<small class="muted">{t('home:gamesPlayed', { count: champion.played })}</small>
                  {:else if order !== 'tier'}<small class="muted">{t('home:masteryPoints', { points: format.compact(champion.mastery) })}</small>{/if}</span
                >
                <TierBadge label={championTierLabel(champion.tier)} color={championTierColor(champion.tier)} size="var(--size-thumbSm)" />
              </button>
            {/each}
          </div>
        </section>
      {/if}
      <section>
        <h3 class="section-title">{t('home:latestGames')}</h3>
        {#if modeMatches.length}
          <div class="panel cut list">
            {#each modeMatches as game, i (game.game_id)}
              <div class="row appear game" class:won={game.win} style="--i:{i}">
                {#if game.champion.icon}<img src={game.champion.icon} alt="" />{/if}
                <span
                  >{game.champion.name}<small class="muted"
                    >{t('home:kda', { kills: game.kills, deaths: game.deaths, assists: game.assists })} · {format.date(game.date)}</small
                  ></span
                >
                <b>{game.win ? t('stats:victory') : t('stats:defeat')}</b>
              </div>
            {/each}
          </div>
        {:else}
          <p class="muted panel cut box">{t('home:noModeGames', { mode: t(`home:modes.${mode}`) })}</p>
        {/if}
      </section>
    </div>

    {#if (mode === 'mayhem' || mode === 'arena') && stats?.augments.length}
      <h3 class="section-title">{t('home:bestAugments')} <small>({t('home:minGames', { count: stats.min_augment_games })})</small></h3>
      <div class="panel cut list">
        {#each stats.augments.slice(0, BEST_AUGMENTS) as augment, i (augment.id)}
          <div class="row appear" style="--i:{i}">
            {#if augment.icon}<img src={augment.icon} alt="" />{/if}
            <span>{augment.name}</span>
            <small class="muted">{format.number(augment.games)}</small>
            <b class="accent">{format.percent(percent(augment.wins, augment.games))}</b>
          </div>
        {/each}
      </div>
    {/if}
  </div>

  <aside class="column">
    <div class="tiles">
      <StatTile label={t('home:recentGames', { mode: t(`home:modes.${mode}`) })} value={modeHistory.length} format={format.number} />
      <StatTile label={t('stats:winRate')} value={modeHistory.length ? percent(modeWins, modeHistory.length) : null} format={format.percent} accent />
    </div>

    {#if select && opponent && select.counter_picks.length}
      <div class="panel cut box">
        <h3 class="section-title"><Swords size={14} />{t('home:counterPicks', { enemy: opponent.name })}</h3>
        <div class="list">
          {#each select.counter_picks as pick (pick.champion.id)}
            <button class="row" onclick={() => app.openBuild(pick.champion.id)}>
              <img src={pick.champion.icon} alt="" />
              <span>{pick.champion.name}</span>
              <b class="accent">{format.percent(pick.win_rate, WIN_RATE_DIGITS)}</b>
            </button>
          {/each}
        </div>
      </div>
    {/if}

    {#if yourChanges.length}
      <div class="panel cut box">
        <h3 class="section-title">{t('home:patchYours', { patch: patch.value?.patch })}</h3>
        <div class="list">
          {#each yourChanges as change (change.champion.id)}
            {@const Icon = VERDICT_ICONS[change.verdict]}
            <button class="row verdict {change.verdict}" onclick={() => app.openMeta('rift')}>
              {#if change.champion.icon}<img src={change.champion.icon} alt="" />{/if}
              <span>{change.champion.name}</span>
              <b><Icon size={14} />{t(`meta:changes.verdict.${change.verdict}`)}</b>
            </button>
          {/each}
        </div>
      </div>
    {/if}

    <div class="panel cut box">
      <h3 class="section-title">{t('home:gameWindow')}</h3>
      {#if engine.borderless === true}
        <p class="ok"><Check size={14} />{t('home:borderlessReady')}</p>
      {:else if engine.borderless === false}
        <p class="warning"><TriangleAlert size={14} />{t('home:fullscreenWarning')}</p>
        <Button variant="primary" onclick={switchToBorderless}>{t('home:setBorderless')}</Button>
      {:else}
        <p class="muted">{t('home:settingsUnreadable')}</p>
      {/if}
      {#if notice && engine.phase !== 'champSelect'}<p class="muted small">{notice}</p>{/if}
    </div>

    <div class="panel cut box">
      <h3 class="section-title">{t('home:system')}</h3>
      <div class="fact"><span class="muted">{t('home:screenReading')}</span><b>{engine.ocr_language ?? '—'}</b></div>
      <div class="fact"><span class="muted">{t('common:nav.labels')}</span><b>{t(`labels:styles.${config.label_style}.name`)}</b></div>
      <div class="fact"><span class="muted">{t('home:voiceHint')}</span><b>{config.voice ? t('common:on') : t('common:off')}</b></div>
      {#if !engine.ocr_language}<p class="warning"><TriangleAlert size={14} />{t('home:noOcr')}</p>{/if}
    </div>
  </aside>
</div>

<style>
  .hero {
    display: flex;
    align-items: center;
    gap: var(--space-6);
    padding: var(--space-6);
    margin-bottom: var(--space-6);
  }
  .hero:not(.live) {
    background: linear-gradient(110deg, color-mix(in srgb, var(--color-accent) 6%, transparent), transparent 55%), var(--color-panel);
    border-color: var(--color-line);
  }
  .hero > img {
    width: var(--size-hero);
    height: var(--size-hero);
    flex: none;
    object-fit: cover;
    box-shadow: 0 0 0 var(--border-thick) var(--color-accent);
  }
  .text {
    flex: 1;
    min-width: 0;
  }
  .eyebrow {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
  }
  .hero h2 {
    margin: var(--space-1) 0 0;
    font-size: var(--text-2xl);
    font-variation-settings: 'wdth' 78;
  }
  .hero em {
    font-style: normal;
    color: var(--color-accentBright);
  }
  .hero p {
    margin: var(--space-2) 0 0;
  }
  .notice {
    color: var(--color-success);
    font-weight: 600;
  }
  .actions {
    display: flex;
    flex-wrap: wrap;
    justify-content: flex-end;
    gap: var(--space-3);
    z-index: 1;
  }
  .modes {
    margin-bottom: var(--space-5);
  }
  .options {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(var(--size-meter), 1fr));
    gap: var(--space-3);
  }
  .option {
    display: grid;
    justify-items: center;
    gap: var(--space-2);
    padding: var(--space-4) var(--space-3);
    text-align: center;
  }
  .option.best {
    border-color: var(--color-accent);
    background: linear-gradient(180deg, color-mix(in srgb, var(--color-accent) 16%, transparent), transparent 70%), var(--color-panel);
  }
  .option small {
    font-size: var(--text-xs);
  }
  .current {
    color: var(--color-accentBright);
    font-weight: 700;
    letter-spacing: var(--tracking-wide);
    text-transform: uppercase;
  }
  .all {
    max-height: calc(var(--size-aside) * 1.4);
    overflow-y: auto;
  }
  .rank-card {
    display: flex;
    align-items: center;
    gap: var(--space-4);
  }
  .rank-card img {
    width: var(--size-portrait);
    height: var(--size-portrait);
  }
  .rank-card span {
    display: grid;
  }
  .rank-card b {
    font-size: var(--text-xl);
  }
  .position {
    display: grid;
    grid-template-columns: var(--size-rangeLabel) repeat(3, minmax(0, 1fr));
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-4);
  }
  .position + .position {
    border-top: var(--border-hairline) solid var(--color-line);
  }
  .position h4 {
    margin: 0;
    color: var(--color-textMuted);
    font-size: var(--text-xs);
    letter-spacing: var(--tracking-wide);
    text-transform: uppercase;
  }
  .pick {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    min-width: 0;
    padding: var(--space-1);
    border: none;
    background: none;
    text-align: left;
  }
  .pick:hover {
    background: color-mix(in srgb, var(--color-accent) 8%, transparent);
  }
  .pick img {
    width: var(--size-thumbSm);
    height: var(--size-thumbSm);
    flex: none;
  }
  .who {
    display: grid;
    min-width: 0;
    line-height: 1.2;
  }
  .who b {
    overflow: hidden;
    font-size: var(--text-sm);
    font-weight: 600;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .who small {
    font-size: var(--text-xs);
  }
  .more {
    margin-top: var(--space-3);
  }
  .verdict b {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    font-size: var(--text-xs);
    letter-spacing: var(--tracking-wide);
    text-transform: uppercase;
  }
  .verdict.buff b {
    color: var(--color-success);
  }
  .verdict.nerf b {
    color: var(--color-accentBright);
  }
  .verdict.adjusted b {
    color: var(--color-warning);
  }
  .grid {
    display: grid;
    grid-template-columns: minmax(0, 1fr) var(--size-aside);
    gap: var(--space-6);
    align-items: start;
  }
  .column {
    display: grid;
    gap: var(--space-4);
    align-content: start;
  }
  .column > .section-title {
    margin: 0;
  }
  .cards {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: var(--space-4);
  }
  .card {
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-8) var(--space-4) var(--space-5);
    text-align: center;
  }
  .card.best {
    border-color: var(--color-accent);
    background: linear-gradient(180deg, color-mix(in srgb, var(--color-accent) 18%, transparent), transparent 65%), var(--color-panel);
    animation:
      appear 0.35s ease-out both,
      pulse 2.4s ease-in-out 0.4s infinite;
    animation-delay: calc(var(--i, 0) * 60ms), 0.4s;
  }
  @keyframes pulse {
    50% {
      background-color: color-mix(in srgb, var(--color-accent) 12%, var(--color-panel));
      border-color: var(--color-accentBright);
    }
  }
  .crown {
    position: absolute;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: var(--space-1);
    top: 0;
    left: 0;
    right: 0;
    padding: var(--space-1);
    background: var(--color-accent);
    font-size: var(--text-xs);
    font-weight: 700;
    letter-spacing: var(--tracking-wider);
    text-transform: uppercase;
  }
  .card img {
    width: var(--size-portrait);
    height: var(--size-portrait);
  }
  .card h4 {
    margin: 0;
    font-size: var(--text-lg);
    font-weight: 600;
  }
  .quality {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    font-size: var(--text-sm);
    font-weight: 700;
    letter-spacing: var(--tracking-wide);
  }
  .bar {
    align-self: stretch;
    height: var(--space-1);
    margin: 0 var(--space-2);
    background: var(--color-line);
  }
  .bar i {
    display: block;
    height: 100%;
    background: var(--color-accent);
  }
  .reroll {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    color: var(--color-warning);
    font-weight: 600;
  }
  .tips {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: var(--space-3);
    margin-bottom: var(--space-4);
  }
  .tip {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-3) var(--space-4);
  }
  .tip img {
    width: var(--size-thumb);
    height: var(--size-thumb);
    flex: none;
  }
  .key {
    display: grid;
    place-items: center;
    width: var(--size-thumb);
    height: var(--size-thumb);
    flex: none;
    background: var(--color-accent);
    color: var(--color-white);
    font-size: var(--text-xl);
    font-weight: 700;
  }
  .key.idle {
    background: var(--color-panelRaised);
    color: var(--color-textMuted);
  }
  .tip-text {
    display: grid;
    min-width: 0;
  }
  .tip-text small.ready {
    color: var(--color-success);
  }
  .chips {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: var(--space-2);
  }
  .rounds {
    display: grid;
    gap: var(--space-2);
    padding: var(--space-3);
  }
  .round {
    display: grid;
    grid-template-columns: var(--size-rangeLabel) repeat(3, minmax(0, 1fr));
    align-items: center;
    gap: var(--space-2);
  }
  .round b {
    font-size: var(--text-sm);
    letter-spacing: var(--tracking-wide);
    text-transform: uppercase;
  }
  .pair {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
    gap: var(--space-4);
    margin-top: var(--space-2);
  }
  .list .row {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    width: 100%;
    padding: var(--space-2) var(--space-4);
    font-size: var(--text-md);
    border: none;
    background: none;
    text-align: left;
  }
  .list .row + .row {
    border-top: var(--border-hairline) solid var(--color-line);
  }
  button.row:hover {
    background: color-mix(in srgb, var(--color-accent) 8%, transparent);
  }
  .row img {
    width: var(--size-thumb);
    height: var(--size-thumb);
  }
  .row span {
    flex: 1;
  }
  .rank {
    width: var(--space-6);
    color: var(--color-textMuted);
    font-size: var(--text-sm);
  }
  .game span,
  .champion span {
    display: flex;
    flex-direction: column;
    line-height: 1.2;
  }
  .game small,
  .champion small {
    font-size: var(--text-xs);
  }
  .order-hint {
    display: block;
    margin: var(--space-2) 0;
    font-size: var(--text-xs);
  }
  .game b {
    font-size: var(--text-sm);
    letter-spacing: var(--tracking-normal);
    text-transform: uppercase;
    color: var(--color-textMuted);
  }
  .game.won b {
    color: var(--color-accentBright);
  }
  .tiles {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--space-4);
  }
  .box {
    padding: var(--space-4);
    font-size: var(--text-md);
  }
  .box p {
    margin: 0 0 var(--space-3);
  }
  .box p:last-child {
    margin-bottom: 0;
  }
  .ok,
  .warning {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }
  .ok {
    color: var(--color-success);
    font-weight: 700;
    letter-spacing: var(--tracking-relaxed);
    text-transform: uppercase;
    font-size: var(--text-sm);
  }
  .warning {
    color: var(--color-warning);
  }
  .small {
    font-size: var(--text-sm);
  }
  .fact {
    display: flex;
    justify-content: space-between;
    padding: var(--space-2) 0;
  }
  .fact + .fact {
    border-top: var(--border-hairline) solid var(--color-line);
  }
</style>
