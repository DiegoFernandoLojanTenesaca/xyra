<script lang="ts">
  import { ArrowDown, ArrowUp, TrendingUp } from '@lucide/svelte';
  import { championTierColor } from '$shared/design/theme';
  import { resource } from '$shared/services/resource.svelte';
  import type { Meta, Position } from '$shared/types';
  import EmptyState from '$shared/ui/EmptyState.svelte';
  import SegmentedControl from '$shared/ui/SegmentedControl.svelte';
  import Skeleton from '$shared/ui/Skeleton.svelte';
  import TierBadge from '$shared/ui/TierBadge.svelte';
  import { link } from '../link.svelte';
  import { mobile } from '../mobile.svelte';

  const POSITIONS: Position[] = ['top', 'jungle', 'mid', 'adc', 'support'];
  const RATE_DIGITS = 1;

  const t = $derived(mobile.t);
  const format = $derived(mobile.format);
  let position = $state<Position>(link.state?.champ_select?.position ?? 'top');

  const meta = resource(
    () => (link.status === 'online' ? true : null),
    () => link.call<Meta>('GET', '/api/meta'),
    (failure) => failure,
  );
  const champions = $derived(meta.value?.positions.find((p) => p.position === position)?.champions ?? []);
</script>

{#if meta.value}<p class="muted patch">{t('meta:subtitle', { patch: meta.value.patch })}</p>{/if}
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
        <TierBadge label={`T${entry.tier}`} color={championTierColor(entry.tier)} size="var(--size-thumbSm)" />
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
    overflow-x: auto;
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
