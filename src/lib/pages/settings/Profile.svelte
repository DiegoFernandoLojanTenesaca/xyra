<script lang="ts">
  import { House } from '@lucide/svelte';
  import { app, percent } from '../../app.svelte';
  import EmptyState from '../../components/EmptyState.svelte';
  import StatTile from '../../components/StatTile.svelte';

  const t = $derived(app.t);
  const stats = $derived(app.stats);
  const profile = $derived(app.profile);
  const compact = $derived(new Intl.NumberFormat(app.language, { notation: 'compact' }));
</script>

{#if profile}
  <section class="card highlight cut">
    <div class="avatar">
      <img class="diamond" src={profile.icon} alt="" />
      <span class="level">{profile.level}</span>
    </div>
    <div>
      <h2 class="condensed">{profile.name} <small>#{profile.tag}</small></h2>
      <div class="chips">
        {#if profile.region}<span class="chip accent-chip">{profile.region}</span>{/if}
        {#if profile.rank}
          <span class="chip rank">
            <img src={profile.rank.crest} alt="" />
            {t(`common:leagues.${profile.rank.tier}`)} {profile.rank.division} · {profile.rank.lp} LP
          </span>
        {:else}
          <span class="chip">{t('settings:unranked')}</span>
        {/if}
        <span class="chip">{app.state.client_locale.replace('_', '-')}</span>
      </div>
    </div>
  </section>
  <div class="tiles">
    <StatTile label={t('stats:games')} value={stats?.games ?? 0} />
    <StatTile label={t('stats:winRate')} value={stats?.games ? `${percent(stats.wins, stats.games)}%` : '—'} accent />
    <StatTile label={t('settings:championsPlayed')} value={stats?.champions.length ?? 0} />
  </div>
  {#if profile.masteries.length}
    <h3 class="section-title">{t('settings:mastery')}</h3>
    <div class="panel cut masteries">
      {#each profile.masteries as mastery (mastery.id)}
        <button onclick={() => app.openBuild(mastery.id)}>
          <img class="cut" src={mastery.icon} alt="" />
          <b>{mastery.name}</b>
          <small class="muted">{compact.format(mastery.points)} · M{mastery.level}</small>
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
    padding: 2px var(--space-3);
    font-size: var(--text-sm);
    font-weight: 800;
    background: var(--color-chrome);
    border: 1px solid var(--color-accentBright);
  }
  .chips {
    display: flex;
    gap: var(--space-2);
    margin-top: var(--space-3);
  }
  .chip {
    padding: var(--space-1) var(--space-3);
    font-size: var(--text-xs);
    letter-spacing: 1.5px;
    font-weight: 700;
    text-transform: uppercase;
    background: color-mix(in srgb, var(--color-white) 6%, transparent);
    border: 1px solid var(--color-line);
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
