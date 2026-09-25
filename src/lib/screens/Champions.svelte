<script lang="ts">
  import { Crown } from '@lucide/svelte';
  import { app } from '../app.svelte';
  import { championTierColor } from '../design/theme';
  import ChampionPortrait from '../ui/ChampionPortrait.svelte';
  import EmptyState from '../ui/EmptyState.svelte';
  import PageHeader from '../ui/PageHeader.svelte';
  import SearchInput from '../ui/SearchInput.svelte';
  import TierBadge from '../ui/TierBadge.svelte';

  const TIERS = [1, 2, 3, 4, 5, null];

  const t = $derived(app.t);
  let query = $state('');
  const filtered = $derived(app.champions.filter((c) => !query.trim() || c.name.toLowerCase().includes(query.trim().toLowerCase())));
  const tiers = $derived(TIERS.map((tier) => ({ tier, champions: filtered.filter((c) => c.tier === tier) })).filter((g) => g.champions.length));
</script>

<PageHeader title={t('common:nav.champions')} subtitle={t('champions:subtitle')}>
  <SearchInput bind:value={query} placeholder={t('champions:search')} />
</PageHeader>

{#if !app.champions.length}
  <EmptyState icon={Crown} text={t('champions:noCatalog')} />
{/if}

{#each tiers as group (group.tier)}
  <section class="tier panel cut" style="--tier-color:{championTierColor(group.tier)}">
    <div class="label">
      <TierBadge label={group.tier ? `T${group.tier}` : '—'} color={championTierColor(group.tier)} size="var(--size-thumbLg)" />
      <small>{app.format.number(group.champions.length)}</small>
    </div>
    <div class="grid">
      {#each group.champions as champion (champion.id)}
        <button class="champion" onclick={() => app.openBuild(champion.id)}>
          <ChampionPortrait {champion} size="var(--size-portrait)" />
          <span>{champion.name}</span>
        </button>
      {/each}
    </div>
  </section>
{/each}

<style>
  .tier {
    display: flex;
    gap: var(--space-5);
    padding: var(--space-4);
    margin-bottom: var(--space-3);
    border-left: var(--border-accent) solid var(--tier-color);
  }
  .label {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-2);
    padding-top: var(--space-2);
    width: var(--size-portraitLg);
    flex: none;
  }
  .label small {
    color: var(--color-textMuted);
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(var(--size-championTile), 1fr));
    gap: var(--space-3);
    flex: 1;
  }
  .champion {
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-1);
    padding: var(--space-2) var(--space-1);
    border: var(--border-hairline) solid transparent;
    background: none;
    transition:
      background 0.15s,
      border-color 0.15s;
  }
  .champion:hover {
    background: color-mix(in srgb, var(--color-accent) 10%, transparent);
    border-color: color-mix(in srgb, var(--color-accent) 40%, transparent);
  }
  .champion > span {
    font-size: var(--text-xs);
    color: var(--color-textMuted);
    max-width: var(--size-championTile);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
