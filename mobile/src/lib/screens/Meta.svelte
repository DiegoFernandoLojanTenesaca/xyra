<script lang="ts">
  import { ArrowDown, ArrowUp, TrendingUp } from '@lucide/svelte';
  import { championTierColor, championTierLabel } from '$shared/design/theme';
  import { resource } from '$shared/services/resource.svelte';
  import type { Meta, PatchChanges, Position } from '$shared/types';
  import EmptyState from '$shared/ui/EmptyState.svelte';
  import PatchChange from '$shared/ui/PatchChange.svelte';
  import SegmentedControl from '$shared/ui/SegmentedControl.svelte';
  import Skeleton from '$shared/ui/Skeleton.svelte';
  import TierBadge from '$shared/ui/TierBadge.svelte';
  import { link } from '../link.svelte';
  import { mobile } from '../mobile.svelte';

  const POSITIONS: Position[] = ['top', 'jungle', 'mid', 'adc', 'support'];
  const RATE_DIGITS = 1;
  const CHANGE_FILTERS = ['yours', 'all'] as const;

  const t = $derived(mobile.t);
  const format = $derived(mobile.format);
  let position = $state<Position>(link.state?.champ_select?.position ?? 'top');

  const meta = resource(
    () => (link.status === 'online' ? true : null),
    () => link.call<Meta>('GET', '/api/meta'),
    (failure) => failure,
  );
  const champions = $derived(meta.value?.positions.find((p) => p.position === position)?.champions ?? []);
  let changeFilter = $state<(typeof CHANGE_FILTERS)[number]>('yours');
  const patch = resource(
    () => (link.status === 'online' ? (link.pc?.language ?? '') : null),
    () => link.call<PatchChanges>('GET', '/api/patch'),
    (failure) => failure,
  );
  const changes = $derived((patch.value?.champions ?? []).filter((change) => changeFilter === 'all' || change.yours));
</script>

{#if meta.value}<p class="muted patch">{t('meta:subtitle', { patch: meta.value.patch })}</p>{/if}

<section class="changes">
  <h2 class="section-title">{t('meta:changes.title')}</h2>
  <SegmentedControl
    options={CHANGE_FILTERS.map((id) => [id, t(`meta:changes.${id}`)] as [(typeof CHANGE_FILTERS)[number], string])}
    bind:value={changeFilter}
  />
  {#if patch.error}
    <p class="muted">{mobile.errorText(patch.error)}</p>
  {:else if !patch.value}
    <Skeleton label={t('meta:changes.loading')} rows={2} />
  {:else if changes.length}
    <div class="change-list">
      {#each changes as change (change.champion.id)}
        <PatchChange {change} label={t(`meta:changes.verdict.${change.verdict}`)} />
      {/each}
    </div>
  {:else}
    <p class="muted">{t(changeFilter === 'yours' ? 'meta:changes.noneYours' : 'meta:changes.none')}</p>
  {/if}
</section>
<div class="tools">
  <SegmentedControl options={POSITIONS.map((p) => [p, t(`build:positions.${p}`)] as [Position, string])} bind:value={position} />
</div>

{#if meta.error}
  <EmptyState icon={TrendingUp} text={mobile.errorText(meta.error)} />
{:else if !meta.value}
  <Skeleton label={t('meta:loading')} rows={10} />
{:else}
  <section class="panel cut list">
    {#each champions as entry (entry.champion.id)}
      <button class="row entry" onclick={() => mobile.openBuild(entry.champion.id, 'rift', position)}>
        <span class="rank muted">{entry.rank}</span>
        {#if entry.champion.icon}<img src={entry.champion.icon} alt="" />{/if}
        <span class="grow">
          <b>{entry.champion.name}</b>
          <small class="muted">{format.percent(entry.win_rate, RATE_DIGITS)} · {t('meta:banRate')} {format.percent(entry.ban_rate, RATE_DIGITS)}</small>
        </span>
        <span class="trend" class:good={(entry.trend ?? 0) > 0} class:accent={(entry.trend ?? 0) < 0}>
          {#if entry.trend && entry.trend > 0}<ArrowUp size={12} />{entry.trend}{:else if entry.trend && entry.trend < 0}<ArrowDown
              size={12}
            />{-entry.trend}{/if}
        </span>
        <TierBadge label={championTierLabel(entry.tier)} color={championTierColor(entry.tier)} size="var(--size-thumbSm)" />
      </button>
    {/each}
  </section>
{/if}

<style>
  .patch {
    margin: 0 0 var(--space-3);
    font-size: var(--text-md);
  }
  .tools {
    margin-bottom: var(--space-4);
  }
  .changes {
    display: grid;
    gap: var(--space-3);
    margin-bottom: var(--space-4);
  }
  .changes .section-title {
    margin: 0;
  }
  .change-list {
    display: grid;
    gap: var(--space-2);
  }
  .list {
    padding: var(--space-2) var(--space-4);
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
  .grow {
    display: grid;
  }
  .grow small {
    font-size: var(--text-xs);
  }
  .trend {
    display: inline-flex;
    align-items: center;
    font-size: var(--text-sm);
    color: var(--color-textMuted);
  }
</style>
