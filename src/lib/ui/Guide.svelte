<script lang="ts">
  import { ChevronRight } from '@lucide/svelte';
  import { app } from '../app.svelte';
  import { GUIDE, type GuideTarget } from '../guide';

  /** Compact shows only the first line of each moment, for the welcome window. */
  let { compact = false, onnavigate }: { compact?: boolean; onnavigate?: () => void } = $props();

  const t = $derived(app.t);
  const label = (target: GuideTarget) => ('page' in target ? t(`common:nav.${target.page}`) : t(`settings:tabs.${target.settings}`));

  function go(target: GuideTarget) {
    onnavigate?.();
    if ('page' in target) app.goTo(target.page);
    else app.openSettings(target.settings);
  }
</script>

<ol class:compact>
  {#each GUIDE as moment, i (moment.id)}
    <li>
      <span class="diamond number">{i + 1}</span>
      <div>
        <b>{t(`help:guide.${moment.id}.title`)}</b>
        {#if !compact}<p class="muted">{t(`help:guide.${moment.id}.text`)}</p>{/if}
        <div class="links">
          {#each moment.targets as target, j (j)}
            <button class="link" onclick={() => go(target)}>{label(target)}<ChevronRight size={14} /></button>
          {/each}
        </div>
      </div>
    </li>
  {/each}
</ol>

<style>
  ol {
    display: grid;
    gap: var(--space-4);
    margin: 0;
    padding: 0;
    list-style: none;
  }
  ol.compact {
    gap: var(--space-3);
  }
  li {
    display: flex;
    align-items: flex-start;
    gap: var(--space-3);
  }
  .number {
    display: grid;
    flex: none;
    place-items: center;
    width: var(--size-iconSm);
    height: var(--size-iconSm);
    background: var(--color-accent);
    color: var(--color-white);
    font-size: var(--text-xs);
    font-weight: 700;
  }
  p {
    margin: var(--space-1) 0 0;
    font-size: var(--text-sm);
  }
  .links {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-3);
    margin-top: var(--space-1);
  }
  .link {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    padding: 0;
    border: none;
    background: none;
    color: var(--color-accentBright);
    font-size: var(--text-sm);
    font-weight: 700;
  }
  .link:hover {
    color: var(--color-text);
  }
</style>
