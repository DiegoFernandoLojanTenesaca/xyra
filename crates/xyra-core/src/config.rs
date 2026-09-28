use crate::i18n;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use ts_rs::TS;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub enum LabelStyle {
    #[default]
    #[serde(alias = "placa")]
    Plate,
    #[serde(alias = "insignia")]
    Badge,
    #[serde(alias = "cinta")]
    Ribbon,
    #[serde(alias = "podio")]
    Podium,
    #[serde(alias = "enfoque")]
    Focus,
}

/// How Xyra ranks the champions it recommends.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub enum ChampionOrder {
    /// OP.GG rank in ARAM: Mayhem.
    #[default]
    Tier,
    /// Mastery points of the account.
    Mastery,
    /// Games of the account that Xyra stored.
    Played,
    /// Best tier first and, within a tier, the champions the player masters most.
    Balanced,
}

impl ChampionOrder {
    pub const ALL: [ChampionOrder; 4] = [ChampionOrder::Tier, ChampionOrder::Mastery, ChampionOrder::Played, ChampionOrder::Balanced];
}

/// How Xyra dresses the champion the player gets in champion select.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub enum SkinChoice {
    /// The client's own choice stays.
    #[default]
    Off,
    /// One of the player's skins at random.
    Random,
    /// The player's favorite of the champion, or one at random without one.
    Favorite,
}

const DEFAULT_ACCEPT_DELAY_SECONDS: u32 = 2;
const PHONE_TOKEN_BYTES: usize = 16;

impl LabelStyle {
    pub const ALL: [LabelStyle; 5] = [LabelStyle::Plate, LabelStyle::Badge, LabelStyle::Ribbon, LabelStyle::Podium, LabelStyle::Focus];
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(default)]
pub struct Config {
    #[serde(alias = "estilo")]
    pub label_style: LabelStyle,
    #[serde(alias = "voz")]
    pub voice: bool,
    pub arena: bool,
    #[serde(alias = "pausado")]
    pub paused: bool,
    /// Language code; None follows the client locale.
    #[serde(alias = "idioma")]
    pub language: Option<String>,
    /// Vertical offset of the labels, in pixels at a 1200 px tall screen.
    pub offset_y: f64,
    #[serde(alias = "escala")]
    pub scale: f64,
    #[serde(alias = "grabar")]
    pub record_screenshots: bool,
    pub autostart: bool,
    /// Opens League when the player opens Xyra.
    pub open_league: bool,
    #[serde(alias = "auto_bordes")]
    pub keep_borderless: bool,
    /// Imports runes, items and summoner spells of the picked champion in champion select.
    #[serde(alias = "auto_runas", alias = "auto_import_runes")]
    pub auto_import_build: bool,
    #[serde(alias = "cerrar_en_partida")]
    pub close_window_in_game: bool,
    pub auto_accept: bool,
    /// Play League's match found sound from Xyra too, for a muted client.
    pub match_sound: bool,
    /// Seconds to wait before accepting a found match.
    pub accept_delay_seconds: u32,
    pub champion_order: ChampionOrder,
    /// Serves Xyra to phones on the same network.
    pub phone_link: bool,
    /// Pairing code the phone sends with every request; empty until the link is first turned on.
    pub phone_token: String,
    pub skin_choice: SkinChoice,
    /// Random skins may also be a chroma the player owns.
    pub skin_chromas: bool,
    /// The favorite skin or chroma of each champion.
    #[ts(type = "Record<number, number>")]
    pub favorite_skins: HashMap<u32, u32>,
    /// Shows on the player's Discord profile what they play.
    pub discord_presence: bool,
    /// The champion, its image and the KDA in the Discord presence.
    pub discord_details: bool,
    /// "Using Xyra" on Discord while the player is not in a game, a lobby or champion select.
    pub discord_idle: bool,
}

impl Config {
    /// Whether `token` is the phone pairing code, compared in constant time.
    pub fn is_phone_token(&self, token: &str) -> bool {
        same_secret(token, &self.phone_token)
    }
}

/// Compares two secrets in constant time; an empty secret never matches.
pub fn same_secret(given: &str, expected: &str) -> bool {
    !expected.is_empty() && given.len() == expected.len() && given.bytes().zip(expected.bytes()).fold(0, |diff, (a, b)| diff | (a ^ b)) == 0
}

/// A new random pairing code for the phone link.
pub fn new_phone_token() -> String {
    let mut bytes = [0u8; PHONE_TOKEN_BYTES];
    ring::rand::SecureRandom::fill(&ring::rand::SystemRandom::new(), &mut bytes).expect("system random numbers");
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

impl Default for Config {
    fn default() -> Self {
        Config {
            label_style: LabelStyle::default(),
            voice: false,
            arena: true,
            paused: false,
            language: None,
            offset_y: 0.0,
            scale: 1.0,
            record_screenshots: false,
            autostart: true,
            open_league: false,
            keep_borderless: true,
            auto_import_build: false,
            close_window_in_game: false,
            auto_accept: false,
            match_sound: false,
            accept_delay_seconds: DEFAULT_ACCEPT_DELAY_SECONDS,
            champion_order: ChampionOrder::default(),
            phone_link: false,
            phone_token: String::new(),
            skin_choice: SkinChoice::default(),
            skin_chromas: false,
            favorite_skins: HashMap::new(),
            discord_presence: false,
            discord_details: true,
            discord_idle: true,
        }
    }
}

impl Config {
    pub fn effective_language(&self, client_locale: &str) -> &'static str {
        i18n::resolve(self.language.as_deref().unwrap_or(client_locale))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checks_the_phone_pairing_code() {
        let config = Config { phone_token: new_phone_token(), ..Config::default() };
        assert_eq!(config.phone_token.len(), 2 * PHONE_TOKEN_BYTES);
        assert!(config.is_phone_token(&config.phone_token.clone()));
        assert!(!config.is_phone_token("0"));
        assert!(!Config::default().is_phone_token(""));
    }
}
