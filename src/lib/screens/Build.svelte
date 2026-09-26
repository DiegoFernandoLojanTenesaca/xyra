<script lang="ts">
  import { ChevronRight, Download, Hammer } from '@lucide/svelte';
  import { app } from '../app.svelte';
  import { getBuild } from '../services/league';
  import { resource } from '../services/resource.svelte';
  import type { Asset, ImportTarget, Matchup, Position } from '../types';
  import Button from '../ui/Button.svelte';
  import ChampionHeader from '../ui/ChampionHeader.svelte';
  import ChampionPicker from '../ui/ChampionPicker.svelte';
  import EmptyState from '../ui/EmptyState.svelte';
  import PageHeader from '../ui/PageHeader.svelte';
  import SegmentedControl from '../ui/SegmentedControl.svelte';

  const SKILLS = ['Q', 'W', 'E', 'R'];
  const RATE_DIGITS = 1;

  const t = $derived(app.t);
  const format = $derived(app.format);
  let notice = $state('');
  let busy = $state(false);

  $effect(app.pickDefaultChampion);

  const build = resource(
    () =>
      app.selectedChampion === null
        ? null
        : { champion: app.selectedChampion, mode: app.buildMode, position: app.buildMode === 'rift' ? app.positionOverride : null },
    ({ champion, mode, position }) => {
      notice = '';
      return getBuild(champion, mode, position);
    },
  );

  const champion = $derived(app.champions.find((c) => c.id === app.selectedChampion) ?? null);
  const current = $derived(build.value);
  const missingNames = $derived(!!current && !current.runes.primary_style.icon);
  const itemBlocks = $derived(
    current
      ? ([
          ['starting', current.starting_items],
          ['boots', current.boots],
          ['core', current.core_items],
          ['situational', current.situational_items],
        ] as const)
      : [],
  );
  const details = $derived([
    t(`build:modes.${app.buildMode}`),
    ...(app.buildMode === 'rift' && current?.position ? [t(`build:positions.${current.position}`)] : []),
    ...(current ? [t('common:winRate', { value: format.percent(current.win_rate, RATE_DIGITS) }), t('common:games', { count: current.games })] : []),
  ]);

  async function importTo(target: ImportTarget) {
    if (!current) return;
    busy = true;
    notice = await app.importBuild(current.champion, target);
    busy = false;
  }
</script>

{#snippet asset(item: Asset, size: string)}
  {#if item.icon}
    <img class="asset" src={item.icon} alt={item.name} title={item.name} style="--asset-size:{size}" />
  {:else}
    <span class="asset missing" title={item.name} style="--asset-size:{size}">{item.name}</span>
  {/if}
{/snippet}

{#snippet matchups(title: string, list: Matchup[])}
  <div class="block">
    <small class="muted">{title}</small>
    {#each list as matchup (matchup.champion.id)}
      <button class="matchup" onclick={() => (app.selectedChampion = matchup.champion.id)}>
        {@render asset(matchup.champion, 'var(--size-thumb)')}
        <span class="name">{matchup.champion.name}</span>
        <span class="muted games">{t('common:games', { count: matchup.games })}</span>
        <b>{format.percent(matchup.win_rate, RATE_DIGITS)}</b>
      </button>
    {/each}
  </div>
{/snippet}

<PageHeader title={t('common:nav.build')} subtitle={t('build:subtitle')}>
  <ChampionPicker />
</PageHeader>

<div class="tools">
  <SegmentedControl
    options={[
      ['aram', t('build:modes.aram')],
      ['rift', t('build:modes.rift')],
    ]}
    bind:value={app.buildMode}
  />
  {#if app.buildMode === 'rift'}
    <SegmentedControl
      options={app.choices.positions.map((p) => [p, t(`build:positions.${p}`)] as [Position, string])}
      bind:value={() => app.positionOverride ?? current?.position ?? app.choices.positions[0], (p) => (app.positionOverride = p)}
    />
  {/if}
</div>

{#if champion}
  <ChampionHeader {champion} showTier={app.buildMode === 'aram'} {details}>
    {#if app.state.phase === 'champSelect'}
      <Button icon={Download} disabled={!current || busy} onclick={() => importTo('spells')}>{t('build:importSpells')}</Button>
    {/if}
    <Button icon={Download} disabled={!current || busy} onclick={() => importTo('items')}>{t('build:importItems')}</Button>
    <Button variant="primary" icon={Download} disabled={!current || busy} onclick={() => importTo('runes')}>{t('build:importRunes')}</Button>
  </ChampionHeader>
{/if}
{#if notice}<p class="notice">{notice}</p>{/if}

{#if build.error}
  <EmptyState icon={Hammer} text={app.errorText(build.error)} />
{:else if !current}
  <p class="muted loading">{t('build:loading')}</p>
{:else}
  {#if missingNames}<p class="muted">{t('build:noCatalog')}</p>{/if}
  <div class="grid">
    <section class="panel cut box">
      <h3 class="section-title">
        {t('build:runes')}
        <small class="facts">
          <span>{t('common:winRate', { value: format.percent(current.runes.win_rate, RATE_DIGITS) })}</span>
          <span>{t('build:pickRate', { value: format.percent(current.runes.pick_rate) })}</span>
        </small>
      </h3>
      <div class="tree">
        <div class="style">{@render asset(current.runes.primary_style, 'var(--size-thumbSm)')}<b>{current.runes.primary_style.name}</b></div>
        <div class="row">
          {#each current.runes.primary as rune, i (i)}
            {@render asset(rune, i === 0 ? 'var(--size-portraitLg)' : 'var(--size-thumbLg)')}
          {/each}
        </div>
      </div>
      <div class="tree">
        <div class="style">{@render asset(current.runes.secondary_style, 'var(--size-thumbSm)')}<b>{current.runes.secondary_style.name}</b></div>
        <div class="row">
          {#each current.runes.secondary as rune, i (i)}
            {@render asset(rune, 'var(--size-thumbMd)')}
          {/each}
        </div>
      </div>
      <div class="tree">
        <div class="style"><b class="muted">{t('build:shards')}</b></div>
        <div class="row">
          {#each current.runes.shards as shard, i (i)}
            {@render asset(shard, 'var(--size-thumbSm)')}
          {/each}
        </div>
      </div>
    </section>

    <div class="column">
      <section class="panel cut box">
        <h3 class="section-title">{t('build:spells')}</h3>
        <div class="row">
          {#each current.spells as spell (spell.id)}
            {@render asset(spell, 'var(--size-thumbLg)')}
          {/each}
        </div>
      </section>
      <section class="panel cut box">
        <h3 class="section-title">{t('build:items')}</h3>
        {#each itemBlocks as [block, items] (block)}
          {#if items.length}
            <div class="block">
              <small class="muted">{t(`build:blocks.${block}`)}</small>
              <div class="row">
                {#each items as item, j (j)}
                  {#if block === 'core' && j > 0}<span class="arrow"><ChevronRight size={16} /></span>{/if}
                  {@render asset(item, block === 'core' ? 'var(--size-thumbLg)' : 'var(--size-thumbMd)')}
                {/each}
              </div>
            </div>
          {/if}
        {/each}
      </section>
    </div>
  </div>

  {#if current.skill_order.length}
    <section class="panel cut box">
      <h3 class="section-title">
        {t('build:skills')}
        <small class="skill-priority"
          >{t('build:maxFirst')}:{#each current.skill_priority as skill (skill)}<b>{skill}</b>{/each}</small
        >
      </h3>
      <div class="skills" style="--levels:{current.skill_order.length}">
        {#each SKILLS as skill (skill)}
          <b class="key">{skill}</b>
          {#each current.skill_order as leveled, level (level)}
            <span class:on={leveled === skill}>{leveled === skill ? level + 1 : ''}</span>
          {/each}
        {/each}
      </div>
    </section>
  {/if}

  {#if app.buildMode === 'rift'}
    <section class="panel cut box">
      <h3 class="section-title">{t('build:matchups')}</h3>
      {#if current.strong_against.length}
        <div class="pair">
          {@render matchups(t('build:strongAgainst'), current.strong_against)}
          {@render matchups(t('build:weakAgainst'), current.weak_against)}
        </div>
      {:else}
        <p class="muted">{t('build:noMatchups')}</p>
      {/if}
    </section>
  {/if}

  <p class="muted note">{t('build:note')}</p>
{/if}

<style>
  .tools {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-3);
    margin: calc(-1 * var(--space-2)) 0 var(--space-5);
  }
  .notice {
    margin: calc(-1 * var(--space-2)) 0 var(--space-5);
    color: var(--color-success);
    font-weight: 600;
  }
  .grid {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
    gap: var(--space-6);
    margin-bottom: var(--space-6);
    align-items: start;
  }
  .column {
    display: grid;
    gap: var(--space-6);
  }
  .box {
    padding: var(--space-4);
  }
  .tree + .tree {
    margin-top: var(--space-4);
    padding-top: var(--space-4);
    border-top: var(--border-hairline) solid var(--color-line);
  }
  .style {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    margin-bottom: var(--space-3);
  }
  .style b {
    font-size: var(--text-md);
    letter-spacing: var(--tracking-relaxed);
    text-transform: uppercase;
  }
  .row {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: var(--space-3);
  }
  .asset {
    width: var(--asset-size);
    height: var(--asset-size);
    flex: none;
  }
  .asset.missing {
    display: grid;
    place-items: center;
    padding: var(--space-1);
    overflow: hidden;
    font-size: var(--text-xs);
    color: var(--color-textMuted);
    background: var(--color-panelRaised);
    border: var(--border-hairline) solid var(--color-line);
  }
  .block + .block {
    margin-top: var(--space-4);
  }
  .block > small {
    display: block;
    margin-bottom: var(--space-2);
    font-size: var(--text-xs);
    letter-spacing: var(--tracking-wide);
    text-transform: uppercase;
    font-weight: 700;
  }
  .arrow {
    display: flex;
    color: var(--color-accentBright);
  }
  .skill-priority {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
  }
  .skill-priority b {
    color: var(--color-accentBright);
  }
  .pair {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
    gap: var(--space-6);
  }
  .pair .block + .block {
    margin-top: 0;
  }
  .matchup {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    width: 100%;
    padding: var(--space-1) 0;
    border: none;
    background: none;
    text-align: left;
  }
  .matchup .name {
    flex: 1;
  }
  .matchup .games {
    font-size: var(--text-sm);
  }
  .matchup:hover .name {
    color: var(--color-accentBright);
  }
  .skills {
    display: grid;
    grid-template-columns: var(--space-8) repeat(var(--levels), minmax(0, 1fr));
    gap: var(--space-1);
  }
  .key {
    display: grid;
    place-items: center;
    color: var(--color-accentBright);
  }
  .skills span {
    display: grid;
    place-items: center;
    height: var(--size-thumbSm);
    font-size: var(--text-sm);
    font-weight: 700;
    background: var(--color-panelRaised);
  }
  .skills span.on {
    background: var(--color-accent);
    color: var(--color-white);
  }
  .note {
    margin-top: var(--space-6);
    font-size: var(--text-sm);
  }
</style>
