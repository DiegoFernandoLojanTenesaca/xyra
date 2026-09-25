<script lang="ts">
  import { Layers } from '@lucide/svelte';
  import { untrack } from 'svelte';
  import { app } from '../app.svelte';
  import { qualityColor, rarityColor } from '../design/theme';
  import { getAugments } from '../services/league';
  import { resource } from '../services/resource.svelte';
  import type { GameMode, Rarity } from '../types';
  import ChampionHeader from '../ui/ChampionHeader.svelte';
  import ChampionPicker from '../ui/ChampionPicker.svelte';
  import EmptyState from '../ui/EmptyState.svelte';
  import PageHeader from '../ui/PageHeader.svelte';
  import SegmentedControl from '../ui/SegmentedControl.svelte';
  import TierBadge from '../ui/TierBadge.svelte';

  const MIN_BAR = 4;
  const PICK_RATE_DIGITS = 1;

  const t = $derived(app.t);
  const format = $derived(app.format);
  const modes = $derived(app.choices.augment_modes);
  let mode = $state<GameMode>(untrack(() => app.choices.augment_modes.find((m) => m === app.state.game?.mode) ?? app.choices.augment_modes[0]));
  let rarity = $state<Rarity | 'all'>('all');

  $effect(app.pickDefaultChampion);

  const augments = resource(
    () => (app.selectedChampion === null ? null : { champion: app.selectedChampion, mode }),
    ({ champion, mode }) => getAugments(champion, mode),
  );

  const champion = $derived(app.champions.find((c) => c.id === app.selectedChampion) ?? null);
  const rows = $derived(augments.value);
  const visible = $derived((rows ?? []).filter((r) => rarity === 'all' || r.rarity === rarity));
  const groups = $derived([...new Set(visible.map((r) => r.tier))].sort((a, b) => a - b).map((tier) => visible.filter((r) => r.tier === tier)));
  const maxPickRate = $derived(Math.max(1, ...(rows ?? []).map((r) => r.pick_rate)));
  const details = $derived([t(`common:modes.${mode}`), ...(rows ? [t('augments:augmentCount', { count: rows.length })] : [])]);
</script>

<PageHeader title={t('common:nav.augments')} subtitle={t('augments:subtitle')} />

<div class="tools">
  <ChampionPicker />
  <SegmentedControl options={modes.map((m) => [m, t(`common:modes.${m}`)] as [GameMode, string])} bind:value={mode} />
  <SegmentedControl
    options={[['all', t('augments:all')], ...app.choices.rarities.map((r) => [r, t(`common:rarity.${r}`)] as [Rarity, string])]}
    bind:value={rarity}
  />
</div>

{#if champion}
  <ChampionHeader {champion} {details} />
{/if}

{#if app.selectedChampion === null}
  <EmptyState icon={Layers} text={t('augments:pickChampion')} />
{:else if augments.error}
  <EmptyState icon={Layers} text={app.errorText(augments.error)} />
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
        <small class="muted">{format.number(group.length)}</small>
      </h3>
      <div class="rows">
        {#each group as row (row.id)}
          <div class="row panel" style="--row-color:{color}">
            <img src={row.icon} alt="" />
            <div class="name">
              <b>{row.name}</b>
              {#if row.rarity}<small style="color:{rarityColor(row.rarity)}">{t(`common:rarity.${row.rarity}`)}</small>{/if}
            </div>
            <div class="measure" title={t('augments:performance')}>
              <div class="bar"><i style="width:{Math.max(MIN_BAR, Math.min(100, row.performance))}%"></i></div>
              <small class="muted">{format.number(row.performance)}</small>
            </div>
            <div class="measure narrow" title={t('augments:pickRate')}>
              <div class="bar pick"><i style="width:{(100 * row.pick_rate) / maxPickRate}%"></i></div>
              <small class="muted">{format.percent(row.pick_rate, PICK_RATE_DIGITS)}</small>
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
    grid-template-columns: repeat(auto-fill, minmax(var(--size-augmentRow), 1fr));
    gap: var(--space-2);
  }
  .row {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-2) var(--space-4) var(--space-2) var(--space-2);
    border-left: var(--border-accent) solid var(--row-color);
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
    letter-spacing: var(--tracking-normal);
    text-transform: uppercase;
  }
  .measure {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    width: var(--size-meter);
  }
  .measure.narrow {
    width: var(--size-meterNarrow);
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
