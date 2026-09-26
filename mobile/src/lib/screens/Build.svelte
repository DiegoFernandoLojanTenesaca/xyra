<script lang="ts">
  import { Download, Hammer } from '@lucide/svelte';
  import { resource } from '$shared/services/resource.svelte';
  import type { Asset, Build, BuildMode, ChampionInfo, Position } from '$shared/types';
  import EmptyState from '$shared/ui/EmptyState.svelte';
  import SearchInput from '$shared/ui/SearchInput.svelte';
  import SegmentedControl from '$shared/ui/SegmentedControl.svelte';
  import Skeleton from '$shared/ui/Skeleton.svelte';
  import { link } from '../link.svelte';
  import { mobile } from '../mobile.svelte';

  const MODES: BuildMode[] = ['aram', 'rift'];
  const POSITIONS: Position[] = ['top', 'jungle', 'mid', 'adc', 'support'];
  const IMPORT_TARGETS = ['runes', 'items', 'spells'] as const;
  const MAX_SUGGESTIONS = 8;
  const RATE_DIGITS = 1;

  const t = $derived(mobile.t);
  const format = $derived(mobile.format);
  const engine = $derived(link.state);
  const live = $derived(engine?.champ_select?.champion ?? engine?.game?.champion ?? null);
  const champion = $derived(mobile.buildChampion ?? live?.id ?? null);
  const mode = $derived(mobile.buildMode ?? engine?.build_mode ?? 'aram');
  const position = $derived(mode === 'rift' ? (mobile.buildPosition ?? engine?.champ_select?.position ?? null) : null);
  const canImport = $derived(!!engine?.champ_select?.champion && engine.champ_select.champion.id === champion);
  let query = $state('');
  let notice = $state('');
  let busy = $state(false);

  const champions = resource(
    () => (link.status === 'online' ? true : null),
    () => link.get<ChampionInfo[]>('/api/champions'),
    (failure) => failure,
  );
  const build = resource(
    () => (champion === null ? null : { champion, mode, position }),
    (key) => link.call<Build>('GET', '/api/build', key),
    (failure) => failure,
  );
  const current = $derived(build.value);
  const info = $derived(champions.value?.find((c) => c.id === champion) ?? null);
  const suggestions = $derived(
    query.trim() ? (champions.value ?? []).filter((c) => c.name.toLowerCase().includes(query.trim().toLowerCase())).slice(0, MAX_SUGGESTIONS) : [],
  );
  const itemBlocks = $derived(
    current
      ? ([
          ['starting', current.starting_items],
          ['boots', current.boots],
          ['core', current.core_items],
          ['situational', current.situational_items],
        ] as const)
      : [],
  );

  function pick(id: number) {
    mobile.buildChampion = id;
    query = '';
  }

  async function importTo(target: (typeof IMPORT_TARGETS)[number]) {
    busy = true;
    notice = await mobile.act('POST', '/api/import', { target }, `build:imported.${target}`);
    busy = false;
  }
</script>

{#snippet icons(assets: Asset[])}
  <div class="chips">
    {#each assets as asset, i (`${asset.id}-${i}`)}
      {#if asset.icon}<img src={asset.icon} alt={asset.name} title={asset.name} />{/if}
    {/each}
  </div>
{/snippet}

<SearchInput bind:value={query} placeholder={t('common:searchChampion')} />
{#if suggestions.length}
  <ul class="panel search-list">
    {#each suggestions as suggestion (suggestion.id)}
      <li>
        <button onclick={() => pick(suggestion.id)}>
          {#if suggestion.icon}<img src={suggestion.icon} alt="" />{/if}<span>{suggestion.name}</span>
        </button>
      </li>
    {/each}
  </ul>
{/if}

<div class="tools">
  <SegmentedControl
    options={MODES.map((m) => [m, t(`build:modes.${m}`)] as [BuildMode, string])}
    bind:value={() => mode, (next) => (mobile.buildMode = next)}
  />
  {#if mode === 'rift'}
    <SegmentedControl
      options={POSITIONS.map((p) => [p, t(`build:positions.${p}`)] as [Position, string])}
      bind:value={() => position ?? current?.position ?? 'mid', (next) => (mobile.buildPosition = next)}
    />
  {/if}
</div>

{#if champion === null}
  <EmptyState icon={Hammer} text={t('mobile:build.pick')} />
{:else if build.error}
  <EmptyState icon={Hammer} text={mobile.errorText(build.error)} />
{:else if !current}
  <Skeleton label={t('build:loading')} rows={6} />
{:else}
  <section class="block panel cut">
    <div class="row">
      {#if info?.icon}<img src={info.icon} alt="" />{/if}
      <span class="grow"><b>{info?.name ?? ''}</b></span>
      <small class="muted">{t('common:winRate', { value: format.percent(current.win_rate, RATE_DIGITS) })}</small>
    </div>
    {#if canImport}
      <div class="actions">
        {#each IMPORT_TARGETS as target (target)}
          <button class="action" disabled={busy} onclick={() => importTo(target)}><Download size={14} />{t(`mobile:live.import.${target}`)}</button>
        {/each}
      </div>
    {/if}
  </section>
  {#if notice}<p class="notice">{notice}</p>{/if}

  <section class="block panel cut">
    <h2 class="section-title">{t('build:runes')}</h2>
    <div class="runes">
      {@render icons([current.runes.primary_style, ...current.runes.primary])}
      {@render icons([current.runes.secondary_style, ...current.runes.secondary])}
      {@render icons(current.runes.shards)}
    </div>
  </section>

  <section class="block panel cut">
    <h2 class="section-title">{t('build:spells')}</h2>
    {@render icons(current.spells)}
  </section>

  <section class="block panel cut">
    <h2 class="section-title">{t('build:items')}</h2>
    {#each itemBlocks as [block, items] (block)}
      {#if items.length}
        <div class="item-block">
          <small class="muted">{t(`build:blocks.${block}`)}</small>
          {@render icons(items)}
        </div>
      {/if}
    {/each}
  </section>

  {#if current.skill_order.length}
    <section class="block panel cut">
      <h2 class="section-title">{t('build:skills')}</h2>
      {#if current.skill_priority.length}<p class="priority">{t('build:maxFirst')}: <b class="accent">{current.skill_priority.join(' › ')}</b></p>{/if}
      <div class="order">
        {#each current.skill_order as skill, i (i)}<span class:ult={skill === 'R'}>{skill}</span>{/each}
      </div>
    </section>
  {/if}
{/if}

<style>
  .tools {
    display: grid;
    gap: var(--space-2);
    margin: var(--space-3) 0 var(--space-4);
  }
  .tools :global([role='tablist']) {
    overflow-x: auto;
  }
  .runes {
    display: grid;
    gap: var(--space-3);
  }
  .item-block {
    display: grid;
    gap: var(--space-2);
    margin-bottom: var(--space-3);
  }
  .priority {
    margin: 0 0 var(--space-3);
    font-size: var(--text-md);
  }
  .order {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-1);
  }
  .order span {
    display: grid;
    place-items: center;
    width: var(--size-thumbSm);
    height: var(--size-thumbSm);
    background: var(--color-panelRaised);
    font-size: var(--text-sm);
    font-weight: 700;
  }
  .order span.ult {
    background: var(--color-accent);
    color: var(--color-white);
  }
</style>
