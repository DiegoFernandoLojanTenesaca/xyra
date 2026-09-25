use crate::{errors::Result, league::Lcu};
use reqwest::Method;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use ts_rs::TS;

const ENDPOINT: &str = "/lol-game-settings/v1/game-settings";

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WindowMode {
    Fullscreen,
    Borderless,
}

impl WindowMode {
    pub fn code(self) -> i64 {
        match self {
            WindowMode::Fullscreen => 0,
            WindowMode::Borderless => 2,
        }
    }
}

/// The official in-game options Xyra may change.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub enum GameOption {
    Borderless,
    AttackRange,
    MinimapTimers,
    TurretRange,
    MinimapScale,
    FlippedMinimap,
}

impl GameOption {
    pub const ALL: [GameOption; 6] = [
        GameOption::Borderless,
        GameOption::AttackRange,
        GameOption::MinimapTimers,
        GameOption::TurretRange,
        GameOption::MinimapScale,
        GameOption::FlippedMinimap,
    ];

    /// Section and key in the client's game settings.
    fn location(self) -> (&'static str, &'static str) {
        match self {
            GameOption::Borderless => ("General", "WindowMode"),
            GameOption::AttackRange => ("HUD", "ShowAttackRadius"),
            GameOption::MinimapTimers => ("HUD", "MinimapEnableAllTimers"),
            GameOption::TurretRange => ("General", "ShowTurretRangeIndicators"),
            GameOption::MinimapScale => ("HUD", "MinimapScale"),
            GameOption::FlippedMinimap => ("HUD", "FlipMiniMap"),
        }
    }

    fn client_value(self, value: SettingValue) -> Value {
        match (self, value) {
            (GameOption::Borderless, SettingValue::Toggle(on)) => json!(if on { WindowMode::Borderless } else { WindowMode::Fullscreen }.code()),
            (_, SettingValue::Toggle(on)) => json!(on),
            (_, SettingValue::Number(number)) => json!(number),
        }
    }

    fn parse_value(self, value: &Value) -> Option<SettingValue> {
        match self {
            GameOption::Borderless => value.as_i64().map(|code| SettingValue::Toggle(code == WindowMode::Borderless.code())),
            GameOption::MinimapScale => value.as_f64().map(SettingValue::Number),
            _ => value.as_bool().map(SettingValue::Toggle),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(untagged)]
pub enum SettingValue {
    Toggle(bool),
    Number(f64),
}

#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct GameSetting {
    pub option: GameOption,
    pub value: SettingValue,
}

pub fn parse(settings: &Value) -> Vec<GameSetting> {
    GameOption::ALL
        .into_iter()
        .filter_map(|option| {
            let (section, key) = option.location();
            Some(GameSetting { option, value: option.parse_value(&settings[section][key])? })
        })
        .collect()
}

pub fn read(lcu: &Lcu) -> Result<Vec<GameSetting>> {
    Ok(parse(&lcu.get(ENDPOINT)?))
}

pub fn update(lcu: &Lcu, option: GameOption, value: SettingValue) -> Result<()> {
    let (section, key) = option.location();
    lcu.request(Method::PATCH, ENDPOINT, Some(&json!({ section: { key: option.client_value(value) } }))).map(drop)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_client_settings_to_options() {
        let settings = json!({
            "General": { "WindowMode": 2, "ShowTurretRangeIndicators": false },
            "HUD": { "ShowAttackRadius": true, "MinimapScale": 1.5, "FlipMiniMap": false }
        });
        let parsed = parse(&settings);
        assert_eq!(parsed.len(), 5);
        assert_eq!(parsed[0], GameSetting { option: GameOption::Borderless, value: SettingValue::Toggle(true) });
        assert_eq!(parsed[3], GameSetting { option: GameOption::MinimapScale, value: SettingValue::Number(1.5) });
        assert!(!parsed.iter().any(|s| s.option == GameOption::MinimapTimers));
        assert_eq!(GameOption::Borderless.client_value(SettingValue::Toggle(false)), json!(0));
        assert_eq!(GameOption::AttackRange.client_value(SettingValue::Toggle(true)), json!(true));
    }
}
