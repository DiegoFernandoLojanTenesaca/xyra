<script lang="ts">
  import { app } from '../app.svelte';
  import PageHeader from '../components/PageHeader.svelte';
  import SegmentedControl from '../components/SegmentedControl.svelte';
  import type { SettingsTab } from '../types';
  import About from './settings/About.svelte';
  import Data from './settings/Data.svelte';
  import General from './settings/General.svelte';
  import Help from './settings/Help.svelte';
  import Profile from './settings/Profile.svelte';
  import Security from './settings/Security.svelte';

  const TABS: SettingsTab[] = ['general', 'profile', 'data', 'security', 'help', 'about'];
  const t = $derived(app.t);
</script>

<PageHeader eyebrow={t('common:nav.settings')} title={t(`settings:tabs.${app.settingsTab}`)} subtitle={t(`settings:subtitles.${app.settingsTab}`)} />
<SegmentedControl tabs options={TABS.map((tab) => [tab, t(`settings:tabs.${tab}`)] as [SettingsTab, string])} bind:value={app.settingsTab} />

{#if app.settingsTab === 'general'}
  <General />
{:else if app.settingsTab === 'profile'}
  <Profile />
{:else if app.settingsTab === 'data'}
  <Data />
{:else if app.settingsTab === 'security'}
  <Security />
{:else if app.settingsTab === 'help'}
  <Help />
{:else}
  <About />
{/if}
