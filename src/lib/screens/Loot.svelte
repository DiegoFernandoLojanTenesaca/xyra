<script lang="ts">
  import { Gem, Gift, ShieldCheck } from '@lucide/svelte';
  import { app } from '../app.svelte';
  import { claimReward, getLoot, runLootAction } from '../services/loot';
  import { resource } from '../services/resource.svelte';
  import type { LootAction, LootActionKind, PendingReward } from '../types';
  import Button from '../ui/Button.svelte';
  import EmptyState from '../ui/EmptyState.svelte';
  import PageHeader from '../ui/PageHeader.svelte';
  import Skeleton from '../ui/Skeleton.svelte';
  import StatTile from '../ui/StatTile.svelte';

  const t = $derived(app.t);
  const format = $derived(app.format);
  /** Changing it reads the loot again, after an action changed it. */
  let reads = $state(0);
  const loot = resource(() => (app.state.account ? [app.state.account, reads] : null), getLoot);
  let busy = $state(false);
  let notice = $state('');
  /** The irreversible action waiting for the player's yes. */
  let confirming = $state<LootActionKind | null>(null);
  /** The choices picked for each reward that offers several. */
  let picked = $state<Record<string, string[]>>({});

  async function act(action: LootAction) {
    if (action.kind === 'disenchantChampionShards' && confirming !== action.kind) {
      confirming = action.kind;
      return;
    }
    confirming = null;
    busy = true;
    try {
      const outcome = await runLootAction(action.kind);
      const items = outcome.gained.map((item) => (item.count > 1 ? `${item.name} ×${item.count}` : item.name)).join(', ');
      notice = [items ? t('loot:gained', { items }) : t('loot:done'), outcome.stopped ? t('loot:stopped', { reason: outcome.stopped }) : '']
        .filter(Boolean)
        .join(' ');
    } catch (error) {
      notice = app.errorText(error);
    }
    busy = false;
    reads++;
  }

  const choicesOf = (reward: PendingReward) =>
    reward.choices.length <= reward.picks ? reward.choices.map((choice) => choice.id) : (picked[reward.grant_id] ?? []);

  function toggle(reward: PendingReward, id: string) {
    const current = picked[reward.grant_id] ?? [];
    const next = current.includes(id) ? current.filter((choice) => choice !== id) : [...current, id].slice(-reward.picks);
    picked = { ...picked, [reward.grant_id]: next };
  }

  async function claim(reward: PendingReward) {
    busy = true;
    notice = await claimReward(reward, choicesOf(reward)).then(() => t('loot:claimed'), app.errorText);
    busy = false;
    reads++;
  }
</script>

<PageHeader title={t('common:nav.loot')} subtitle={t('loot:subtitle')} />

{#if !app.state.account}
  <EmptyState icon={Gem} text={t('lobby:noClient')} />
{:else if loot.error}
  <EmptyState icon={Gem} text={app.errorText(loot.error)} />
{:else if !loot.value}
  <Skeleton label={t('loot:loading')} rows={5} />
{:else}
  {@const summary = loot.value}
  {#if notice}<p class="notice">{notice}</p>{/if}
  <div class="tiles">
    <StatTile label={t('loot:blueEssence')} value={summary.blue_essence} format={format.number} accent />
    <StatTile label={t('loot:orangeEssence')} value={summary.orange_essence} format={format.number} />
    <StatTile label={t('loot:chests')} value={summary.chests} format={format.number} />
    <StatTile label={t('loot:keys')} value={summary.keys} format={format.number} />
  </div>

  {#if summary.rewards.length}
    <h3 class="section-title"><Gift size={14} />{t('loot:rewards')}</h3>
    <div class="cards">
      {#each summary.rewards as reward (reward.grant_id)}
        {@const choosing = reward.choices.length > reward.picks}
        <div class="panel cut box card">
          {#if choosing}<small class="muted">{t('loot:pick', { count: reward.picks })}</small>{/if}
          <div class="choices">
            {#each reward.choices as choice (choice.id)}
              <label class="choice" class:chosen={choicesOf(reward).includes(choice.id)}>
                {#if choosing}<input type="checkbox" checked={choicesOf(reward).includes(choice.id)} onchange={() => toggle(reward, choice.id)} />{/if}
                {#if choice.icon}<img src={choice.icon} alt="" />{/if}
                <b>{choice.name}</b>
              </label>
            {/each}
          </div>
          <Button
            variant="primary"
            icon={Gift}
            disabled={busy || choicesOf(reward).length !== Math.min(reward.picks, reward.choices.length)}
            onclick={() => claim(reward)}>{t('loot:claim')}</Button
          >
        </div>
      {/each}
    </div>
  {/if}

  <h3 class="section-title">{t('loot:actions')}</h3>
  {#if summary.actions.length}
    <div class="cards">
      {#each summary.actions as action (action.kind)}
        <div class="panel cut box card">
          <b>{t(`loot:kinds.${action.kind}.title`, { count: action.times })}</b>
          <small class="muted">{t(`loot:kinds.${action.kind}.detail`, { essence: format.number(action.essence) })}</small>
          <div class="items">
            {#each action.items as item, i (i)}
              <span class="item" title={item.name}
                >{#if item.icon}<img src={item.icon} alt="" />{/if}<small>×{item.count}</small></span
              >
            {/each}
          </div>
          {#if confirming === action.kind}
            <p class="confirm">{t('loot:confirm')}</p>
            <div class="buttons">
              <Button variant="danger" disabled={busy} onclick={() => act(action)}>{t('loot:confirmYes')}</Button>
              <Button onclick={() => (confirming = null)}>{t('loot:cancel')}</Button>
            </div>
          {:else}
            <Button variant={action.kind === 'disenchantChampionShards' ? 'default' : 'primary'} disabled={busy} onclick={() => act(action)}
              >{t(`loot:kinds.${action.kind}.button`)}</Button
            >
          {/if}
        </div>
      {/each}
    </div>
  {:else if !summary.rewards.length}
    <p class="muted panel cut box">{t('loot:nothing')}</p>
  {/if}
  <p class="muted note"><ShieldCheck size={14} />{t('loot:safe')}</p>
{/if}

<style>
  .notice {
    margin: calc(-1 * var(--space-2)) 0 var(--space-4);
    color: var(--color-success);
    font-weight: 600;
  }
  .tiles {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: var(--space-4);
    margin-bottom: var(--space-6);
  }
  .cards {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(var(--size-aside), 1fr));
    gap: var(--space-4);
    margin-bottom: var(--space-6);
  }
  .card {
    display: grid;
    align-content: start;
    gap: var(--space-3);
  }
  .choices,
  .items {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
  }
  .choice {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-3);
    border: var(--border-hairline) solid var(--color-line);
    background: var(--color-panelRaised);
  }
  .choice.chosen {
    border-color: var(--color-accent);
  }
  .choice img,
  .item img {
    width: var(--size-thumb);
    height: var(--size-thumb);
  }
  .item {
    position: relative;
  }
  .item small {
    position: absolute;
    right: 0;
    bottom: 0;
    padding: 0 var(--space-1);
    background: var(--color-background);
    font-size: var(--text-xs);
    font-weight: 700;
  }
  .confirm {
    margin: 0;
    color: var(--color-warning);
    font-weight: 600;
  }
  .buttons {
    display: flex;
    gap: var(--space-2);
  }
  .note {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    font-size: var(--text-sm);
  }
</style>
