use crate::{
    errors::{AppError, Result},
    league::Lcu,
};
use reqwest::Method;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use ts_rs::TS;

const GAME_ENDPOINT: &str = "/lol-game-settings/v1/game-settings";
const INPUT_ENDPOINT: &str = "/lol-game-settings/v1/input-settings";
const RIGHT_CLICK: &str = "[Button 2]";
const UNBOUND: &str = "[<Unbound>]";
const KEY_SLOTS: usize = 2;

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
    AlwaysAttackRange,
    MinimapTimers,
    TurretRange,
    MinimapScale,
    FlippedMinimap,
}

impl GameOption {
    pub const ALL: [GameOption; 7] = [
        GameOption::Borderless,
        GameOption::AttackRange,
        GameOption::AlwaysAttackRange,
        GameOption::MinimapTimers,
        GameOption::TurretRange,
        GameOption::MinimapScale,
        GameOption::FlippedMinimap,
    ];

    /// Section and key in the client's game or key settings.
    fn location(self) -> (&'static str, &'static str) {
        match self {
            GameOption::Borderless => ("General", "WindowMode"),
            GameOption::AttackRange => ("HUD", "ShowAttackRadius"),
            GameOption::AlwaysAttackRange => ("GameEvents", "evtPlayerAttackMove"),
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

    fn endpoint(self) -> &'static str {
        match self {
            GameOption::AlwaysAttackRange => INPUT_ENDPOINT,
            _ => GAME_ENDPOINT,
        }
    }

    fn parse_value(self, value: &Value) -> Option<SettingValue> {
        match self {
            GameOption::AlwaysAttackRange => value.as_str().map(|keys| SettingValue::Toggle(keys.split(',').any(|key| key == RIGHT_CLICK))),
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

pub fn parse(game: &Value, input: &Value) -> Vec<GameSetting> {
    GameOption::ALL
        .into_iter()
        .filter_map(|option| {
            let (section, key) = option.location();
            let settings = if option.endpoint() == INPUT_ENDPOINT { input } else { game };
            Some(GameSetting { option, value: option.parse_value(&settings[section][key])? })
        })
        .collect()
}

pub fn read(lcu: &Lcu) -> Result<Vec<GameSetting>> {
    Ok(parse(&lcu.get(GAME_ENDPOINT)?, &lcu.get(INPUT_ENDPOINT)?))
}

/// Changes one option and returns every option as the client now has them.
pub fn update(lcu: &Lcu, option: GameOption, value: SettingValue) -> Result<Vec<GameSetting>> {
    let (section, key) = option.location();
    let client_value = match (option, value) {
        (GameOption::AlwaysAttackRange, SettingValue::Toggle(on)) => {
            let keys = with_right_click(lcu.get(INPUT_ENDPOINT)?[section][key].as_str().unwrap_or_default(), on)?;
            if on {
                update(lcu, GameOption::AttackRange, value)?;
            }
            json!(keys)
        }
        _ => option.client_value(value),
    };
    lcu.request(Method::PATCH, option.endpoint(), Some(&json!({ section: { key: client_value } })))?;
    read(lcu)
}

/// Adds or removes the right click among an action's key slots, keeping the player's other keys.
fn with_right_click(keys: &str, on: bool) -> Result<String> {
    let mut slots: Vec<&str> = keys.split(',').filter(|slot| !slot.is_empty()).collect();
    match (on, slots.contains(&RIGHT_CLICK)) {
        (true, false) => match slots.iter().position(|&slot| slot == UNBOUND) {
            Some(free) => slots[free] = RIGHT_CLICK,
            None if slots.len() < KEY_SLOTS => slots.push(RIGHT_CLICK),
            None => return Err(AppError::NoFreeKeySlot),
        },
        (false, true) => slots.iter_mut().filter(|slot| **slot == RIGHT_CLICK).for_each(|slot| *slot = UNBOUND),
        _ => {}
    }
    Ok(slots.join(","))
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
        let input = json!({ "GameEvents": { "evtPlayerAttackMove": "[Button 2],[x]" } });
        let parsed = parse(&settings, &input);
        assert_eq!(parsed.len(), 6);
        assert_eq!(parsed[2], GameSetting { option: GameOption::AlwaysAttackRange, value: SettingValue::Toggle(true) });
        assert_eq!(parsed[0], GameSetting { option: GameOption::Borderless, value: SettingValue::Toggle(true) });
        assert_eq!(parsed[4], GameSetting { option: GameOption::MinimapScale, value: SettingValue::Number(1.5) });
        assert!(!parsed.iter().any(|s| s.option == GameOption::MinimapTimers));
        assert_eq!(GameOption::Borderless.client_value(SettingValue::Toggle(false)), json!(0));
        assert_eq!(GameOption::AttackRange.client_value(SettingValue::Toggle(true)), json!(true));
    }

    #[test]
    fn binds_the_right_click_without_losing_other_keys() {
        assert_eq!(with_right_click("[<Unbound>],[x]", true).unwrap(), "[Button 2],[x]");
        assert_eq!(with_right_click("[a]", true).unwrap(), "[a],[Button 2]");
        assert_eq!(with_right_click("", true).unwrap(), "[Button 2]");
        assert_eq!(with_right_click("[Button 2],[x]", true).unwrap(), "[Button 2],[x]");
        assert_eq!(with_right_click("[a],[x]", true), Err(AppError::NoFreeKeySlot));
        assert_eq!(with_right_click("[Button 2],[x]", false).unwrap(), "[<Unbound>],[x]");
        assert_eq!(with_right_click("[a],[x]", false).unwrap(), "[a],[x]");
    }
}
