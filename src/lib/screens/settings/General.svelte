<script lang="ts">
  import { Folder } from '@lucide/svelte';
  import { app } from '../../app.svelte';
  import { LANGUAGES, languageName } from '../../i18n';
  import { openFolder } from '../../services/data';
  import Button from '../../ui/Button.svelte';
  import SegmentedControl from '../../ui/SegmentedControl.svelte';
  import ToggleRow from '../../ui/ToggleRow.svelte';

  const t = $derived(app.t);
  const config = $derived(app.config);
</script>

<div class="pair">
  <section class="panel cut box">
    <h3 class="section-title">{t('settings:tabs.general')}</h3>
    <div class="language">
      <b>{t('settings:language')}</b>
      <SegmentedControl
        options={[['', t('settings:automaticLanguage')], ...LANGUAGES.map((language) => [language, languageName(language)] as [string, string])]}
        bind:value={() => config.language ?? '', (language) => app.saveConfig({ language: language || null })}
      />
      <small class="muted">{t('settings:languageHint')}</small>
    </div>
    <ToggleRow
      title={t('settings:autostart.title')}
      description={t('settings:autostart.description')}
      checked={config.autostart}
      onchange={() => app.saveConfig({ autostart: !config.autostart })}
    />
    <ToggleRow
      title={t('settings:openLeague.title')}
      description={t('settings:openLeague.description')}
      checked={config.open_league}
      onchange={() => app.saveConfig({ open_league: !config.open_league })}
    />
    <ToggleRow
      title={t('settings:arena.title')}
      description={t('settings:arena.description')}
      checked={config.arena}
      onchange={() => app.saveConfig({ arena: !config.arena })}
    />
  </section>

  <section class="panel cut box">
    <h3 class="section-title">{t('settings:diagnostics')}</h3>
    <ToggleRow
      title={t('settings:recordScreenshots.title')}
      description={t('settings:recordScreenshots.description')}
      checked={config.record_screenshots}
      onchange={() => app.saveConfig({ record_screenshots: !config.record_screenshots })}
    />
    <div class="buttons">
      <Button icon={Folder} onclick={() => openFolder('screenshots')}>{t('settings:openScreenshots')}</Button>
      <Button icon={Folder} onclick={() => openFolder('log')}>{t('settings:openLog')}</Button>
    </div>
  </section>
</div>

<style>
  .pair {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
    gap: var(--space-6);
    align-items: start;
  }
  .box {
    padding: var(--space-4);
  }
  .language {
    display: grid;
    justify-items: start;
    gap: var(--space-2);
    padding: var(--space-1) 0 var(--space-3);
    border-bottom: var(--border-hairline) solid var(--color-line);
  }
  .language b {
    font-weight: 600;
  }
  .buttons {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
    margin-top: var(--space-3);
  }
</style>
