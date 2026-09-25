<script lang="ts">
  import { Folder } from '@lucide/svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { app } from '../../app.svelte';
  import Button from '../../components/Button.svelte';
  import ToggleRow from '../../components/ToggleRow.svelte';
  import { LANGUAGES, languageName } from '../../i18n';

  const t = $derived(app.t);
  const config = $derived(app.config);
</script>

<div class="pair">
  <section class="panel cut box">
    <h3 class="section-title">{t('settings:tabs.general')}</h3>
    <label class="row">
      <b>{t('settings:language')}</b>
      <select onchange={(e) => app.saveConfig({ language: e.currentTarget.value })}>
        <option value="auto" selected={config.language === 'auto'}>{t('settings:automaticLanguage')}</option>
        {#each LANGUAGES as language (language)}
          <option value={language} selected={config.language === language}>{languageName(language)}</option>
        {/each}
      </select>
    </label>
    <ToggleRow
      title={t('settings:autostart.title')}
      description={t('settings:autostart.description')}
      checked={config.autostart}
      onchange={() => app.saveConfig({ autostart: !config.autostart })}
    />
    <ToggleRow title={t('settings:arena.title')} description={t('settings:arena.description')} checked={config.arena} onchange={() => app.saveConfig({ arena: !config.arena })} />
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
      <Button icon={Folder} onclick={() => invoke('open_folder', { target: 'screenshots' })}>{t('settings:openScreenshots')}</Button>
      <Button icon={Folder} onclick={() => invoke('open_folder', { target: 'log' })}>{t('settings:openLog')}</Button>
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
  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-4);
    padding: var(--space-1) 0 var(--space-3);
    border-bottom: 1px solid var(--color-line);
  }
  .row b {
    font-weight: 600;
  }
  .buttons {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
    margin-top: var(--space-3);
  }
</style>
