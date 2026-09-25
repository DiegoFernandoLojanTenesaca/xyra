use crate::{
    engine::{EngineEvent, Shared, emit, update_state},
    tray,
};
use std::sync::Arc;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_opener::OpenerExt;
use xyra_core::{
    config::Config,
    errors::{AppError, Result},
    game_settings::{self, GameOption, GameSetting, SettingValue},
    model::{AppEvent, AugmentRow, Build, BuildMode, ChampionInfo, Choices, EngineState, GameMode, ImportTarget, Position},
    opgg,
    profile::{self, Profile},
    stats::{self, StatsSummary},
    storage::{DataFolder, DataUsage},
};

pub type App = Arc<Shared>;

const CSV_FILE: &str = "xyra-games.csv";

async fn blocking<T: Send + 'static>(task: impl FnOnce() -> Result<T> + Send + 'static) -> Result<T> {
    tauri::async_runtime::spawn_blocking(task).await.map_err(AppError::platform)?
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
pub fn get_choices() -> Choices {
    Choices::all()
}

#[tauri::command]
pub fn set_config(app: AppHandle, shared: State<App>, config: Config) -> Result<Config> {
    apply_config(&app, &shared, config)
}

pub fn apply_config(app: &AppHandle, shared: &Shared, config: Config) -> Result<Config> {
    if config.autostart != shared.config().autostart {
        let launcher = app.autolaunch();
        if config.autostart { launcher.enable() } else { launcher.disable() }.map_err(AppError::platform)?;
    }
    shared.storage.save_config(&config)?;
    *shared.config.lock().unwrap() = config.clone();
    let language = shared.language();
    update_state(app, shared, |s| s.language = language.into());
    emit(app, shared, AppEvent::Config, config.clone());
    tray::rebuild_menu(app, shared);
    shared.send(EngineEvent::ConfigChanged);
    Ok(config)
}

#[tauri::command]
pub fn get_stats(shared: State<App>) -> StatsSummary {
    shared.stats_summary()
}

#[tauri::command]
pub fn get_champions(shared: State<App>) -> Vec<ChampionInfo> {
    shared.champions()
}

#[tauri::command]
pub async fn get_augments(shared: State<'_, App>, champion: u32, mode: GameMode) -> Result<Vec<AugmentRow>> {
    let shared = Arc::clone(&shared);
    blocking(move || Ok(opgg::augment_rows(opgg::fetch_augments(&shared.opgg, champion, mode)?, &shared.catalog()))).await
}

#[tauri::command]
pub async fn get_build(shared: State<'_, App>, champion: u32, mode: BuildMode, position: Option<Position>) -> Result<Build> {
    let shared = Arc::clone(&shared);
    blocking(move || shared.fetch_build(champion, mode, position)).await
}

#[tauri::command]
pub async fn import_build(shared: State<'_, App>, champion: u32, target: ImportTarget, mode: BuildMode, position: Option<Position>) -> Result<()> {
    let shared = Arc::clone(&shared);
    blocking(move || shared.import_build(champion, mode, position, target)).await
}

#[tauri::command]
pub async fn get_game_settings(shared: State<'_, App>) -> Result<Vec<GameSetting>> {
    let shared = Arc::clone(&shared);
    blocking(move || game_settings::read(&shared.lcu()?)).await
}

#[tauri::command]
pub async fn set_game_setting(shared: State<'_, App>, option: GameOption, value: SettingValue) -> Result<()> {
    let shared = Arc::clone(&shared);
    blocking(move || game_settings::update(&shared.lcu()?, option, value)).await
}

/// Reads the profile from the client and keeps a copy; with League closed, returns that copy.
#[tauri::command]
pub async fn get_profile(shared: State<'_, App>) -> Result<Option<Profile>> {
    let shared = Arc::clone(&shared);
    blocking(move || {
        let Ok(lcu) = shared.lcu() else { return shared.storage.load_profile() };
        let profile = profile::read(&lcu, &shared.catalog())?;
        shared.storage.save_profile(&profile)?;
        Ok(Some(profile))
    })
    .await
}

#[tauri::command]
pub fn get_data_usage(shared: State<App>) -> DataUsage {
    shared.storage.usage()
}

/// Saves the games as CSV in Downloads and shows the file.
#[tauri::command]
pub fn export_csv(app: AppHandle, shared: State<App>) -> Result<String> {
    let path = app.path().download_dir().map_err(AppError::platform)?.join(CSV_FILE);
    stats::write_csv(&path, &shared.games.lock().unwrap(), &shared.catalog(), shared.language())?;
    app.opener().reveal_item_in_dir(&path).map_err(AppError::platform)?;
    Ok(path.to_string_lossy().into())
}

#[tauri::command]
pub fn delete_data(app: AppHandle, shared: State<App>) -> Result<()> {
    shared.storage.delete_personal_data()?;
    shared.games.lock().unwrap().clear();
    emit(&app, &shared, AppEvent::Data, ());
    Ok(())
}

#[tauri::command]
pub fn test_overlay(shared: State<App>) {
    shared.send(EngineEvent::ShowDemo);
}

#[tauri::command]
pub fn test_voice(shared: State<App>) {
    shared.send(EngineEvent::TestVoice);
}

#[tauri::command]
pub fn set_borderless(shared: State<App>) -> Result<()> {
    shared.set_borderless()
}

#[tauri::command]
pub fn open_folder(app: AppHandle, shared: State<App>, folder: DataFolder) -> Result<()> {
    let path = shared.storage.folder(folder)?;
    app.opener().open_path(path.to_string_lossy(), None::<&str>).map_err(AppError::platform)
}
