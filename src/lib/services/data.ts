import { invoke } from '@tauri-apps/api/core';
import type { DataFolder, DataUsage, StatsSummary } from '../types';

export const getStats = () => invoke<StatsSummary>('get_stats');
export const getDataUsage = () => invoke<DataUsage>('get_data_usage');
export const exportCsv = () => invoke<string>('export_csv');
export const deleteData = () => invoke<void>('delete_data');
export const openFolder = (folder: DataFolder) => invoke<void>('open_folder', { folder });
