<script lang="ts">
  import '$lib/design/theme.css';
  import {
    ChartColumn,
    CircleQuestionMark,
    Crown,
    Download,
    ExternalLink,
    Gamepad2,
    Hammer,
    House,
    Layers,
    Minus,
    Settings as SettingsIcon,
    Square,
    Tag,
    X,
  } from '@lucide/svelte';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { onMount, type Component } from 'svelte';
  import { app, type Page } from '$lib/app.svelte';
  import { applyTokens } from '$lib/design/theme';
  import { APP_NAME, openExternal, REPOSITORY, STUDIOS } from '$lib/project';
  import Augments from '$lib/screens/Augments.svelte';
  import Build from '$lib/screens/Build.svelte';
  import Champions from '$lib/screens/Champions.svelte';
  import Game from '$lib/screens/Game.svelte';
  import Home from '$lib/screens/Home.svelte';
  import Labels from '$lib/screens/Labels.svelte';
  import Settings from '$lib/screens/Settings.svelte';
  import Stats from '$lib/screens/Stats.svelte';
  import Logo from '$lib/ui/Logo.svelte';

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

  const SCREENS: Record<Page, Component> = {
    home: Home,
    build: Build,
    augments: Augments,
    champions: Champions,
    stats: Stats,
    labels: Labels,
    game: Game,
    settings: Settings,
  };

  const appWindow = getCurrentWindow();
  const t = $derived(app.t);
  const Screen = $derived(SCREENS[app.page]);

  $effect(() => {
    document.documentElement.lang = app.language;
  });

  onMount(app.init);
</script>

<div class="shell">
  <header class="titlebar" data-tauri-drag-region>
    <div class="brand condensed" data-tauri-drag-region><Logo size={26} phase={app.ready ? app.state.phase : 'noClient'} />{APP_NAME}</div>
    {#if app.ready}
      {@const game = app.state.game}
      <span class="status cut-sm {app.state.phase}" data-tauri-drag-region>
        <i></i>
        <span class="facts">
          <span>{t(`common:phases.${app.state.phase}`)}</span>
          {#if game?.champion}<span>{game.champion.name}</span>{/if}
          {#if game}<span>{t(`common:modes.${game.mode}`)}</span>{/if}
        </span>
      </span>
    {/if}
    <div class="end">
      {#if app.profile}
        <button class="profile" onclick={() => app.openSettings('profile')}>
          <img class="diamond" src={app.profile.icon} alt="" />
          <span
            ><b>{app.profile.name}</b><small class="facts"
              ><span>{t('common:levelValue', { level: app.profile.level })}</span>{#if app.profile.region}<span>{app.profile.region}</span>{/if}</small
            ></span
          >
        </button>
      {/if}
      <button
        class="settings"
        class:active={app.page === 'settings'}
        onclick={() => app.openSettings('general')}
        aria-label={t('common:nav.settings')}
        title={t('common:nav.settings')}
      >
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
      {#if app.update}
        <button class="update" disabled={app.updateProgress !== null} onclick={app.installUpdate} title={t('about:install')}>
          <Download size={16} />{app.updateProgress === null
            ? t('about:updateTo', { version: app.update.version })
            : t('about:downloading', { value: app.format.percent(app.updateProgress * 100) })}
        </button>
      {/if}
      <button class:active={app.page === 'settings' && app.settingsTab === 'help'} onclick={() => app.openSettings('help')}>
        <CircleQuestionMark size={16} />{t('settings:tabs.help')}
      </button>
      <button class="signature" onclick={() => openExternal(REPOSITORY)} title={REPOSITORY}>
        <span class="facts"
          >{#if app.ready}<span>v{app.state.version}</span>{/if}<span>{STUDIOS}</span></span
        >
        <ExternalLink size={12} />
      </button>
    </div>
  </nav>

  <main>
    {#if app.ready}
      {#key app.page}
        <div class="page"><Screen /></div>
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
    border-bottom: var(--border-hairline) solid var(--color-line);
  }
  .brand {
    width: var(--size-brand);
    display: flex;
    align-items: center;
    gap: var(--space-3);
    font-size: var(--text-xl);
    letter-spacing: var(--tracking-widest);
    font-variation-settings: 'wdth' 78;
  }
  .status {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-1) var(--space-3);
    font-size: var(--text-sm);
    letter-spacing: var(--tracking-relaxed);
    font-weight: 600;
    text-transform: uppercase;
    background: color-mix(in srgb, var(--color-white) 4%, transparent);
    border: var(--border-hairline) solid var(--color-line);
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
  .status.champSelect,
  .status.inGame {
    background: color-mix(in srgb, var(--color-accent) 12%, transparent);
    border-color: color-mix(in srgb, var(--color-accent) 45%, transparent);
  }
  .status.champSelect i,
  .status.inGame i {
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
    border-left: var(--border-hairline) solid var(--color-line);
    text-align: left;
  }
  .profile img {
    width: var(--size-thumbSm);
    height: var(--size-thumbSm);
  }
  .profile > span {
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
    letter-spacing: var(--tracking-normal);
  }
  .end .settings {
    width: var(--size-titlebar);
    display: grid;
    place-items: center;
    border-left: var(--border-hairline) solid var(--color-line);
  }
  .end .settings.active {
    color: var(--color-accentBright);
    background: color-mix(in srgb, var(--color-accent) 12%, transparent);
  }
  .window-controls {
    display: flex;
    border-left: var(--border-hairline) solid var(--color-line);
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
    gap: var(--space-1);
    padding: var(--space-6) 0 var(--space-4);
    background: var(--color-chrome);
    border-right: var(--border-hairline) solid var(--color-line);
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
    letter-spacing: var(--tracking-wide);
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
    width: var(--border-accent);
    background: var(--color-accent);
    box-shadow: 0 0 var(--space-3) var(--color-accent);
  }
  nav button :global(svg) {
    flex: none;
  }
  nav button.active :global(svg) {
    color: var(--color-accentBright);
  }
  .badge {
    margin-left: auto;
    padding: 0 var(--space-2);
    font-size: var(--text-xs);
    letter-spacing: var(--tracking-normal);
    background: var(--color-accent);
    color: var(--color-white);
  }
  .footer {
    margin-top: auto;
    padding: var(--space-3) 0 0;
    border-top: var(--border-hairline) solid var(--color-line);
  }
  .footer button {
    width: 100%;
    padding: var(--space-2) var(--space-6);
    font-size: var(--text-sm);
  }
  .footer .update {
    color: var(--color-white);
    background: var(--color-accent);
  }
  .footer .update:hover {
    color: var(--color-white);
    background: var(--color-accentBright);
  }
  .footer .signature {
    display: flex;
    align-items: center;
    gap: var(--space-1);
    padding-top: var(--space-1);
    font-size: var(--text-xs);
    letter-spacing: var(--tracking-tight);
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
