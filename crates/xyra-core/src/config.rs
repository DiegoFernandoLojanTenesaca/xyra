use crate::i18n;
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};
use ts_rs::TS;

pub const LABEL_STYLES: [&str; 5] = ["plate", "badge", "ribbon", "podium", "focus"];

#[derive(Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(default)]
pub struct Config {
    pub label_style: String,
    pub voice: bool,
    pub arena: bool,
    pub paused: bool,
    /// "auto" follows the client locale.
    pub language: String,
    /// Vertical offset of the labels, in pixels at a 1200 px tall screen.
    pub offset_y: f64,
    pub scale: f64,
    pub record_screenshots: bool,
    pub autostart: bool,
    pub keep_borderless: bool,
    pub auto_import_runes: bool,
    pub close_window_in_game: bool,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            label_style: LABEL_STYLES[0].into(),
            voice: false,
            arena: true,
            paused: false,
            language: "auto".into(),
            offset_y: 0.0,
            scale: 1.0,
            record_screenshots: false,
            autostart: true,
            keep_borderless: true,
            auto_import_runes: false,
            close_window_in_game: false,
        }
    }
}

impl Config {
    pub fn load(path: &Path) -> Config {
        fs::read_to_string(path).ok().and_then(|text| serde_json::from_str(&text).ok()).unwrap_or_default()
    }

    pub fn save(&self, path: &Path) -> Result<(), String> {
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        fs::write(path, serde_json::to_string_pretty(self).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
    }

    pub fn effective_language(&self, client_locale: &str) -> &'static str {
        i18n::resolve(if self.language == "auto" { client_locale } else { &self.language })
    }
}
