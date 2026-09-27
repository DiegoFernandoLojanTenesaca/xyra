<script lang="ts">
  import { BellRing, Download, Monitor, RefreshCw, Smartphone, Unlink } from '@lucide/svelte';
  import type { ChampionOrder, PhoneSettings } from '$shared/types';
  import SegmentedControl from '$shared/ui/SegmentedControl.svelte';
  import ToggleRow from '$shared/ui/ToggleRow.svelte';
  import { android } from '../android.svelte';
  import { link } from '../link.svelte';
  import { mobile } from '../mobile.svelte';

  const ORDERS: ChampionOrder[] = ['tier', 'mastery', 'played', 'balanced'];
  const TOGGLES = ['auto_accept', 'auto_import_build', 'paused'] as const;

  const t = $derived(mobile.t);
  let settings = $state<PhoneSettings | null>(null);
  let notice = $state('');

  $effect(() => {
    if (link.status !== 'online') return;
    link.get<PhoneSettings>('/api/settings').then(
      (value) => (settings = value),
      (error) => (notice = mobile.errorText(error)),
    );
  });

  async function change(values: Record<string, string>) {
    notice = '';
    try {
      settings = await link.call<PhoneSettings>('POST', '/api/settings', values);
    } catch (error) {
      notice = mobile.errorText(error);
    }
  }
</script>

<section class="block panel cut">
  <h2 class="section-title">{t('mobile:settings.pc')}</h2>
  <div class="row">
    <Monitor size={20} />
    <span class="grow"><b>{link.pc?.name ?? '—'}</b></span>
    <small class="muted">{link.pc ? `Xyra ${link.pc.version}` : ''}</small>
  </div>
</section>

{#if android.available}
  <section class="block panel cut">
    <h2 class="section-title">{t('mobile:phone.title')}</h2>
    <ToggleRow
      title={t('mobile:phone.watch.title')}
      description={t('mobile:phone.watch.description')}
      checked={android.watching}
      onchange={() => android.setWatching(!android.watching)}
    />
    {#if android.watching && link.pairing}
      <button class="action wide test" onclick={android.testWatch}><BellRing size={14} />{t('mobile:phone.test')}</button>
    {/if}
    <div class="row app">
      <Smartphone size={20} />
      <span class="grow">
        <b>{t('mobile:phone.version', { version: android.version })}</b>
        <small class:accent={android.release} class="muted">
          {#if android.failed}{t('mobile:update.failed')}
          {:else if android.progress !== null}{t('mobile:update.downloading', { value: mobile.format.percent(android.progress) })}
          {:else if android.release}{t('mobile:update.available', { version: android.release.version })}
          {:else if android.checking}{t('mobile:update.checking')}
          {:else}{t('mobile:update.upToDate')}{/if}
        </small>
      </span>
    </div>
    {#if android.release}
      <button class="action primary wide" disabled={android.progress !== null} onclick={android.install}
        ><Download size={14} />{t('mobile:update.install')}</button
      >
      <p class="muted hint">{t('mobile:update.hint')}</p>
    {:else}
      <button class="action wide" disabled={android.checking} onclick={android.check}><RefreshCw size={14} />{t('mobile:update.check')}</button>
    {/if}
  </section>
{/if}

{#if settings}
  {@const current = settings}
  <section class="block panel cut">
    <h2 class="section-title">{t('mobile:settings.title')}</h2>
    {#if !link.pc?.permissions.settings}<p class="muted">{t('mobile:settings.readOnly')}</p>{/if}
    {#each TOGGLES as key (key)}
      <ToggleRow
        title={t(`mobile:settings.${key}.title`)}
        description={t(`mobile:settings.${key}.description`)}
        checked={current[key]}
        onchange={() => change({ [key]: String(!current[key]) })}
      />
    {/each}
    <div class="order">
      <b>{t('mobile:settings.order')}</b>
      <SegmentedControl
        options={ORDERS.map((order) => [order, t(`home:order.${order}`)] as [ChampionOrder, string])}
        bind:value={() => current.champion_order, (order) => change({ champion_order: order })}
      />
    </div>
  </section>
{/if}
{#if notice}<p class="notice">{notice}</p>{/if}

<button class="action wide" onclick={link.forget}><Unlink size={14} />{t('mobile:settings.forget')}</button>
<p class="muted hint">{t('mobile:settings.forgetHint')}</p>

<style>
  .order {
    display: grid;
    gap: var(--space-2);
    padding-top: var(--space-3);
    font-size: var(--text-md);
  }
  .wide {
    width: 100%;
  }
  .hint {
    font-size: var(--text-sm);
  }
  .app {
    margin-bottom: var(--space-3);
  }
  .test {
    margin: var(--space-2) 0 var(--space-4);
  }
  .app .grow {
    display: grid;
  }
  .app small {
    font-size: var(--text-sm);
  }
</style>
