<script lang="ts">
  import { Download, ExternalLink, Smartphone, Sparkles } from '@lucide/svelte';
  import { app } from '../app.svelte';
  import { openExternal } from '../project';
  import Button from './Button.svelte';

  const t = $derived(app.t);
  const release = $derived(app.update);
  const downloading = $derived(app.updateProgress !== null);
  /** Offered once per version, never over a game; the sidebar keeps the button meanwhile. */
  const open = $derived(!!release && (downloading || (release.version !== app.skippedUpdate && !app.state.game)));
</script>

{#if open && release}
  <div class="backdrop" role="dialog" aria-modal="true" aria-labelledby="update-title">
    <section class="panel cut dialog">
      <header>
        <span class="icon diamond"><Sparkles size={18} /></span>
        <div>
          <h2 id="update-title">{t('about:dialogTitle', { version: release.version })}</h2>
          <p class="muted">{t('about:dialogVersions', { current: app.state.version, version: release.version })}</p>
        </div>
      </header>

      {#if release.highlights.length}
        <ul>
          {#each release.highlights as highlight (highlight)}
            <li>{highlight}</li>
          {/each}
        </ul>
      {/if}

      <p class="phone"><Smartphone size={16} />{t('about:dialogPhone')}</p>

      {#if downloading}
        <p>{t('about:downloading', { value: app.format.percent((app.updateProgress ?? 0) * 100) })}</p>
        <div class="progress"><i style="width:{(app.updateProgress ?? 0) * 100}%"></i></div>
        <p class="muted hint">{t('about:installHint')}</p>
      {:else}
        {#if app.updateError}<p class="error">{app.updateError}</p>{/if}
        <footer>
          <button class="link" onclick={() => openExternal(release.notes_url)}><ExternalLink size={14} />{t('about:notes')}</button>
          <Button onclick={app.skipUpdate}>{t('about:later')}</Button>
          <Button variant="primary" icon={Download} onclick={app.installUpdate}>{t('about:updateNow')}</Button>
        </footer>
      {/if}
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
    animation: appear 0.2s ease-out both;
  }
  .dialog {
    display: grid;
    gap: var(--space-4);
    width: min(100%, var(--size-prose));
    padding: var(--space-6);
    border-color: var(--color-accent);
    background: linear-gradient(180deg, color-mix(in srgb, var(--color-accent) 12%, transparent), transparent 40%), var(--color-panel);
  }
  header {
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }
  .icon {
    display: grid;
    flex: none;
    place-items: center;
    width: var(--size-thumb);
    height: var(--size-thumb);
    background: var(--color-accent);
    color: var(--color-white);
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
  ul {
    display: grid;
    gap: var(--space-2);
    margin: 0;
    padding-left: var(--space-5);
  }
  li::marker {
    color: var(--color-accentBright);
  }
  .phone {
    display: flex;
    gap: var(--space-2);
    margin: 0;
    padding: var(--space-3);
    border-left: var(--border-accent) solid var(--color-accent);
    background: var(--color-panelRaised);
    font-size: var(--text-sm);
  }
  .phone :global(svg) {
    flex: none;
    color: var(--color-accentBright);
  }
  footer {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: flex-end;
    gap: var(--space-3);
  }
  .link {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    margin-right: auto;
    border: none;
    background: none;
    color: var(--color-textSubtle);
    font-size: var(--text-sm);
  }
  .link:hover {
    color: var(--color-text);
  }
  .progress {
    height: var(--space-2);
    background: var(--color-panelRaised);
  }
  .progress i {
    display: block;
    height: 100%;
    background: var(--color-accent);
  }
  .hint,
  .error {
    margin: 0;
    font-size: var(--text-sm);
  }
  .error {
    color: var(--color-accentBright);
  }
  @keyframes appear {
    from {
      opacity: 0;
    }
  }
</style>
