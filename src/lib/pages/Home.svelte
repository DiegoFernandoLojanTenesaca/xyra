<script lang="ts">
  import { Download, Hammer, Layers, Pause, Play } from '@lucide/svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { app, percent } from '../app.svelte';
  import Button from '../components/Button.svelte';
  import EmptyState from '../components/EmptyState.svelte';
  import Logo from '../components/Logo.svelte';
  import StatTile from '../components/StatTile.svelte';
  import TierBadge from '../components/TierBadge.svelte';
  import { around } from '../i18n';
  import { championTierColor, qualityColor } from '../theme';
  import type { ChampionInfo } from '../types';

  const TOP_CHAMPIONS = 5;
  const RECENT_GAMES = 5;
  const BEST_AUGMENTS = 4;
  const MIN_BAR = 4;

  const t = $derived(app.t);
  const engine = $derived(app.state);
  const config = $derived(app.config);
  const stats = $derived(app.stats);

  let notice = $state('');
  const cards = $derived([...engine.cards].sort((a, b) => a.x - b.x));
  const best = $derived(cards.find((c) => c.best) ?? null);
  const mine = $derived(engine.champ_select?.champion ?? null);
  /** The best ranked bench champion, only when it beats yours. */
  const betterOnBench = $derived.by(() => {
    const ranked = (engine.champ_select?.bench ?? []).filter((c) => c.rank !== null).sort((a, b) => a.rank! - b.rank!);
    const top = ranked[0];
    return top && (!mine?.rank || top.rank! < mine.rank) ? top : null;
  });
  const inGameChampion = $derived(app.champions.find((c) => c.name === engine.champion) ?? null);
  const topChampions = $derived(app.champions.filter((c) => c.rank !== null).slice(0, TOP_CHAMPIONS));
  const mode = $derived(engine.mode ? t(`common:modes.${engine.mode}`) : '');

  const hero = $derived.by(() => {
    if (engine.phase === 'in_game') {
      const base = { icon: inGameChampion?.icon, eyebrow: `● ${t('home:live')} · ${mode}` };
      return best
        ? { ...base, title: t('home:pickCard', { card: best.name }), strong: best.name, text: t('home:bestOf', { count: cards.length, champion: engine.champion ?? '' }) }
        : { ...base, title: engine.champion ?? '', strong: '', text: t('home:inGame') };
    }
    if (engine.phase === 'champ_select') {
      const base = { icon: (betterOnBench ?? mine)?.icon, eyebrow: `● ${t('common:phases.champSelect')}` };
      return betterOnBench
        ? {
            ...base,
            title: t('home:takeFromBench', { champion: betterOnBench.name }),
            strong: betterOnBench.name,
            text: t('home:strongerThan', { champion: mine?.name ?? '', rank: betterOnBench.rank }),
          }
        : { ...base, title: mine?.name ?? t('common:phases.champSelect'), strong: mine?.name ?? '', text: t('home:yourChampion', { rank: mine?.rank ?? '—' }) };
    }
    const key = engine.phase === 'paused' ? 'paused' : engine.phase === 'client' ? 'client' : 'noClient';
    return { icon: undefined, eyebrow: t(`common:phases.${key}`), title: t(`home:${key}.title`), strong: '', text: t(`home:${key}.text`) };
  });
  const [heroBefore, heroAfter] = $derived(hero.strong ? around(hero.title, hero.strong) : [hero.title, '']);

  async function setBorderless() {
    notice = await invoke('set_borderless').then(() => t('home:borderlessDone'), app.errorText);
  }
</script>

{#snippet portrait(champion: ChampionInfo, highlighted: boolean)}
  <button class="portrait" class:best={highlighted} onclick={() => app.openAugments(champion.id)} title="{champion.name} #{champion.rank ?? '—'}">
    <img src={champion.icon} alt={champion.name} />
    <span><TierBadge label={champion.tier ? `T${champion.tier}` : '—'} color={championTierColor(champion.tier)} size="var(--size-thumbSm)" /></span>
  </button>
{/snippet}

<section class="hero highlight cut" class:live={engine.phase === 'in_game' || engine.phase === 'champ_select'}>
  {#if hero.icon}<img class="cut" src={hero.icon} alt="" />{:else}<Logo size={76} phase={engine.phase} />{/if}
  <div class="text">
    <span class="eyebrow">{hero.eyebrow}</span>
    <h2 class="condensed">{heroBefore}{#if hero.strong}<em>{hero.strong}</em>{/if}{heroAfter}</h2>
    <p class="muted">{hero.text}</p>
    {#if notice && engine.phase === 'champ_select'}<p class="notice">{notice}</p>{/if}
  </div>
  <div class="actions">
    {#if engine.phase === 'champ_select' && mine}
      <Button icon={Hammer} onclick={() => app.openBuild(mine.id)}>{t('home:openBuild')}</Button>
      <Button variant="primary" icon={Download} onclick={async () => (notice = await app.importBuild(mine.id, 'runes'))}>{t('build:importRunes')}</Button>
    {:else}
      <Button icon={Play} disabled={engine.phase === 'in_game'} onclick={() => invoke('test_overlay')}>{t('home:test')}</Button>
      <Button variant="primary" icon={config.paused ? Play : Pause} onclick={() => app.saveConfig({ paused: !config.paused })}>
        {config.paused ? t('home:resume') : t('home:pause')}
      </Button>
    {/if}
  </div>
</section>

<div class="grid">
  <div class="column">
    <h3 class="section-title">{engine.phase === 'in_game' ? t('home:cardsOnScreen') : t('home:latestCards')}</h3>
    {#if cards.length}
      <div class="cards">
        {#each cards as card, i (card.id)}
          <div class="card panel cut appear" class:best={card.best} style="--i:{i}">
            {#if card.best}<div class="crown">★ {t('home:bestPick')}</div>{/if}
            {#if card.icon}<img src={card.icon} alt="" />{/if}
            <h4>{card.name || `#${card.id}`}</h4>
            <span class="quality" style="color:{qualityColor(card.quality)}">
              <TierBadge label={card.grade} color={qualityColor(card.quality)} />{t(`common:quality.${card.quality}`)}
            </span>
            <div class="bar"><i style="width:{card.tier === null ? 0 : Math.max(MIN_BAR, Math.min(100, card.performance))}%"></i></div>
            {#if card.reroll}<small class="reroll">↻ {t('common:reroll')}</small>{/if}
          </div>
        {/each}
      </div>
    {:else}
      <div class="panel cut"><EmptyState icon={Layers} text={t('home:noCards')} /></div>
    {/if}

    <div class="pair">
      {#if topChampions.length}
        <section>
          <h3 class="section-title">{t('home:topMayhem')}</h3>
          <div class="panel cut list">
            {#each topChampions as champion, i (champion.id)}
              <button class="row appear" style="--i:{i}" onclick={() => app.openBuild(champion.id)}>
                <b class="rank">#{champion.rank}</b>
                <img src={champion.icon} alt="" />
                <span>{champion.name}</span>
                <TierBadge label={`T${champion.tier}`} color={championTierColor(champion.tier)} size="var(--size-thumbSm)" />
              </button>
            {/each}
          </div>
        </section>
      {/if}
      {#if stats?.recent.length}
        <section>
          <h3 class="section-title">{t('home:latestGames')}</h3>
          <div class="panel cut list">
            {#each stats.recent.slice(0, RECENT_GAMES) as game, i (game.game_id)}
              <button class="row appear game" class:won={game.win} style="--i:{i}" onclick={() => app.goTo('stats')}>
                {#if game.icon}<img src={game.icon} alt="" />{/if}
                <span>{game.champion}<small class="muted">{t(`common:modes.${game.mode}`)}</small></span>
                <b>{game.win ? t('stats:victory') : t('stats:defeat')}</b>
              </button>
            {/each}
          </div>
        </section>
      {/if}
    </div>

    {#if stats?.augments.length}
      <h3 class="section-title">{t('home:bestAugments')} <small>({t('home:minGames')})</small></h3>
      <div class="panel cut list">
        {#each stats.augments.slice(0, BEST_AUGMENTS) as augment, i (augment.id)}
          <div class="row appear" style="--i:{i}">
            {#if augment.icon}<img src={augment.icon} alt="" />{/if}
            <span>{augment.name}</span>
            <small class="muted">{augment.games}</small>
            <b class="accent">{percent(augment.wins, augment.games)}%</b>
          </div>
        {/each}
      </div>
    {/if}
  </div>

  <aside class="column">
    <div class="tiles">
      <StatTile label={t('stats:games')} value={stats?.games ?? 0} />
      <StatTile label={t('stats:winRate')} value={stats?.games ? `${percent(stats.wins, stats.games)}%` : '—'} accent />
    </div>

    {#if engine.champ_select}
      <div class="panel cut box">
        <h3 class="section-title">{t('home:bench')}</h3>
        <div class="bench">
          {#if mine}{@render portrait(mine, false)}{/if}
          {#each engine.champ_select.bench as champion (champion.id)}
            {@render portrait(champion, champion.id === betterOnBench?.id)}
          {/each}
        </div>
        {#if !engine.champ_select.bench.length}<p class="muted">{t('home:emptyBench')}</p>{/if}
      </div>
    {/if}

    <div class="panel cut box">
      <h3 class="section-title">{t('home:gameWindow')}</h3>
      {#if engine.borderless === true}
        <p class="ok">■ {t('home:borderlessReady')}</p>
      {:else if engine.borderless === false}
        <p class="warning">⚠ {t('home:fullscreenWarning')}</p>
        <Button variant="primary" onclick={setBorderless}>{t('home:setBorderless')}</Button>
      {:else}
        <p class="muted">{t('home:settingsUnreadable')}</p>
      {/if}
      {#if notice && engine.phase !== 'champ_select'}<p class="muted small">{notice}</p>{/if}
    </div>

    <div class="panel cut box">
      <h3 class="section-title">{t('home:system')}</h3>
      <div class="fact"><span class="muted">{t('home:screenReading')}</span><b>{engine.ocr_language ?? '—'}</b></div>
      <div class="fact"><span class="muted">{t('common:nav.labels')}</span><b>{t(`labels:styles.${config.label_style}.name`)}</b></div>
      <div class="fact"><span class="muted">{t('home:voiceHint')}</span><b>{config.voice ? 'ON' : 'OFF'}</b></div>
      {#if !engine.ocr_language}<p class="warning">⚠ {t('home:noOcr')}</p>{/if}
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
    box-shadow: 0 0 0 2px var(--color-accent);
  }
  .text {
    flex: 1;
    min-width: 0;
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
    gap: var(--space-3);
    z-index: 1;
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
    animation: appear 0.35s ease-out both, pulse 2.4s ease-in-out 0.4s infinite;
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
    top: 0;
    left: 0;
    right: 0;
    padding: var(--space-1);
    background: var(--color-accent);
    font-size: var(--text-xs);
    font-weight: 700;
    letter-spacing: 3px;
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
    letter-spacing: 2px;
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
    color: var(--color-warning);
    font-weight: 600;
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
    border-top: 1px solid var(--color-line);
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
  .game span {
    display: flex;
    flex-direction: column;
    line-height: 1.2;
  }
  .game small {
    font-size: var(--text-xs);
  }
  .game b {
    font-size: var(--text-sm);
    letter-spacing: 1px;
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
  .ok {
    color: var(--color-success);
    font-weight: 700;
    letter-spacing: 1.5px;
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
    border-top: 1px solid var(--color-line);
  }
  .bench {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-3);
  }
  .portrait {
    position: relative;
    padding: 0;
    border: none;
    background: none;
    opacity: 0.65;
    transition: opacity 0.15s;
  }
  .portrait:first-child,
  .portrait:hover,
  .portrait.best {
    opacity: 1;
  }
  .portrait img {
    width: var(--size-thumbLg);
    height: var(--size-thumbLg);
  }
  .portrait.best img {
    box-shadow: 0 0 0 2px var(--color-accentBright), 0 0 var(--space-4) var(--color-accent);
  }
  .portrait span {
    position: absolute;
    right: calc(-1 * var(--space-2));
    bottom: calc(-1 * var(--space-2));
  }
</style>
