<script lang="ts">
  import { ExternalLink, Folder } from '@lucide/svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { app } from '../../app.svelte';
  import Button from '../../components/Button.svelte';
  import { CREATORS, githubProfile, LINKS, openExternal } from '../../project';

  type Answer = { question: string; answer: string };

  const t = $derived(app.t);
  const steps = $derived(Object.values(t('help:steps', { returnObjects: true }) as Record<string, string>));
  const faq = $derived(Object.values(t('help:faq', { returnObjects: true }) as Record<string, Answer>));
</script>

<div class="pair">
  <section class="panel cut box">
    <h3 class="section-title">{t('help:gettingStarted')}</h3>
    <ol>
      {#each steps as step, i (i)}
        <li><span class="diamond number">{i + 1}</span>{step}</li>
      {/each}
    </ol>
  </section>

  <section class="panel cut box">
    <h3 class="section-title">{t('help:creators')}</h3>
    {#each CREATORS as creator (creator.user)}
      <button class="creator" onclick={() => openExternal(githubProfile(creator.user))} title={githubProfile(creator.user)}>
        <img class="diamond" src={creator.photo} alt={creator.name} />
        <span class="who">
          <b>{creator.name}</b>
          {#if !creator.name.startsWith('@')}<small class="muted">@{creator.user}</small>{/if}
        </span>
        <span class="team"><small>{t(`help:roles.${creator.role}`)}</small><b class="accent">{creator.team}</b></span>
      </button>
    {/each}
    <p class="muted thanks">{t('help:thanks')}</p>
  </section>
</div>

<h3 class="section-title">{t('help:faqTitle')}</h3>
<div class="faq">
  {#each faq as entry (entry.question)}
    <details class="panel">
      <summary>{entry.question}</summary>
      <p class="muted">{entry.answer}</p>
    </details>
  {/each}
</div>

<section class="panel cut box support">
  <div>
    <h3 class="section-title">{t('help:supportTitle')}</h3>
    <p class="muted">{t('help:support')}</p>
  </div>
  <div class="buttons">
    <Button variant="primary" icon={ExternalLink} onclick={() => openExternal(LINKS.reportIssue)}>{t('help:report')}</Button>
    <Button icon={Folder} onclick={() => invoke('open_folder', { target: 'log' })}>{t('help:openLog')}</Button>
  </div>
</section>

<style>
  .pair {
    display: grid;
    grid-template-columns: minmax(0, 1.3fr) minmax(0, 1fr);
    gap: var(--space-6);
    margin-bottom: var(--space-6);
    align-items: start;
  }
  .box {
    padding: var(--space-4);
  }
  ol {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: var(--space-3);
  }
  li {
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }
  .number {
    display: grid;
    place-items: center;
    flex: none;
    width: var(--size-thumbSm);
    height: var(--size-thumbSm);
    background: var(--color-accent);
    font-weight: 800;
    font-size: var(--text-md);
  }
  .creator {
    display: flex;
    align-items: center;
    gap: var(--space-4);
    width: 100%;
    padding: var(--space-3) var(--space-2);
    border: none;
    background: none;
    text-align: left;
  }
  .creator:hover {
    background: color-mix(in srgb, var(--color-accent) 8%, transparent);
  }
  .creator + .creator {
    border-top: 1px solid var(--color-line);
  }
  .creator img {
    width: var(--size-portrait);
    height: var(--size-portrait);
    flex: none;
  }
  .who,
  .team {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .who {
    flex: 1;
  }
  .team {
    align-items: flex-end;
  }
  .team small {
    letter-spacing: 2px;
    text-transform: uppercase;
    color: var(--color-textMuted);
  }
  small {
    font-size: var(--text-sm);
  }
  .thanks {
    margin: var(--space-3) 0 0;
    font-size: var(--text-sm);
  }
  .faq {
    display: grid;
    gap: var(--space-2);
    margin-bottom: var(--space-6);
  }
  details {
    padding: 0 var(--space-4);
  }
  details[open] {
    border-color: color-mix(in srgb, var(--color-accent) 45%, transparent);
  }
  summary {
    padding: var(--space-3) 0;
    font-weight: 600;
    cursor: pointer;
    list-style: none;
  }
  summary::before {
    content: '+';
    display: inline-block;
    width: var(--space-5);
    color: var(--color-accentBright);
    font-weight: 800;
  }
  details[open] summary::before {
    content: '–';
  }
  details p {
    margin: 0 0 var(--space-4) var(--space-5);
    line-height: 1.55;
  }
  .support {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-5);
  }
  .support p {
    margin: 0;
  }
  .buttons {
    display: flex;
    gap: var(--space-3);
    flex: none;
  }
</style>
