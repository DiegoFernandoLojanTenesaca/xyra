import { invoke } from '@tauri-apps/api/core';
import type { Friend, LobbyQueue } from '../types';

export const getLobbyQueues = () => invoke<LobbyQueue[]>('get_lobby_queues');
export const getFriends = () => invoke<Friend[]>('get_friends');
export const createLobby = (queue: number) => invoke<void>('create_lobby', { queue });
export const inviteFriend = (friend: Friend) => invoke<void>('invite_friend', { puuid: friend.puuid, summonerId: friend.summoner_id });
/** Starts or stops looking for a match with the lobby. */
export const searchMatch = (start: boolean) => invoke<void>('search_match', { start });
export const leaveLobby = () => invoke<void>('leave_lobby');
