use crate::{
    commands::{App, apply_config},
    engine::Shared,
    show_main_window,
};
use std::sync::Arc;
use tauri::{
    AppHandle, Manager, Wry,
    menu::{CheckMenuItem, IsMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu},
};
use xyra_core::{config::LabelStyle, i18n};

pub const TRAY_ID: &str = "main";
const STYLE_PREFIX: &str = "style:";

pub fn rebuild_menu(app: &AppHandle, shared: &Shared) {
    let config = shared.config();
    let language = shared.language();
    let t = |key: &str| i18n::t(language, key);
    let menu = (|| -> tauri::Result<Menu<Wry>> {
        let styles: Vec<CheckMenuItem<Wry>> = LabelStyle::ALL
            .into_iter()
            .map(|style| {
                let key = i18n::variant_key(style);
                CheckMenuItem::with_id(
                    app,
                    format!("{STYLE_PREFIX}{key}"),
                    t(&format!("labels:styles.{key}.name")),
                    true,
                    config.label_style == style,
                    None::<&str>,
                )
            })
            .collect::<Result<_, _>>()?;
        let style_items: Vec<&dyn IsMenuItem<Wry>> = styles.iter().map(|s| s as _).collect();
        Menu::with_items(
            app,
            &[
                &MenuItem::with_id(app, "open", t("tray:open"), true, None::<&str>)?,
                &PredefinedMenuItem::separator(app)?,
                &CheckMenuItem::with_id(app, "pause", t("tray:pause"), true, config.paused, None::<&str>)?,
                &Submenu::with_items(app, t("tray:style"), true, &style_items)?,
                &CheckMenuItem::with_id(app, "voice", t("tray:voice"), true, config.voice, None::<&str>)?,
                &PredefinedMenuItem::separator(app)?,
                &MenuItem::with_id(app, "quit", t("tray:quit"), true, None::<&str>)?,
            ],
        )
    })();
    match (menu, app.tray_by_id(TRAY_ID)) {
        (Ok(menu), Some(tray)) => {
            if let Err(e) = tray.set_menu(Some(menu)) {
                shared.log_error("tray menu", e);
            }
        }
        (Err(e), _) => shared.log_error("tray menu", e),
        _ => {}
    }
}

pub fn on_menu_event(app: &AppHandle, id: &str) {
    let shared: App = Arc::clone(app.state::<App>().inner());
    let mut config = shared.config();
    match id {
        "open" => return show_main_window(app),
        "quit" => return app.exit(0),
        "pause" => config.paused = !config.paused,
        "voice" => config.voice = !config.voice,
        other => match LabelStyle::ALL.into_iter().find(|style| other.strip_prefix(STYLE_PREFIX) == Some(i18n::variant_key(*style).as_str())) {
            Some(style) => config.label_style = style,
            None => return,
        },
    }
    if let Err(e) = apply_config(app, &shared, config) {
        shared.log_error("tray config", e);
    }
}
