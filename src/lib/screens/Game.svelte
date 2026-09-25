<script lang="ts">
  import { Gamepad2, ShieldCheck } from '@lucide/svelte';
  import { app } from '../app.svelte';
  import { getGameSettings, setGameSetting } from '../services/league';
  import { resource } from '../services/resource.svelte';
  import type { Config, GameSetting, SettingValue } from '../types';
  import EmptyState from '../ui/EmptyState.svelte';
  import PageHeader from '../ui/PageHeader.svelte';
  import ToggleRow from '../ui/ToggleRow.svelte';

  const MINIMAP_RANGE = { min: 1, max: 3, step: 0.1 };
  const MINIMAP_DIGITS = 1;
  const AUTOMATIONS = ['keepBorderless', 'autoImportRunes', 'closeWindowInGame'] as const;
  const AUTOMATION_KEYS: Record<(typeof AUTOMATIONS)[number], keyof Config> = {
    keepBorderless: 'keep_borderless',
    autoImportRunes: 'auto_import_runes',
    closeWindowInGame: 'close_window_in_game',
  };

  const t = $derived(app.t);
  const config = $derived(app.config);
  let notice = $state('');

  const settings = resource(() => app.state.phase, getGameSettings);

  async function update(setting: GameSetting, value: SettingValue) {
    notice = '';
    try {
      await setGameSetting(setting.option, value);
      setting.value = value;
    } catch (error) {
      notice = app.errorText(error);
    }
  }
</script>

<PageHeader title={t('common:nav.game')} subtitle={t('game:subtitle')} />

<div class="pair">
  <section class="panel cut box">
    <h3 class="section-title">{t('game:options')}</h3>
    <p class="muted hint">{t('game:optionsHint')}</p>
    {#if settings.error}
      <EmptyState icon={Gamepad2} text={settings.error.code === 'clientClosed' ? t('game:noClient') : app.errorText(settings.error)} />
    {:else if settings.value}
      {#each settings.value as setting (setting.option)}
        {#if typeof setting.value === 'number'}
          <label class="range">
            <span><b>{t(`game:settings.${setting.option}.title`)}</b><small class="muted">{t(`game:settings.${setting.option}.description`)}</small></span>
            <input type="range" {...MINIMAP_RANGE} value={setting.value} onchange={(e) => update(setting, +e.currentTarget.value)} />
            <b class="value">{app.format.number(setting.value, MINIMAP_DIGITS)}</b>
          </label>
        {:else}
          <ToggleRow
            title={t(`game:settings.${setting.option}.title`)}
            description={t(`game:settings.${setting.option}.description`)}
            checked={setting.value}
            onchange={() => update(setting, !setting.value)}
          />
        {/if}
      {/each}
    {/if}
    {#if notice}<p class="warning">{notice}</p>{/if}
  </section>

  <section class="panel cut box">
    <h3 class="section-title">{t('game:automatic')}</h3>
    <p class="muted hint">{t('game:automaticHint')}</p>
    {#each AUTOMATIONS as automation (automation)}
      {@const key = AUTOMATION_KEYS[automation]}
      <ToggleRow
        title={t(`game:${automation}.title`)}
        description={t(`game:${automation}.description`)}
        checked={config[key] as boolean}
        onchange={() => app.saveConfig({ [key]: !config[key] })}
      />
    {/each}
    <p class="safe"><ShieldCheck size={16} />{t('game:safe')}</p>
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
  .hint {
    margin: calc(-1 * var(--space-1)) 0 var(--space-2);
    font-size: var(--text-md);
  }
  .range {
    display: grid;
    grid-template-columns: 1fr var(--size-slider) var(--space-8);
    align-items: center;
    gap: var(--space-4);
    padding: var(--space-3) 0;
    border-top: var(--border-hairline) solid var(--color-line);
  }
  .range span {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }
  .range b {
    font-weight: 600;
  }
  .range small {
    font-size: var(--text-sm);
  }
  .value {
    text-align: right;
    font-variant-numeric: tabular-nums;
  }
  .warning {
    margin: var(--space-3) 0 0;
    color: var(--color-warning);
  }
  .safe {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    margin: var(--space-3) 0 0;
    padding-top: var(--space-3);
    border-top: var(--border-hairline) solid var(--color-line);
    color: var(--color-success);
    font-size: var(--text-sm);
  }
</style>
