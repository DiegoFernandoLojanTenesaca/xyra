<script lang="ts">
  import { app } from '../app.svelte';
  import { championTierColor } from '../theme';
  import type { ChampionInfo } from '../types';
  import SearchInput from './SearchInput.svelte';
  import TierBadge from './TierBadge.svelte';

  const MAX_SUGGESTIONS = 8;
  const BLUR_DELAY_MS = 150;

  let query = $state('');
  let open = $state(false);

  const suggestions = $derived(
    query.trim() ? app.champions.filter((c) => c.name.toLowerCase().includes(query.trim().toLowerCase())).slice(0, MAX_SUGGESTIONS) : [],
  );

  function pick(champion: ChampionInfo) {
    app.selectedChampion = champion.id;
    query = '';
    open = false;
  }
</script>

<SearchInput bind:value={query} placeholder={app.t('common:searchChampion')} onfocus={() => (open = true)} onblur={() => setTimeout(() => (open = false), BLUR_DELAY_MS)}>
  {#if open && suggestions.length}
    <ul>
      {#each suggestions as champion (champion.id)}
        <li>
          <button onclick={() => pick(champion)}>
            <img src={champion.icon} alt="" />
            <span>{champion.name}</span>
            <TierBadge label={champion.tier ? `T${champion.tier}` : '—'} color={championTierColor(champion.tier)} size="var(--size-thumbSm)" />
          </button>
        </li>
      {/each}
    </ul>
  {/if}
</SearchInput>

<style>
  ul {
    position: absolute;
    top: calc(100% + var(--space-1));
    left: 0;
    right: 0;
    z-index: 5;
    list-style: none;
    margin: 0;
    padding: var(--space-2);
    background: var(--color-panelRaised);
    border: 1px solid var(--color-line);
    box-shadow: 0 var(--space-3) var(--space-8) rgb(0 0 0 / 60%);
  }
  button {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    width: 100%;
    padding: var(--space-2);
    border: none;
    background: none;
    text-align: left;
  }
  button:hover {
    background: color-mix(in srgb, var(--color-accent) 14%, transparent);
  }
  img {
    width: var(--size-thumbSm);
    height: var(--size-thumbSm);
  }
  span {
    flex: 1;
  }
</style>
