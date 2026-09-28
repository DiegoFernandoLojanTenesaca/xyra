<script lang="ts">
  import { Download, Monitor, Sparkles } from '@lucide/svelte';
  import { android, isNewer } from './android.svelte';
  import { link } from './link.svelte';
  import { mobile } from './mobile.svelte';

  /** Each version keeps its highlights in About's texts, read from the repository at its tag. */
  const REPOSITORY_FILES = 'https://raw.githubusercontent.com/DiegoFernandoLojanTenesaca/xyra';
  const SKIPPED_KEY = 'xyra-update-later';

  const t = $derived(mobile.t);
  const release = $derived(android.release);
  const pc = $derived(link.state?.version ?? null);
  let skipped = $state(localStorage.getItem(SKIPPED_KEY));
  let highlights = $state<string[]>([]);
  /** Offered once per version, never over a match found; Settings keeps the button meanwhile. */
  const open = $derived(!!release && (android.progress !== null || (release.version !== skipped && !link.state?.ready_check)));

  $effect(() => {
    if (!release) return;
    highlights = [];
    fetch(`${REPOSITORY_FILES}/${release.version}/locales/${mobile.language}/about.json`)
      .then((response) => (response.ok ? response.json() : Promise.reject(response.status)))
      .then((about: { changes?: Record<string, string> }) => (highlights = Object.values(about.changes ?? {})))
      .catch(() => {});
  });

  function later() {
    if (!release) return;
    localStorage.setItem(SKIPPED_KEY, release.version);
    skipped = release.version;
  }
</script>

{#if open && release}
  <div class="backdrop" role="dialog" aria-modal="true" aria-labelledby="update-title">
    <section class="panel cut dialog">
      <header>
        <span class="icon diamond"><Sparkles size={18} /></span>
        <h2 id="update-title">{t('mobile:update.title', { version: release.version })}</h2>
      </header>
      <p class="muted">{t('mobile:update.versions', { current: android.version })}</p>
      {#if highlights.length}
        <ul>
          {#each highlights as highlight (highlight)}
            <li>{highlight}</li>
          {/each}
        </ul>
      {/if}

      <p class="pc">
        <Monitor size={16} />
        {#if !pc}{t('mobile:update.pcUnknown')}
        {:else if isNewer(release.version, pc)}{t('mobile:update.pcOlder', { pc })}
        {:else}{t('mobile:update.pcReady', { pc })}{/if}
      </p>

      {#if android.progress !== null}
        <p>{t('mobile:update.downloading', { value: mobile.format.percent(android.progress) })}</p>
        <div class="progress"><i style="width:{android.progress}%"></i></div>
        <p class="muted small">{t('mobile:update.hint')}</p>
      {:else}
        {#if android.failed}<p class="error">{t('mobile:update.failed')}</p>{/if}
        <button class="action primary wide" onclick={android.install}><Download size={16} />{t('mobile:update.install')}</button>
        <button class="action wide" onclick={later}>{t('mobile:update.later')}</button>
      {/if}
    </section>
  </div>
{/if}

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 9;
    display: grid;
    align-content: center;
    padding: var(--space-5);
    background: color-mix(in srgb, var(--color-background) 82%, transparent);
    animation: appear 0.2s ease-out both;
  }
  .dialog {
    display: grid;
    gap: var(--space-3);
    max-height: 100%;
    overflow-y: auto;
    padding: var(--space-5);
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
    font-size: var(--text-lg);
    letter-spacing: var(--tracking-wide);
    text-transform: uppercase;
  }
  p {
    margin: 0;
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
  .pc {
    display: flex;
    gap: var(--space-2);
    padding: var(--space-3);
    border-left: var(--border-accent) solid var(--color-accent);
    background: var(--color-panelRaised);
    font-size: var(--text-sm);
  }
  .pc :global(svg) {
    flex: none;
    color: var(--color-accentBright);
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
  .small,
  .error {
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
