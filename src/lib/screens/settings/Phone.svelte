<script lang="ts">
  import { Download, RefreshCw, ShieldCheck, Smartphone, Unlink } from '@lucide/svelte';
  import { app } from '../../app.svelte';
  import { LINKS, openExternal } from '../../project';
  import { forgetPhone, getPhoneDevices, getPhoneLink, prepareWindowsForPhone, setPhonePermissions } from '../../services/engine';
  import { resource } from '../../services/resource.svelte';
  import type { PhoneDeviceView, PhonePermissions } from '../../types';
  import Button from '../../ui/Button.svelte';
  import EmptyState from '../../ui/EmptyState.svelte';
  import ToggleRow from '../../ui/ToggleRow.svelte';

  const t = $derived(app.t);
  const config = $derived(app.config);
  const link = resource(() => (config.phone_link ? config.phone_token : null), getPhoneLink);
  let preparing = $state(false);
  let notice = $state('');
  let devices = $state<PhoneDeviceView[]>([]);

  /** A phone counts as connected while its app keeps asking; it asks at least every 20 seconds. */
  const CONNECTED_SECONDS = 45;
  const REFRESH_MS = 5000;
  const PERMISSIONS: (keyof PhonePermissions)[] = ['accept', 'import', 'bench', 'lobby', 'settings'];

  $effect(() => {
    const refresh = () => getPhoneDevices().then((list) => (devices = list));
    refresh();
    const timer = setInterval(refresh, REFRESH_MS);
    return () => clearInterval(timer);
  });

  const seen = (device: PhoneDeviceView) =>
    device.seen_ago === null
      ? t('settings:phone.devices.notSeen')
      : device.seen_ago < CONNECTED_SECONDS
        ? t('settings:phone.devices.connected')
        : t('settings:phone.devices.seen', { minutes: app.format.number(Math.max(1, Math.round(device.seen_ago / 60))) });

  async function allow(device: PhoneDeviceView, permission: keyof PhonePermissions) {
    devices = await setPhonePermissions(device.id, { ...device.permissions, [permission]: !device.permissions[permission] });
  }

  /** Lets the phone in through Windows, which asks for administrator approval. */
  async function prepareWindows() {
    preparing = true;
    notice = await prepareWindowsForPhone().then(() => t('settings:phone.prepared'), app.errorText);
    preparing = false;
  }

  async function toggle() {
    const enabling = !config.phone_link;
    await app.saveConfig({ phone_link: enabling });
    if (enabling) await prepareWindows();
  }
</script>

<div class="pair">
  <section class="panel cut box">
    <h3 class="section-title">{t('settings:phone.title')}</h3>
    <ToggleRow title={t('settings:phone.toggle.title')} description={t('settings:phone.toggle.description')} checked={config.phone_link} onchange={toggle} />
    <ul class="muted steps">
      <li>{t('settings:phone.getApp')}</li>
      <li>{t('settings:phone.sameNetwork')}</li>
      <li>{t('settings:phone.firewall')}</li>
      <li>{t('settings:phone.canDo')}</li>
    </ul>
    <div class="buttons">
      <Button icon={Download} onclick={() => openExternal(LINKS.releases)}>{t('settings:phone.download')}</Button>
      <Button icon={ShieldCheck} disabled={preparing || !config.phone_link} onclick={prepareWindows}>{t('settings:phone.prepare')}</Button>
    </div>
    {#if notice}<p class="muted notice">{notice}</p>{/if}
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
      <Button icon={RefreshCw} onclick={() => app.saveConfig({ phone_token: '' })}>{t('settings:phone.newCode')}</Button>
      <small class="muted">{t('settings:phone.newCodeHint')}</small>
    {:else}
      <EmptyState icon={Smartphone} text={t('settings:phone.starting')} />
    {/if}
  </section>
</div>

<section class="panel cut box devices">
  <h3 class="section-title">{t('settings:phone.devices.title')} <small>({app.format.number(devices.length)})</small></h3>
  {#each devices as device (device.id)}
    {@const connected = device.seen_ago !== null && device.seen_ago < CONNECTED_SECONDS}
    <div class="device">
      <Smartphone size={22} />
      <div class="about">
        <b>{device.name || t('settings:phone.devices.unnamed')}</b>
        <small class:connected class="muted"
          >{seen(device)} · {t('settings:phone.devices.paired', { date: app.format.date(new Date(device.paired_at * 1000).toISOString()) })}</small
        >
      </div>
      <div class="permissions">
        {#each PERMISSIONS as permission (permission)}
          <label
            ><input type="checkbox" checked={device.permissions[permission]} onchange={() => allow(device, permission)} />{t(
              `settings:phone.devices.permissions.${permission}`,
            )}</label
          >
        {/each}
      </div>
      <Button variant="danger" icon={Unlink} onclick={async () => (devices = await forgetPhone(device.id))}>{t('settings:phone.devices.forget')}</Button>
    </div>
  {:else}
    <p class="muted">{t('settings:phone.devices.none')}</p>
  {/each}
  <small class="muted">{t('settings:phone.devices.hint')}</small>
</section>

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
  .buttons {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
    margin-bottom: var(--space-3);
  }
  .notice {
    margin: 0 0 var(--space-3);
    font-size: var(--text-md);
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
  .devices {
    margin-top: var(--space-6);
  }
  /* The phone, its name and the disconnect button on one row; its permissions on their own row below the name, so
     they never squeeze the name however many there are. */
  .device {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr) auto;
    align-items: center;
    gap: var(--space-3) var(--space-4);
    padding: var(--space-4) 0;
    border-bottom: var(--border-hairline) solid var(--color-line);
  }
  .device > :global(svg) {
    grid-row: 1;
    grid-column: 1;
  }
  .about {
    display: grid;
    grid-row: 1;
    grid-column: 2;
    min-width: 0;
  }
  .device > :global(.button) {
    grid-row: 1;
    grid-column: 3;
  }
  .about small.connected {
    color: var(--color-success);
  }
  .permissions {
    display: flex;
    flex-wrap: wrap;
    grid-row: 2;
    grid-column: 2 / -1;
    gap: var(--space-2) var(--space-5);
    font-size: var(--text-md);
  }
  .permissions label {
    display: flex;
    align-items: center;
    gap: var(--space-1);
    cursor: pointer;
  }
  .permissions input {
    accent-color: var(--color-accent);
  }
  .devices > small {
    display: block;
    margin-top: var(--space-3);
  }
</style>
