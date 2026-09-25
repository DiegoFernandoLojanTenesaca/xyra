<script lang="ts">
  import { Crown } from '@lucide/svelte';
  import { app } from '../app.svelte';
  import EmptyState from '../components/EmptyState.svelte';
  import PageHeader from '../components/PageHeader.svelte';
  import SearchInput from '../components/SearchInput.svelte';
  import TierBadge from '../components/TierBadge.svelte';
  import { championTierColor } from '../theme';

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
      <small>{group.champions.length}</small>
    </div>
    <div class="grid">
      {#each group.champions as champion (champion.id)}
        <button class="champion" onclick={() => app.openBuild(champion.id)} title="{champion.name} #{champion.rank ?? '—'}">
          <img src={champion.icon} alt="" loading="lazy" />
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
    border-left: 3px solid var(--tier-color);
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
    grid-template-columns: repeat(auto-fill, minmax(76px, 1fr));
    gap: var(--space-3);
    flex: 1;
  }
  .champion {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-1);
    padding: var(--space-2) 2px;
    border: 1px solid transparent;
    background: none;
    transition: background 0.15s, border-color 0.15s;
  }
  .champion:hover {
    background: color-mix(in srgb, var(--color-accent) 10%, transparent);
    border-color: color-mix(in srgb, var(--color-accent) 40%, transparent);
  }
  .champion img {
    width: var(--size-portrait);
    height: var(--size-portrait);
  }
  .champion span {
    font-size: var(--text-xs);
    color: var(--color-textMuted);
    max-width: 74px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
