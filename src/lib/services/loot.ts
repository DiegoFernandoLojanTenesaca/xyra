import { invoke } from '@tauri-apps/api/core';
import type { LootActionKind, LootOutcome, LootSummary, PendingReward } from '../types';

export const getLoot = () => invoke<LootSummary>('get_loot');
export const runLootAction = (kind: LootActionKind) => invoke<LootOutcome>('run_loot_action', { kind });
export const claimReward = (reward: PendingReward, choices: string[]) =>
  invoke<void>('claim_reward', { grantId: reward.grant_id, groupId: reward.group_id, choices });
