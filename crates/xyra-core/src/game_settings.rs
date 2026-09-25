use crate::lol::Lcu;
use reqwest::{blocking::Client, Method};
use serde::Serialize;
use serde_json::{json, Value};
use ts_rs::TS;

/// The official in-game options Xyra may change, as (section, key) of `/lol-game-settings`.
pub const ALLOWED: [(&str, &str); 6] = [
    ("General", "WindowMode"),
    ("HUD", "ShowAttackRadius"),
    ("HUD", "MinimapEnableAllTimers"),
    ("General", "ShowTurretRangeIndicators"),
    ("HUD", "MinimapScale"),
    ("HUD", "FlipMiniMap"),
];

pub const BORDERLESS: i64 = 2;
const ENDPOINT: &str = "/lol-game-settings/v1/game-settings";

#[derive(Clone, Debug, Serialize, TS)]
#[ts(export)]
pub struct GameSetting {
    pub section: String,
    pub key: String,
    /// Boolean toggles, or a number (WindowMode: 0 fullscreen, 1 windowed, 2 borderless; MinimapScale).
    #[ts(type = "boolean | number")]
    pub value: Value,
}

pub fn read(lcu: &Lcu, http: &Client) -> Result<Vec<GameSetting>, String> {
    let settings = lcu.get(http, ENDPOINT)?;
    Ok(ALLOWED
        .iter()
        .filter(|(section, key)| !settings[*section][*key].is_null())
        .map(|(section, key)| GameSetting { section: section.to_string(), key: key.to_string(), value: settings[*section][*key].clone() })
        .collect())
}

pub fn update(lcu: &Lcu, http: &Client, section: &str, key: &str, value: Value) -> Result<(), String> {
    if !ALLOWED.contains(&(section, key)) {
        return Err("settingNotAllowed".into());
    }
    lcu.request(http, Method::PATCH, ENDPOINT, Some(&json!({ section: { key: value } }))).map(|_| ())
}
