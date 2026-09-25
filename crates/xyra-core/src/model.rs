use crate::cards::Card;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct ChampionInfo {
    pub id: u32,
    pub name: String,
    pub icon: String,
    /// ARAM: Mayhem tier, 1 = best … 5.
    pub tier: Option<u8>,
    pub rank: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct ChampSelect {
    pub champion: Option<ChampionInfo>,
    pub bench: Vec<ChampionInfo>,
    /// Client game mode: "ARAM", "KIWI" (ARAM: Mayhem), "CLASSIC" (Summoner's Rift), "CHERRY" (Arena)…
    pub mode: String,
    pub position: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[ts(export, rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    NoClient,
    Client,
    ChampSelect,
    InGame,
    Paused,
}

#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct EngineState {
    pub phase: Phase,
    pub champion: Option<String>,
    pub mode: Option<String>,
    pub cards: Vec<Card>,
    pub champ_select: Option<ChampSelect>,
    /// None when the game settings could not be read.
    pub borderless: Option<bool>,
    pub client_locale: String,
    /// UI language already resolved from the preference and the client locale.
    pub language: String,
    pub ocr_language: Option<String>,
    pub version: String,
}

/// Events the engine pushes to the UI.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, TS)]
#[ts(export, rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub enum AppEvent {
    State,
    Config,
    Stats,
}

impl AppEvent {
    pub fn name(self) -> &'static str {
        match self {
            AppEvent::State => "state",
            AppEvent::Config => "config",
            AppEvent::Stats => "stats",
        }
    }
}

#[derive(Clone, Debug, Serialize, TS)]
#[ts(export)]
pub struct Asset {
    pub id: u32,
    pub name: String,
    pub icon: String,
}

#[derive(Clone, Debug, Serialize, TS)]
#[ts(export)]
pub struct RunePage {
    pub primary_style: Asset,
    pub secondary_style: Asset,
    pub primary: Vec<Asset>,
    pub secondary: Vec<Asset>,
    pub shards: Vec<Asset>,
    pub win_rate: f64,
    pub pick_rate: f64,
}

#[derive(Clone, Debug, Serialize, TS)]
#[ts(export)]
pub struct Build {
    pub champion: u32,
    pub runes: RunePage,
    pub spells: Vec<Asset>,
    pub starting_items: Vec<Asset>,
    pub boots: Vec<Asset>,
    pub core_items: Vec<Asset>,
    pub situational_items: Vec<Asset>,
    pub skill_order: Vec<String>,
    pub skill_priority: Vec<String>,
    pub win_rate: f64,
    pub games: u32,
    pub position: Option<String>,
    /// Positions the champion is played in, most played first.
    pub positions: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Deserialize, Serialize, TS)]
#[ts(export, rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum BuildMode {
    Aram,
    Rift,
}

impl BuildMode {
    pub fn from_game_mode(mode: &str) -> BuildMode {
        if mode == "CLASSIC" {
            BuildMode::Rift
        } else {
            BuildMode::Aram
        }
    }
}

#[derive(Clone, Debug, Serialize, TS)]
#[ts(export)]
pub struct AugmentRow {
    pub id: u32,
    pub name: String,
    pub icon: String,
    /// "kSilver" | "kGold" | "kPrismatic"
    pub rarity: String,
    pub tier: u8,
    pub quality: Quality,
    pub grade: String,
    pub performance: f64,
    pub pick_rate: f64,
}

/// i18n key and color token of an augment tier.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, TS)]
#[ts(export, rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub enum Quality {
    Excellent,
    Great,
    Good,
    Fair,
    Bad,
    RarePick,
}

const GRADES: [&str; 7] = ["S", "A", "B", "C", "D", "E", "F"];

impl Quality {
    pub fn of(tier: Option<u8>) -> Quality {
        match tier {
            None => Quality::RarePick,
            Some(0) => Quality::Excellent,
            Some(1) => Quality::Great,
            Some(2) => Quality::Good,
            Some(3) => Quality::Fair,
            Some(_) => Quality::Bad,
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Quality::Excellent => "excellent",
            Quality::Great => "great",
            Quality::Good => "good",
            Quality::Fair => "fair",
            Quality::Bad => "bad",
            Quality::RarePick => "rarePick",
        }
    }
}

pub fn grade(tier: Option<u8>) -> String {
    tier.map_or("?", |t| GRADES[(t as usize).min(GRADES.len() - 1)]).to_string()
}
