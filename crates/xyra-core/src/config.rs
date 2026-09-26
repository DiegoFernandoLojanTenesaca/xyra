use crate::i18n;
use serde::{Deserialize, Serialize};
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

const DEFAULT_ACCEPT_DELAY_SECONDS: u32 = 2;

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
    /// Seconds to wait before accepting a found match.
    pub accept_delay_seconds: u32,
    pub champion_order: ChampionOrder,
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
            accept_delay_seconds: DEFAULT_ACCEPT_DELAY_SECONDS,
            champion_order: ChampionOrder::default(),
        }
    }
}

impl Config {
    pub fn effective_language(&self, client_locale: &str) -> &'static str {
        i18n::resolve(self.language.as_deref().unwrap_or(client_locale))
    }
}
