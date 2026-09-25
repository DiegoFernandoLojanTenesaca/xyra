<script lang="ts">
  import { Play, Volume2 } from '@lucide/svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { app } from '../app.svelte';
  import Button from '../components/Button.svelte';
  import PageHeader from '../components/PageHeader.svelte';
  import ToggleRow from '../components/ToggleRow.svelte';

  const OFFSET_RANGE = { min: -80, max: 80, step: 2 };
  const SCALE_RANGE = { min: 0.7, max: 1.4, step: 0.05 };

  const t = $derived(app.t);
  const config = $derived(app.config);

  function testVoice() {
    const utterance = new SpeechSynthesisUtterance(t('overlay:voice.right'));
    utterance.lang = app.language;
    speechSynthesis.cancel();
    speechSynthesis.speak(utterance);
  }
</script>

<PageHeader title={t('common:nav.labels')} subtitle={t('labels:subtitle')}>
  <Button variant="primary" icon={Play} disabled={app.state.phase === 'in_game'} onclick={() => invoke('test_overlay')}>{t('home:test')}</Button>
</PageHeader>

<div class="gallery">
  {#each app.labelStyles as style (style)}
    <button class="style panel cut" class:active={config.label_style === style} onclick={() => app.saveConfig({ label_style: style })}>
      <img src="/styles/{style}.jpg" alt={t(`labels:styles.${style}.name`)} />
      <div class="name">
        <b class="condensed">{t(`labels:styles.${style}.name`)}</b>
        {#if config.label_style === style}<span class="tag">{t('labels:active')}</span>{/if}
      </div>
      <small class="muted">{t(`labels:styles.${style}.description`)}</small>
    </button>
  {/each}
</div>

<div class="pair">
  <section class="panel cut box">
    <h3 class="section-title">{t('labels:calibration')}</h3>
    <p class="muted">{t('labels:calibrationHint')}</p>
    <label class="range">
      <span>{t('labels:height')}</span>
      <input type="range" {...OFFSET_RANGE} value={config.offset_y} onchange={(e) => app.saveConfig({ offset_y: +e.currentTarget.value })} />
      <b>{config.offset_y > 0 ? '+' : ''}{config.offset_y}</b>
    </label>
    <label class="range">
      <span>{t('labels:size')}</span>
      <input type="range" {...SCALE_RANGE} value={config.scale} onchange={(e) => app.saveConfig({ scale: +e.currentTarget.value })} />
      <b>{Math.round(config.scale * 100)}%</b>
    </label>
  </section>

  <section class="panel cut box">
    <h3 class="section-title">{t('labels:voice')}</h3>
    <ToggleRow title={t('labels:voice')} description={t('labels:voiceHint')} checked={config.voice} onchange={() => app.saveConfig({ voice: !config.voice })} />
    <Button icon={Volume2} onclick={testVoice}>{t('labels:testVoice')}</Button>
  </section>
</div>

<style>
  .gallery {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
    gap: var(--space-4);
    margin-bottom: var(--space-6);
  }
  .style {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    padding: var(--space-3) var(--space-3) var(--space-4);
    text-align: left;
    transition: border-color 0.15s, background 0.15s;
  }
  .style:hover {
    border-color: var(--color-lineStrong);
  }
  .style.active {
    border-color: var(--color-accent);
    background: linear-gradient(180deg, color-mix(in srgb, var(--color-accent) 16%, transparent), transparent 60%), var(--color-panel);
  }
  .style img {
    width: 100%;
    aspect-ratio: 16 / 9;
    object-fit: cover;
    object-position: center 60%;
  }
  .name {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: 0 var(--space-1);
  }
  .name b {
    font-size: var(--text-xl);
  }
  .style small {
    padding: 0 var(--space-1);
  }
  .tag {
    padding: 2px var(--space-2);
    font-size: var(--text-xs);
    font-weight: 700;
    letter-spacing: 2px;
    text-transform: uppercase;
    background: var(--color-accent);
  }
  .pair {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
    gap: var(--space-6);
  }
  .box {
    padding: var(--space-4);
  }
  .box p {
    margin: 0 0 var(--space-4);
  }
  .range {
    display: grid;
    grid-template-columns: 72px 1fr 48px;
    align-items: center;
    gap: var(--space-3);
    margin-bottom: var(--space-3);
  }
  .range b {
    text-align: right;
    font-variant-numeric: tabular-nums;
  }
</style>
