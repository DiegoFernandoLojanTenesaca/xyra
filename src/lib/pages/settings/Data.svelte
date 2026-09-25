<script lang="ts">
  import { Download, Folder, Trash2 } from '@lucide/svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { app } from '../../app.svelte';
  import Button from '../../components/Button.svelte';

  const CONFIRM_WINDOW_MS = 4000;
  const DATA_FOLDER = '%APPDATA%\\com.indagalab.xyra';

  const t = $derived(app.t);
  const stats = $derived(app.stats);
  const bytes = $derived(new Intl.NumberFormat(app.language, { style: 'unit', unit: 'kilobyte', maximumFractionDigits: 0 }));

  let size = $state<number | null>(null);
  let confirming = $state(false);
  let notice = $state('');

  const refreshSize = async () => (size = await invoke<number>('get_data_size'));
  $effect(() => void refreshSize());

  async function exportCsv() {
    notice = await invoke<string>('export_csv').then((path) => t('settings:savedTo', { path }), app.errorText);
  }

  async function deleteData() {
    if (!confirming) {
      confirming = true;
      setTimeout(() => (confirming = false), CONFIRM_WINDOW_MS);
      return;
    }
    confirming = false;
    notice = await invoke('delete_data').then(() => t('settings:dataDeleted'), app.errorText);
    app.profile = null;
    await Promise.all([app.reloadData(), refreshSize()]);
  }
</script>

<div class="pair">
  <section class="panel cut box">
    <h3 class="section-title">{t('settings:tabs.data')}</h3>
    <div class="fact"><span class="muted">{t('settings:storedGames')}</span><b>{stats?.games ?? 0}</b></div>
    <div class="fact"><span class="muted">{t('settings:spaceUsed')}</span><b>{size === null ? '…' : bytes.format(Math.max(1, size / 1024))}</b></div>
    <div class="fact"><span class="muted">{t('settings:folder')}</span><b>{DATA_FOLDER}</b></div>
    <div class="buttons">
      <Button icon={Download} onclick={exportCsv}>{t('settings:exportCsv')}</Button>
      <Button icon={Folder} onclick={() => invoke('open_folder', { target: 'data' })}>{t('settings:openFolder')}</Button>
      <Button variant={confirming ? 'primary' : 'danger'} icon={Trash2} onclick={deleteData}>{confirming ? t('settings:confirmDelete') : t('settings:deleteData')}</Button>
    </div>
    {#if notice}<p class="muted notice">{notice}</p>{/if}
  </section>
  <section class="panel cut box">
    <h3 class="section-title">{t('settings:tabs.security')}</h3>
    <p class="privacy"><b>■ {t('settings:noTelemetry')}</b> {t('settings:connections')}</p>
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
  .fact {
    display: flex;
    justify-content: space-between;
    gap: var(--space-3);
    padding: var(--space-2) 0;
    font-size: var(--text-md);
  }
  .fact + .fact {
    border-top: 1px solid var(--color-line);
  }
  .buttons {
    display: grid;
    gap: var(--space-2);
    margin-top: var(--space-4);
  }
  .notice {
    margin: var(--space-3) 0 0;
    font-size: var(--text-sm);
    word-break: break-all;
  }
  .privacy {
    margin: 0;
    line-height: 1.6;
    color: var(--color-textMuted);
  }
  .privacy b {
    color: var(--color-success);
  }
</style>
