import { invoke } from '@tauri-apps/api/core';
import type { Release } from '../types';

export const checkUpdate = () => invoke<Release | null>('check_update');
export const installUpdate = () => invoke<void>('install_update');
