import type { Page, SettingsTab } from './app.svelte';

/** Where each moment of the guide leads: an app page, or a settings tab. */
export type GuideTarget = { page: Page } | { settings: SettingsTab };

/** How Xyra is used, moment by moment of a game; texts live in `help:guide.<id>`. */
export const GUIDE: { id: string; targets: GuideTarget[] }[] = [
  { id: 'before', targets: [{ page: 'lobby' }, { page: 'meta' }] },
  { id: 'select', targets: [{ page: 'game' }, { page: 'build' }] },
  { id: 'game', targets: [{ page: 'labels' }] },
  { id: 'after', targets: [{ page: 'stats' }, { page: 'loot' }] },
  { id: 'phone', targets: [{ settings: 'phone' }] },
];
