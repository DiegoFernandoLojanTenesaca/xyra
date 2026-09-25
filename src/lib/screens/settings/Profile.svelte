<script lang="ts">
  import { House } from '@lucide/svelte';
  import { app, percent } from '../../app.svelte';
  import EmptyState from '../../ui/EmptyState.svelte';
  import StatTile from '../../ui/StatTile.svelte';

  const t = $derived(app.t);
  const format = $derived(app.format);
  const stats = $derived(app.stats);
  const profile = $derived(app.profile);
</script>

{#if profile}
  <section class="card highlight cut">
    <div class="avatar">
      <img class="diamond" src={profile.icon} alt="" />
      <span class="level">{format.number(profile.level)}</span>
    </div>
    <div>
      <h2 class="condensed">{profile.name} <small>#{profile.tag}</small></h2>
      <div class="chips">
        {#if profile.region}<span class="chip accent-chip">{profile.region}</span>{/if}
        {#if profile.rank}
          <span class="chip rank">
            <img src={profile.rank.crest} alt="" />
            {t('settings:rankValue', { tier: t(`common:leagues.${profile.rank.tier}`), division: profile.rank.division, lp: format.number(profile.rank.lp) })}
          </span>
        {:else}
          <span class="chip">{t('settings:unranked')}</span>
        {/if}
        <span class="chip">{app.state.client_locale.replace('_', '-')}</span>
      </div>
    </div>
  </section>

  <div class="tiles">
    <StatTile label={t('stats:games')} value={stats?.games ?? 0} format={format.number} />
    <StatTile label={t('stats:winRate')} value={stats?.games ? percent(stats.wins, stats.games) : null} format={format.percent} accent />
    <StatTile label={t('settings:championsPlayed')} value={stats?.champions.length ?? 0} format={format.number} />
  </div>

  {#if profile.masteries.length}
    <h3 class="section-title">{t('settings:mastery')}</h3>
    <div class="panel cut masteries">
      {#each profile.masteries as mastery (mastery.id)}
        <button onclick={() => app.openBuild(mastery.id)}>
          <img class="cut" src={mastery.icon} alt="" />
          <b>{mastery.name}</b>
          <small class="muted">{t('settings:masteryValue', { points: format.compact(mastery.points), level: mastery.level })}</small>
        </button>
      {/each}
    </div>
  {/if}
{:else}
  <div class="panel cut"><EmptyState icon={House} text={t('settings:noProfile')} /></div>
{/if}

<style>
  .card {
    display: flex;
    align-items: center;
    gap: var(--space-6);
    padding: var(--space-6);
    margin-bottom: var(--space-5);
  }
  h2 {
    margin: 0;
    font-size: var(--text-3xl);
    font-variation-settings: 'wdth' 78;
    text-transform: none;
  }
  h2 small {
    font-size: var(--text-xl);
    color: var(--color-textMuted);
    font-weight: 400;
  }
  .avatar {
    position: relative;
    flex: none;
  }
  .avatar img {
    width: var(--size-avatar);
    height: var(--size-avatar);
    filter: drop-shadow(0 0 var(--space-3) color-mix(in srgb, var(--color-accent) 60%, transparent));
  }
  .level {
    position: absolute;
    bottom: calc(-1 * var(--space-2));
    left: 50%;
    transform: translateX(-50%);
    padding: var(--space-1) var(--space-3);
    font-size: var(--text-sm);
    font-weight: 800;
    background: var(--color-chrome);
    border: var(--border-hairline) solid var(--color-accentBright);
  }
  .chips {
    display: flex;
    gap: var(--space-2);
    margin-top: var(--space-3);
  }
  .chip {
    padding: var(--space-1) var(--space-3);
    font-size: var(--text-xs);
    letter-spacing: var(--tracking-relaxed);
    font-weight: 700;
    text-transform: uppercase;
    background: color-mix(in srgb, var(--color-white) 6%, transparent);
    border: var(--border-hairline) solid var(--color-line);
  }
  .accent-chip {
    border-color: color-mix(in srgb, var(--color-accent) 50%, transparent);
    color: var(--color-accentSoft);
  }
  .rank {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    padding-left: var(--space-2);
  }
  .rank img {
    width: var(--space-5);
    height: var(--space-5);
  }
  .tiles {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: var(--space-4);
    margin-bottom: var(--space-6);
  }
  .masteries {
    display: grid;
    grid-template-columns: repeat(5, minmax(0, 1fr));
    padding: var(--space-4);
  }
  .masteries button {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-1);
    padding: var(--space-2);
    border: none;
    background: none;
  }
  .masteries button:hover {
    background: color-mix(in srgb, var(--color-accent) 8%, transparent);
  }
  .masteries img {
    width: var(--size-portrait);
    height: var(--size-portrait);
    margin-bottom: var(--space-1);
  }
  .masteries small {
    font-size: var(--text-xs);
  }
</style>
