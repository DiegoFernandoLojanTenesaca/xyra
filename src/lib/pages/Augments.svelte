<script lang="ts">
  import { Layers } from '@lucide/svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { untrack } from 'svelte';
  import { app } from '../app.svelte';
  import ChampionHeader from '../components/ChampionHeader.svelte';
  import ChampionPicker from '../components/ChampionPicker.svelte';
  import EmptyState from '../components/EmptyState.svelte';
  import PageHeader from '../components/PageHeader.svelte';
  import SegmentedControl from '../components/SegmentedControl.svelte';
  import TierBadge from '../components/TierBadge.svelte';
  import { qualityColor, rarityColor } from '../theme';
  import type { AugmentRow } from '../types';

  const MODES = ['KIWI', 'CHERRY'] as const;
  const RARITIES = ['kPrismatic', 'kGold', 'kSilver'] as const;
  const MIN_BAR = 4;

  const t = $derived(app.t);
  let mode = $state<(typeof MODES)[number]>(untrack(() => app.state.mode) === 'CHERRY' ? 'CHERRY' : 'KIWI');
  let rarity = $state<'' | (typeof RARITIES)[number]>('');
  let rows = $state<AugmentRow[] | null>(null);
  let failed = $state(false);

  $effect(app.pickDefaultChampion);

  $effect(() => {
    const [champion, selectedMode] = [app.selectedChampion, mode];
    if (champion === null) return;
    rows = null;
    failed = false;
    invoke<AugmentRow[]>('get_augments', { champion, mode: selectedMode })
      .then((r) => {
        if (champion === app.selectedChampion && selectedMode === mode) rows = r;
      })
      .catch(() => (failed = true));
  });

  const champion = $derived(app.champions.find((c) => c.id === app.selectedChampion) ?? null);
  const visible = $derived((rows ?? []).filter((r) => !rarity || r.rarity === rarity));
  const groups = $derived([...new Set(visible.map((r) => r.tier))].sort((a, b) => a - b).map((tier) => visible.filter((r) => r.tier === tier)));
  const maxPickRate = $derived(Math.max(1, ...(rows ?? []).map((r) => r.pick_rate)));
</script>

<PageHeader title={t('common:nav.augments')} subtitle={t('augments:subtitle')} />

<div class="tools">
  <ChampionPicker />
  <SegmentedControl options={MODES.map((m) => [m, t(`common:modes.${m}`)] as [(typeof MODES)[number], string])} bind:value={mode} />
  <SegmentedControl
    options={[['', t('augments:all')], ...RARITIES.map((r) => [r, t(`common:rarity.${r}`)] as [typeof r, string])]}
    bind:value={rarity}
  />
</div>

{#if champion}
  <ChampionHeader {champion} detail="{t(`common:modes.${mode}`)}{rows ? ` · ${t('augments:augmentCount', { count: rows.length })}` : ''}" />
{/if}

{#if app.selectedChampion === null}
  <EmptyState icon={Layers} text={t('augments:pickChampion')} />
{:else if failed}
  <EmptyState icon={Layers} text={t('augments:loadError')} />
{:else if rows === null}
  <p class="muted loading">{t('augments:loading')}</p>
{:else if !groups.length}
  <EmptyState icon={Layers} text={t('augments:noData')} />
{:else}
  {#each groups as group (group[0].tier)}
    {@const color = qualityColor(group[0].quality)}
    <section class="group">
      <h3 class="section-title" style="color:{color}">
        <TierBadge label={group[0].grade} {color} size="var(--size-thumbSm)" />{t(`common:quality.${group[0].quality}`)}
        <small class="muted">{group.length}</small>
      </h3>
      <div class="rows">
        {#each group as row (row.id)}
          <div class="row panel" style="--row-color:{color}">
            <img src={row.icon} alt="" />
            <div class="name">
              <b>{row.name}</b>
              <small style="color:{rarityColor(row.rarity)}">{t(`common:rarity.${row.rarity}`)}</small>
            </div>
            <div class="measure" title={t('augments:performance')}>
              <div class="bar"><i style="width:{Math.max(MIN_BAR, Math.min(100, row.performance))}%"></i></div>
              <small class="muted">{row.performance.toFixed(0)}</small>
            </div>
            <div class="measure narrow" title={t('augments:pickRate')}>
              <div class="bar pick"><i style="width:{(100 * row.pick_rate) / maxPickRate}%"></i></div>
              <small class="muted">{row.pick_rate.toFixed(1)}%</small>
            </div>
          </div>
        {/each}
      </div>
    </section>
  {/each}
{/if}

<style>
  .tools {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-3);
    margin-bottom: var(--space-5);
  }
  .group {
    margin-bottom: var(--space-6);
  }
  .group .section-title {
    font-size: var(--text-md);
  }
  .rows {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(460px, 1fr));
    gap: var(--space-2);
  }
  .row {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-2) var(--space-4) var(--space-2) var(--space-2);
    border-left: 3px solid var(--row-color);
  }
  .row img {
    width: var(--size-thumbMd);
    height: var(--size-thumbMd);
  }
  .name {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .name b {
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .name small {
    font-size: var(--text-xs);
    letter-spacing: 1px;
    text-transform: uppercase;
  }
  .measure {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    width: 120px;
  }
  .measure.narrow {
    width: 100px;
  }
  .measure small {
    width: var(--space-10);
    text-align: right;
    font-variant-numeric: tabular-nums;
  }
  .bar {
    flex: 1;
    height: var(--space-1);
    background: var(--color-line);
  }
  .bar i {
    display: block;
    height: 100%;
    background: var(--row-color);
  }
  .bar.pick i {
    background: var(--quality-fair);
  }
</style>
