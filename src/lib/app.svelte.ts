import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import i18next from 'i18next';
import { BASE_LANGUAGE, translator } from './i18n';
import type { AppEvent, BuildMode, ChampionInfo, Config, EngineState, Page, Profile, SettingsTab, StatsSummary } from './types';

const on = <T>(event: AppEvent, handler: (payload: T) => void) => listen<T>(event, (e) => handler(e.payload));

class App {
  state = $state<EngineState>(null!);
  config = $state<Config>(null!);
  ready = $derived(!!this.state && !!this.config);
  stats = $state<StatsSummary | null>(null);
  champions = $state<ChampionInfo[]>([]);
  profile = $state<Profile | null>(null);
  labelStyles = $state<string[]>([]);
  page = $state<Page>('home');
  settingsTab = $state<SettingsTab>('general');
  selectedChampion = $state<number | null>(null);
  buildMode = $state<BuildMode>('aram');
  /** null = the champion's most played position. */
  position = $state<string | null>(null);
  newGames = $state(0);
  language = $derived(this.state?.language ?? BASE_LANGUAGE);
  t = $derived(translator(this.language));

  init = () => {
    invoke<EngineState>('get_state').then((s) => (this.state = s));
    invoke<Config>('get_config').then((c) => (this.config = c));
    invoke<string[]>('get_label_styles').then((s) => (this.labelStyles = s));
    this.reloadData();
    this.loadProfile();
    const listeners = [
      on<EngineState>('state', (next) => {
        if (this.state?.phase === 'no_client' && next.phase !== 'no_client') this.loadProfile();
        this.state = next;
      }),
      on<Config>('config', (next) => (this.config = next)),
      on<null>('stats', () => this.reloadData()),
    ];
    return () => listeners.forEach((l) => void l.then((stop) => stop()));
  };

  reloadData = async () => {
    const previous = this.stats?.games;
    const stats = await invoke<StatsSummary>('get_stats');
    if (previous !== undefined && stats.games > previous && this.page !== 'stats') this.newGames += stats.games - previous;
    this.stats = stats;
    this.champions = await invoke<ChampionInfo[]>('get_champions');
  };

  loadProfile = async () => {
    this.profile = await invoke<Profile | null>('get_profile').catch(() => this.profile);
  };

  saveConfig = async (changes: Partial<Config>) => {
    this.config = await invoke<Config>('set_config', { config: { ...this.config, ...changes } });
  };

  goTo = (page: Page) => {
    this.page = page;
    if (page === 'stats') this.newGames = 0;
  };

  openAugments = (champion: number) => {
    this.selectedChampion = champion;
    this.goTo('augments');
  };

  openBuild = (champion: number) => {
    this.selectedChampion = champion;
    this.followGame();
    this.goTo('build');
  };

  openSettings = (tab: SettingsTab) => {
    this.settingsTab = tab;
    this.goTo('settings');
  };

  /** Build mode and position from the current champion select or game (Summoner's Rift for normals and ranked). */
  followGame = () => {
    const mode = this.state.champ_select?.mode || this.state.mode;
    if (!mode) return;
    this.buildMode = mode === 'CLASSIC' ? 'rift' : 'aram';
    this.position = this.state.champ_select?.position ?? null;
  };

  /** Champion select pick, current game champion, most played or the tier list leader. */
  pickDefaultChampion = () => {
    if (this.selectedChampion !== null || !this.champions.length) return;
    const inGame = this.champions.find((c) => c.name === this.state.champion)?.id;
    this.selectedChampion = this.state.champ_select?.champion?.id ?? inGame ?? this.stats?.champions[0]?.id ?? this.champions[0].id;
    this.followGame();
  };

  errorText = (error: unknown) => {
    const key = `common:errors.${String(error)}`;
    return i18next.exists(key, { lng: this.language }) ? this.t(key) : this.t('common:errors.unknown', { detail: String(error) });
  };

  importBuild = async (champion: number, target: 'runes' | 'items') => {
    try {
      await invoke('import_build', { champion, target, mode: this.buildMode, position: this.position });
      return this.t(target === 'runes' ? 'build:runesImported' : 'build:itemsImported');
    } catch (error) {
      return this.errorText(error);
    }
  };
}

export const app = new App();

export const percent = (part: number, total: number) => (total ? Math.round((100 * part) / total) : 0);
