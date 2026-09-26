<script lang="ts">
  import { RefreshCw, ShieldCheck, Smartphone } from '@lucide/svelte';
  import { app } from '../../app.svelte';
  import { getPhoneLink } from '../../services/engine';
  import { resource } from '../../services/resource.svelte';
  import Button from '../../ui/Button.svelte';
  import EmptyState from '../../ui/EmptyState.svelte';
  import ToggleRow from '../../ui/ToggleRow.svelte';

  const t = $derived(app.t);
  const config = $derived(app.config);
  const link = resource(() => (config.phone_link ? config.phone_token : null), getPhoneLink);
</script>

<div class="pair">
  <section class="panel cut box">
    <h3 class="section-title">{t('settings:phone.title')}</h3>
    <ToggleRow
      title={t('settings:phone.toggle.title')}
      description={t('settings:phone.toggle.description')}
      checked={config.phone_link}
      onchange={() => app.saveConfig({ phone_link: !config.phone_link })}
    />
    <ul class="muted steps">
      <li>{t('settings:phone.sameNetwork')}</li>
      <li>{t('settings:phone.firewall')}</li>
      <li>{t('settings:phone.canDo')}</li>
    </ul>
    <p class="privacy"><b><ShieldCheck size={14} />{t('settings:phone.privateTitle')}</b> {t('settings:phone.private')}</p>
  </section>

  <section class="panel cut box qr">
    {#if !config.phone_link}
      <EmptyState icon={Smartphone} text={t('settings:phone.off')} />
    {:else if link.error}
      <EmptyState icon={Smartphone} text={app.errorText(link.error)} />
    {:else if link.value}
      <img class="code" src={`data:image/svg+xml;charset=utf-8,${encodeURIComponent(link.value.qr)}`} alt={t('settings:phone.qrAlt')} />
      <p class="muted">{t('settings:phone.scan')}</p>
      <code>{link.value.url.split('?')[0]}</code>
      <Button icon={RefreshCw} onclick={() => app.saveConfig({ phone_token: '' })}>{t('settings:phone.newCode')}</Button>
      <small class="muted">{t('settings:phone.newCodeHint')}</small>
    {:else}
      <EmptyState icon={Smartphone} text={t('settings:phone.starting')} />
    {/if}
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
  .steps {
    margin: var(--space-3) 0;
    padding-left: var(--space-5);
    font-size: var(--text-md);
  }
  .steps li {
    margin-bottom: var(--space-2);
  }
  .privacy {
    margin: 0;
    color: var(--color-textMuted);
    font-size: var(--text-md);
  }
  .privacy b {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    color: var(--color-success);
  }
  .qr {
    display: grid;
    justify-items: center;
    gap: var(--space-3);
    text-align: center;
  }
  .code {
    padding: var(--space-3);
    background: var(--color-white);
  }
  code {
    color: var(--color-textSubtle);
  }
</style>
