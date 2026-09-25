<script lang="ts">
  import { CircleQuestionMark, Download, ExternalLink, Square } from '@lucide/svelte';
  import { app } from '../../app.svelte';
  import { APP_NAME, CREATORS, githubHandle, githubProfile, LINKS, openExternal } from '../../project';
  import Button from '../../ui/Button.svelte';
  import Logo from '../../ui/Logo.svelte';

  const t = $derived(app.t);
  const changes = $derived(Object.values(t('about:changes', { returnObjects: true }) as Record<string, string>));
</script>

<section class="card highlight cut">
  <Logo size={92} phase={app.state.phase} />
  <div class="text">
    <h2 class="condensed">{APP_NAME} <small>v{app.state.version}</small></h2>
    <p class="muted facts"><span>{t('common:tagline')}</span><span>{t('about:license')}</span></p>
  </div>
  <div class="signatures">
    <small class="muted">{t('about:madeBy')}</small>
    {#each CREATORS as creator (creator.user)}
      <button onclick={() => openExternal(githubProfile(creator.user))} title={githubProfile(creator.user)}>
        <img class="diamond" src={creator.photo} alt="" />
        <span><b>{githubHandle(creator.user)}</b><small class="accent">{creator.team}</small></span>
      </button>
    {/each}
  </div>
</section>

<div class="pair">
  <section class="panel cut box">
    <h3 class="section-title">{t('about:whatsNew')}</h3>
    <ul>
      {#each changes as change (change)}
        <li><span class="accent"><Square size={10} fill="currentColor" /></span>{change}</li>
      {/each}
    </ul>
    <p class="muted footnote">{t('about:builtWith')}</p>
  </section>

  <section class="panel cut box">
    <h3 class="section-title">{t('about:links')}</h3>
    <div class="links">
      <Button icon={ExternalLink} onclick={() => openExternal(LINKS.repository)}>{t('help:viewCode')}</Button>
      <Button icon={Download} onclick={() => openExternal(LINKS.releases)}>{t('help:releases')}</Button>
      <Button icon={ExternalLink} onclick={() => openExternal(LINKS.reportIssue)}>{t('help:report')}</Button>
      <Button icon={CircleQuestionMark} onclick={() => app.openSettings('help')}>{t('settings:tabs.help')}</Button>
    </div>
    <p class="muted footnote">{t('about:sources')}</p>
  </section>
</div>

<p class="muted legal">{t('about:riot')}</p>

<style>
  .card {
    display: flex;
    align-items: center;
    gap: var(--space-6);
    padding: var(--space-6);
    margin-bottom: var(--space-6);
  }
  .text {
    flex: 1;
  }
  h2 {
    margin: 0;
    font-size: var(--text-3xl);
    font-variation-settings: 'wdth' 78;
  }
  h2 small {
    font-size: var(--text-xl);
    color: var(--color-textMuted);
    font-weight: 400;
  }
  .card p {
    margin: var(--space-2) 0 0;
  }
  .signatures {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    z-index: 1;
  }
  .signatures > small {
    font-size: var(--text-xs);
    letter-spacing: var(--tracking-wide);
    text-transform: uppercase;
  }
  .signatures button {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-1) var(--space-2) var(--space-1) var(--space-1);
    border: none;
    background: rgb(0 0 0 / 35%);
    text-align: left;
  }
  .signatures button:hover {
    background: color-mix(in srgb, var(--color-accent) 14%, transparent);
  }
  .signatures img {
    width: var(--size-thumbMd);
    height: var(--size-thumbMd);
  }
  .signatures span {
    display: flex;
    flex-direction: column;
    line-height: 1.2;
  }
  .signatures span small {
    font-size: var(--text-xs);
    font-weight: 700;
    letter-spacing: var(--tracking-normal);
  }
  .pair {
    display: grid;
    grid-template-columns: minmax(0, 1.3fr) minmax(0, 1fr);
    gap: var(--space-6);
    margin-bottom: var(--space-5);
    align-items: start;
  }
  .box {
    padding: var(--space-4);
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: var(--space-3);
  }
  li {
    display: flex;
    gap: var(--space-3);
  }
  .footnote {
    margin: var(--space-4) 0 0;
    padding-top: var(--space-3);
    border-top: var(--border-hairline) solid var(--color-line);
    font-size: var(--text-sm);
  }
  .links {
    display: grid;
    gap: var(--space-2);
  }
  .legal {
    font-size: var(--text-sm);
    max-width: var(--size-prose);
  }
</style>
