import { invoke } from '@tauri-apps/api/core';
import type { AugmentRow, Build, BuildMode, ChampionInfo, GameMode, GameOption, GameSetting, ImportTarget, Position, Profile, SettingValue } from '../types';

export const getChampions = () => invoke<ChampionInfo[]>('get_champions');
export const getAugments = (champion: number, mode: GameMode) => invoke<AugmentRow[]>('get_augments', { champion, mode });
export const getBuild = (champion: number, mode: BuildMode, position: Position | null) => invoke<Build>('get_build', { champion, mode, position });
export const importBuild = (champion: number, target: ImportTarget, mode: BuildMode, position: Position | null) =>
  invoke<void>('import_build', { champion, target, mode, position });
export const getGameSettings = () => invoke<GameSetting[]>('get_game_settings');
export const setGameSetting = (option: GameOption, value: SettingValue) => invoke<void>('set_game_setting', { option, value });
export const getProfile = () => invoke<Profile | null>('get_profile');
