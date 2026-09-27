<script lang="ts">
  import { ArrowDown, ArrowUp, ExternalLink, TrendingUp } from '@lucide/svelte';
  import { untrack } from 'svelte';
  import { app, META_MODES, type MetaMode } from '../app.svelte';
  import { championTierColor, championTierLabel } from '../design/theme';
  import { openExternal } from '../project';
  import { getMeta, getModeChampions, getPatchChanges } from '../services/league';
  import { resource } from '../services/resource.svelte';
  import type { Position } from '../types';
  import Button from '../ui/Button.svelte';
  import EmptyState from '../ui/EmptyState.svelte';
  import PageHeader from '../ui/PageHeader.svelte';
  import SearchInput from '../ui/SearchInput.svelte';
  import SegmentedControl from '../ui/SegmentedControl.svelte';
  import Skeleton from '../ui/Skeleton.svelte';
  import PatchChange from '../ui/PatchChange.svelte';
  import TierBadge from '../ui/TierBadge.svelte';

  const RATE_DIGITS = 1;
  const CHANGE_FILTERS = ['yours', 'all'] as const;

  const t = $derived(app.t);
  const format = $derived(app.format);
  let position = $state<Position>(untrack(() => app.state.champ_select?.position ?? app.choices.positions[0]));
  let query = $state('');
  let changeFilter = $state<(typeof CHANGE_FILTERS)[number]>('yours');

  const meta = resource(() => true, getMeta);
  const modeChampions = resource(() => (app.metaMode === 'rift' ? null : app.metaMode), getModeChampions);
  const patch = resource(() => app.language, getPatchChanges);
  const changes = $derived((patch.value?.champions ?? []).filter((change) => changeFilter === 'all' || change.yours));
  const rift = $derived(app.metaMode === 'rift');
  const list = $derived(rift ? meta : modeChampions);
  const champions = $derived(
    ((rift ? meta.value?.positions.find((p) => p.position === position)?.champions : modeChampions.value) ?? []).filter(
      (c) => !query.trim() || c.champion.name.toLowerCase().includes(query.trim().toLowerCase()),
    ),
  );

  /** A champion's build in the Rift position being looked at or in ARAM; Arena has augments instead. */
  function open(champion: number) {
    if (app.metaMode === 'rift') app.openRiftBuild(champion, position);
    else if (app.metaMode === 'arena') app.openModeAugments(champion, 'arena');
    else app.openModeBuild(champion, 'aram');
  }
</script>

<PageHeader title={t('common:nav.meta')} subtitle={meta.value ? t('meta:subtitle', { patch: meta.value.patch }) : t('meta:loading')}>
  {#if patch.value}
    {@const notes = patch.value}
    <Button icon={ExternalLink} onclick={() => openExternal(notes.notes_url)}>{t('meta:patchNotes', { patch: notes.patch })}</Button>
  {/if}
</PageHeader>

<section class="changes">
  <div class="changes-head">
    <h3 class="section-title">{t('meta:changes.title')}</h3>
    <SegmentedControl
      options={CHANGE_FILTERS.map((id) => [id, t(`meta:changes.${id}`)] as [(typeof CHANGE_FILTERS)[number], string])}
      bind:value={changeFilter}
    />
  </div>
  {#if patch.error}
    <p class="muted">{app.errorText(patch.error)}</p>
  {:else if !patch.value}
    <Skeleton label={t('meta:changes.loading')} rows={2} />
  {:else if changes.length}
    <div class="change-grid">
      {#each changes as change (change.champion.id)}
        <PatchChange {change} label={t(`meta:changes.verdict.${change.verdict}`)} />
      {/each}
    </div>
  {:else}
    <p class="muted">{t(changeFilter === 'yours' ? 'meta:changes.noneYours' : 'meta:changes.none')}</p>
  {/if}
</section>

<div class="tools">
  <SegmentedControl tabs options={META_MODES.map((id) => [id, t(`meta:modes.${id}`)] as [MetaMode, string])} bind:value={app.metaMode} />
  {#if rift}
    <SegmentedControl options={app.choices.positions.map((p) => [p, t(`build:positions.${p}`)] as [Position, string])} bind:value={position} />
  {/if}
  <SearchInput bind:value={query} placeholder={t('common:searchChampion')} />
</div>

{#if list.error}
  <EmptyState icon={TrendingUp} text={app.errorText(list.error)} />
{:else if !list.value}
  <Skeleton label={t('meta:loading')} rows={10} />
{:else}
  <div
    class="panel cut table"
    role="table"
    aria-label={rift ? t('meta:table', { position: t(`build:positions.${position}`) }) : t(`meta:modes.${app.metaMode}`)}
  >
    <div class="row head" role="row">
      <span role="columnheader">#</span>
      <span role="columnheader">{t('meta:champion')}</span>
      <span role="columnheader">{t('meta:tier')}</span>
      <span role="columnheader">{app.metaMode === 'arena' ? t('meta:topFour') : t('meta:winRate')}</span>
      <span role="columnheader">{t('meta:pickRate')}</span>
      <span role="columnheader">{t('meta:banRate')}</span>
      <span role="columnheader">{t('meta:trend')}</span>
    </div>
    {#each champions as entry (entry.champion.id)}
      <button class="row" role="row" onclick={() => open(entry.champion.id)} title={t(app.metaMode === 'arena' ? 'meta:openAugments' : 'meta:openBuild')}>
        <span class="muted">{entry.rank}</span>
        <span class="champion"
          >{#if entry.champion.icon}<img src={entry.champion.icon} alt="" />{/if}{entry.champion.name}</span
        >
        <span><TierBadge label={championTierLabel(entry.tier)} color={championTierColor(entry.tier)} size="var(--size-thumbSm)" /></span>
        <b>{format.percent(entry.win_rate, RATE_DIGITS)}</b>
        <span>{format.percent(entry.pick_rate, RATE_DIGITS)}</span>
        <span>{format.percent(entry.ban_rate, RATE_DIGITS)}</span>
        <span class="trend" class:up={(entry.trend ?? 0) > 0} class:down={(entry.trend ?? 0) < 0}>
          {#if entry.trend && entry.trend > 0}<ArrowUp size={14} />{format.number(entry.trend)}
          {:else if entry.trend && entry.trend < 0}<ArrowDown size={14} />{format.number(-entry.trend)}
          {:else}—{/if}
        </span>
      </button>
    {:else}
      <p class="muted empty">{t('meta:noMatch')}</p>
    {/each}
  </div>
  <p class="muted note">{t(`meta:sources.${app.metaMode}`)}</p>
{/if}

<style>
  .changes {
    margin-bottom: var(--space-6);
  }
  .changes-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-3);
    margin-bottom: var(--space-3);
  }
  .changes-head h3 {
    margin: 0;
  }
  .change-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(var(--size-aside), 1fr));
    gap: var(--space-2);
    align-items: start;
  }
  .tools {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-3);
    margin-bottom: var(--space-4);
  }
  .table {
    display: grid;
  }
  .row {
    display: grid;
    grid-template-columns: var(--space-8) minmax(0, 2.4fr) repeat(5, minmax(0, 1fr));
    align-items: center;
    gap: var(--space-3);
    width: 100%;
    padding: var(--space-2) var(--space-4);
    border: none;
    background: none;
    font-size: var(--text-md);
    text-align: left;
  }
  .row + .row {
    border-top: var(--border-hairline) solid var(--color-line);
  }
  button.row:hover {
    background: color-mix(in srgb, var(--color-accent) 8%, transparent);
  }
  .head {
    color: var(--color-textMuted);
    font-size: var(--text-xs);
    font-weight: 700;
    letter-spacing: var(--tracking-wide);
    text-transform: uppercase;
  }
  .champion {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    min-width: 0;
  }
  .champion img {
    width: var(--size-thumb);
    height: var(--size-thumb);
    flex: none;
  }
  .trend {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    color: var(--color-textMuted);
  }
  .trend.up {
    color: var(--color-success);
  }
  .trend.down {
    color: var(--color-accentBright);
  }
  .empty,
  .note {
    padding: var(--space-4);
  }
  .note {
    padding-inline: 0;
    font-size: var(--text-sm);
  }
</style>
