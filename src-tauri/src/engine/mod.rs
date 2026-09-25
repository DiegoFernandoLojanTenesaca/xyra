mod card_reader;
mod champ_select;
mod watch;

pub use card_reader::demo_cards;

use crate::screen;
use card_reader::CardReader;
use champ_select::ChampSelectTracker;
use notify::RecommendedWatcher;
use serde::Serialize;
use serde_json::Value;
use std::{
    collections::{HashMap, HashSet},
    fmt::Display,
    sync::{
        Arc, Mutex, RwLock,
        mpsc::{self, Receiver, RecvTimeoutError, Sender},
    },
    thread,
    time::{Duration, Instant},
};
use tauri::{AppHandle, Emitter, Manager};
use xyra_core::{
    catalog::Catalog,
    champ_select as client_champ_select, client_import,
    config::Config,
    errors::{AppError, Result},
    game_settings::{self, GameOption, SettingValue},
    gameflow::{self, GameflowPhase},
    league::{self, Installation, Lcu, LcuEvent},
    model::{AppEvent, Build, BuildMode, ChampionInfo, CurrentGame, EngineState, GameMode, ImportTarget, Matchup, Phase, Position},
    opgg, profile,
    stats::{self, StatsSummary, StoredGame},
    storage::Storage,
};

pub const MAIN_WINDOW: &str = "main";
const SUBSCRIPTIONS: [&str; 4] = [profile::CURRENT_SUMMONER, gameflow::SESSION, client_champ_select::SESSION, stats::END_OF_GAME];
const RECONNECT_DELAY: Duration = Duration::from_secs(2);
const MAX_RECONNECTS: u32 = 5;
const BORDERLESS_FIX_COOLDOWN: Duration = Duration::from_secs(10);

pub enum EngineEvent {
    LockfileChanged,
    GameConfigChanged,
    Client { generation: u64, event: LcuEvent },
    ClientClosed { generation: u64, error: Option<AppError> },
    CounterPicks { enemy: u32, position: Position, picks: Option<Vec<Matchup>> },
    ChampionTiers(HashMap<u32, (u8, u32)>),
    ConfigChanged,
    ShowDemo,
    TestVoice,
}

/// State shared by the engine thread, the UI commands and the tray.
pub struct Shared {
    pub config: Mutex<Config>,
    pub state: Mutex<EngineState>,
    pub games: Mutex<Vec<StoredGame>>,
    catalog: RwLock<Arc<Catalog>>,
    /// ARAM: Mayhem champion tiers from OP.GG: id -> (tier, rank).
    champion_tiers: RwLock<HashMap<u32, (u8, u32)>>,
    /// Champions the signed-in account can play; None while unknown.
    available: RwLock<Option<HashSet<u32>>>,
    pub storage: Storage,
    pub installation: Installation,
    pub opgg: opgg::Client,
    events: Sender<EngineEvent>,
}

impl Shared {
    pub fn new(storage: Storage, installation: Installation) -> (Shared, Receiver<EngineEvent>) {
        let config = storage.load_config().unwrap_or_else(|e| {
            storage.log_error("config", &e);
            Config::default()
        });
        let games = storage.load_games().unwrap_or_else(|e| {
            storage.log_error("stats", &e);
            Vec::new()
        });
        let catalog = storage.load_catalog().unwrap_or_else(|e| {
            storage.log_error("catalog", &e);
            None
        });
        let state = EngineState {
            phase: Phase::NoClient,
            account: None,
            game: None,
            champ_select: None,
            build_mode: None,
            cards: Vec::new(),
            borderless: league::is_borderless(&installation),
            client_locale: installation.locale.clone(),
            language: config.effective_language(&installation.locale).into(),
            ocr_language: None,
            version: env!("CARGO_PKG_VERSION").into(),
        };
        let (events, received) = mpsc::channel();
        let shared = Shared {
            config: Mutex::new(config),
            state: Mutex::new(state),
            games: Mutex::new(games),
            catalog: RwLock::new(Arc::new(catalog.unwrap_or_default())),
            champion_tiers: RwLock::new(HashMap::new()),
            available: RwLock::new(None),
            storage,
            installation,
            opgg: opgg::client(),
            events,
        };
        (shared, received)
    }

    /// Records a failure that degrades Xyra and needs a human to look at it.
    pub fn log_error(&self, context: &str, error: impl Display) {
        self.storage.log_error(context, error);
    }

    /// Sends an event to the engine thread.
    pub fn send(&self, event: EngineEvent) {
        self.events.send(event).ok();
    }

    pub fn config(&self) -> Config {
        self.config.lock().unwrap().clone()
    }

    pub fn language(&self) -> &'static str {
        self.config().effective_language(&self.installation.locale)
    }

    pub fn catalog(&self) -> Arc<Catalog> {
        Arc::clone(&self.catalog.read().unwrap())
    }

    pub fn account(&self) -> Option<String> {
        self.state.lock().unwrap().account.clone()
    }

    pub fn lcu(&self) -> Result<Lcu> {
        Lcu::connect(&self.installation)
    }

    pub fn is_available(&self, champion: u32) -> bool {
        self.available.read().unwrap().as_ref().is_none_or(|available| available.contains(&champion))
    }

    pub fn champion_info(&self, id: u32) -> ChampionInfo {
        let champion = self.catalog().champion(id);
        let tier = self.champion_tiers.read().unwrap().get(&id).copied();
        ChampionInfo { id, name: champion.name, icon: champion.icon, tier: tier.map(|t| t.0), rank: tier.map(|t| t.1), locked: !self.is_available(id) }
    }

    /// Every champion, ranked ones first by their ARAM: Mayhem rank, then by name.
    pub fn champions(&self) -> Vec<ChampionInfo> {
        let mut champions: Vec<ChampionInfo> = self.catalog().champions.keys().map(|&id| self.champion_info(id)).collect();
        champions.sort_by(|a, b| (a.rank.is_none(), a.rank, &a.name).cmp(&(b.rank.is_none(), b.rank, &b.name)));
        champions
    }

    pub fn fetch_build(&self, champion: u32, mode: BuildMode, position: Option<Position>) -> Result<Build> {
        opgg::fetch_build(&self.opgg, champion, mode, position, &self.catalog())
    }

    pub fn import_build(&self, champion: u32, mode: BuildMode, position: Option<Position>, target: ImportTarget) -> Result<()> {
        let lcu = self.lcu()?;
        let build = self.fetch_build(champion, mode, position)?;
        let name = self.catalog().champion(champion).name;
        match target {
            ImportTarget::Runes => client_import::import_runes(&lcu, &build, &name),
            ImportTarget::Items => client_import::import_items(&lcu, &build, &name, self.language()),
        }
    }

    pub fn stats_summary(&self) -> StatsSummary {
        stats::summarize(&self.games.lock().unwrap(), self.account().as_deref(), &self.catalog())
    }

    /// Sets borderless through the client when it is open, or in game.cfg otherwise.
    pub fn set_borderless(&self) -> Result<()> {
        if self.state.lock().unwrap().game.is_some() {
            return Err(AppError::GameInProgress);
        }
        match self.lcu() {
            Ok(lcu) => game_settings::update(&lcu, GameOption::Borderless, SettingValue::Toggle(true)),
            Err(_) => league::set_borderless(&self.installation),
        }
    }
}

pub fn emit(app: &AppHandle, shared: &Shared, event: AppEvent, payload: impl Serialize + Clone) {
    if let Err(e) = app.emit_to(MAIN_WINDOW, event.name(), payload) {
        shared.log_error("ui event", e);
    }
}

pub fn update_state(app: &AppHandle, shared: &Shared, change: impl FnOnce(&mut EngineState)) {
    let changed = {
        let mut state = shared.state.lock().unwrap();
        let before = state.clone();
        change(&mut state);
        (*state != before).then(|| state.clone())
    };
    if let Some(state) = changed {
        emit(app, shared, AppEvent::State, state);
    }
}

struct ClientSession {
    lcu: Lcu,
    catalog_ready: bool,
}

struct GameTracking {
    mode: GameMode,
    champion: Option<u32>,
}

/// Reacts to the League client, the game files and the UI; owns everything that runs on the engine thread.
struct Engine {
    app: AppHandle,
    shared: Arc<Shared>,
    client: Option<ClientSession>,
    generation: u64,
    reconnect: Option<(Instant, u32)>,
    account: Option<String>,
    mode: GameMode,
    game: Option<GameTracking>,
    history_pending: bool,
    borderless: Option<bool>,
    borderless_fixed_at: Option<Instant>,
    champ_select: ChampSelectTracker,
    cards: CardReader,
    _watcher: Option<RecommendedWatcher>,
}

pub fn run(app: AppHandle, shared: Arc<Shared>, events: Receiver<EngineEvent>) {
    if let Err(e) = screen::init_thread() {
        shared.log_error("screen reading", e);
    }
    let mut engine = Engine::new(app, shared);
    engine.start();
    loop {
        let event = match engine.next_deadline() {
            Some(deadline) => match events.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
                Ok(event) => Some(event),
                Err(RecvTimeoutError::Timeout) => None,
                Err(RecvTimeoutError::Disconnected) => return,
            },
            None => match events.recv() {
                Ok(event) => Some(event),
                Err(_) => return,
            },
        };
        match event {
            Some(event) => engine.handle(event),
            None => engine.on_deadline(),
        }
        engine.publish();
    }
}

impl Engine {
    fn new(app: AppHandle, shared: Arc<Shared>) -> Engine {
        let cards = CardReader::new(&shared);
        let watcher = watch::start(&shared).map_err(|e| shared.log_error("file watcher", e)).ok();
        Engine {
            app,
            client: None,
            generation: 0,
            reconnect: None,
            account: None,
            mode: GameMode::Other,
            game: None,
            history_pending: false,
            borderless: league::is_borderless(&shared.installation),
            borderless_fixed_at: None,
            champ_select: ChampSelectTracker::default(),
            cards,
            _watcher: watcher,
            shared,
        }
    }

    fn start(&mut self) {
        let ocr_language = self.cards.ocr_language();
        update_state(&self.app, &self.shared, |s| s.ocr_language = ocr_language);
        self.fetch_champion_tiers();
        self.connect();
        self.check_borderless();
    }

    fn emit(&self, event: AppEvent) {
        emit(&self.app, &self.shared, event, ());
    }

    fn lcu(&self) -> Option<&Lcu> {
        self.client.as_ref().map(|c| &c.lcu)
    }

    fn fetch_champion_tiers(&self) {
        let shared = Arc::clone(&self.shared);
        thread::spawn(move || match opgg::fetch_champion_tiers(&shared.opgg) {
            Ok(tiers) => shared.send(EngineEvent::ChampionTiers(tiers)),
            Err(e) => shared.log_error("OP.GG champion tiers", e),
        });
    }

    fn next_deadline(&self) -> Option<Instant> {
        [self.reconnect.map(|r| r.0), self.champ_select.rune_deadline(), self.cards.next_read()].into_iter().flatten().min()
    }

    fn handle(&mut self, event: EngineEvent) {
        match event {
            EngineEvent::LockfileChanged => {
                self.reconnect = None;
                self.connect();
            }
            EngineEvent::GameConfigChanged | EngineEvent::ConfigChanged => self.check_borderless(),
            EngineEvent::Client { generation, event } if generation == self.generation => {
                self.reconnect = None;
                self.on_client_event(event);
            }
            EngineEvent::ClientClosed { generation, error } if generation == self.generation => self.on_client_closed(error),
            EngineEvent::Client { .. } | EngineEvent::ClientClosed { .. } => {}
            EngineEvent::CounterPicks { enemy, position, picks } => self.champ_select.set_counter_picks(enemy, position, picks),
            EngineEvent::ChampionTiers(tiers) => {
                *self.shared.champion_tiers.write().unwrap() = tiers;
                self.emit(AppEvent::Data);
            }
            EngineEvent::ShowDemo if self.game.is_none() => self.cards.demo(&self.shared),
            EngineEvent::ShowDemo => {}
            EngineEvent::TestVoice => self.cards.test_voice(&self.shared),
        }
    }

    fn on_deadline(&mut self) {
        let now = Instant::now();
        if self.reconnect.is_some_and(|(at, _)| at <= now) {
            self.connect();
        }
        if let Some(champion) = self.champ_select.take_due_rune_import(now) {
            let position = self.champ_select.position();
            if let Err(e) = self.shared.import_build(champion, self.mode.build_mode(), position, ImportTarget::Runes) {
                self.shared.log_error("auto import runes", e);
            }
        }
        if self.cards.next_read().is_some_and(|at| at <= now)
            && let Some((champion, mode)) = self.reading_target()
        {
            self.cards.read(champion, mode, &self.shared);
        }
    }

    fn connect(&mut self) {
        let Ok(lcu) = self.shared.lcu() else { return };
        if self.client.as_ref().is_some_and(|c| c.lcu.same_session(&lcu)) {
            return;
        }
        self.generation += 1;
        let (generation, listener, shared) = (self.generation, lcu.clone(), Arc::clone(&self.shared));
        thread::spawn(move || {
            let result = listener.listen(&SUBSCRIPTIONS, |event| shared.send(EngineEvent::Client { generation, event }));
            shared.send(EngineEvent::ClientClosed { generation, error: result.err() });
        });
        self.client = Some(ClientSession { lcu, catalog_ready: false });
        self.on_connected();
    }

    /// Reads the client's current account and game flow.
    fn on_connected(&mut self) {
        let Some(lcu) = self.lcu().cloned() else { return };
        if let Ok(summoner) = lcu.get(profile::CURRENT_SUMMONER) {
            self.set_account(profile::account(&summoner));
        }
        self.on_gameflow(lcu.get(gameflow::SESSION).ok());
        if self.shared.champion_tiers.read().unwrap().is_empty() {
            self.fetch_champion_tiers();
        }
        self.check_borderless();
    }

    fn on_client_closed(&mut self, error: Option<AppError>) {
        self.client = None;
        self.champ_select.clear();
        let attempts = self.reconnect.map_or(0, |r| r.1) + 1;
        self.reconnect = match error {
            Some(error) if self.shared.installation.lockfile().exists() => {
                if attempts > MAX_RECONNECTS {
                    self.shared.log_error("League client connection", error);
                    None
                } else {
                    Some((Instant::now() + RECONNECT_DELAY, attempts))
                }
            }
            _ => None,
        };
    }

    fn on_client_event(&mut self, event: LcuEvent) {
        match event.uri.as_str() {
            profile::CURRENT_SUMMONER => self.set_account(event.data.as_ref().and_then(profile::account)),
            gameflow::SESSION => self.on_gameflow(event.data),
            client_champ_select::SESSION => self.on_champ_select(event.data),
            stats::END_OF_GAME if event.data.is_some() => {
                self.history_pending = self.mode.has_augments();
                self.import_games();
            }
            _ => {}
        }
    }

    fn set_account(&mut self, account: Option<String>) {
        self.load_catalog();
        if account == self.account {
            return;
        }
        self.account = account;
        *self.shared.available.write().unwrap() = None;
        if let Some(account) = self.account.clone() {
            self.refresh_available();
            let mut games = self.shared.games.lock().unwrap();
            if stats::claim_unowned(&mut games, &account)
                && let Err(e) = self.shared.storage.save_games(&games)
            {
                self.shared.log_error("save stats", e);
            }
            drop(games);
            self.import_games();
        }
        self.publish();
        self.emit(AppEvent::Data);
    }

    fn refresh_available(&self) {
        let Some(lcu) = self.lcu() else { return };
        match profile::read_available_champions(lcu) {
            Ok(available) => *self.shared.available.write().unwrap() = Some(available),
            Err(e) => self.shared.log_error("owned champions", e),
        }
    }

    /// Reads the game data catalog once per client session.
    fn load_catalog(&mut self) {
        let Some(client) = self.client.as_mut().filter(|c| !c.catalog_ready) else { return };
        match Catalog::read(&client.lcu) {
            Ok(catalog) => {
                client.catalog_ready = true;
                if let Err(e) = self.shared.storage.save_catalog(&catalog) {
                    self.shared.log_error("save catalog", e);
                }
                *self.shared.catalog.write().unwrap() = Arc::new(catalog);
                self.emit(AppEvent::Data);
            }
            Err(AppError::EmptyCatalog | AppError::Client(_)) => {}
            Err(e) => self.shared.log_error("catalog", e),
        }
    }

    fn on_gameflow(&mut self, session: Option<Value>) {
        let flow = session.map(|s| gameflow::parse(&s, self.account.as_deref()));
        let phase = flow.as_ref().map_or(GameflowPhase::None, |f| f.phase);
        if let Some(flow) = &flow {
            self.mode = flow.mode;
        }
        match (flow, &mut self.game) {
            (Some(flow), Some(game)) if phase.is_in_game() => game.champion = game.champion.or(flow.champion),
            (Some(flow), None) if phase.is_in_game() => {
                self.game = Some(GameTracking { mode: flow.mode, champion: flow.champion.or(self.champ_select.last_champion()) });
                self.on_game_started();
            }
            (_, Some(_)) => {
                self.game = None;
                self.on_game_ended();
            }
            _ => {}
        }
        if phase == GameflowPhase::ChampSelect {
            self.history_pending = false;
            if !self.champ_select.is_active() {
                let session = self.lcu().and_then(|lcu| lcu.get(client_champ_select::SESSION).ok());
                self.on_champ_select(session);
            }
        } else {
            self.champ_select.clear();
        }
        if self.history_pending {
            self.import_games();
        }
        self.load_catalog();
    }

    fn on_game_started(&mut self) {
        self.champ_select.clear();
        if !self.shared.config().close_window_in_game {
            return;
        }
        if let Some(window) = self.app.get_webview_window(MAIN_WINDOW)
            && let Err(e) = window.close()
        {
            self.shared.log_error("close window in game", e);
        }
    }

    fn on_game_ended(&mut self) {
        self.cards.stop();
        self.history_pending = self.mode.has_augments();
        self.check_borderless();
    }

    fn on_champ_select(&mut self, session: Option<Value>) {
        let Some(parsed) = session.as_ref().and_then(client_champ_select::parse) else {
            return self.champ_select.clear();
        };
        if !self.champ_select.is_active() {
            self.refresh_available();
        }
        let pickable = self.lcu().and_then(|lcu| client_champ_select::read_pickable(lcu).ok());
        let auto_runes = self.shared.config().auto_import_runes;
        for (enemy, position) in self.champ_select.update(parsed, pickable, self.mode, auto_runes) {
            let shared = Arc::clone(&self.shared);
            thread::spawn(move || {
                let picks = opgg::fetch_counter_picks(&shared.opgg, enemy, position, &shared.catalog()).unwrap_or_else(|e| {
                    shared.log_error("OP.GG counter picks", e);
                    None
                });
                shared.send(EngineEvent::CounterPicks { enemy, position, picks });
            });
        }
    }

    fn import_games(&mut self) {
        let (Some(lcu), Some(account)) = (self.lcu(), self.account.as_deref()) else { return };
        let added = {
            let mut games = self.shared.games.lock().unwrap();
            match stats::import_recent(lcu, account, &mut games) {
                Ok(0) => 0,
                Ok(added) => {
                    if let Err(e) = self.shared.storage.save_games(&games) {
                        self.shared.log_error("save stats", e);
                    }
                    added
                }
                Err(e) => {
                    self.shared.log_error("match history", e);
                    0
                }
            }
        };
        if added > 0 {
            self.history_pending = false;
            self.emit(AppEvent::Data);
        }
    }

    fn check_borderless(&mut self) {
        self.borderless = league::is_borderless(&self.shared.installation);
        let cooled_down = self.borderless_fixed_at.is_none_or(|at| at.elapsed() > BORDERLESS_FIX_COOLDOWN);
        if self.shared.config().keep_borderless && self.borderless == Some(false) && self.game.is_none() && cooled_down {
            self.borderless_fixed_at = Some(Instant::now());
            if let Err(e) = self.shared.set_borderless() {
                self.shared.log_error("keep borderless", e);
            }
        }
    }

    /// The champion and mode whose cards are read, while a game with augments is on and Xyra is not paused.
    fn reading_target(&self) -> Option<(u32, GameMode)> {
        let config = self.shared.config();
        let game = self.game.as_ref().filter(|g| !config.paused && g.mode.has_augments() && (g.mode != GameMode::Arena || config.arena))?;
        Some((game.champion?, game.mode))
    }

    fn publish(&mut self) {
        match self.reading_target() {
            Some(_) => self.cards.start(),
            None => self.cards.stop(),
        }
        let paused = self.shared.config().paused;
        let champ_select = self.champ_select.view(&self.shared, self.mode);
        let game = self.game.as_ref().map(|g| CurrentGame { mode: g.mode, champion: g.champion.map(|id| self.shared.champion_info(id)) });
        let phase = match () {
            _ if paused => Phase::Paused,
            _ if game.is_some() => Phase::InGame,
            _ if champ_select.is_some() => Phase::ChampSelect,
            _ if self.client.is_some() => Phase::Client,
            _ => Phase::NoClient,
        };
        let build_mode = game.as_ref().map(|g| g.mode).or(champ_select.as_ref().map(|c| c.mode)).map(GameMode::build_mode);
        let (account, cards, borderless) = (self.account.clone(), self.cards.cards().to_vec(), self.borderless);
        update_state(&self.app, &self.shared, |state| {
            state.phase = phase;
            state.account = account;
            state.game = game;
            state.champ_select = champ_select;
            state.build_mode = build_mode;
            state.cards = cards;
            state.borderless = borderless;
        });
    }
}
