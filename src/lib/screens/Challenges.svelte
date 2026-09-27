<script lang="ts">
  import { Trophy } from '@lucide/svelte';
  import { app } from '../app.svelte';
  import { getChallenges } from '../services/league';
  import { resource } from '../services/resource.svelte';
  import EmptyState from '../ui/EmptyState.svelte';
  import PageHeader from '../ui/PageHeader.svelte';
  import Skeleton from '../ui/Skeleton.svelte';
  import StatTile from '../ui/StatTile.svelte';

  /** The level Riot gives a challenge nobody has started. */
  const NO_LEVEL = 'NONE';
  const TOP_DIGITS = 1;

  const t = $derived(app.t);
  const format = $derived(app.format);
  const challenges = resource(() => app.state.account, getChallenges);
  const levelName = (level: string) => (level === NO_LEVEL || !level ? t('challenges:noLevel') : t(`common:leagues.${level.toLowerCase()}`));
</script>

<PageHeader title={t('common:nav.challenges')} subtitle={t('challenges:subtitle')} />

{#if challenges.error}
  <EmptyState icon={Trophy} text={app.errorText(challenges.error)} />
{:else if !challenges.value}
  <Skeleton label={t('challenges:loading')} rows={8} />
{:else}
  {@const summary = challenges.value}
  <p class="overall condensed">{t('challenges:overall', { level: levelName(summary.level) })}</p>
  <div class="tiles">
    <StatTile label={t('challenges:points')} value={summary.points} format={format.number} accent />
    <StatTile label={t('challenges:toNext')} value={summary.points_to_next} format={format.number} />
    <StatTile label={t('challenges:top')} value={summary.top_percent} format={(value) => format.percent(value, TOP_DIGITS)} />
  </div>

  <h3 class="section-title">{t('challenges:categories')}</h3>
  <div class="categories">
    {#each summary.categories as category (category.category)}
      <div class="panel cut category">
        <small class="muted">{t(`challenges:category.${category.category}`, { defaultValue: category.category })}</small>
        <b>{levelName(category.level)}</b>
        <span class="bar"><i style="width:{(100 * category.points) / Math.max(1, category.max)}%"></i></span>
        <small class="muted"
          >{format.number(category.points)} / {format.number(category.max)} · {t('challenges:topValue', {
            value: format.percent(category.top_percent, TOP_DIGITS),
          })}</small
        >
      </div>
    {/each}
  </div>

  <h3 class="section-title">{t('challenges:closest')}</h3>
  <div class="closest">
    {#each summary.closest as challenge (challenge.id)}
      <div class="panel cut challenge">
        {#if challenge.icon}<img src={challenge.icon} alt="" />{/if}
        <span class="info">
          <span class="line"><b>{challenge.name}</b><small class="next">{t('challenges:nextLevel', { level: levelName(challenge.next_level) })}</small></span>
          <small class="muted">{challenge.description}</small>
          <span class="bar"><i style="width:{challenge.progress}%"></i></span>
          <small class="muted"
            >{format.number(challenge.value)} / {format.number(challenge.next_value)} · {t(`challenges:category.${challenge.category}`, {
              defaultValue: challenge.category,
            })}</small
          >
        </span>
      </div>
    {:else}
      <p class="muted">{t('challenges:noneClose')}</p>
    {/each}
  </div>
  <p class="muted note">{t('challenges:source')}</p>
{/if}

<style>
  .tiles {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: var(--space-4);
    margin-bottom: var(--space-6);
  }
  .overall {
    margin: 0 0 var(--space-4);
    color: var(--color-accentBright);
    font-size: var(--text-xl);
    text-transform: uppercase;
  }
  .categories {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(var(--size-slider), 1fr));
    gap: var(--space-3);
    margin-bottom: var(--space-6);
  }
  .category {
    display: grid;
    gap: var(--space-1);
    padding: var(--space-4);
  }
  .category b {
    font-size: var(--text-lg);
  }
  .closest {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(var(--size-aside), 1fr));
    gap: var(--space-2);
    margin-bottom: var(--space-4);
  }
  .challenge {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-3) var(--space-4);
  }
  .challenge img {
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
  .next {
    flex: none;
    color: var(--color-accentBright);
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
  .note {
    font-size: var(--text-sm);
  }
</style>
