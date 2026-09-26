<script lang="ts">
  import { ArrowDown, ArrowUp, ExternalLink, TrendingUp } from '@lucide/svelte';
  import { untrack } from 'svelte';
  import { app } from '../app.svelte';
  import { championTierColor } from '../design/theme';
  import { openExternal } from '../project';
  import { getMeta } from '../services/league';
  import { resource } from '../services/resource.svelte';
  import type { Position } from '../types';
  import Button from '../ui/Button.svelte';
  import EmptyState from '../ui/EmptyState.svelte';
  import PageHeader from '../ui/PageHeader.svelte';
  import SearchInput from '../ui/SearchInput.svelte';
  import SegmentedControl from '../ui/SegmentedControl.svelte';
  import Skeleton from '../ui/Skeleton.svelte';
  import TierBadge from '../ui/TierBadge.svelte';

  /** League site locale for each language of the app. */
  const SITE_LOCALES: Record<string, string> = { es: 'es-mx', en: 'en-us' };
  const DEFAULT_SITE_LOCALE = 'en-us';
  const RATE_DIGITS = 1;

  const t = $derived(app.t);
  const format = $derived(app.format);
  let position = $state<Position>(untrack(() => app.state.champ_select?.position ?? app.choices.positions[0]));
  let query = $state('');

  const meta = resource(() => true, getMeta);
  const notesUrl = $derived(
    meta.value &&
      `https://www.leagueoflegends.com/${SITE_LOCALES[app.language] ?? DEFAULT_SITE_LOCALE}/news/game-updates/league-of-legends-patch-${meta.value.patch.replace('.', '-')}-notes/`,
  );
  const champions = $derived(
    (meta.value?.positions.find((p) => p.position === position)?.champions ?? []).filter(
      (c) => !query.trim() || c.champion.name.toLowerCase().includes(query.trim().toLowerCase()),
    ),
  );
</script>

<PageHeader title={t('common:nav.meta')} subtitle={meta.value ? t('meta:subtitle', { patch: meta.value.patch }) : t('meta:loading')}>
  {#if notesUrl}
    <Button icon={ExternalLink} onclick={() => openExternal(notesUrl)}>{t('meta:patchNotes', { patch: meta.value?.patch })}</Button>
  {/if}
</PageHeader>

<div class="tools">
  <SegmentedControl options={app.choices.positions.map((p) => [p, t(`build:positions.${p}`)] as [Position, string])} bind:value={position} />
  <SearchInput bind:value={query} placeholder={t('common:searchChampion')} />
</div>

{#if meta.error}
  <EmptyState icon={TrendingUp} text={app.errorText(meta.error)} />
{:else if !meta.value}
  <Skeleton label={t('meta:loading')} rows={10} />
{:else}
  <div class="panel cut table" role="table" aria-label={t('meta:table', { position: t(`build:positions.${position}`) })}>
    <div class="row head" role="row">
      <span role="columnheader">#</span>
      <span role="columnheader">{t('meta:champion')}</span>
      <span role="columnheader">{t('meta:tier')}</span>
      <span role="columnheader">{t('meta:winRate')}</span>
      <span role="columnheader">{t('meta:pickRate')}</span>
      <span role="columnheader">{t('meta:banRate')}</span>
      <span role="columnheader">{t('meta:trend')}</span>
    </div>
    {#each champions as entry (entry.champion.id)}
      <button class="row" role="row" onclick={() => app.openRiftBuild(entry.champion.id, position)} title={t('meta:openBuild')}>
        <span class="muted">{entry.rank}</span>
        <span class="champion"
          >{#if entry.champion.icon}<img src={entry.champion.icon} alt="" />{/if}{entry.champion.name}</span
        >
        <span><TierBadge label={`T${entry.tier}`} color={championTierColor(entry.tier)} size="var(--size-thumbSm)" /></span>
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
  <p class="muted note">{t('meta:source')}</p>
{/if}

<style>
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
