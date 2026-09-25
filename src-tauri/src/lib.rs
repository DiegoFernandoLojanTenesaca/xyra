mod commands;
mod engine;
mod overlay;
mod previews;
mod screen;
mod tray;
mod voice;

use commands::App;
use engine::{Shared, MAIN_WINDOW};
use std::{
    sync::{atomic::Ordering, Arc},
    thread,
};
use tauri::{
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    window::Color,
    AppHandle, Manager, RunEvent, Theme, WebviewUrl, WebviewWindowBuilder,
};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt};
use xyra_core::{lol, theme};

const HIDDEN_FLAG: &str = "--hidden";
const DEMO_FLAG: &str = "--demo";
const PREVIEWS_FLAG: &str = "--previews";
const DEFAULT_PREVIEW_BACKGROUND: &str = "docs/cards-background.png";
const DEFAULT_PREVIEW_OUTPUT: &str = "previews";
const DEFAULT_PREVIEW_LANGUAGE: &str = "es";
const WINDOW_SIZE: (f64, f64) = (1180.0, 760.0);
const WINDOW_MIN_SIZE: (f64, f64) = (960.0, 620.0);

fn background_color() -> Color {
    let rgb = theme::color("color.background");
    Color((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8, 255)
}

/// The main window is created on open and destroyed on close: no browser runs while you play.
pub fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW) {
        let _ = window.unminimize();
        let _ = window.set_focus();
        return;
    }
    let created = WebviewWindowBuilder::new(app, MAIN_WINDOW, WebviewUrl::App("index.html".into()))
        .title("Xyra")
        .inner_size(WINDOW_SIZE.0, WINDOW_SIZE.1)
        .min_inner_size(WINDOW_MIN_SIZE.0, WINDOW_MIN_SIZE.1)
        .center()
        .decorations(false)
        .shadow(true)
        .theme(Some(Theme::Dark))
        .background_color(background_color())
        .build();
    if let Err(e) = created {
        app.state::<App>().log_error("main window", e);
    }
}

fn flag(name: &str) -> bool {
    std::env::args().any(|a| a == name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let args: Vec<String> = std::env::args().collect();
    if let Some(i) = args.iter().position(|a| a == PREVIEWS_FLAG) {
        let arg = |n: usize, default: &'static str| args.get(i + n).map_or(default, |s| s.as_str());
        if let Err(e) = previews::render_style_previews(arg(1, DEFAULT_PREVIEW_BACKGROUND).as_ref(), arg(2, DEFAULT_PREVIEW_OUTPUT).as_ref(), arg(3, DEFAULT_PREVIEW_LANGUAGE)) {
            eprintln!("previews: {e}");
        }
        return;
    }
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| show_main_window(app)))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, Some(vec![HIDDEN_FLAG])))
        .setup(|app| {
            let shared: App = Arc::new(Shared::new(app.path().app_data_dir()?));
            app.manage(Arc::clone(&shared));
            if shared.config().autostart && !app.autolaunch().is_enabled().unwrap_or(false) {
                app.autolaunch().enable()?;
            }
            TrayIconBuilder::with_id(tray::TRAY_ID)
                .icon(app.default_window_icon().cloned().ok_or("missing app icon")?)
                .tooltip("Xyra")
                .show_menu_on_left_click(false)
                .on_menu_event(|app, e| tray::on_menu_event(app, e.id().as_ref()))
                .on_tray_icon_event(|tray, e| {
                    if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = e {
                        show_main_window(tray.app_handle());
                    }
                })
                .build(app)?;
            tray::rebuild_menu(app.handle(), &shared);
            if flag(DEMO_FLAG) {
                shared.demo_requested.store(true, Ordering::Relaxed);
            }
            let start_hidden = flag(HIDDEN_FLAG) || flag(DEMO_FLAG) || lol::read_live_game(&shared.http).is_some();
            if !start_hidden {
                show_main_window(app.handle());
            }
            let handle = app.handle().clone();
            thread::spawn(move || engine::run(handle, shared));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_state,
            commands::get_config,
            commands::get_label_styles,
            commands::set_config,
            commands::get_stats,
            commands::get_champions,
            commands::get_augments,
            commands::get_profile,
            commands::get_build,
            commands::import_build,
            commands::get_game_settings,
            commands::set_game_setting,
            commands::get_data_size,
            commands::export_csv,
            commands::delete_data,
            commands::test_overlay,
            commands::set_borderless,
            commands::open_folder
        ])
        .build(tauri::generate_context!())
        .expect("Xyra failed to start")
        .run(|_, event| {
            if let RunEvent::ExitRequested { api, code: None, .. } = event {
                api.prevent_exit();
            }
        });
}
