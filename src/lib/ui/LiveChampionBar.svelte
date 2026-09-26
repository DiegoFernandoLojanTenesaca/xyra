<script lang="ts">
  import { Radio } from '@lucide/svelte';
  import { app } from '../app.svelte';
  import Button from './Button.svelte';

  const live = $derived(app.liveChampion);
  const following = $derived(live?.id === app.selectedChampion);
</script>

{#if live}
  <div class="live" class:away={!following} role="status">
    <Radio size={14} />
    {#if following}
      <span>{app.t('common:followingGame', { champion: live.name })}</span>
    {:else}
      <span>{app.t('common:viewingOther', { champion: live.name })}</span>
      <Button variant="primary" onclick={app.followLive}>{app.t('common:backToChampion', { champion: live.name })}</Button>
    {/if}
  </div>
{/if}

<style>
  .live {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    margin-bottom: var(--space-3);
    padding: var(--space-2) var(--space-3);
    border: var(--border-hairline) solid var(--color-line);
    background: var(--color-panel);
    color: var(--color-success);
    font-size: var(--text-sm);
    font-weight: 700;
    letter-spacing: var(--tracking-wide);
    text-transform: uppercase;
  }
  .live.away {
    border-color: var(--color-accent);
    color: var(--color-text);
  }
  span {
    flex: 1;
  }
</style>
