use crate::{
    cards::Card,
    config::{ChampionOrder, LabelStyle},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::cmp::Reverse;
use ts_rs::TS;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub enum GameMode {
    #[serde(alias = "KIWI")]
    Mayhem,
    #[serde(alias = "CHERRY")]
    Arena,
    #[serde(alias = "ARAM")]
    Aram,
    #[serde(alias = "CLASSIC")]
    SummonersRift,
    #[serde(other)]
    Other,
}

impl GameMode {
    pub const WITH_AUGMENTS: [GameMode; 2] = [GameMode::Mayhem, GameMode::Arena];

    pub fn from_client(code: &str) -> GameMode {
        GameMode::deserialize(Value::String(code.into())).unwrap_or(GameMode::Other)
    }

    pub fn has_augments(self) -> bool {
        GameMode::WITH_AUGMENTS.contains(&self)
    }

    /// Modes with runes, summoner spells and item sets to import: Arena has none of them.
    pub fn has_builds(self) -> bool {
        !matches!(self, GameMode::Arena | GameMode::Other)
    }

    pub fn build_mode(self) -> BuildMode {
        if self == GameMode::SummonersRift { BuildMode::Rift } else { BuildMode::Aram }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Deserialize, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub enum BuildMode {
    Aram,
    Rift,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub enum Position {
    Top,
    Jungle,
    Mid,
    Adc,
    Support,
}

impl Position {
    pub const ALL: [Position; 5] = [Position::Top, Position::Jungle, Position::Mid, Position::Adc, Position::Support];

    /// Parses the client's assigned position ("middle", "bottom", "utility"…).
    pub fn from_client(name: &str) -> Option<Position> {
        match name {
            "top" => Some(Position::Top),
            "jungle" => Some(Position::Jungle),
            "middle" => Some(Position::Mid),
            "bottom" => Some(Position::Adc),
            "utility" => Some(Position::Support),
            _ => None,
        }
    }

    pub fn from_opgg(name: &str) -> Option<Position> {
        Position::ALL.into_iter().find(|p| p.opgg().eq_ignore_ascii_case(name))
    }

    pub fn opgg(self) -> &'static str {
        match self {
            Position::Top => "top",
            Position::Jungle => "jungle",
            Position::Mid => "mid",
            Position::Adc => "adc",
            Position::Support => "support",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub enum Rarity {
    #[serde(alias = "kSilver")]
    Silver,
    #[serde(alias = "kGold")]
    Gold,
    #[serde(alias = "kPrismatic")]
    Prismatic,
}

impl Rarity {
    pub const ALL: [Rarity; 3] = [Rarity::Prismatic, Rarity::Gold, Rarity::Silver];

    pub fn from_client(code: &str) -> Option<Rarity> {
        Rarity::deserialize(Value::String(code.into())).ok()
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct ChampionInfo {
    pub id: u32,
    pub name: String,
    pub icon: String,
    /// ARAM: Mayhem tier, 1 = best … 5.
    pub tier: Option<u8>,
    pub rank: Option<u32>,
    /// The account can neither play it nor take it from the bench.
    pub locked: bool,
    pub recommendable: bool,
    /// Mastery points of the account.
    #[ts(type = "number")]
    pub mastery: u64,
    /// Games of the account that Xyra stored.
    pub played: u32,
}

impl ChampionInfo {
    pub fn new(champion: Asset, tier_and_rank: Option<(u8, u32)>, locked: bool) -> ChampionInfo {
        let (tier, rank) = tier_and_rank.unzip();
        ChampionInfo { id: champion.id, name: champion.name, icon: champion.icon, tier, rank, locked, recommendable: false, mastery: 0, played: 0 }
            .with_lock(locked)
    }

    /// Sort key with the player's criterion: lower goes first, unranked champions last.
    pub fn preference(&self, order: ChampionOrder) -> (bool, u8, Reverse<u64>, Option<u32>) {
        let unranked = self.rank.is_none();
        match order {
            ChampionOrder::Tier => (unranked, 0, Reverse(0), self.rank),
            ChampionOrder::Mastery => (unranked, 0, Reverse(self.mastery), self.rank),
            ChampionOrder::Played => (unranked, 0, Reverse(self.played.into()), self.rank),
            ChampionOrder::Balanced => (unranked, self.tier.unwrap_or(u8::MAX), Reverse(self.mastery), self.rank),
        }
    }

    pub fn with_lock(self, locked: bool) -> ChampionInfo {
        ChampionInfo { locked, recommendable: self.rank.is_some() && !locked, ..self }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct Asset {
    pub id: u32,
    pub name: String,
    pub icon: String,
}

/// Who the phone app is talking to, and what this phone may do.
#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct PcInfo {
    pub name: String,
    pub version: String,
    pub language: String,
    pub permissions: PhonePermissions,
}

/// What a paired phone may do besides looking.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(default)]
pub struct PhonePermissions {
    pub accept: bool,
    pub import: bool,
    /// Take the recommended champion from the ARAM bench.
    pub bench: bool,
    pub settings: bool,
}

impl Default for PhonePermissions {
    fn default() -> PhonePermissions {
        PhonePermissions { accept: true, import: true, bench: true, settings: true }
    }
}

/// A phone paired with the phone link, with the token only it knows.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PhoneDevice {
    pub id: String,
    pub name: String,
    pub token: String,
    /// Unix seconds.
    pub paired_at: u64,
    #[serde(default)]
    pub permissions: PhonePermissions,
}

/// A paired phone as Settings show it.
#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct PhoneDeviceView {
    pub id: String,
    pub name: String,
    /// Unix seconds.
    #[ts(type = "number")]
    pub paired_at: u64,
    /// Seconds since its last request; None when it has not connected since Xyra started.
    #[ts(type = "number | null")]
    pub seen_ago: Option<u64>,
    pub permissions: PhonePermissions,
}

/// The settings the phone app can change.
#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct PhoneSettings {
    pub auto_accept: bool,
    pub accept_delay_seconds: u32,
    pub auto_import_build: bool,
    pub paused: bool,
    pub champion_order: ChampionOrder,
}

impl From<&crate::config::Config> for PhoneSettings {
    fn from(config: &crate::config::Config) -> PhoneSettings {
        PhoneSettings {
            auto_accept: config.auto_accept,
            accept_delay_seconds: config.accept_delay_seconds,
            auto_import_build: config.auto_import_build,
            paused: config.paused,
            champion_order: config.champion_order,
        }
    }
}

/// QR code the Xyra phone app scans to pair with this PC.
#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct PhoneLink {
    /// SVG image of the pairing code: this PC's addresses, the port and the pairing token.
    pub qr: String,
}

/// What to do next in the running game, from the build of the champion.
#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct GameTips {
    /// "Q", "W", "E" or "R" while a skill point is free.
    pub skill: Option<String>,
    pub next_item: Option<ItemTip>,
}

#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct ItemTip {
    pub item: Asset,
    pub price: Option<u32>,
    /// Gold still missing; 0 once the player can buy it.
    pub missing: Option<u32>,
}

/// Ranked tier list of the current patch, by position.
#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct Meta {
    /// Public patch name, like "26.19".
    pub patch: String,
    pub positions: Vec<PositionMeta>,
}

#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct PositionMeta {
    pub position: Position,
    /// Best first.
    pub champions: Vec<MetaChampion>,
}

#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct MetaChampion {
    pub champion: Asset,
    /// 1 = best … 5.
    pub tier: u8,
    pub rank: u32,
    /// Percentages, 0-100.
    pub win_rate: f64,
    pub pick_rate: f64,
    pub ban_rate: f64,
    /// Places gained since the previous patch; negative when it dropped.
    pub trend: Option<i32>,
}

/// Whether a patch made a champion stronger, weaker or changed it both ways.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub enum ChangeVerdict {
    Buff,
    Nerf,
    Adjusted,
}

/// What the official patch notes changed on one champion.
#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct ChampionChange {
    pub champion: Asset,
    pub verdict: ChangeVerdict,
    /// Why the designers changed it.
    pub context: String,
    pub groups: Vec<ChangeGroup>,
    /// One of the champions the player has the most mastery on.
    pub yours: bool,
}

/// The changed lines of one ability, or of the base stats.
#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct ChangeGroup {
    pub title: String,
    pub lines: Vec<String>,
}

/// Champion changes of the current patch, from its official notes.
#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct PatchChanges {
    pub patch: String,
    pub notes_url: String,
    pub champions: Vec<ChampionChange>,
}

#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct Matchup {
    pub champion: Asset,
    pub games: u32,
    /// Win rate of the champion being looked at against `champion`, 0-100.
    pub win_rate: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct ChampSelect {
    pub mode: GameMode,
    pub position: Option<Position>,
    pub champion: Option<ChampionInfo>,
    pub bench: Vec<ChampionInfo>,
    pub bench_pick: Option<ChampionInfo>,
    pub lane_opponent: Option<ChampionInfo>,
    /// Champions the account can pick that beat the lane opponent; `win_rate` is theirs.
    pub counter_picks: Vec<Matchup>,
}

#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct CurrentGame {
    pub mode: GameMode,
    pub champion: Option<ChampionInfo>,
}

#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
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
    /// PUUID of the signed-in account.
    pub account: Option<String>,
    pub game: Option<CurrentGame>,
    pub champ_select: Option<ChampSelect>,
    /// Build mode of the current champion select or game.
    pub build_mode: Option<BuildMode>,
    pub cards: Vec<Card>,
    /// Every augment choice seen in the current or last game, in order.
    pub rounds: Vec<Vec<Card>>,
    /// None when the game settings could not be read.
    pub borderless: Option<bool>,
    pub client_locale: String,
    /// UI language already resolved from the preference and the client locale.
    pub language: String,
    pub ocr_language: Option<String>,
    pub version: String,
    /// Next skill and item while a game with builds is running.
    pub tips: Option<GameTips>,
    /// A found match waits for the player's answer.
    pub ready_check: bool,
}

/// Events the engine pushes to the UI.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub enum AppEvent {
    State,
    Config,
    Data,
    UpdateProgress,
}

impl AppEvent {
    pub fn name(self) -> &'static str {
        match self {
            AppEvent::State => "state",
            AppEvent::Config => "config",
            AppEvent::Data => "data",
            AppEvent::UpdateProgress => "updateProgress",
        }
    }
}

/// Fixed option lists the UI offers, declared once here.
#[derive(Clone, Debug, Serialize, TS)]
#[ts(export)]
pub struct Choices {
    pub label_styles: Vec<LabelStyle>,
    pub champion_orders: Vec<ChampionOrder>,
    pub positions: Vec<Position>,
    pub augment_modes: Vec<GameMode>,
    pub rarities: Vec<Rarity>,
}

impl Choices {
    pub fn all() -> Choices {
        Choices {
            label_styles: LabelStyle::ALL.to_vec(),
            champion_orders: ChampionOrder::ALL.to_vec(),
            positions: Position::ALL.to_vec(),
            augment_modes: GameMode::WITH_AUGMENTS.to_vec(),
            rarities: Rarity::ALL.to_vec(),
        }
    }
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
    pub position: Option<Position>,
    /// Positions the champion is played in, most played first.
    pub positions: Vec<Position>,
    pub strong_against: Vec<Matchup>,
    pub weak_against: Vec<Matchup>,
}

#[derive(Clone, Copy, Debug, PartialEq, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub enum ImportTarget {
    Runes,
    Items,
    Spells,
}

impl ImportTarget {
    pub const ALL: [ImportTarget; 3] = [ImportTarget::Runes, ImportTarget::Items, ImportTarget::Spells];
}

#[derive(Clone, Debug, Serialize, TS)]
#[ts(export)]
pub struct AugmentRow {
    pub id: u32,
    pub name: String,
    pub icon: String,
    pub rarity: Option<Rarity>,
    pub tier: u8,
    pub quality: Quality,
    pub grade: String,
    pub performance: f64,
    pub pick_rate: f64,
}

/// i18n key and color token of an augment tier.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, TS)]
#[ts(export)]
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
}

pub fn grade(tier: Option<u8>) -> String {
    tier.map_or("?", |t| GRADES[(t as usize).min(GRADES.len() - 1)]).to_string()
}
