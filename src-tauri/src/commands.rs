use crate::{
    engine::{update_state, Shared, MAIN_WINDOW},
    tray,
};
use std::{
    fs,
    path::Path,
    sync::{atomic::Ordering, Arc},
};
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_opener::OpenerExt;
use xyra_core::{
    catalog::Catalog,
    config::{Config, LABEL_STYLES},
    game_settings::{self, GameSetting},
    lol,
    model::{grade, AppEvent, AugmentRow, Build, BuildMode, ChampionInfo, EngineState, Phase, Quality},
    opgg,
    profile::{self, Profile},
    stats::{self, StatsSummary},
};

pub type App = Arc<Shared>;

const CSV_FILE: &str = "xyra-games.csv";
const UTF8_BOM: char = '\u{feff}';

async fn blocking<T: Send + 'static>(task: impl FnOnce() -> Result<T, String> + Send + 'static) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(task).await.map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn get_state(shared: State<App>) -> EngineState {
    shared.state.lock().unwrap().clone()
}

#[tauri::command]
pub fn get_config(shared: State<App>) -> Config {
    shared.config()
}

#[tauri::command]
pub fn get_label_styles() -> Vec<&'static str> {
    LABEL_STYLES.to_vec()
}

#[tauri::command]
pub fn set_config(app: AppHandle, shared: State<App>, config: Config) -> Result<Config, String> {
    apply_config(&app, &shared, config)
}

pub fn apply_config(app: &AppHandle, shared: &Shared, config: Config) -> Result<Config, String> {
    if config.autostart != shared.config().autostart {
        let launcher = app.autolaunch();
        if config.autostart { launcher.enable() } else { launcher.disable() }.map_err(|e| e.to_string())?;
    }
    config.save(&shared.paths.config)?;
    *shared.config.lock().unwrap() = config.clone();
    let language = shared.language();
    update_state(app, shared, |s| s.language = language.into());
    let _ = app.emit_to(MAIN_WINDOW, AppEvent::Config.name(), config.clone());
    tray::rebuild_menu(app, shared);
    Ok(config)
}

#[tauri::command]
pub fn get_stats(shared: State<App>) -> StatsSummary {
    let catalog = shared.catalog.lock().unwrap();
    stats::summarize(&shared.games.lock().unwrap(), catalog.as_ref().unwrap_or(&Catalog::default()))
}

#[tauri::command]
pub fn test_overlay(shared: State<App>) {
    if shared.state.lock().unwrap().phase != Phase::InGame {
        shared.demo_requested.store(true, Ordering::Relaxed);
    }
}

#[tauri::command]
pub fn get_champions(shared: State<App>) -> Vec<ChampionInfo> {
    let ids: Vec<u32> = shared.catalog.lock().unwrap().as_ref().map(|c| c.champions.keys().copied().collect()).unwrap_or_default();
    let mut champions: Vec<ChampionInfo> = ids.into_iter().map(|id| shared.champion_info(id)).collect();
    champions.sort_by(|a, b| (a.rank.is_none(), a.rank, &a.name).cmp(&(b.rank.is_none(), b.rank, &b.name)));
    champions
}

#[tauri::command]
pub async fn get_augments(shared: State<'_, App>, champion: u32, mode: String) -> Result<Vec<AugmentRow>, String> {
    let shared = Arc::clone(&shared);
    blocking(move || {
        let stats = if mode == "CHERRY" { opgg::fetch_arena_augments(&shared.http, champion)? } else { opgg::fetch_mayhem_augments(&shared.http, champion)? };
        let catalog = shared.catalog.lock().unwrap();
        let Some(catalog) = catalog.as_ref() else { return Ok(Vec::new()) };
        let mut rows: Vec<AugmentRow> = stats
            .into_iter()
            .filter_map(|(id, stat)| {
                let (name, icon) = catalog.augments.get(&id)?.clone();
                Some(AugmentRow {
                    id,
                    name,
                    icon,
                    rarity: catalog.rarity.get(&id).cloned().unwrap_or_default(),
                    tier: stat.tier,
                    quality: Quality::of(Some(stat.tier)),
                    grade: grade(Some(stat.tier)),
                    performance: stat.performance,
                    pick_rate: stat.pick_rate,
                })
            })
            .collect();
        rows.sort_by(|a, b| a.tier.cmp(&b.tier).then(b.performance.total_cmp(&a.performance)));
        Ok(rows)
    })
    .await
}

#[tauri::command]
pub async fn get_build(shared: State<'_, App>, champion: u32, mode: BuildMode, position: Option<String>) -> Result<Build, String> {
    let shared = Arc::clone(&shared);
    blocking(move || shared.fetch_build(champion, mode, position.as_deref())).await
}

#[tauri::command]
pub async fn import_build(shared: State<'_, App>, champion: u32, target: String, mode: BuildMode, position: Option<String>) -> Result<(), String> {
    let shared = Arc::clone(&shared);
    blocking(move || shared.import_build(champion, mode, position.as_deref(), &target)).await
}

#[tauri::command]
pub async fn get_game_settings(shared: State<'_, App>) -> Result<Vec<GameSetting>, String> {
    let shared = Arc::clone(&shared);
    blocking(move || game_settings::read(&lol::Lcu::connect(&shared.installation.dir).ok_or("noClient")?, &shared.http)).await
}

#[tauri::command]
pub async fn set_game_setting(shared: State<'_, App>, section: String, key: String, value: serde_json::Value) -> Result<(), String> {
    let shared = Arc::clone(&shared);
    blocking(move || game_settings::update(&lol::Lcu::connect(&shared.installation.dir).ok_or("noClient")?, &shared.http, &section, &key, value)).await
}

/// Reads the profile from the client and caches it; with League closed, returns the cached one.
#[tauri::command]
pub async fn get_profile(shared: State<'_, App>) -> Result<Option<Profile>, String> {
    let shared = Arc::clone(&shared);
    blocking(move || {
        let champions = shared.catalog.lock().unwrap().as_ref().map(|c| c.champions.clone()).unwrap_or_default();
        let Some(lcu) = lol::Lcu::connect(&shared.installation.dir) else { return Ok(profile::load(&shared.paths.profile)) };
        let fresh = profile::read(&lcu, &shared.http, &champions)?;
        profile::save(&shared.paths.profile, &fresh)?;
        Ok(Some(fresh))
    })
    .await
}

fn dir_size(path: &Path) -> u64 {
    fs::read_dir(path)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.metadata().map_or(0, |m| if m.is_dir() { dir_size(&entry.path()) } else { m.len() }))
        .sum()
}

#[tauri::command]
pub fn get_data_size(shared: State<App>) -> u64 {
    dir_size(&shared.paths.data)
}

/// Saves the games as CSV in Downloads (with a BOM so Excel reads accents) and reveals it.
#[tauri::command]
pub fn export_csv(app: AppHandle, shared: State<App>) -> Result<String, String> {
    let csv = {
        let catalog = shared.catalog.lock().unwrap();
        stats::to_csv(&shared.games.lock().unwrap(), catalog.as_ref().unwrap_or(&Catalog::default()), shared.language())
    };
    let path = app.path().download_dir().map_err(|e| e.to_string())?.join(CSV_FILE);
    fs::write(&path, format!("{UTF8_BOM}{csv}")).map_err(|e| e.to_string())?;
    app.opener().reveal_item_in_dir(&path).map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().into())
}

/// Deletes the personal data (games, profile, screenshots, log); settings stay.
#[tauri::command]
pub fn delete_data(shared: State<App>) -> Result<(), String> {
    shared.games.lock().unwrap().clear();
    for file in [&shared.paths.stats, &shared.paths.profile, &shared.paths.log] {
        if file.exists() {
            fs::remove_file(file).map_err(|e| e.to_string())?;
        }
    }
    if shared.paths.screenshots.exists() {
        fs::remove_dir_all(&shared.paths.screenshots).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn set_borderless(shared: State<App>) -> Result<(), String> {
    lol::set_borderless(&shared.installation.dir, &shared.http)?;
    shared.state.lock().unwrap().borderless = lol::is_borderless(&shared.installation.dir);
    Ok(())
}

/// `target`: "screenshots", "log" or "data".
#[tauri::command]
pub fn open_folder(app: AppHandle, shared: State<App>, target: String) -> Result<(), String> {
    let path = match target.as_str() {
        "screenshots" => shared.paths.screenshots.clone(),
        "log" => shared.paths.log.clone(),
        _ => shared.paths.data.clone(),
    };
    if target == "screenshots" {
        fs::create_dir_all(&path).map_err(|e| e.to_string())?;
    }
    app.opener().open_path(path.to_string_lossy(), None::<&str>).map_err(|e| e.to_string())
}
