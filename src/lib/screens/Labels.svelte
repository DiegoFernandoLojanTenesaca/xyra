<script lang="ts">
  import { Play, Volume2 } from '@lucide/svelte';
  import { app } from '../app.svelte';
  import { testOverlay, testVoice } from '../services/engine';
  import Button from '../ui/Button.svelte';
  import PageHeader from '../ui/PageHeader.svelte';
  import ToggleRow from '../ui/ToggleRow.svelte';

  const OFFSET_RANGE = { min: -80, max: 80, step: 2 };
  const SCALE_RANGE = { min: 0.7, max: 1.4, step: 0.05 };

  const t = $derived(app.t);
  const format = $derived(app.format);
  const config = $derived(app.config);
</script>

<PageHeader title={t('common:nav.labels')} subtitle={t('labels:subtitle')}>
  <Button variant="primary" icon={Play} disabled={app.state.phase === 'inGame'} onclick={testOverlay}>{t('home:test')}</Button>
</PageHeader>

<div class="gallery">
  {#each app.choices.label_styles as style (style)}
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
      <b>{format.signed(config.offset_y)}</b>
    </label>
    <label class="range">
      <span>{t('labels:size')}</span>
      <input type="range" {...SCALE_RANGE} value={config.scale} onchange={(e) => app.saveConfig({ scale: +e.currentTarget.value })} />
      <b>{format.percent(config.scale * 100)}</b>
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
    grid-template-columns: repeat(auto-fill, minmax(var(--size-stylePreview), 1fr));
    gap: var(--space-4);
    margin-bottom: var(--space-6);
  }
  .style {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    padding: var(--space-3) var(--space-3) var(--space-4);
    text-align: left;
    transition:
      border-color 0.15s,
      background 0.15s;
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
    padding: var(--space-1) var(--space-2);
    font-size: var(--text-xs);
    font-weight: 700;
    letter-spacing: var(--tracking-wide);
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
    grid-template-columns: var(--size-rangeLabel) 1fr var(--size-rangeValue);
    align-items: center;
    gap: var(--space-3);
    margin-bottom: var(--space-3);
  }
  .range b {
    text-align: right;
    font-variant-numeric: tabular-nums;
  }
</style>
