<script lang="ts">
  import { Download, Hammer, Play, Star } from '@lucide/svelte';
  import { qualityColor } from '$shared/design/theme';
  import { resource } from '$shared/services/resource.svelte';
  import type { AugmentRow, Asset, StatsSummary } from '$shared/types';
  import Skeleton from '$shared/ui/Skeleton.svelte';
  import TierBadge from '$shared/ui/TierBadge.svelte';
  import { link } from '../link.svelte';
  import { mobile } from '../mobile.svelte';

  const COUNTERS_SHOWN = 3;
  const BEST_AUGMENTS = 6;
  const RECENT_GAMES = 5;
  const IMPORT_TARGETS = ['runes', 'items', 'spells'] as const;
  const AUGMENT_MODES = ['mayhem', 'arena'];

  const t = $derived(mobile.t);
  const format = $derived(mobile.format);
  const engine = $derived(link.state);
  const select = $derived(engine?.champ_select ?? null);
  const game = $derived(engine?.phase === 'inGame' ? engine.game : null);
  const cards = $derived([...(engine?.cards ?? [])].sort((a, b) => a.x - b.x));
  let notice = $state('');
  let busy = $state(false);

  const stats = resource(
    () => (engine && !select && !game ? engine.phase : null),
    () => link.get<StatsSummary>('/api/stats'),
    (failure) => failure,
  );
  const augments = resource(
    () => (game?.champion && AUGMENT_MODES.includes(game.mode) ? { champion: game.champion.id, mode: game.mode } : null),
    (key) => link.call<AugmentRow[]>('GET', '/api/augments', key),
    (failure) => failure,
  );

  async function run(path: string, params: Record<string, string> = {}) {
    busy = true;
    notice = await mobile.act('POST', path, params);
    busy = false;
  }
</script>

{#snippet row(asset: Asset, detail = '')}
  <div class="row">
    {#if asset.icon}<img src={asset.icon} alt="" />{/if}
    <span class="grow">{asset.name}</span>
    {#if detail}<b>{detail}</b>{/if}
  </div>
{/snippet}

{#if !engine}
  <Skeleton label={t('mobile:status.connecting')} rows={5} />
{:else}
  {#if engine.ready_check}
    <section class="block panel cut accept">
      <h2 class="section-title">{t('mobile:live.matchFound')}</h2>
      <button class="action primary huge" disabled={busy} onclick={() => run('/api/accept')}>{t('mobile:live.accept')}</button>
    </section>
  {/if}
  {#if notice}<p class="notice">{notice}</p>{/if}

  <section class="block panel cut phase">
    <span class="muted">{t(`common:phases.${engine.phase}`)}</span>
    {#if game}<b>{t(`common:modes.${game.mode}`)}</b>{/if}
    {#if engine.phase === 'paused'}
      <button class="action small" disabled={busy} onclick={() => run('/api/settings', { paused: 'false' })}><Play size={14} />{t('mobile:live.resume')}</button
      >
    {/if}
  </section>

  {#if select}
    {#if select.champion}
      <section class="block panel cut">
        <h2 class="section-title">{t('mobile:live.yourChampion')}</h2>
        {@render row(select.champion, select.champion.rank ? `#${select.champion.rank}` : '')}
        <div class="actions">
          {#each IMPORT_TARGETS as target (target)}
            <button class="action" disabled={busy} onclick={() => run('/api/import', { target })}
              ><Download size={14} />{t(`mobile:live.import.${target}`)}</button
            >
          {/each}
        </div>
        <button class="action wide" onclick={() => select.champion && mobile.openBuild(select.champion.id, engine.build_mode, select.position)}>
          <Hammer size={14} />{t('mobile:live.openBuild')}
        </button>
      </section>
    {/if}
    {#if select.bench_pick}
      <section class="block panel cut">
        <h2 class="section-title">{t('mobile:live.bench')}</h2>
        {@render row(select.bench_pick, select.bench_pick.rank ? `#${select.bench_pick.rank}` : '')}
      </section>
    {/if}
    {#if select.lane_opponent && select.counter_picks.length}
      <section class="block panel cut">
        <h2 class="section-title">{t('mobile:live.counters', { enemy: select.lane_opponent.name })}</h2>
        {#each select.counter_picks.slice(0, COUNTERS_SHOWN) as pick (pick.champion.id)}
          {@render row(pick.champion, format.percent(pick.win_rate, 1))}
        {/each}
      </section>
    {/if}
  {:else if game}
    {#if engine.tips}
      {@const next = engine.tips.next_item}
      <section class="block panel cut">
        <h2 class="section-title">{t('home:tips')}</h2>
        <div class="row">
          <span class="key" class:idle={!engine.tips.skill}>{engine.tips.skill ?? '—'}</span>
          <span class="grow">{engine.tips.skill ? t('home:levelSkill', { skill: engine.tips.skill }) : t('home:noSkillPoint')}</span>
        </div>
        {#if next}
          <div class="row">
            {#if next.item.icon}<img src={next.item.icon} alt="" />{/if}
            <span class="grow">{next.item.name}</span>
            <small class:good={next.missing === 0} class="muted">
              {next.missing === null
                ? t('home:nextItem')
                : next.missing === 0
                  ? t('home:canBuy')
                  : t('home:goldMissing', { gold: format.number(next.missing) })}
            </small>
          </div>
        {/if}
      </section>
    {/if}
    {#if cards.length}
      <section class="block panel cut">
        <h2 class="section-title">{t('home:cardsOnScreen')}</h2>
        {#each cards as card (card.id)}
          <div class="row" class:best={card.best}>
            {#if card.icon}<img src={card.icon} alt="" />{/if}
            <span class="grow">{card.name}</span>
            {#if card.best}<Star size={14} />{/if}
            <TierBadge label={card.grade} color={qualityColor(card.quality)} />
          </div>
        {/each}
      </section>
    {/if}
    {#if augments.value?.length && game.champion}
      <section class="block panel cut">
        <h2 class="section-title">{t('home:bestForChampion', { champion: game.champion.name })}</h2>
        {#each augments.value.slice(0, BEST_AUGMENTS) as augment (augment.id)}
          <div class="row">
            {#if augment.icon}<img src={augment.icon} alt="" />{/if}
            <span class="grow">{augment.name}</span>
            <TierBadge label={augment.grade} color={qualityColor(augment.quality)} />
          </div>
        {/each}
      </section>
    {/if}
  {:else}
    <section class="block panel cut">
      <p class="muted idle-text">{t('mobile:live.idle')}</p>
      {#if stats.value?.games}
        <div class="tiles">
          <div><small class="muted">{t('stats:games')}</small><b>{format.number(stats.value.games)}</b></div>
          <div><small class="muted">{t('stats:winRate')}</small><b class="accent">{format.percent((100 * stats.value.wins) / stats.value.games)}</b></div>
        </div>
      {/if}
    </section>
    {#if stats.value?.recent.length}
      <section class="block panel cut">
        <h2 class="section-title">{t('home:latestGames')}</h2>
        {#each stats.value.recent.slice(0, RECENT_GAMES) as recent (recent.game_id)}
          <div class="row">
            {#if recent.champion.icon}<img src={recent.champion.icon} alt="" />{/if}
            <span class="grow">{recent.champion.name}<small class="muted"> · {t(`common:modes.${recent.mode}`)}</small></span>
            <b class:accent={recent.win}>{recent.win ? t('stats:victory') : t('stats:defeat')}</b>
          </div>
        {/each}
      </section>
    {/if}
  {/if}
{/if}

<style>
  .phase {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    font-size: var(--text-md);
    text-transform: uppercase;
    letter-spacing: var(--tracking-wide);
  }
  .phase .small {
    margin-left: auto;
    padding: var(--space-2) var(--space-3);
  }
  .row.best {
    color: var(--color-accentBright);
    font-weight: 700;
  }
  .wide {
    width: 100%;
    margin-top: var(--space-2);
  }
  .huge {
    width: 100%;
    padding: var(--space-6);
    font-size: var(--text-xl);
  }
  .accept {
    border-color: var(--color-accent);
  }
  .key {
    display: grid;
    place-items: center;
    flex: none;
    width: var(--size-thumb);
    height: var(--size-thumb);
    background: var(--color-accent);
    color: var(--color-white);
    font-weight: 700;
  }
  .key.idle {
    background: var(--color-panelRaised);
    color: var(--color-textMuted);
  }
  .idle-text {
    margin: 0 0 var(--space-3);
  }
  .tiles {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--space-3);
  }
  .tiles div {
    display: grid;
  }
  .tiles b {
    font-size: var(--text-2xl);
  }
</style>
