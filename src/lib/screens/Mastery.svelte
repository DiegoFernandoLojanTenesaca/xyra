<script lang="ts">
  import { Medal } from '@lucide/svelte';
  import { app } from '../app.svelte';
  import { getMasteries } from '../services/league';
  import { resource } from '../services/resource.svelte';
  import type { MasteryProgress } from '../types';
  import EmptyState from '../ui/EmptyState.svelte';
  import PageHeader from '../ui/PageHeader.svelte';
  import SearchInput from '../ui/SearchInput.svelte';
  import SegmentedControl from '../ui/SegmentedControl.svelte';
  import Skeleton from '../ui/Skeleton.svelte';
  import StatTile from '../ui/StatTile.svelte';

  const SORTS = ['points', 'next', 'recent'] as const;
  type Sort = (typeof SORTS)[number];
  /** Level from which a champion shows off its mastery crest. */
  const HIGH_LEVEL = 10;
  const ORDER: Record<Sort, (a: MasteryProgress, b: MasteryProgress) => number> = {
    points: (a, b) => b.points - a.points,
    next: (a, b) => a.until_next - b.until_next,
    recent: (a, b) => b.last_played - a.last_played,
  };

  const t = $derived(app.t);
  const format = $derived(app.format);
  let sort = $state<Sort>('points');
  let query = $state('');

  const masteries = resource(() => app.state.account, getMasteries);
  const all = $derived(masteries.value ?? []);
  const shown = $derived(
    all
      .filter((m) => !query.trim() || m.champion.name.toLowerCase().includes(query.trim().toLowerCase()))
      .filter((m) => sort !== 'next' || m.until_next > 0)
      .sort(ORDER[sort]),
  );
  const done = (m: MasteryProgress) => (100 * m.since_level) / Math.max(1, m.since_level + m.until_next);
</script>

<PageHeader title={t('common:nav.mastery')} subtitle={t('mastery:subtitle')} />

{#if masteries.error}
  <EmptyState icon={Medal} text={app.errorText(masteries.error)} />
{:else if !masteries.value}
  <Skeleton label={t('mastery:loading')} rows={8} />
{:else}
  <div class="tiles">
    <StatTile label={t('mastery:champions')} value={all.length} format={format.number} />
    <StatTile label={t('mastery:points')} value={all.reduce((total, m) => total + m.points, 0)} format={format.compact} accent />
    <StatTile label={t('mastery:highLevel', { level: HIGH_LEVEL })} value={all.filter((m) => m.level >= HIGH_LEVEL).length} format={format.number} />
    <StatTile label={t('mastery:marks')} value={all.reduce((total, m) => total + m.marks, 0)} format={format.number} />
  </div>

  <div class="tools">
    <SegmentedControl options={SORTS.map((id) => [id, t(`mastery:sort.${id}`)] as [Sort, string])} bind:value={sort} />
    <SearchInput bind:value={query} placeholder={t('common:searchChampion')} />
  </div>

  <div class="cards">
    {#each shown as mastery (mastery.champion.id)}
      <button class="card panel cut" onclick={() => app.openBuild(mastery.champion.id)} title={t('mastery:openBuild')}>
        {#if mastery.champion.icon}<img src={mastery.champion.icon} alt="" />{/if}
        <span class="info">
          <span class="line"><b>{mastery.champion.name}</b><span class="level">{t('mastery:level', { level: mastery.level })}</span></span>
          <small class="muted">{t('mastery:pointsValue', { points: format.number(mastery.points) })}</small>
          <span class="bar"><i style="width:{done(mastery)}%"></i></span>
          <small class="muted">
            {#if mastery.until_next}{t('mastery:untilNext', { points: format.number(mastery.until_next) })}{/if}
            {#if mastery.marks_needed}·
              {mastery.marks >= mastery.marks_needed
                ? t('mastery:marksReady')
                : t('mastery:marksValue', { marks: mastery.marks, needed: mastery.marks_needed })}{/if}
            {#if mastery.best_grade}· {t('mastery:bestGrade', { grade: mastery.best_grade })}{/if}
          </small>
        </span>
      </button>
    {:else}
      <p class="muted">{t('mastery:none')}</p>
    {/each}
  </div>
{/if}

<style>
  .tiles {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: var(--space-4);
    margin-bottom: var(--space-6);
  }
  .tools {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-3);
    margin-bottom: var(--space-4);
  }
  .cards {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(var(--size-aside), 1fr));
    gap: var(--space-2);
  }
  .card {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-3) var(--space-4);
    text-align: left;
  }
  .card:hover {
    border-color: color-mix(in srgb, var(--color-accent) 45%, transparent);
  }
  .card img {
    width: var(--size-thumbLg);
    height: var(--size-thumbLg);
    flex: none;
  }
  .info {
    display: grid;
    gap: var(--space-1);
    flex: 1;
    min-width: 0;
  }
  .line {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-2);
  }
  .level {
    color: var(--color-accentBright);
    font-size: var(--text-sm);
    font-weight: 700;
  }
  small {
    font-size: var(--text-sm);
  }
  .bar {
    display: block;
    height: var(--space-1);
    background: var(--color-line);
  }
  .bar i {
    display: block;
    height: 100%;
    background: var(--color-accent);
  }
</style>
