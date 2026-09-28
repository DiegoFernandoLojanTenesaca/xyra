<script lang="ts">
  import { ArrowLeftRight, Download, Hammer, Play, Shuffle, Star } from '@lucide/svelte';
  import { qualityColor } from '$shared/design/theme';
  import { resource } from '$shared/services/resource.svelte';
  import { championTierColor, championTierLabel } from '$shared/design/theme';
  import type { AugmentRow, Asset, Build, ChampionInfo, Friend, LobbyQueue, Meta, StatsSummary } from '$shared/types';
  import LiveStrip from '$shared/ui/LiveStrip.svelte';
  import Skeleton from '$shared/ui/Skeleton.svelte';
  import TierBadge from '$shared/ui/TierBadge.svelte';
  import { link } from '../link.svelte';
  import { mobile } from '../mobile.svelte';

  const COUNTERS_SHOWN = 3;
  const BEST_AUGMENTS = 6;
  const RECENT_GAMES = 5;
  const IMPORT_TARGETS = ['runes', 'items', 'spells'] as const;
  const AUGMENT_MODES = ['mayhem', 'arena'];
  const RIFT_DEFAULT_POSITION = 'mid';

  const t = $derived(mobile.t);
  const format = $derived(mobile.format);
  const engine = $derived(link.state);
  const select = $derived(engine?.champ_select ?? null);
  const game = $derived(engine?.phase === 'inGame' ? engine.game : null);
  const cards = $derived([...(engine?.cards ?? [])].sort((a, b) => a.x - b.x));
  const can = $derived(link.pc?.permissions ?? { accept: false, import: false, bench: false, settings: false, lobby: false });
  /** The queues to open a lobby for and the friends online, while the player is in the client. */
  const lobbyOptions = resource(
    () => (engine?.phase === 'client' && link.status === 'online' ? engine.account : null),
    () => link.call<{ queues: LobbyQueue[]; friends: Friend[] }>('GET', '/api/lobby'),
    (failure) => failure,
  );
  const queueName = (id: number) => lobbyOptions.value?.queues.find((queue) => queue.id === id)?.name ?? `#${id}`;
  let notice = $state('');
  let busy = $state(false);

  const stats = resource(
    () => (engine && !select && !game ? engine.phase : null),
    () => link.get<StatsSummary>('/api/stats'),
    (failure) => failure,
  );
  const build = resource(
    () =>
      select?.champion && link.status === 'online'
        ? { champion: select.champion.id, mode: engine?.build_mode ?? (select.mode === 'summonersRift' ? 'rift' : 'aram'), position: select.position }
        : null,
    (key) => link.call<Build>('GET', '/api/build', key),
    (failure) => failure,
  );
  const champions = resource(
    () => (select && link.status === 'online' ? true : null),
    () => link.get<ChampionInfo[]>('/api/champions'),
    (failure) => failure,
  );
  const meta = resource(
    () => (select?.mode === 'summonersRift' && link.status === 'online' ? true : null),
    () => link.call<Meta>('GET', '/api/meta'),
    (failure) => failure,
  );
  /** Where each champion stands with the player's criterion: the PC sends them sorted by it. */
  const preference = $derived(new Map((champions.value ?? []).map((champion, i) => [champion.id, i])));
  const byPreference = (a: ChampionInfo, b: ChampionInfo) => (preference.get(a.id) ?? Infinity) - (preference.get(b.id) ?? Infinity);
  /** In ARAM, the champion the player has and those on the bench, best first. */
  const options = $derived(
    select && select.mode !== 'summonersRift' ? [select.champion, ...select.bench].filter((c): c is ChampionInfo => !!c).sort(byPreference) : [],
  );
  /** Every other champion the player can play; in the Rift, those of the position. */
  const everyone = $derived.by(() => {
    if (!select) return [];
    const all = champions.value ?? [];
    if (select.mode !== 'summonersRift') {
      const taken = new Set(options.map((c) => c.id));
      return all.filter((c) => c.recommendable && !taken.has(c.id));
    }
    const info = new Map(all.map((c) => [c.id, c]));
    const slot = meta.value?.positions.find((p) => p.position === (select.position ?? RIFT_DEFAULT_POSITION));
    return (slot?.champions ?? []).map((entry) => info.get(entry.champion.id)).filter((c): c is ChampionInfo => !!c && !c.locked);
  });
  const augments = resource(
    () => (game?.champion && AUGMENT_MODES.includes(game.mode) ? { champion: game.champion.id, mode: game.mode } : null),
    (key) => link.call<AugmentRow[]>('GET', '/api/augments', key),
    (failure) => failure,
  );

  async function run(path: string, params: Record<string, string> = {}, done = 'mobile:done') {
    busy = true;
    notice = await mobile.act('POST', path, params, done);
    busy = false;
  }
</script>

{#snippet icon(asset: Asset)}
  {#if asset.icon}<img src={asset.icon} alt={asset.name} title={asset.name} />{/if}
{/snippet}

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
        {#if build.value}
          {@const current = build.value}
          <div class="preview">
            <div class="line">
              <small class="muted">{t('build:spells')}</small>
              <span class="icons"
                >{#each current.spells as spell, i (i)}{@render icon(spell)}{/each}</span
              >
            </div>
            <div class="line">
              <small class="muted">{t('build:runes')}</small>
              <span class="icons">
                {#if current.runes.primary[0]}{@render icon(current.runes.primary[0])}{/if}
                {@render icon(current.runes.primary_style)}{@render icon(current.runes.secondary_style)}
              </span>
            </div>
            <div class="line">
              <small class="muted">{t('build:blocks.core')}</small>
              <span class="icons"
                >{#each current.core_items as item, i (i)}{@render icon(item)}{/each}</span
              >
            </div>
            {#if current.skill_priority.length}
              <div class="line">
                <small class="muted">{t('build:maxFirst')}</small><b class="accent">{current.skill_priority.join(' › ')}</b>
              </div>
            {/if}
          </div>
        {:else if build.error}
          <p class="muted">{mobile.errorText(build.error)}</p>
        {:else}
          <Skeleton label={t('build:loading')} rows={2} />
        {/if}
        {#if can.import}
          <button class="action wide" disabled={busy} onclick={() => run('/api/skin', {}, 'mobile:live.skinChanged')}>
            <Shuffle size={14} />{t('mobile:live.randomSkin')}
          </button>
          <button class="action primary wide" disabled={busy} onclick={() => run('/api/import', { target: 'all' }, 'build:imported.all')}>
            <Download size={14} />{t('build:importAll')}
          </button>
          <div class="actions">
            {#each IMPORT_TARGETS as target (target)}
              <button class="action" disabled={busy} onclick={() => run('/api/import', { target }, `build:imported.${target}`)}
                ><Download size={14} />{t(`mobile:live.import.${target}`)}</button
              >
            {/each}
          </div>
        {/if}
        <button class="action wide" onclick={() => select.champion && mobile.openBuild(select.champion.id, engine.build_mode, select.position)}>
          <Hammer size={14} />{t('mobile:live.openBuild')}
        </button>
      </section>
    {/if}
    {#if options.length > 1}
      <section class="block panel cut">
        <h2 class="section-title">{t('home:yourOptions')}</h2>
        {#each options as option, i (option.id)}
          <div class="row" class:best={i === 0}>
            {#if option.icon}<img src={option.icon} alt="" />{/if}
            <span class="grow">{option.name}</span>
            <TierBadge label={championTierLabel(option.tier)} color={championTierColor(option.tier)} size="var(--size-thumbSm)" />
            {#if option.id === select.champion?.id}
              <small class="accent">{t('home:current')}</small>
            {:else if can.bench}
              <button class="action small" class:primary={i === 0} disabled={busy} onclick={() => run('/api/bench', { champion: String(option.id) })}
                ><ArrowLeftRight size={14} />{t('home:take')}</button
              >
            {/if}
          </div>
        {/each}
      </section>
    {/if}
    {#if everyone.length}
      <details class="block panel cut everyone">
        <summary class="section-title">
          {select.mode === 'summonersRift'
            ? t('home:allForPosition', { position: t(`build:positions.${select.position ?? RIFT_DEFAULT_POSITION}`) })
            : t('home:allChampions')} ({everyone.length})
        </summary>
        {#each everyone as champion, i (champion.id)}
          <button class="row entry" onclick={() => mobile.openBuild(champion.id, engine.build_mode, select.position)}>
            <span class="rank muted">#{i + 1}</span>
            {#if champion.icon}<img src={champion.icon} alt="" />{/if}
            <span class="grow">{champion.name}</span>
            <TierBadge label={championTierLabel(champion.tier)} color={championTierColor(champion.tier)} size="var(--size-thumbSm)" />
          </button>
        {/each}
      </details>
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
    {#if engine.live}
      <div class="block"><LiveStrip live={engine.live} {t} number={format.number} /></div>
    {/if}
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
    {#if engine.phase === 'client'}
      <section class="block panel cut">
        {#if engine.lobby}
          {@const lobby = engine.lobby}
          <h2 class="section-title">{t('lobby:yourLobby', { queue: queueName(lobby.queue_id) })}</h2>
          {#each lobby.members as member, i (i)}
            <div class="row">
              <img src={member.icon} alt="" />
              <span class="grow">{member.name || '—'}</span>
              {#if member.leader}<small class="accent">{t('lobby:leader')}</small>{/if}
            </div>
          {/each}
          {#if lobby.invited.length}<small class="muted">{t('lobby:invited', { names: lobby.invited.join(', ') })}</small>{/if}
          {#if can.lobby}
            {#if lobby.searching}
              <p class="accent searching">{t('lobby:searching')}</p>
              <button class="action wide" disabled={busy} onclick={() => run('/api/lobby/cancel')}>{t('lobby:cancel')}</button>
            {:else if lobby.leader}
              <button class="action primary wide" disabled={busy || !lobby.can_search} onclick={() => run('/api/lobby/search')}>{t('lobby:search')}</button>
            {/if}
            <button class="action wide" disabled={busy || lobby.searching} onclick={() => run('/api/lobby/leave')}>{t('lobby:leave')}</button>
          {/if}
        {:else}
          <h2 class="section-title">{t('lobby:chooseMode')}</h2>
          {#if !can.lobby}
            <p class="muted">{t('mobile:live.notAllowed')}</p>
          {:else if lobbyOptions.value}
            <div class="queues">
              {#each lobbyOptions.value.queues as queue (queue.id)}
                <button class="action" disabled={busy} onclick={() => run('/api/lobby/create', { queue: String(queue.id) }, 'lobby:created')}
                  >{queue.name}</button
                >
              {/each}
            </div>
          {:else if !lobbyOptions.error}
            <Skeleton label={t('lobby:loading')} rows={2} />
          {/if}
        {/if}
      </section>
      {#if can.lobby && lobbyOptions.value?.friends.length}
        <details class="block panel cut friends" open={!!engine.lobby}>
          <summary class="section-title">{t('lobby:friends')} ({lobbyOptions.value.friends.length})</summary>
          {#each lobbyOptions.value.friends as friend (friend.puuid)}
            <div class="row">
              <img src={friend.icon} alt="" />
              <span class="grow stack">{friend.name}<small class="muted">{t(`lobby:status.${friend.status}`)}</small></span>
              {#if engine.lobby?.players.includes(friend.puuid)}
                <small class="muted">{t('lobby:alreadyIn')}</small>
              {:else}
                <button
                  class="action small"
                  disabled={busy || !engine.lobby || !friend.can_join}
                  onclick={() => run('/api/lobby/invite', { puuid: friend.puuid, summoner: String(friend.summoner_id) }, 'lobby:inviteDone')}
                  >{t('lobby:invite')}</button
                >
              {/if}
            </div>
          {/each}
        </details>
      {/if}
    {/if}
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
  .queues {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: var(--space-2);
  }
  .searching {
    margin: var(--space-2) 0;
    font-weight: 700;
  }
  .friends summary {
    cursor: pointer;
    list-style: none;
  }
  .stack {
    display: grid;
  }
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
  .row .small {
    padding: var(--space-1) var(--space-2);
    font-size: var(--text-xs);
  }
  .everyone summary {
    cursor: pointer;
    list-style: none;
  }
  .entry {
    width: 100%;
    border: none;
    background: none;
    text-align: left;
  }
  .rank {
    width: var(--space-6);
    flex: none;
    font-size: var(--text-sm);
  }
  .preview {
    display: grid;
    gap: var(--space-2);
    margin: var(--space-3) 0;
  }
  .line {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-3);
  }
  .icons {
    display: flex;
    gap: var(--space-1);
  }
  .icons img {
    width: var(--size-thumbSm);
    height: var(--size-thumbSm);
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
