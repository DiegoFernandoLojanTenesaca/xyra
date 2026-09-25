<script lang="ts">
  import { Download, Hammer } from '@lucide/svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { app } from '../app.svelte';
  import Button from '../components/Button.svelte';
  import ChampionHeader from '../components/ChampionHeader.svelte';
  import ChampionPicker from '../components/ChampionPicker.svelte';
  import EmptyState from '../components/EmptyState.svelte';
  import PageHeader from '../components/PageHeader.svelte';
  import SegmentedControl from '../components/SegmentedControl.svelte';
  import type { Asset, Build } from '../types';

  const POSITIONS = ['top', 'jungle', 'mid', 'adc', 'support'];
  const SKILLS = ['Q', 'W', 'E', 'R'];
  const DEFAULT_POSITION = 'mid';

  const t = $derived(app.t);
  let build = $state<Build | null>(null);
  let failed = $state(false);
  let notice = $state('');
  let busy = $state(false);

  $effect(app.pickDefaultChampion);

  $effect(() => {
    const [champion, mode, position] = [app.selectedChampion, app.buildMode, app.buildMode === 'rift' ? app.position : null];
    if (champion === null) return;
    build = null;
    failed = false;
    notice = '';
    invoke<Build>('get_build', { champion, mode, position })
      .then((b) => {
        if (champion === app.selectedChampion && mode === app.buildMode) build = b;
      })
      .catch(() => (failed = true));
  });

  const champion = $derived(app.champions.find((c) => c.id === app.selectedChampion) ?? null);
  const missingNames = $derived(!!build && !build.runes.primary_style.icon);
  const itemBlocks = $derived(
    build
      ? ([
          ['starting', build.starting_items],
          ['boots', build.boots],
          ['core', build.core_items],
          ['situational', build.situational_items],
        ] as const)
      : [],
  );
  const place = $derived(
    app.buildMode === 'rift' ? `${t('build:modes.rift')} · ${build?.position ? t(`build:positions.${build.position}`) : ''}` : t('build:modes.aram'),
  );

  async function importTo(target: 'runes' | 'items') {
    if (!build) return;
    busy = true;
    notice = await app.importBuild(build.champion, target);
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

<PageHeader title={t('common:nav.build')} subtitle={t('build:subtitle')}>
  <ChampionPicker />
</PageHeader>

<div class="tools">
  <SegmentedControl options={[['aram', t('build:modes.aram')], ['rift', t('build:modes.rift')]]} bind:value={app.buildMode} />
  {#if app.buildMode === 'rift'}
    <SegmentedControl
      options={POSITIONS.map((p) => [p, t(`build:positions.${p}`)] as [string, string])}
      bind:value={() => app.position ?? build?.position ?? DEFAULT_POSITION, (p) => (app.position = p)}
    />
  {/if}
</div>

{#if champion}
  <ChampionHeader
    {champion}
    showTier={app.buildMode === 'aram'}
    detail={build ? `${place} · ${t('common:winRate', { value: build.win_rate.toFixed(1) })} · ${t('common:games', { count: build.games })}` : place}
  >
    <Button icon={Download} disabled={!build || busy} onclick={() => importTo('items')}>{t('build:importItems')}</Button>
    <Button variant="primary" icon={Download} disabled={!build || busy} onclick={() => importTo('runes')}>{t('build:importRunes')}</Button>
  </ChampionHeader>
{/if}

{#if notice}<p class="notice">{notice}</p>{/if}

{#if failed}
  <EmptyState icon={Hammer} text={t('build:loadError')} />
{:else if !build}
  <p class="muted loading">{t('build:loading')}</p>
{:else}
  {#if missingNames}<p class="muted">{t('build:noCatalog')}</p>{/if}
  <div class="grid">
    <section class="panel cut box">
      <h3 class="section-title">
        {t('build:runes')} <small>{t('common:winRate', { value: build.runes.win_rate.toFixed(1) })} · {t('build:pickRate', { value: build.runes.pick_rate.toFixed(0) })}</small>
      </h3>
      <div class="tree">
        <div class="style">{@render asset(build.runes.primary_style, 'var(--size-thumbSm)')}<b>{build.runes.primary_style.name}</b></div>
        <div class="row">
          {#each build.runes.primary as rune, i (i)}
            {@render asset(rune, i === 0 ? 'var(--size-portraitLg)' : 'var(--size-thumbLg)')}
          {/each}
        </div>
      </div>
      <div class="tree">
        <div class="style">{@render asset(build.runes.secondary_style, 'var(--size-thumbSm)')}<b>{build.runes.secondary_style.name}</b></div>
        <div class="row">
          {#each build.runes.secondary as rune, i (i)}
            {@render asset(rune, 'var(--size-thumbMd)')}
          {/each}
        </div>
      </div>
      <div class="tree">
        <div class="style"><b class="muted">{t('build:shards')}</b></div>
        <div class="row">
          {#each build.runes.shards as shard, i (i)}
            {@render asset(shard, 'var(--size-thumbSm)')}
          {/each}
        </div>
      </div>
    </section>

    <div class="column">
      <section class="panel cut box">
        <h3 class="section-title">{t('build:spells')}</h3>
        <div class="row">
          {#each build.spells as spell (spell.id)}
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
                  {#if block === 'core' && j > 0}<span class="arrow">›</span>{/if}
                  {@render asset(item, block === 'core' ? 'var(--size-thumbLg)' : 'var(--size-thumbMd)')}
                {/each}
              </div>
            </div>
          {/if}
        {/each}
      </section>
    </div>
  </div>

  {#if build.skill_order.length}
    <section class="panel cut box">
      <h3 class="section-title">{t('build:skills')} <small>{t('build:maxFirst')}: {build.skill_priority.join(' › ')}</small></h3>
      <div class="skills" style="--levels:{build.skill_order.length}">
        {#each SKILLS as skill (skill)}
          <b class="key">{skill}</b>
          {#each build.skill_order as leveled, level (level)}
            <span class:on={leveled === skill}>{leveled === skill ? level + 1 : ''}</span>
          {/each}
        {/each}
      </div>
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
    border-top: 1px solid var(--color-line);
  }
  .style {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    margin-bottom: var(--space-3);
  }
  .style b {
    font-size: var(--text-md);
    letter-spacing: 1.5px;
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
    padding: 2px;
    overflow: hidden;
    font-size: 9px;
    color: var(--color-textMuted);
    background: var(--color-panelRaised);
    border: 1px solid var(--color-line);
  }
  .block + .block {
    margin-top: var(--space-4);
  }
  .block small {
    display: block;
    margin-bottom: var(--space-2);
    font-size: var(--text-xs);
    letter-spacing: 2px;
    text-transform: uppercase;
    font-weight: 700;
  }
  .arrow {
    color: var(--color-accentBright);
    font-size: var(--text-xl);
    font-weight: 700;
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
    font-size: var(--text-sm);
  }
</style>
