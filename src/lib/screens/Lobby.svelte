<script lang="ts">
  import { Crown, LogOut, RefreshCw, Search, UserPlus, Users, X } from '@lucide/svelte';
  import { app } from '../app.svelte';
  import { createLobby, getFriends, getLobbyQueues, inviteFriend, leaveLobby, searchMatch } from '../services/lobby';
  import { resource } from '../services/resource.svelte';
  import type { Friend } from '../types';
  import Button from '../ui/Button.svelte';
  import EmptyState from '../ui/EmptyState.svelte';
  import PageHeader from '../ui/PageHeader.svelte';
  import Skeleton from '../ui/Skeleton.svelte';

  const t = $derived(app.t);
  const engine = $derived(app.state);
  const lobby = $derived(engine.lobby);
  /** Changing it reads the friends again. */
  let friendsRead = $state(0);
  let busy = $state(false);
  let notice = $state('');

  const queues = resource(() => engine.account, getLobbyQueues);
  const friends = resource(() => (engine.account ? [engine.account, friendsRead] : null), getFriends);
  const queueName = (id: number) => queues.value?.find((queue) => queue.id === id)?.name ?? `#${id}`;
  /** A friend already in the lobby or invited to it. */
  const joined = (friend: Friend) => lobby?.players.includes(friend.puuid) ?? false;

  async function act(action: () => Promise<void>, done = '') {
    busy = true;
    notice = await action().then(() => done, app.errorText);
    busy = false;
  }
</script>

<PageHeader title={t('common:nav.lobby')} subtitle={t('lobby:subtitle')} />

{#if engine.phase === 'noClient' || !engine.account}
  <EmptyState icon={Users} text={t('lobby:noClient')} />
{:else}
  {#if notice}<p class="notice">{notice}</p>{/if}
  <div class="grid">
    <section class="panel cut box">
      {#if lobby}
        <h3 class="section-title">{t('lobby:yourLobby', { queue: queueName(lobby.queue_id) })}</h3>
        <div class="members">
          {#each lobby.members as member, i (i)}
            <div class="member">
              <img class="diamond" src={member.icon} alt="" />
              <b>{member.name || '—'}</b>
              {#if member.leader}<small class="accent"><Crown size={12} />{t('lobby:leader')}</small>{/if}
            </div>
          {/each}
        </div>
        {#if lobby.invited.length}<p class="muted">{t('lobby:invited', { names: lobby.invited.join(', ') })}</p>{/if}
        <div class="actions">
          {#if lobby.searching}
            <p class="searching"><Search size={16} />{t('lobby:searching')}</p>
            <Button icon={X} disabled={busy} onclick={() => act(() => searchMatch(false))}>{t('lobby:cancel')}</Button>
          {:else if lobby.leader}
            <Button variant="primary" icon={Search} disabled={busy || !lobby.can_search} onclick={() => act(() => searchMatch(true))}
              >{t('lobby:search')}</Button
            >
          {:else}
            <p class="muted">{t('lobby:onlyLeader')}</p>
          {/if}
          <Button icon={LogOut} disabled={busy || lobby.searching} onclick={() => act(leaveLobby)}>{t('lobby:leave')}</Button>
        </div>
      {:else}
        <h3 class="section-title">{t('lobby:chooseMode')}</h3>
        {#if queues.error}
          <p class="muted">{app.errorText(queues.error)}</p>
        {:else if !queues.value}
          <Skeleton label={t('lobby:loading')} rows={3} />
        {:else}
          <div class="queues">
            {#each queues.value as queue (queue.id)}
              <button class="queue cut-sm" disabled={busy} onclick={() => act(() => createLobby(queue.id), t('lobby:created'))}>
                <b>{queue.name}</b>
                {#if queue.ranked}<small class="accent">{t('lobby:ranked')}</small>{/if}
              </button>
            {/each}
          </div>
        {/if}
      {/if}
    </section>

    <section class="panel cut box">
      <h3 class="section-title">
        {t('lobby:friends')}
        <button class="refresh" title={t('lobby:refresh')} aria-label={t('lobby:refresh')} onclick={() => friendsRead++}><RefreshCw size={14} /></button>
      </h3>
      {#if friends.error}
        <p class="muted">{app.errorText(friends.error)}</p>
      {:else if !friends.value}
        <Skeleton label={t('lobby:loading')} rows={4} />
      {:else}
        <div class="friends">
          {#each friends.value as friend (friend.puuid)}
            <div class="friend">
              <img class="diamond" src={friend.icon} alt="" />
              <span class="who">
                <b>{friend.name}</b>
                <small class="status {friend.status}">{t(`lobby:status.${friend.status}`)}</small>
              </span>
              {#if joined(friend)}
                <small class="muted">{t('lobby:alreadyIn')}</small>
              {:else}
                <Button
                  icon={UserPlus}
                  disabled={busy || !lobby || !friend.can_join}
                  title={lobby ? '' : t('lobby:createFirst')}
                  onclick={() => act(() => inviteFriend(friend), t('lobby:inviteSent', { name: friend.name }))}>{t('lobby:invite')}</Button
                >
              {/if}
            </div>
          {:else}
            <p class="muted">{t('lobby:noFriends')}</p>
          {/each}
        </div>
      {/if}
    </section>
  </div>
{/if}

<style>
  .notice {
    margin: calc(-1 * var(--space-2)) 0 var(--space-4);
    color: var(--color-success);
    font-weight: 600;
  }
  .grid {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
    gap: var(--space-6);
    align-items: start;
  }
  .members,
  .friends {
    display: grid;
    gap: var(--space-2);
  }
  .member,
  .friend {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-2) var(--space-3);
    background: var(--color-panelRaised);
  }
  .member img,
  .friend img {
    width: var(--size-thumb);
    height: var(--size-thumb);
  }
  .member small {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    margin-left: auto;
    font-size: var(--text-xs);
    font-weight: 700;
    text-transform: uppercase;
  }
  .who {
    display: grid;
    flex: 1;
    min-width: 0;
  }
  .status {
    color: var(--color-success);
    font-size: var(--text-xs);
  }
  .status.away,
  .status.busy {
    color: var(--color-warning);
  }
  .status.inQueue,
  .status.champSelect,
  .status.inGame {
    color: var(--color-textMuted);
  }
  .actions {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-3);
    margin-top: var(--space-4);
  }
  .searching {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    margin: 0;
    color: var(--color-accentBright);
    font-weight: 700;
  }
  .queues {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(var(--size-meter), 1fr));
    gap: var(--space-3);
  }
  .queue {
    display: grid;
    gap: var(--space-1);
    padding: var(--space-4);
    border: var(--border-hairline) solid var(--color-line);
    background: var(--color-panelRaised);
    text-align: left;
  }
  .queue:hover:not(:disabled) {
    border-color: var(--color-accent);
  }
  .queue small {
    font-size: var(--text-xs);
    font-weight: 700;
    text-transform: uppercase;
  }
  .refresh {
    margin-left: auto;
    border: none;
    background: none;
    color: var(--color-textMuted);
  }
  .refresh:hover {
    color: var(--color-text);
  }
</style>
