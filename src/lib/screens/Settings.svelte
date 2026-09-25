<script lang="ts">
  import type { Component } from 'svelte';
  import { app, SETTINGS_TABS, type SettingsTab } from '../app.svelte';
  import PageHeader from '../ui/PageHeader.svelte';
  import SegmentedControl from '../ui/SegmentedControl.svelte';
  import About from './settings/About.svelte';
  import Data from './settings/Data.svelte';
  import General from './settings/General.svelte';
  import Help from './settings/Help.svelte';
  import Profile from './settings/Profile.svelte';
  import Security from './settings/Security.svelte';

  const TAB_SCREENS: Record<SettingsTab, Component> = { general: General, profile: Profile, data: Data, security: Security, help: Help, about: About };

  const t = $derived(app.t);
  const Tab = $derived(TAB_SCREENS[app.settingsTab]);
</script>

<PageHeader eyebrow={t('common:nav.settings')} title={t(`settings:tabs.${app.settingsTab}`)} subtitle={t(`settings:subtitles.${app.settingsTab}`)} />

<SegmentedControl tabs options={SETTINGS_TABS.map((tab) => [tab, t(`settings:tabs.${tab}`)] as [SettingsTab, string])} bind:value={app.settingsTab} />

<Tab />
