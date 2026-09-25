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
    #[serde(alias = "auto_bordes")]
    pub keep_borderless: bool,
    #[serde(alias = "auto_runas")]
    pub auto_import_runes: bool,
    #[serde(alias = "cerrar_en_partida")]
    pub close_window_in_game: bool,
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
            keep_borderless: true,
            auto_import_runes: false,
            close_window_in_game: false,
        }
    }
}

impl Config {
    pub fn effective_language(&self, client_locale: &str) -> &'static str {
        i18n::resolve(self.language.as_deref().unwrap_or(client_locale))
    }
}
