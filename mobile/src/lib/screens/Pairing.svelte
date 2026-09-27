<script lang="ts">
  import { QrCode } from '@lucide/svelte';
  import Logo from '$shared/ui/Logo.svelte';
  import { link, LinkError, parsePairing, type Pairing } from '../link.svelte';
  import { mobile } from '../mobile.svelte';
  import { scanQr } from '../scan';
  import Troubleshoot from '../Troubleshoot.svelte';

  const STEPS = ['open', 'turnOn', 'scan'] as const;

  const t = $derived(mobile.t);
  let notice = $state('');
  let pairing = $state(false);
  let unreachable = $state<Pairing | null>(null);

  async function scan() {
    notice = '';
    const raw = await scanQr(t('mobile:pairing.pasteCode'));
    if (raw === null) return;
    const code = parsePairing(raw);
    if (!code) {
      notice = t('mobile:pairing.invalid');
      return;
    }
    await pair(code);
  }

  async function pair(code: Pairing) {
    notice = '';
    unreachable = null;
    pairing = true;
    try {
      await link.pair(code);
    } catch (error) {
      if (error instanceof LinkError) notice = error.failure.code === 'phoneNotAllowed' ? t('mobile:pairing.expired') : mobile.errorText(error);
      else unreachable = code;
    }
    pairing = false;
  }
</script>

<div class="pairing">
  <Logo size={96} phase="client" />
  <h1 class="condensed">{t('mobile:pairing.title')}</h1>
  <p class="muted">{link.status === 'codeChanged' ? t('mobile:pairing.codeChanged') : t('mobile:pairing.text')}</p>

  <ol class="panel cut steps">
    {#each STEPS as step, i (step)}
      <li><b>{i + 1}</b><span>{t(`mobile:pairing.steps.${step}`)}</span></li>
    {/each}
  </ol>

  <button class="scan" disabled={pairing} onclick={scan}><QrCode size={22} />{pairing ? t('mobile:pairing.pairing') : t('mobile:pairing.scan')}</button>
  {#if notice}<p class="notice">{notice}</p>{/if}
  {#if unreachable}
    {@const code = unreachable}
    <Troubleshoot hosts={code.hosts} onretry={() => pair(code)} />
  {/if}
  <small class="muted">{t('mobile:pairing.private')}</small>
</div>

<style>
  .pairing {
    display: grid;
    justify-items: center;
    align-content: center;
    gap: var(--space-4);
    min-height: 100vh;
    min-height: 100dvh;
    padding: calc(env(safe-area-inset-top) + var(--space-6)) var(--space-5) calc(env(safe-area-inset-bottom) + var(--space-6));
    overflow-y: auto;
    text-align: center;
  }
  h1 {
    margin: 0;
    font-size: var(--text-3xl);
  }
  p {
    margin: 0;
    max-width: 30ch;
  }
  .steps {
    display: grid;
    gap: var(--space-3);
    width: 100%;
    margin: 0;
    padding: var(--space-4);
    list-style: none;
    text-align: left;
  }
  .steps li {
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }
  .steps b {
    display: grid;
    place-items: center;
    flex: none;
    width: var(--size-thumbSm);
    height: var(--size-thumbSm);
    background: var(--color-accent);
    color: var(--color-white);
  }
  .scan {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: var(--space-3);
    width: 100%;
    padding: var(--space-5);
    border: none;
    background: var(--color-accent);
    color: var(--color-white);
    font-size: var(--text-lg);
    font-weight: 700;
    letter-spacing: var(--tracking-wide);
    text-transform: uppercase;
  }
  .notice {
    color: var(--color-accentBright);
  }
  small {
    max-width: 34ch;
  }
</style>
