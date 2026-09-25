<script lang="ts">
  import '$lib/theme.css';
  import { ChartColumn, CircleQuestionMark, Crown, Gamepad2, Hammer, House, Layers, Minus, Settings as SettingsIcon, Square, Tag, X } from '@lucide/svelte';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { onMount } from 'svelte';
  import { app } from '$lib/app.svelte';
  import Logo from '$lib/components/Logo.svelte';
  import Augments from '$lib/pages/Augments.svelte';
  import Build from '$lib/pages/Build.svelte';
  import Champions from '$lib/pages/Champions.svelte';
  import Game from '$lib/pages/Game.svelte';
  import Home from '$lib/pages/Home.svelte';
  import Labels from '$lib/pages/Labels.svelte';
  import Settings from '$lib/pages/Settings.svelte';
  import Stats from '$lib/pages/Stats.svelte';
  import { openExternal, REPOSITORY } from '$lib/project';
  import { applyTokens } from '$lib/theme';
  import type { Page } from '$lib/types';

  applyTokens();

  const NAVIGATION = [
    { page: 'home', icon: House },
    { page: 'build', icon: Hammer },
    { page: 'augments', icon: Layers },
    { page: 'champions', icon: Crown },
    { page: 'stats', icon: ChartColumn },
    { page: 'labels', icon: Tag },
    { page: 'game', icon: Gamepad2 },
  ] as const satisfies { page: Page; icon: unknown }[];

  const PAGES = { home: Home, build: Build, augments: Augments, champions: Champions, stats: Stats, labels: Labels, game: Game, settings: Settings };

  const appWindow = getCurrentWindow();
  const t = $derived(app.t);
  const CurrentPage = $derived(PAGES[app.page]);

  $effect(() => {
    document.documentElement.lang = app.language;
  });

  onMount(app.init);
</script>

<div class="shell">
  <header class="titlebar" data-tauri-drag-region>
    <div class="brand condensed" data-tauri-drag-region><Logo size={26} phase={app.ready ? app.state.phase : 'no_client'} />XYRA</div>
    {#if app.ready}
      {@const phase = app.state.phase}
      <span class="status cut-sm {phase}" data-tauri-drag-region>
        <i></i>{t(`common:phases.${phase === 'no_client' ? 'noClient' : phase === 'champ_select' ? 'champSelect' : phase === 'in_game' ? 'inGame' : phase}`)}
        {phase === 'in_game' && app.state.champion ? ` · ${app.state.champion} · ${app.state.mode ? t(`common:modes.${app.state.mode}`) : ''}` : ''}
      </span>
    {/if}
    <div class="end">
      {#if app.profile}
        <button class="profile" onclick={() => app.openSettings('profile')}>
          <img class="diamond" src={app.profile.icon} alt="" />
          <span><b>{app.profile.name}</b><small>{t('common:level')} {app.profile.level}{app.profile.region ? ` · ${app.profile.region}` : ''}</small></span>
        </button>
      {/if}
      <button class="settings" class:active={app.page === 'settings'} onclick={() => app.openSettings('general')} aria-label={t('common:nav.settings')} title={t('common:nav.settings')}>
        <SettingsIcon size={20} />
      </button>
      <div class="window-controls">
        <button onclick={() => appWindow.minimize()} aria-label={t('common:window.minimize')}><Minus size={16} /></button>
        <button onclick={() => appWindow.toggleMaximize()} aria-label={t('common:window.maximize')}><Square size={14} /></button>
        <button class="close" onclick={() => appWindow.close()} aria-label={t('common:window.close')}><X size={16} /></button>
      </div>
    </div>
  </header>

  <nav>
    {#each NAVIGATION as item (item.page)}
      <button class:active={app.page === item.page} onclick={() => app.goTo(item.page)}>
        <item.icon size={18} />{t(`common:nav.${item.page}`)}
        {#if item.page === 'stats' && app.newGames}<span class="badge">+{app.newGames}</span>{/if}
      </button>
    {/each}
    <div class="footer">
      <button class:active={app.page === 'settings' && app.settingsTab === 'help'} onclick={() => app.openSettings('help')}>
        <CircleQuestionMark size={16} />{t('settings:tabs.help')}
      </button>
      <button class="signature" onclick={() => openExternal(REPOSITORY)} title={REPOSITORY}>
        {app.ready ? `v${app.state.version} · ` : ''}Xynitra × IndagaLab ↗
      </button>
    </div>
  </nav>

  <main>
    {#if app.ready}
      {#key app.page}
        <div class="page"><CurrentPage /></div>
      {/key}
    {/if}
  </main>
</div>

<style>
  .shell {
    display: grid;
    grid-template-columns: var(--size-sidebar) minmax(0, 1fr);
    grid-template-rows: var(--size-titlebar) minmax(0, 1fr);
    height: 100vh;
  }
  .titlebar {
    grid-column: 1 / 3;
    display: flex;
    align-items: center;
    gap: var(--space-4);
    padding-left: var(--space-5);
    background: var(--color-chrome);
    border-bottom: 1px solid var(--color-line);
  }
  .brand {
    width: 192px;
    display: flex;
    align-items: center;
    gap: var(--space-3);
    font-size: var(--text-xl);
    letter-spacing: 5px;
    font-variation-settings: 'wdth' 78;
  }
  .status {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-1) var(--space-3);
    font-size: var(--text-sm);
    letter-spacing: 1.5px;
    font-weight: 600;
    text-transform: uppercase;
    background: color-mix(in srgb, var(--color-white) 4%, transparent);
    border: 1px solid var(--color-line);
    white-space: nowrap;
  }
  .status i {
    width: var(--space-2);
    height: var(--space-2);
    background: var(--color-textFaint);
  }
  .status.client i {
    background: var(--color-success);
    box-shadow: 0 0 var(--space-3) var(--color-success);
  }
  .status.champ_select,
  .status.in_game {
    background: color-mix(in srgb, var(--color-accent) 12%, transparent);
    border-color: color-mix(in srgb, var(--color-accent) 45%, transparent);
  }
  .status.champ_select i,
  .status.in_game i {
    background: var(--color-accentBright);
    box-shadow: 0 0 var(--space-3) var(--color-accentBright);
  }
  .status.paused i {
    background: var(--color-warning);
  }
  .end {
    margin-left: auto;
    display: flex;
    align-items: stretch;
    height: 100%;
  }
  .end button {
    border: none;
    background: none;
    color: var(--color-textMuted);
  }
  .end button:hover {
    background: color-mix(in srgb, var(--color-white) 5%, transparent);
    color: var(--color-text);
  }
  .end .profile {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: 0 var(--space-4);
    border-left: 1px solid var(--color-line);
    text-align: left;
  }
  .profile img {
    width: var(--size-thumbSm);
    height: var(--size-thumbSm);
  }
  .profile span {
    display: flex;
    flex-direction: column;
  }
  .profile b {
    font-size: var(--text-md);
    line-height: 1.1;
    color: var(--color-text);
  }
  .profile small {
    font-size: var(--text-xs);
    letter-spacing: 1px;
  }
  .end .settings {
    width: var(--size-titlebar);
    display: grid;
    place-items: center;
    border-left: 1px solid var(--color-line);
  }
  .end .settings.active {
    color: var(--color-accentBright);
    background: color-mix(in srgb, var(--color-accent) 12%, transparent);
  }
  .window-controls {
    display: flex;
    border-left: 1px solid var(--color-line);
  }
  .window-controls button {
    width: var(--size-titlebar);
    display: grid;
    place-items: center;
  }
  .window-controls .close:hover {
    background: var(--color-danger);
    color: var(--color-white);
  }
  nav {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: var(--space-6) 0 var(--space-4);
    background: var(--color-chrome);
    border-right: 1px solid var(--color-line);
  }
  nav button {
    position: relative;
    display: flex;
    align-items: center;
    gap: var(--space-4);
    padding: var(--space-3) var(--space-6);
    border: none;
    background: none;
    color: var(--color-textMuted);
    font-size: var(--text-md);
    font-weight: 600;
    letter-spacing: 2px;
    text-transform: uppercase;
    text-align: left;
  }
  nav button:hover {
    color: var(--color-text);
  }
  nav button.active {
    color: var(--color-text);
    background: linear-gradient(90deg, color-mix(in srgb, var(--color-accent) 22%, transparent), transparent 80%);
  }
  nav button.active::before {
    content: '';
    position: absolute;
    left: 0;
    top: 0;
    bottom: 0;
    width: 3px;
    background: var(--color-accent);
    box-shadow: 0 0 var(--space-3) var(--color-accent);
  }
  nav button.active :global(svg) {
    color: var(--color-accentBright);
  }
  .badge {
    margin-left: auto;
    padding: 1px var(--space-2);
    font-size: var(--text-xs);
    letter-spacing: 1px;
    background: var(--color-accent);
    color: var(--color-white);
  }
  .footer {
    margin-top: auto;
    padding: var(--space-3) 0 0;
    border-top: 1px solid var(--color-line);
  }
  .footer button {
    width: 100%;
    padding: var(--space-2) var(--space-6);
    font-size: var(--text-sm);
  }
  .footer .signature {
    padding-top: var(--space-1);
    font-size: var(--text-xs);
    letter-spacing: 0.5px;
    text-transform: none;
    color: var(--color-textFaint);
  }
  .footer .signature:hover {
    color: var(--color-accentBright);
  }
  main {
    overflow-y: auto;
    overflow-x: hidden;
    padding: var(--space-6) var(--space-8) var(--space-8);
  }
  .page {
    max-width: var(--size-content);
    margin: 0 auto;
    animation: enter 160ms ease-out;
  }
  @keyframes enter {
    from {
      opacity: 0;
      transform: translateY(var(--space-2));
    }
  }
</style>
