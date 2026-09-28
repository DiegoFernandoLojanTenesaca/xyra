import { onEvent, getChoices, getConfig, getState, setConfig } from './services/engine';
import { getLpGames, getStats } from './services/data';
import { toAppError } from './services/errors';
import { getChampions, getProfile, importBuild, importWholeBuild } from './services/league';
import { checkUpdate, installUpdate } from './services/updates';
import { BASE_LANGUAGE, formatter, translator } from './i18n';
import type { BuildMode, ChampionInfo, Choices, Config, EngineState, GameMode, ImportTarget, LpGame, Position, Profile, Release, StatsSummary } from './types';

export const PAGES = [
  'home',
  'lobby',
  'build',
  'augments',
  'champions',
  'meta',
  'stats',
  'history',
  'mastery',
  'challenges',
  'loot',
  'labels',
  'game',
  'settings',
] as const;
export type Page = (typeof PAGES)[number];

export const SETTINGS_TABS = ['general', 'phone', 'profile', 'data', 'security', 'help', 'about'] as const;
export type SettingsTab = (typeof SETTINGS_TABS)[number];

/** The modes Home shows, each with what matters in it. */
export const HOME_MODES = ['normal', 'ranked', 'aram', 'mayhem', 'arena'] as const;
export type HomeMode = (typeof HOME_MODES)[number];

/** The tier lists Meta shows. */
export const META_MODES = ['rift', 'aram', 'arena'] as const;
export type MetaMode = (typeof META_MODES)[number];

const CHAMPION_PAGES: readonly Page[] = ['build', 'augments'];
const HOME_MODE_KEY = 'xyra-home-mode';
/** The version the player chose not to install for now. */
const SKIPPED_UPDATE_KEY = 'xyra-update-later';
/** The Home tab of each game mode; the Rift has two, normal and ranked. */
const HOME_TABS: Partial<Record<GameMode, HomeMode>> = { aram: 'aram', mayhem: 'mayhem', arena: 'arena', summonersRift: 'normal' };
/** The game mode of each Home tab; normal and ranked are both the Rift. */
export const HOME_GAME_MODES: Record<HomeMode, GameMode> = { normal: 'summonersRift', ranked: 'summonersRift', aram: 'aram', mayhem: 'mayhem', arena: 'arena' };

/** Whether a game of the match history belongs to a Home mode. */
export const inHomeMode = (game: { mode: GameMode; ranked: boolean }, mode: HomeMode) =>
  game.mode === HOME_GAME_MODES[mode] && (mode === 'ranked' ? game.ranked : mode === 'normal' ? !game.ranked : true);

const savedHomeMode = () => HOME_MODES.find((mode) => mode === localStorage.getItem(HOME_MODE_KEY)) ?? 'normal';

const playing = (state: EngineState | null) => state?.champ_select?.champion?.id ?? state?.game?.champion?.id ?? null;

/** The champion being played, its position and whether its game started, so any of them changing counts as a new pick. */
const livePick = (state: EngineState | null) => {
  const champion = playing(state);
  return champion === null ? null : `${state?.game ? 'game' : 'select'}:${champion}:${state?.champ_select?.position ?? ''}`;
};

class App {
  state = $state<EngineState>(null!);
  config = $state<Config>(null!);
  choices = $state<Choices>(null!);
  ready = $derived(!!this.state && !!this.config && !!this.choices);
  stats = $state<StatsSummary | null>(null);
  /** Ranked games with the LP each gave or took, newest first. */
  lp = $state<LpGame[]>([]);
  champions = $state<ChampionInfo[]>([]);
  profile = $state<Profile | null>(null);
  page = $state<Page>('home');
  settingsTab = $state<SettingsTab>('general');
  selectedChampion = $state<number | null>(null);
  buildMode = $state<BuildMode>('aram');
  /** Mode of the Augments tier list; null shows the first mode with augments. */
  augmentMode = $state<GameMode | null>(null);
  positionOverride = $state<Position | null>(null);
  homeMode = $state<HomeMode>(savedHomeMode());
  metaMode = $state<MetaMode>('rift');
  profileError = $state('');
  newGames = $state(0);
  update = $state<Release | null>(null);
  skippedUpdate = $state(localStorage.getItem(SKIPPED_UPDATE_KEY));
  updateProgress = $state<number | null>(null);
  updateError = $state('');
  checkingUpdate = $state(false);
  /** The champion of the current champion select or game. */
  liveChampion = $derived(this.state?.champ_select?.champion ?? this.state?.game?.champion ?? null);
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
        const changed = champion !== null && livePick(next) !== livePick(this.state);
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
    this.lp = await getLpGames();
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

  /** Build and Augments open on the champion being played; openBuild and openAugments open the one the player chose. */
  goTo = (page: Page) => {
    if (CHAMPION_PAGES.includes(page)) this.followLive();
    this.showPage(page);
  };

  followLive = () => {
    if (!this.liveChampion) return;
    this.selectedChampion = this.liveChampion.id;
    this.followGame();
  };

  showPage = (page: Page) => {
    this.page = page;
    if (page === 'stats') this.newGames = 0;
  };

  openAugments = (champion: number) => {
    this.selectedChampion = champion;
    this.showPage('augments');
  };

  openBuild = (champion: number) => {
    this.selectedChampion = champion;
    this.followGame();
    this.showPage('build');
  };

  /** Opens the Summoner's Rift build of a champion in a position, as picked from the tier list. */
  openRiftBuild = (champion: number, position: Position) => {
    this.selectedChampion = champion;
    this.buildMode = 'rift';
    this.positionOverride = position;
    this.showPage('build');
  };

  /** Opens the build of a champion in ARAM or the Rift, in the position OP.GG sees most. */
  openModeBuild = (champion: number, mode: BuildMode) => {
    this.selectedChampion = champion;
    this.buildMode = mode;
    this.positionOverride = null;
    this.showPage('build');
  };

  openMeta = (mode: MetaMode) => {
    this.metaMode = mode;
    this.showPage('meta');
  };

  openModeAugments = (champion: number, mode: GameMode) => {
    this.augmentMode = mode;
    this.openAugments(champion);
  };

  setHomeMode = (mode: HomeMode) => {
    this.homeMode = mode;
    localStorage.setItem(HOME_MODE_KEY, mode);
  };

  /** Shows the Home tab of the mode being played; a Rift game keeps normal or ranked, whichever is open. */
  followHomeMode = (mode: GameMode) => {
    const tab = HOME_TABS[mode];
    if (!tab || (tab === 'normal' && this.homeMode === 'ranked')) return;
    this.homeMode = tab;
  };

  openSettings = (tab: SettingsTab) => {
    this.settingsTab = tab;
    this.goTo('settings');
  };

  followGame = () => {
    const mode = this.state.game?.mode ?? this.state.champ_select?.mode;
    if (mode && this.choices?.augment_modes.includes(mode)) this.augmentMode = mode;
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

  /** Stops offering this version in a window; the sidebar and About still offer it. */
  skipUpdate = () => {
    if (!this.update) return;
    localStorage.setItem(SKIPPED_UPDATE_KEY, this.update.version);
    this.skippedUpdate = this.update.version;
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

  /** Imports runes, items and spells of the champion being played, as the client's champion select asks for them. */
  importWholeBuild = async (champion: number) => {
    this.followGame();
    try {
      await importWholeBuild(champion, this.buildMode, this.buildMode === 'rift' ? this.positionOverride : null);
      return this.t('build:imported.all');
    } catch (error) {
      return this.errorText(error);
    }
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
