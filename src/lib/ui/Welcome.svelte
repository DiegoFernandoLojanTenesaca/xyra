<script lang="ts">
  import { app } from '../app.svelte';
  import Button from './Button.svelte';
  import Guide from './Guide.svelte';
  import Logo from './Logo.svelte';

  /** Set once the player closed the welcome, so it shows only the first time. */
  const WELCOMED_KEY = 'xyra-welcomed';

  const t = $derived(app.t);
  let open = $state(!localStorage.getItem(WELCOMED_KEY));

  function close() {
    localStorage.setItem(WELCOMED_KEY, '1');
    open = false;
  }
</script>

{#if open && !app.update && !app.state.game}
  <div class="backdrop" role="dialog" aria-modal="true" aria-labelledby="welcome-title">
    <section class="panel cut dialog">
      <header>
        <Logo size={40} phase="client" />
        <div>
          <h2 id="welcome-title">{t('help:welcome.title')}</h2>
          <p class="muted">{t('help:welcome.text')}</p>
        </div>
      </header>
      <Guide compact onnavigate={close} />
      <footer>
        <Button
          onclick={() => {
            close();
            app.openSettings('help');
          }}>{t('help:welcome.guide')}</Button
        >
        <Button variant="primary" onclick={close}>{t('help:welcome.start')}</Button>
      </footer>
    </section>
  </div>
{/if}

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 20;
    display: grid;
    place-items: center;
    padding: var(--space-6);
    background: color-mix(in srgb, var(--color-background) 78%, transparent);
    backdrop-filter: blur(4px);
  }
  .dialog {
    display: grid;
    gap: var(--space-5);
    width: min(100%, var(--size-prose));
    padding: var(--space-6);
    border-color: var(--color-accent);
  }
  header {
    display: flex;
    align-items: center;
    gap: var(--space-4);
  }
  h2 {
    margin: 0;
    font-size: var(--text-xl);
    letter-spacing: var(--tracking-wide);
    text-transform: uppercase;
  }
  header p {
    margin: var(--space-1) 0 0;
    font-size: var(--text-sm);
  }
  footer {
    display: flex;
    justify-content: flex-end;
    gap: var(--space-3);
  }
</style>
