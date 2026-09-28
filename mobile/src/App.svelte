<script lang="ts">
  import { ChartColumn, Download, Hammer, Monitor, Radio, RefreshCw, Settings as SettingsIcon, TrendingUp } from '@lucide/svelte';
  import { onMount, type Component } from 'svelte';
  import Logo from '$shared/ui/Logo.svelte';
  import { android, isNewer, type WatchTexts } from './lib/android.svelte';
  import { link } from './lib/link.svelte';
  import { mobile, TABS, type Tab } from './lib/mobile.svelte';
  import Build from './lib/screens/Build.svelte';
  import Live from './lib/screens/Live.svelte';
  import Meta from './lib/screens/Meta.svelte';
  import Pairing from './lib/screens/Pairing.svelte';
  import Settings from './lib/screens/Settings.svelte';
  import Stats from './lib/screens/Stats.svelte';
  import MatchFound from './lib/MatchFound.svelte';
  import Troubleshoot from './lib/Troubleshoot.svelte';
  import UpdateDialog from './lib/UpdateDialog.svelte';

  const SCREENS: Record<Tab, Component> = { live: Live, build: Build, meta: Meta, stats: Stats, settings: Settings };
  const ICONS: Record<Tab, Component<{ size?: number }>> = { live: Radio, build: Hammer, meta: TrendingUp, stats: ChartColumn, settings: SettingsIcon };
  const WATCH_TEXTS = [
    'watchingChannel',
    'matchChannel',
    'watching',
    'watchingText',
    'matchFound',
    'matchText',
    'accept',
    'accepted',
    'acceptFailed',
    'decline',
    'declined',
    'champSelect',
    'champSelectText',
    'testDone',
  ] as const;

  const t = $derived(mobile.t);
  const Screen = $derived(SCREENS[mobile.tab]);
  /** The PC's Xyra, when it is older than this app. */
  const pcBehind = $derived(link.state && android.version && isNewer(android.version, link.state.version) ? link.state.version : null);

  onMount(() => {
    link.connect();
    android.check();
  });

  $effect(() => {
    const texts = Object.fromEntries(WATCH_TEXTS.map((key) => [key, t(`mobile:notify.${key}`)])) as WatchTexts;
    android.follow(link.status === 'codeChanged' ? null : link.pairing, texts);
  });
</script>

{#if !link.pairing || link.status === 'codeChanged'}
  <Pairing />
{:else}
  <div class="shell">
    <header>
      <Logo size={28} phase={link.state?.phase ?? 'noClient'} />
      <b class="brand">XYRA</b>
      <span class="pc muted">{link.pc?.name ?? ''}</span>
      <span class="status {link.status}">{t(`mobile:status.${link.status}`)}</span>
    </header>

    {#if link.status === 'offline'}
      <button class="banner" onclick={link.connect}><RefreshCw size={14} />{t('mobile:offline')}</button>
    {:else if android.release && mobile.tab !== 'settings'}
      <button class="banner" onclick={() => (mobile.tab = 'settings')}
        ><Download size={14} />{t('mobile:update.banner', { version: android.release.version })}</button
      >
    {:else if pcBehind}
      <p class="banner"><Monitor size={14} />{t('mobile:update.pcBehind', { pc: pcBehind, app: android.version })}</p>
    {/if}

    <main>
      {#if link.status === 'offline'}
        <Troubleshoot hosts={link.pairing.hosts} onretry={link.connect} />
      {:else}
        {#key mobile.tab}<Screen />{/key}
      {/if}
    </main>

    <MatchFound />
    <UpdateDialog />

    <nav>
      {#each TABS as tab (tab)}
        {@const Icon = ICONS[tab]}
        <button class:active={mobile.tab === tab} onclick={() => (mobile.tab = tab)}>
          <Icon size={20} /><span>{t(`mobile:tabs.${tab}`)}</span>
        </button>
      {/each}
    </nav>
  </div>
{/if}

<style>
  .shell {
    display: flex;
    flex-direction: column;
    height: 100vh;
    height: 100dvh;
  }
  header {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: calc(env(safe-area-inset-top) + var(--space-3)) var(--space-4) var(--space-3);
    border-bottom: var(--border-hairline) solid var(--color-line);
    background: var(--color-chrome);
  }
  .brand {
    letter-spacing: var(--tracking-widest);
  }
  .pc {
    flex: 1;
    overflow: hidden;
    font-size: var(--text-sm);
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .status {
    padding: var(--space-1) var(--space-2);
    border: var(--border-hairline) solid var(--color-line);
    color: var(--color-textMuted);
    font-size: var(--text-xs);
    font-weight: 700;
    letter-spacing: var(--tracking-wide);
    text-transform: uppercase;
  }
  .status.online {
    border-color: var(--color-success);
    color: var(--color-success);
  }
  .status.offline {
    border-color: var(--color-accent);
    color: var(--color-accentBright);
  }
  .banner {
    display: flex;
    margin: 0;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-3) var(--space-4);
    border: none;
    background: color-mix(in srgb, var(--color-accent) 20%, var(--color-panel));
    font-size: var(--text-md);
    text-align: left;
  }
  main {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: var(--space-4);
    user-select: text;
  }
  nav {
    display: grid;
    grid-template-columns: repeat(5, 1fr);
    padding-bottom: env(safe-area-inset-bottom);
    border-top: var(--border-hairline) solid var(--color-line);
    background: var(--color-chrome);
  }
  nav button {
    display: grid;
    justify-items: center;
    gap: var(--space-1);
    padding: var(--space-3) 0 var(--space-2);
    border: none;
    background: none;
    color: var(--color-textMuted);
    font-size: var(--text-xs);
    font-weight: 700;
    letter-spacing: var(--tracking-normal);
    text-transform: uppercase;
  }
  nav button.active {
    color: var(--color-accentBright);
  }
</style>
