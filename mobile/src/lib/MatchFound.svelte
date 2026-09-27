<script lang="ts">
  import { Check, X } from '@lucide/svelte';
  import Logo from '$shared/ui/Logo.svelte';
  import { link } from './link.svelte';
  import { mobile } from './mobile.svelte';

  type Answer = 'accepted' | 'declined';

  const t = $derived(mobile.t);
  const found = $derived(!!link.state?.ready_check);
  const can = $derived(link.pc?.permissions.accept ?? false);
  let answered = $state<Answer | null>(null);
  let busy = $state(false);
  let notice = $state('');

  $effect(() => {
    if (found) return;
    answered = null;
    notice = '';
  });

  async function answer(path: string, result: Answer) {
    busy = true;
    notice = '';
    try {
      await link.call('POST', path);
      answered = result;
    } catch (error) {
      notice = mobile.errorText(error);
    }
    busy = false;
  }
</script>

{#if found}
  <div class="backdrop" role="dialog" aria-modal="true" aria-label={t('mobile:live.matchFound')}>
    <div class="ring">
      <svg class="arc" viewBox="0 0 100 100" aria-hidden="true"><circle cx="50" cy="50" r="48" /></svg>
      <div class="inside">
        <Logo size={72} phase="champSelect" />
        <h1 class="condensed">{t('mobile:live.matchFound')}</h1>
        {#if link.pc}<small class="muted">{link.pc.name}</small>{/if}
      </div>
    </div>

    {#if answered}
      <p class="answered" class:declined={answered === 'declined'}>
        {#if answered === 'accepted'}<Check size={18} />{:else}<X size={18} />{/if}{t(`mobile:live.${answered}`)}
      </p>
    {:else if can}
      <button class="accept cut" disabled={busy} onclick={() => answer('/api/accept', 'accepted')}>{t('mobile:live.acceptBig')}</button>
      <button class="decline" disabled={busy} onclick={() => answer('/api/decline', 'declined')}>{t('mobile:live.decline')}</button>
    {:else}
      <p class="muted">{t('mobile:live.notAllowed')}</p>
    {/if}
    {#if notice}<p class="notice">{notice}</p>{/if}
  </div>
{/if}

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 10;
    display: grid;
    align-content: center;
    justify-items: center;
    gap: var(--space-5);
    padding: var(--space-6);
    background:
      radial-gradient(circle at 50% 38%, color-mix(in srgb, var(--color-accent) 28%, transparent), transparent 62%),
      color-mix(in srgb, var(--color-background) 94%, transparent);
    text-align: center;
    animation: appear 0.25s ease-out both;
  }
  .ring {
    position: relative;
    display: grid;
    place-items: center;
    width: min(78vw, var(--size-aside));
    aspect-ratio: 1;
    border: var(--border-thick) solid color-mix(in srgb, var(--color-accent) 55%, transparent);
    border-radius: 50%;
    background: radial-gradient(circle, var(--color-panelRaised), var(--color-background) 72%);
    box-shadow:
      0 0 0 var(--space-2) color-mix(in srgb, var(--color-accent) 12%, transparent),
      0 0 var(--space-8) color-mix(in srgb, var(--color-accent) 45%, transparent);
  }
  .arc {
    position: absolute;
    inset: calc(-1 * var(--space-3));
    width: calc(100% + 2 * var(--space-3));
    height: calc(100% + 2 * var(--space-3));
    animation: spin 1.6s linear infinite;
  }
  .arc circle {
    fill: none;
    stroke: var(--color-accentBright);
    stroke-width: 1.4;
    stroke-linecap: round;
    stroke-dasharray: 60 242;
  }
  .inside {
    display: grid;
    justify-items: center;
    gap: var(--space-2);
    padding: var(--space-5);
  }
  h1 {
    margin: 0;
    font-size: var(--text-2xl);
    letter-spacing: var(--tracking-wide);
    text-transform: uppercase;
  }
  .accept {
    width: min(78vw, var(--size-aside));
    padding: var(--space-4) var(--space-6);
    border: none;
    background: var(--color-accent);
    color: var(--color-white);
    font-size: var(--text-xl);
    font-weight: 800;
    letter-spacing: var(--tracking-wide);
    text-transform: uppercase;
    animation: glow 1.4s ease-in-out infinite;
  }
  .decline {
    padding: var(--space-2) var(--space-6);
    border: var(--border-hairline) solid var(--color-lineStrong);
    background: var(--color-panel);
    color: var(--color-textSubtle);
    font-size: var(--text-sm);
    font-weight: 700;
    letter-spacing: var(--tracking-wide);
    text-transform: uppercase;
  }
  .answered {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    margin: 0;
    color: var(--color-success);
    font-size: var(--text-lg);
    font-weight: 700;
  }
  .answered.declined {
    color: var(--color-accentBright);
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  @keyframes glow {
    50% {
      box-shadow: 0 0 var(--space-6) color-mix(in srgb, var(--color-accent) 70%, transparent);
    }
  }
  @keyframes appear {
    from {
      opacity: 0;
      transform: scale(0.96);
    }
  }
</style>
