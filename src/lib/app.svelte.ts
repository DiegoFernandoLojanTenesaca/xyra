import { onEvent, getChoices, getConfig, getState, setConfig } from './services/engine';
import { getStats } from './services/data';
import { toAppError } from './services/errors';
import { getChampions, getProfile, importBuild } from './services/league';
import { checkUpdate, installUpdate } from './services/updates';
import { BASE_LANGUAGE, formatter, translator } from './i18n';
import type { BuildMode, ChampionInfo, Choices, Config, EngineState, ImportTarget, Position, Profile, Release, StatsSummary } from './types';

export const PAGES = ['home', 'build', 'augments', 'champions', 'stats', 'labels', 'game', 'settings'] as const;
export type Page = (typeof PAGES)[number];

export const SETTINGS_TABS = ['general', 'profile', 'data', 'security', 'help', 'about'] as const;
export type SettingsTab = (typeof SETTINGS_TABS)[number];

const playing = (state: EngineState | null) => state?.champ_select?.champion?.id ?? state?.game?.champion?.id ?? null;

class App {
  state = $state<EngineState>(null!);
  config = $state<Config>(null!);
  choices = $state<Choices>(null!);
  ready = $derived(!!this.state && !!this.config && !!this.choices);
  stats = $state<StatsSummary | null>(null);
  champions = $state<ChampionInfo[]>([]);
  profile = $state<Profile | null>(null);
  page = $state<Page>('home');
  settingsTab = $state<SettingsTab>('general');
  selectedChampion = $state<number | null>(null);
  buildMode = $state<BuildMode>('aram');
  positionOverride = $state<Position | null>(null);
  profileError = $state('');
  newGames = $state(0);
  update = $state<Release | null>(null);
  updateProgress = $state<number | null>(null);
  updateError = $state('');
  checkingUpdate = $state(false);
  language = $derived(this.state?.language ?? BASE_LANGUAGE);
  t = $derived(translator(this.language));
  format = $derived(formatter(this.language));

  init = () => {
    getState().then((state) => (this.state = state));
    getConfig().then((config) => (this.config = config));
    getChoices().then((choices) => (this.choices = choices));
    this.reloadData();
    this.loadProfile();
    this.checkUpdate();
    const listeners = [
      onEvent<EngineState>('state', (next) => {
        if (this.state && next.account !== this.state.account) this.loadProfile();
        const champion = playing(next);
        const changed = champion !== null && champion !== playing(this.state);
        this.state = next;
        if (changed) {
          this.selectedChampion = champion;
          this.followGame();
        }
      }),
      onEvent<number>('updateProgress', (done) => (this.updateProgress = done)),
      onEvent<Config>('config', (next) => (this.config = next)),
      onEvent<null>('data', () => this.reloadData()),
    ];
    return () => listeners.forEach((listener) => void listener.then((stop) => stop()));
  };

  reloadData = async () => {
    const previous = this.stats?.games;
    const stats = await getStats();
    if (previous !== undefined && stats.games > previous && this.page !== 'stats') this.newGames += stats.games - previous;
    this.stats = stats;
    this.champions = await getChampions();
  };

  loadProfile = async () => {
    try {
      this.profile = await getProfile();
      this.profileError = '';
    } catch (error) {
      this.profileError = this.errorText(error);
    }
  };

  saveConfig = async (changes: Partial<Config>) => {
    this.config = await setConfig({ ...this.config, ...changes });
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

  followGame = () => {
    if (!this.state.build_mode) return;
    this.buildMode = this.state.build_mode;
    this.positionOverride = this.state.champ_select?.position ?? null;
  };

  pickDefaultChampion = () => {
    if (this.selectedChampion !== null || !this.champions.length) return;
    const current = this.state.champ_select?.champion ?? this.state.game?.champion;
    this.selectedChampion = current?.id ?? this.stats?.champions[0]?.id ?? this.champions[0].id;
    this.followGame();
  };

  checkUpdate = async () => {
    this.checkingUpdate = true;
    try {
      this.update = await checkUpdate();
      this.updateError = '';
    } catch (error) {
      this.updateError = this.errorText(error);
    }
    this.checkingUpdate = false;
  };

  installUpdate = async () => {
    this.updateProgress = 0;
    try {
      await installUpdate();
    } catch (error) {
      this.updateError = this.errorText(error);
      this.updateProgress = null;
    }
  };

  errorText = (error: unknown) => {
    const failure = toAppError(error);
    return this.t(`errors:${failure.code}`, { detail: 'detail' in failure ? failure.detail : '' });
  };

  importBuild = async (champion: number, target: ImportTarget) => {
    try {
      await importBuild(champion, target, this.buildMode, this.buildMode === 'rift' ? this.positionOverride : null);
      return this.t(`build:imported.${target}`);
    } catch (error) {
      return this.errorText(error);
    }
  };
}

export const app = new App();

export const percent = (part: number, total: number) => (total ? (100 * part) / total : 0);
