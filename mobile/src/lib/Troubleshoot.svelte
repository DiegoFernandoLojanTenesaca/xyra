<script lang="ts">
  import { Check, RefreshCw, X } from '@lucide/svelte';
  import { android } from './android.svelte';
  import { mobile } from './mobile.svelte';

  /** Home networks are /24: two addresses share one when their first three numbers match. */
  const subnet = (address: string) => address.split('.').slice(0, 3).join('.');

  let { hosts, onretry }: { hosts: string[]; onretry: () => void } = $props();

  const t = $derived(mobile.t);
  let network = $state(android.network());
  const sameNetwork = $derived(!!network?.address && hosts.some((host) => subnet(host) === subnet(network!.address)));
  /** What most likely keeps the phone from the PC. */
  const cause = $derived(!network ? 'unknown' : !network.wifi ? 'noWifi' : sameNetwork ? 'waiting' : 'otherNetwork');

  function retry() {
    network = android.network();
    onretry();
  }
</script>

{#snippet check(ok: boolean, text: string)}
  <li class:good={ok} class:bad={!ok}>
    {#if ok}<Check size={16} />{:else}<X size={16} />{/if}{text}
  </li>
{/snippet}

<section class="block panel cut">
  <h2 class="section-title">{t('mobile:help.title')}</h2>
  {#if network}
    <ul>
      {@render check(network.wifi, t(network.wifi ? 'mobile:help.wifi' : 'mobile:help.noWifi'))}
      {#if network.wifi}{@render check(sameNetwork, t(sameNetwork ? 'mobile:help.sameNetwork' : 'mobile:help.otherNetwork'))}{/if}
    </ul>
  {/if}
  <p class="cause">{t(`mobile:help.cause.${cause}`)}</p>
  <p class="muted small">{t('mobile:help.open')}</p>
  <button class="action wide" onclick={retry}><RefreshCw size={14} />{t('mobile:help.retry')}</button>
</section>

<style>
  section {
    width: 100%;
    text-align: left;
  }
  ul {
    display: grid;
    gap: var(--space-2);
    margin: 0 0 var(--space-3);
    padding: 0;
    list-style: none;
    font-size: var(--text-md);
  }
  li {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }
  .good {
    color: var(--color-success);
  }
  .bad {
    color: var(--color-accentBright);
  }
  .cause {
    margin: 0 0 var(--space-2);
    font-size: var(--text-md);
    line-height: 1.45;
  }
  .small {
    margin: 0 0 var(--space-4);
    font-size: var(--text-sm);
  }
  .wide {
    width: 100%;
  }
</style>
