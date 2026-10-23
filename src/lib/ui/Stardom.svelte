<script lang="ts">
  import { Check, Lock } from '@lucide/svelte';
  import type { TFunction } from 'i18next';
  import type { Stardom } from '../types';

  /** The player's road to stardom in ARAM: Mayhem: fame, rank and the audition missions left with each band. */
  let { stardom, t, number }: { stardom: Stardom; t: TFunction; number: (value: number) => string } = $props();
  const toNext = $derived(stardom.next ? Math.min(100, (100 * stardom.fame) / stardom.next.fame) : 100);
</script>

<div class="panel cut box stardom">
  <p class="fame">
    <b>{stardom.rank}</b>
    <span class="muted">{t('home:stardom.fame', { fame: number(stardom.fame) })}</span>
  </p>
  <div class="meter"><i style="width:{toNext}%"></i></div>
  {#if stardom.next}
    <small class="muted">{t('home:stardom.next', { points: number(stardom.next.fame - stardom.fame), rank: stardom.next.name })}</small>
  {/if}

  <div class="bands">
    {#each stardom.bands as band (band.name)}
      {@const done = band.auditions.filter((audition) => audition.state === 'done').length}
      <div class="band" class:locked={band.locked}>
        <header>
          {#if band.logo}<img src={band.logo} alt="" />{/if}
          <b>{band.name}</b>
          {#if band.locked}
            <small class="muted"><Lock size={12} /> {t('home:stardom.locked')}</small>
          {:else}
            <small>{t('home:stardom.milestones', { done: band.milestones, total: band.total })}</small>
          {/if}
        </header>
        {#if !band.locked}
          <div class="meter"><i style="width:{band.total ? (100 * band.milestones) / band.total : 0}%"></i></div>
          <ul>
            {#each band.auditions.filter((audition) => audition.state !== 'done') as audition (audition.title)}
              <li title={audition.description}>{audition.title}</li>
            {/each}
            {#if done}<li class="muted"><Check size={12} /> {t('home:stardom.done', { count: done })}</li>{/if}
          </ul>
        {/if}
      </div>
    {/each}
  </div>

  {#if stardom.missions.length}
    <div class="missions">
      <small class="muted">{t('home:stardom.fameMissions')}</small>
      {#each stardom.missions as mission (mission.text)}
        <p><span>{mission.text}</span><b>{mission.progress}/{mission.total} · {mission.reward}</b></p>
      {/each}
    </div>
  {/if}
</div>

<style>
  .stardom {
    display: grid;
    gap: var(--space-3);
    padding: var(--space-4);
  }
  .fame {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
    align-items: baseline;
    margin: 0;
  }
  .fame b {
    font-size: var(--text-xl);
  }
  .meter {
    height: var(--space-1);
    background: var(--color-line);
  }
  .meter i {
    display: block;
    height: 100%;
    background: var(--color-accent);
  }
  .bands {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(var(--size-aside), 1fr));
    gap: var(--space-3);
  }
  .band {
    display: grid;
    align-content: start;
    gap: var(--space-2);
    padding: var(--space-3);
    background: var(--color-panelRaised);
  }
  .band.locked {
    opacity: 0.55;
  }
  header {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
    align-items: center;
  }
  header img {
    height: var(--size-thumbSm);
  }
  header small {
    margin-left: auto;
    font-size: var(--text-xs);
    font-weight: 700;
  }
  ul {
    display: grid;
    gap: var(--space-1);
    margin: 0;
    padding: 0;
    font-size: var(--text-sm);
    list-style: none;
  }
  li::before {
    content: '◆ ';
    color: var(--color-accent);
  }
  li.muted::before {
    content: '';
  }
  .missions {
    display: grid;
    gap: var(--space-1);
  }
  .missions small {
    font-size: var(--text-xs);
    font-weight: 700;
    letter-spacing: var(--tracking-wide);
    text-transform: uppercase;
  }
  .missions p {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
    justify-content: space-between;
    margin: 0;
    font-size: var(--text-sm);
  }
</style>
