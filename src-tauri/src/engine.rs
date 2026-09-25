use crate::{
    overlay::{Overlay, OverlayTexts},
    screen,
    voice::{self, Voice},
};
use std::{
    collections::HashMap,
    fs::{self, OpenOptions},
    io::Write,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tauri::{AppHandle, Emitter, Manager};
use xyra_core::{
    cards::{self, Candidate, Card, CardTracker, TrackerAction},
    catalog::{Catalog, NamedAssets},
    config::Config,
    game_settings, import, lol,
    model::{AppEvent, Build, BuildMode, ChampSelect, ChampionInfo, EngineState, Phase},
    opgg::{self, AugmentStat},
    stats,
};

pub const MAIN_WINDOW: &str = "main";
const MAX_LOG_BYTES: u64 = 1_000_000;
const FRAME: Duration = Duration::from_millis(50);
const WATCH_CARDS: Duration = Duration::from_millis(250);
const WATCH_GAME: Duration = Duration::from_millis(700);
const IDLE_IN_GAME: Duration = Duration::from_secs(3);
const IDLE_CLIENT: Duration = Duration::from_millis(1500);
const UNFOCUSED: Duration = Duration::from_secs(1);
const RETRY: Duration = Duration::from_secs(5);
const SETTINGS_CHECK: Duration = Duration::from_secs(10);
const DEMO_DURATION: Duration = Duration::from_secs(5);
const HISTORY_DELAY: Duration = Duration::from_secs(15);
const HISTORY_TIMEOUT: Duration = Duration::from_secs(150);
const AUTO_RUNES_SETTLE: Duration = Duration::from_secs(3);
const LABELED_MODES: [&str; 2] = ["KIWI", "CHERRY"];
const ARENA_MODE: &str = "CHERRY";
const DEMO_CHAMPION: &str = "Brand";
const REFERENCE_HEIGHT: f64 = 1200.0;
/// Demo cards measured in a real game (docs/cards-background.png): augment id, x offset from center, tier, performance.
const DEMO_CARDS: [(u32, f64, Option<u8>, f64); 3] = [(1211, -410.0, Some(0), 83.0), (1098, 0.0, Some(5), 70.0), (2128, 410.0, Some(0), 91.0)];
const DEMO_ROW: f64 = 0.4175;

pub struct Paths {
    pub data: PathBuf,
    pub config: PathBuf,
    pub stats: PathBuf,
    pub catalog: PathBuf,
    pub profile: PathBuf,
    pub log: PathBuf,
    pub screenshots: PathBuf,
}

/// State shared by the engine thread, the UI commands and the tray.
pub struct Shared {
    pub config: Mutex<Config>,
    pub state: Mutex<EngineState>,
    pub games: Mutex<Vec<stats::StoredGame>>,
    pub catalog: Mutex<Option<Catalog>>,
    /// ARAM: Mayhem champion tiers from OP.GG: id -> (tier, rank).
    pub champion_tiers: Mutex<HashMap<u32, (u8, u32)>>,
    pub paths: Paths,
    pub installation: lol::Installation,
    pub http: reqwest::blocking::Client,
    pub demo_requested: AtomicBool,
}

impl Shared {
    pub fn new(data: PathBuf) -> Shared {
        let paths = Paths {
            config: data.join("config.json"),
            stats: data.join("stats.json"),
            catalog: data.join("catalog.json"),
            profile: data.join("profile.json"),
            log: data.join("xyra.log"),
            screenshots: data.join("screenshots"),
            data: data.clone(),
        };
        let _ = fs::create_dir_all(&data);
        if fs::metadata(&paths.log).is_ok_and(|m| m.len() > MAX_LOG_BYTES) {
            let _ = fs::remove_file(&paths.log);
        }
        let installation = lol::read_installation();
        let config = Config::load(&paths.config);
        Shared {
            state: Mutex::new(EngineState {
                phase: Phase::NoClient,
                champion: None,
                mode: None,
                cards: Vec::new(),
                champ_select: None,
                borderless: lol::is_borderless(&installation.dir),
                client_locale: installation.locale.clone(),
                language: config.effective_language(&installation.locale).into(),
                ocr_language: None,
                version: env!("CARGO_PKG_VERSION").into(),
            }),
            config: Mutex::new(config),
            games: Mutex::new(stats::load(&paths.stats)),
            catalog: Mutex::new(Catalog::load(&paths.catalog)),
            champion_tiers: Mutex::new(HashMap::new()),
            paths,
            installation,
            http: lol::http_client(),
            demo_requested: AtomicBool::new(false),
        }
    }

    /// Records a failure that degrades Xyra and needs a human to look at it.
    pub fn log_error(&self, context: &str, error: impl std::fmt::Display) {
        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&self.paths.log) {
            let now = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs());
            let _ = writeln!(file, "{now} {context}: {error}");
        }
    }

    pub fn champion_info(&self, id: u32) -> ChampionInfo {
        let (name, icon) = self.catalog.lock().unwrap().as_ref().and_then(|c| c.champions.get(&id).cloned()).unwrap_or_default();
        let tier = self.champion_tiers.lock().unwrap().get(&id).copied();
        ChampionInfo { id, name, icon, tier: tier.map(|t| t.0), rank: tier.map(|t| t.1) }
    }

    pub fn config(&self) -> Config {
        self.config.lock().unwrap().clone()
    }

    pub fn language(&self) -> &'static str {
        self.config().effective_language(&self.installation.locale)
    }

    pub fn fetch_build(&self, champion: u32, mode: BuildMode, position: Option<&str>) -> Result<Build, String> {
        let catalog = self.catalog.lock().unwrap().clone().unwrap_or_default();
        opgg::fetch_build(&self.http, champion, mode, position, &catalog)
    }

    /// `target`: "runes" or "items".
    pub fn import_build(&self, champion: u32, mode: BuildMode, position: Option<&str>, target: &str) -> Result<(), String> {
        let lcu = lol::Lcu::connect(&self.installation.dir).ok_or("noClient")?;
        let build = self.fetch_build(champion, mode, position)?;
        let name = self.champion_info(champion).name;
        if target == "runes" {
            import::import_runes(&lcu, &self.http, &build, &name)
        } else {
            import::import_items(&lcu, &self.http, &build, &name, self.language())
        }
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
        let _ = app.emit_to(MAIN_WINDOW, AppEvent::State.name(), state);
    }
}

fn import_games(shared: &Shared, lcu: &lol::Lcu) -> usize {
    let mut games = shared.games.lock().unwrap();
    match stats::import_recent(lcu, &shared.http, &mut games) {
        Ok(0) => 0,
        Ok(added) => {
            if let Err(e) = stats::save(&shared.paths.stats, &games) {
                shared.log_error("save stats", e);
            }
            added
        }
        Err(e) => {
            shared.log_error("match history", e);
            0
        }
    }
}

fn save_screenshot(shared: &Shared, width: i32, height: i32) -> Result<(), String> {
    let bgra = screen::capture(0, 0, width, height).ok_or("capture failed")?;
    let rgba: Vec<u8> = bgra.chunks_exact(4).flat_map(|p| [p[2], p[1], p[0], 255]).collect();
    fs::create_dir_all(&shared.paths.screenshots).map_err(|e| e.to_string())?;
    let name = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs());
    let file = fs::File::create(shared.paths.screenshots.join(format!("{name}.png"))).map_err(|e| e.to_string())?;
    let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), width as u32, height as u32);
    encoder.set_color(png::ColorType::Rgba);
    encoder.write_header().and_then(|mut w| w.write_image_data(&rgba)).map_err(|e| e.to_string())
}

fn read_champ_select(shared: &Shared, lcu: &lol::Lcu) -> Option<ChampSelect> {
    if lcu.get(&shared.http, "/lol-gameflow/v1/gameflow-phase").ok()?.as_str()? != "ChampSelect" {
        return None;
    }
    let session = lcu.get(&shared.http, "/lol-champ-select/v1/session").ok()?;
    let me = session["localPlayerCellId"].as_i64()?;
    let cell = session["myTeam"].as_array()?.iter().find(|m| m["cellId"].as_i64() == Some(me))?;
    let mine = cell["championId"].as_u64().unwrap_or(0);
    let bench = session["benchChampions"].as_array().into_iter().flatten().filter_map(|b| b["championId"].as_u64());
    let mode = lcu.get(&shared.http, "/lol-gameflow/v1/session").ok().and_then(|g| g["gameData"]["queue"]["gameMode"].as_str().map(String::from));
    let position = match cell["assignedPosition"].as_str().unwrap_or_default() {
        "top" => Some("top"),
        "jungle" => Some("jungle"),
        "middle" => Some("mid"),
        "bottom" => Some("adc"),
        "utility" => Some("support"),
        _ => None,
    };
    Some(ChampSelect {
        champion: (mine > 0).then(|| shared.champion_info(mine as u32)),
        bench: bench.map(|id| shared.champion_info(id as u32)).collect(),
        mode: mode.unwrap_or_default(),
        position: position.map(String::from),
    })
}

pub fn demo_cards(augments: &NamedAssets, width: i32, height: i32) -> Vec<Card> {
    let k = height as f64 / REFERENCE_HEIGHT;
    let (cx, y) = (width as f64 / 2.0, height as f64 * DEMO_ROW);
    let candidates: Vec<Candidate> = DEMO_CARDS.iter().map(|&(id, dx, ..)| Candidate { ids: vec![id], x: cx + dx * k, y }).collect();
    let stats = DEMO_CARDS
        .iter()
        .filter_map(|&(id, _, tier, performance)| Some((id, AugmentStat { tier: tier?, performance, pick_rate: 0.0 })))
        .collect();
    cards::rate_cards(&candidates, &stats, augments)
}

/// Engine loop: runs on its own thread for the app lifetime and owns the overlay.
pub fn run(app: AppHandle, shared: Arc<Shared>) {
    screen::init_thread();
    let ocr = screen::Ocr::new(&shared.installation.locale);
    let ocr_language = ocr.as_ref().map(|o| o.language.clone());
    update_state(&app, &shared, |s| s.ocr_language = ocr_language.clone());
    let (width, height) = screen::primary_size();
    let overlay = Overlay::new(width, height).map_err(|e| shared.log_error("overlay", e)).ok();
    let voice = Voice::new();

    thread::spawn({
        let (app, shared) = (app.clone(), Arc::clone(&shared));
        move || match opgg::fetch_champion_tiers(&shared.http) {
            Ok(tiers) => {
                *shared.champion_tiers.lock().unwrap() = tiers;
                let _ = app.emit_to(MAIN_WINDOW, AppEvent::Stats.name(), ());
            }
            Err(e) => shared.log_error("OP.GG champion tiers", e),
        }
    });

    let zone = (width / 8, height / 10, width * 3 / 4, height * 7 / 10);
    let mut catalog_fresh = false;
    let mut augment_stats: HashMap<u32, AugmentStat> = HashMap::new();
    let mut stats_for: Option<(u32, String)> = None;
    let mut tracker = CardTracker::default();
    let mut was_in_game = false;
    let mut rune_candidate: Option<(u32, Instant)> = None;
    let mut runes_imported_for: Option<u32> = None;
    let mut history_pending_since: Option<Instant> = None;
    let mut last_settings_check = Instant::now() - SETTINGS_CHECK;
    let mut demo_until: Option<Instant> = None;

    let wait = |duration: Duration, tracker: &mut CardTracker, demo_until: &mut Option<Instant>| {
        let end = Instant::now() + duration;
        while Instant::now() < end {
            if let Some(overlay) = &overlay {
                overlay.pump_messages();
                if shared.demo_requested.swap(false, Ordering::Relaxed) {
                    let config = shared.config();
                    let demo = {
                        let catalog = shared.catalog.lock().unwrap();
                        demo_cards(catalog.as_ref().map_or(&HashMap::new(), |c| &c.augments), width, height)
                    };
                    if let Err(e) = overlay.show(&demo, DEMO_CHAMPION, &config, &OverlayTexts::new(shared.language())) {
                        shared.log_error("overlay demo", e);
                    }
                    *demo_until = Some(Instant::now() + DEMO_DURATION);
                    tracker.reset();
                }
                if demo_until.is_some_and(|until| Instant::now() > until) {
                    *demo_until = None;
                    overlay.hide();
                }
            }
            thread::sleep(FRAME);
        }
    };
    let hide = |tracker: &mut CardTracker| {
        if let Some(overlay) = &overlay {
            overlay.hide();
        }
        tracker.reset();
    };

    loop {
        let config = shared.config();
        let lcu = lol::Lcu::connect(&shared.installation.dir);

        if let (Some(lcu), false) = (&lcu, catalog_fresh) {
            match Catalog::read(lcu, &shared.http) {
                Ok(catalog) => {
                    if let Err(e) = catalog.save(&shared.paths.catalog) {
                        shared.log_error("save catalog", e);
                    }
                    *shared.catalog.lock().unwrap() = Some(catalog);
                    catalog_fresh = true;
                    import_games(&shared, lcu);
                    match opgg::fetch_champion_tiers(&shared.http) {
                        Ok(tiers) => *shared.champion_tiers.lock().unwrap() = tiers,
                        Err(e) => shared.log_error("OP.GG champion tiers", e),
                    }
                    let _ = app.emit_to(MAIN_WINDOW, AppEvent::Stats.name(), ());
                }
                Err(e) => shared.log_error("catalog", e),
            }
        }
        if lcu.is_none() {
            catalog_fresh = false;
        }
        let game = lol::read_live_game(&shared.http);
        if was_in_game && game.is_none() {
            history_pending_since = Some(Instant::now());
        }
        if !was_in_game && game.is_some() && config.close_window_in_game {
            if let Some(window) = app.get_webview_window(MAIN_WINDOW) {
                let _ = window.close();
            }
        }
        was_in_game = game.is_some();
        if let (Some(since), Some(lcu)) = (history_pending_since, &lcu) {
            if since.elapsed() > HISTORY_DELAY {
                let added = import_games(&shared, lcu);
                if added > 0 {
                    let _ = app.emit_to(MAIN_WINDOW, AppEvent::Stats.name(), ());
                }
                if added > 0 || since.elapsed() > HISTORY_TIMEOUT {
                    history_pending_since = None;
                }
            }
        }

        let champ_select = if game.is_none() { lcu.as_ref().and_then(|l| read_champ_select(&shared, l)) } else { None };
        let phase = match (&game, &lcu) {
            _ if config.paused => Phase::Paused,
            (Some(_), _) => Phase::InGame,
            _ if champ_select.is_some() => Phase::ChampSelect,
            (None, Some(_)) => Phase::Client,
            _ => Phase::NoClient,
        };
        let check_settings = game.is_none() && last_settings_check.elapsed() > SETTINGS_CHECK;
        if check_settings {
            last_settings_check = Instant::now();
            if config.keep_borderless && lol::is_borderless(&shared.installation.dir) == Some(false) {
                let result = match &lcu {
                    Some(l) => game_settings::update(l, &shared.http, "General", "WindowMode", game_settings::BORDERLESS.into()),
                    None => lol::set_borderless(&shared.installation.dir, &shared.http),
                };
                if let Err(e) = result {
                    shared.log_error("keep borderless", e);
                }
            }
        }

        match champ_select.as_ref().and_then(|s| s.champion.as_ref().map(|c| (c.id, s))) {
            Some((id, select)) if config.auto_import_runes && runes_imported_for != Some(id) => match rune_candidate {
                Some((candidate, since)) if candidate == id && since.elapsed() > AUTO_RUNES_SETTLE => {
                    if let Err(e) = shared.import_build(id, BuildMode::from_game_mode(&select.mode), select.position.as_deref(), "runes") {
                        shared.log_error("auto import runes", e);
                    }
                    runes_imported_for = Some(id);
                }
                Some((candidate, _)) if candidate == id => {}
                _ => rune_candidate = Some((id, Instant::now())),
            },
            Some(_) => {}
            None => {
                rune_candidate = None;
                runes_imported_for = None;
            }
        }
        update_state(&app, &shared, |s| {
            s.phase = phase.clone();
            s.champion = game.as_ref().map(|g| g.champion.clone());
            s.mode = game.as_ref().map(|g| g.mode.clone());
            s.champ_select = champ_select.clone();
            if check_settings {
                s.borderless = lol::is_borderless(&shared.installation.dir);
            }
        });

        let labeled = game.filter(|g| !config.paused && LABELED_MODES.contains(&g.mode.as_str()) && (g.mode != ARENA_MODE || config.arena));
        let Some(game) = labeled else {
            if tracker.is_active() {
                hide(&mut tracker);
            }
            wait(if phase == Phase::InGame { IDLE_IN_GAME } else { IDLE_CLIENT }, &mut tracker, &mut demo_until);
            continue;
        };

        let champion = shared.catalog.lock().unwrap().as_ref().and_then(|c| c.aliases.get(&game.alias).copied());
        let Some(champion) = champion else {
            shared.log_error("unknown champion", &game.alias);
            wait(IDLE_IN_GAME, &mut tracker, &mut demo_until);
            continue;
        };
        if stats_for.as_ref() != Some(&(champion, game.mode.clone())) {
            let fetched = if game.mode == ARENA_MODE {
                opgg::fetch_arena_augments(&shared.http, champion)
            } else {
                opgg::fetch_mayhem_augments(&shared.http, champion)
            };
            match fetched {
                Ok(fetched) => {
                    augment_stats = fetched;
                    stats_for = Some((champion, game.mode.clone()));
                }
                Err(e) => {
                    shared.log_error("OP.GG augments", e);
                    wait(RETRY, &mut tracker, &mut demo_until);
                    continue;
                }
            }
        }

        let (Some(ocr), true) = (&ocr, screen::is_game_focused()) else {
            if tracker.is_active() {
                hide(&mut tracker);
                update_state(&app, &shared, |s| s.cards.clear());
            }
            wait(UNFOCUSED, &mut tracker, &mut demo_until);
            continue;
        };
        let lines = screen::capture(zone.0, zone.1, zone.2, zone.3)
            .and_then(|bgra| ocr.read(&bgra, zone.2, zone.3).map_err(|e| shared.log_error("ocr", e)).ok())
            .unwrap_or_default();
        let candidates = {
            let catalog = shared.catalog.lock().unwrap();
            let Some(catalog) = catalog.as_ref() else { continue };
            cards::card_row(&cards::find_candidates(&lines, &catalog.augment_names, zone.0 as f64, zone.1 as f64), height as f64)
        };

        match tracker.observe(candidates) {
            TrackerAction::Idle => {}
            TrackerAction::Hide => {
                if let Some(overlay) = &overlay {
                    overlay.hide();
                }
                update_state(&app, &shared, |s| s.cards.clear());
            }
            TrackerAction::Show(candidates) => {
                let rated = {
                    let catalog = shared.catalog.lock().unwrap();
                    cards::rate_cards(&candidates, &augment_stats, catalog.as_ref().map_or(&HashMap::new(), |c| &c.augments))
                };
                update_state(&app, &shared, |s| s.cards = rated.clone());
                let language = shared.language();
                if let Some(overlay) = &overlay {
                    if let Err(e) = overlay.show(&rated, &game.champion, &config, &OverlayTexts::new(language)) {
                        shared.log_error("overlay", e);
                    }
                }
                if config.voice {
                    if let (Some(voice), Some(phrase)) = (&voice, voice::best_card_phrase(&rated, language)) {
                        if let Err(e) = voice.speak(&phrase, language) {
                            shared.log_error("voice", e);
                        }
                    }
                }
                if config.record_screenshots {
                    if let Err(e) = save_screenshot(&shared, width, height) {
                        shared.log_error("screenshot", e);
                    }
                }
            }
        }
        wait(if tracker.is_active() { WATCH_CARDS } else { WATCH_GAME }, &mut tracker, &mut demo_until);
    }
}
