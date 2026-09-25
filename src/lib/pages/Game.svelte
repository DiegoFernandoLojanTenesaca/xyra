<script lang="ts">
  import { Gamepad2, ShieldCheck } from '@lucide/svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { app } from '../app.svelte';
  import EmptyState from '../components/EmptyState.svelte';
  import PageHeader from '../components/PageHeader.svelte';
  import ToggleRow from '../components/ToggleRow.svelte';
  import type { Config, GameSetting } from '../types';

  const BORDERLESS = 2;
  const FULLSCREEN = 0;
  const MINIMAP_RANGE = { min: 1, max: 3, step: 0.1 };
  const AUTOMATIONS = ['keepBorderless', 'autoImportRunes', 'closeWindowInGame'] as const;
  const AUTOMATION_KEYS: Record<(typeof AUTOMATIONS)[number], keyof Config> = {
    keepBorderless: 'keep_borderless',
    autoImportRunes: 'auto_import_runes',
    closeWindowInGame: 'close_window_in_game',
  };

  const t = $derived(app.t);
  const config = $derived(app.config);
  let settings = $state<GameSetting[] | null>(null);
  let noClient = $state(false);

  $effect(() => {
    void app.state.phase;
    invoke<GameSetting[]>('get_game_settings')
      .then((s) => ((settings = s), (noClient = false)))
      .catch(() => (noClient = true));
  });

  const isOn = (setting: GameSetting) => (setting.key === 'WindowMode' ? setting.value === BORDERLESS : setting.value === true);

  async function update(setting: GameSetting, value: boolean | number) {
    await invoke('set_game_setting', { section: setting.section, key: setting.key, value });
    setting.value = value;
  }

  const toggle = (setting: GameSetting) => update(setting, setting.key === 'WindowMode' ? (isOn(setting) ? FULLSCREEN : BORDERLESS) : !isOn(setting));
</script>

<PageHeader title={t('common:nav.game')} subtitle={t('game:subtitle')} />

<div class="pair">
  <section class="panel cut box">
    <h3 class="section-title">{t('game:options')}</h3>
    <p class="muted hint">{t('game:optionsHint')}</p>
    {#if noClient}
      <EmptyState icon={Gamepad2} text={t('game:noClient')} />
    {:else if settings}
      {#each settings as setting (setting.key)}
        {#if setting.key === 'MinimapScale'}
          <label class="range">
            <span><b>{t(`game:settings.${setting.key}.title`)}</b><small class="muted">{t(`game:settings.${setting.key}.description`)}</small></span>
            <input type="range" {...MINIMAP_RANGE} value={setting.value} onchange={(e) => update(setting, +e.currentTarget.value)} />
            <b class="value">{Number(setting.value).toFixed(1)}</b>
          </label>
        {:else}
          <ToggleRow
            title={t(`game:settings.${setting.key}.title`)}
            description={t(`game:settings.${setting.key}.description`)}
            checked={isOn(setting)}
            onchange={() => toggle(setting)}
          />
        {/if}
      {/each}
    {/if}
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
    grid-template-columns: 1fr 140px var(--space-8);
    align-items: center;
    gap: var(--space-4);
    padding: var(--space-3) 0;
    border-top: 1px solid var(--color-line);
  }
  .range span {
    display: flex;
    flex-direction: column;
    gap: 2px;
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
  .safe {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    margin: var(--space-3) 0 0;
    padding-top: var(--space-3);
    border-top: 1px solid var(--color-line);
    color: var(--color-success);
    font-size: var(--text-sm);
  }
</style>
