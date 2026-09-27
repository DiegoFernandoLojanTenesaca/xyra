import { invoke } from '@tauri-apps/api/core';
import type {
  AugmentRow,
  Build,
  BuildMode,
  ChampionInfo,
  GameMode,
  GameOption,
  GameSetting,
  ImportTarget,
  Challenges,
  MasteryProgress,
  MatchSummary,
  Meta,
  MetaChampion,
  PatchChanges,
  Position,
  Profile,
  SettingValue,
} from '../types';

export const getChampions = () => invoke<ChampionInfo[]>('get_champions');
export const getAugments = (champion: number, mode: GameMode) => invoke<AugmentRow[]>('get_augments', { champion, mode });
export const getMeta = () => invoke<Meta>('get_meta');
export const getRecentMatches = () => invoke<MatchSummary[]>('get_recent_matches');
export const getMasteries = () => invoke<MasteryProgress[]>('get_masteries');
export const getChallenges = () => invoke<Challenges>('get_challenges');
export const getModeChampions = (mode: GameMode) => invoke<MetaChampion[]>('get_mode_champions', { mode });
export const getPatchChanges = () => invoke<PatchChanges>('get_patch_changes');
export const takeBenchPick = () => invoke<void>('take_bench_pick');
export const getBuild = (champion: number, mode: BuildMode, position: Position | null) => invoke<Build>('get_build', { champion, mode, position });
export const importBuild = (champion: number, target: ImportTarget, mode: BuildMode, position: Position | null) =>
  invoke<void>('import_build', { champion, target, mode, position });
export const importWholeBuild = (champion: number, mode: BuildMode, position: Position | null) =>
  invoke<void>('import_whole_build', { champion, mode, position });
export const getGameSettings = () => invoke<GameSetting[]>('get_game_settings');
export const setGameSetting = (option: GameOption, value: SettingValue) => invoke<void>('set_game_setting', { option, value });
export const getProfile = () => invoke<Profile | null>('get_profile');
