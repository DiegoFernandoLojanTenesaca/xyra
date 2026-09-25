<script lang="ts">
  import { Download, Folder, ShieldCheck, Trash2 } from '@lucide/svelte';
  import { app } from '../../app.svelte';
  import { deleteData, exportCsv, getDataUsage, openFolder } from '../../services/data';
  import type { DataUsage } from '../../types';
  import Button from '../../ui/Button.svelte';

  const CONFIRM_WINDOW_MS = 4000;

  const t = $derived(app.t);
  const stats = $derived(app.stats);
  let usage = $state<DataUsage | null>(null);
  let confirming = $state(false);
  let notice = $state('');

  const refreshUsage = async () => (usage = await getDataUsage());
  $effect(() => void refreshUsage());

  async function exportGames() {
    notice = await exportCsv().then((path) => t('settings:savedTo', { path }), app.errorText);
  }

  async function removeData() {
    if (!confirming) {
      confirming = true;
      setTimeout(() => (confirming = false), CONFIRM_WINDOW_MS);
      return;
    }
    confirming = false;
    notice = await deleteData().then(() => t('settings:dataDeleted'), app.errorText);
    app.profile = null;
    await refreshUsage();
  }
</script>

<div class="pair">
  <section class="panel cut box">
    <h3 class="section-title">{t('settings:tabs.data')}</h3>
    <div class="fact"><span class="muted">{t('settings:storedGames')}</span><b>{app.format.number(stats?.games ?? 0)}</b></div>
    <div class="fact"><span class="muted">{t('settings:spaceUsed')}</span><b>{usage ? app.format.kilobytes(usage.bytes) : '…'}</b></div>
    <div class="fact"><span class="muted">{t('settings:folder')}</span><b class="path" title={usage?.folder}>{usage?.folder ?? '…'}</b></div>
    <div class="buttons">
      <Button icon={Download} onclick={exportGames}>{t('settings:exportCsv')}</Button>
      <Button icon={Folder} onclick={() => openFolder('data')}>{t('settings:openFolder')}</Button>
      <Button variant={confirming ? 'primary' : 'danger'} icon={Trash2} onclick={removeData}
        >{confirming ? t('settings:confirmDelete') : t('settings:deleteData')}</Button
      >
    </div>
    {#if notice}<p class="muted notice">{notice}</p>{/if}
  </section>

  <section class="panel cut box">
    <h3 class="section-title">{t('settings:tabs.security')}</h3>
    <p class="privacy"><b><ShieldCheck size={14} />{t('settings:noTelemetry')}</b> {t('settings:connections')}</p>
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
    border-top: var(--border-hairline) solid var(--color-line);
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
  .path {
    max-width: 60%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .privacy b {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    color: var(--color-success);
  }
</style>
