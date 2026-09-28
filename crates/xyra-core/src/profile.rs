use crate::{
    catalog::{Catalog, named},
    errors::{AppError, Result},
    league::{Lcu, asset_url, parse},
    model::Asset,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    cmp::Reverse,
    collections::{HashMap, HashSet},
};
use ts_rs::TS;

pub const CURRENT_SUMMONER: &str = "/lol-summoner/v1/current-summoner";
const REGION: &str = "/riotclient/region-locale";
const RANKED_STATS: &str = "/lol-ranked/v1/current-ranked-stats";
const MASTERIES: &str = "/lol-champion-mastery/v1/local-player/champion-mastery";
pub const OWNED_CHAMPIONS: &str = "/lol-champions/v1/owned-champions-minimal";
const SOLO_QUEUE: &str = "RANKED_SOLO_5x5";
const FLEX_QUEUE: &str = "RANKED_FLEX_SR";
const SOLO_QUEUE_ID: u32 = 420;
const FLEX_QUEUE_ID: u32 = 440;
/// Each division of the tiers below Master holds 100 LP, and each of those tiers four divisions.
const DIVISION_LP: i64 = 100;
const DIVISIONS: [&str; 4] = ["IV", "III", "II", "I"];
const NO_DIVISION: &str = "NA";
const TOP_MASTERIES: usize = 5;
const RANKED_CRESTS: &str = "https://raw.communitydragon.org/latest/plugins/rcp-fe-lol-static-assets/global/default/images/ranked-mini-crests";

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub enum LeagueTier {
    Iron,
    Bronze,
    Silver,
    Gold,
    Platinum,
    Emerald,
    Diamond,
    Master,
    Grandmaster,
    Challenger,
}

impl LeagueTier {
    fn from_client(tier: &str) -> Option<LeagueTier> {
        LeagueTier::deserialize(Value::String(tier.to_lowercase())).ok()
    }
}

/// A ranked queue whose LP Xyra follows.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub enum RankedQueue {
    Solo,
    Flex,
}

impl RankedQueue {
    /// The ranked queue a matchmaking queue belongs to, if any.
    pub fn of_queue(id: u32) -> Option<RankedQueue> {
        match id {
            SOLO_QUEUE_ID => Some(RankedQueue::Solo),
            FLEX_QUEUE_ID => Some(RankedQueue::Flex),
            _ => None,
        }
    }

    fn client_name(self) -> &'static str {
        match self {
            RankedQueue::Solo => SOLO_QUEUE,
            RankedQueue::Flex => FLEX_QUEUE,
        }
    }
}

/// Where the player stands in a ranked queue.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Standing {
    pub tier: LeagueTier,
    /// Roman numeral; empty from Master up.
    pub division: String,
    #[ts(type = "number")]
    pub lp: i64,
    pub wins: u32,
    pub losses: u32,
}

impl Standing {
    pub fn games(&self) -> u32 {
        self.wins + self.losses
    }

    /// LP counted from the bottom of Iron, so a new division or tier still gives the right difference; Master and
    /// the tiers above share one LP count.
    fn ladder_lp(&self) -> i64 {
        let tier = (self.tier as i64).min(LeagueTier::Master as i64);
        let division = DIVISIONS.iter().position(|division| *division == self.division).unwrap_or(0) as i64;
        let divisions = if self.tier < LeagueTier::Master { division } else { 0 };
        tier * DIVISIONS.len() as i64 * DIVISION_LP + divisions * DIVISION_LP + self.lp
    }

    /// The LP gained, or lost when negative, since `before`.
    pub fn lp_since(&self, before: &Standing) -> i64 {
        self.ladder_lp() - before.ladder_lp()
    }
}

/// The player's standing in a ranked queue; None while unranked in it.
pub fn read_standing(lcu: &Lcu, queue: RankedQueue) -> Result<Option<Standing>> {
    Ok(standing(&lcu.get_as(RANKED_STATS)?, queue))
}

fn standing(stats: &RankedStats, queue: RankedQueue) -> Option<Standing> {
    let entry = stats.queue_map.get(queue.client_name())?;
    Some(Standing {
        tier: LeagueTier::from_client(&entry.tier)?,
        division: if entry.division == NO_DIVISION { String::new() } else { entry.division.clone() },
        lp: entry.league_points,
        wins: entry.wins,
        losses: entry.losses,
    })
}

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Mastery {
    pub id: u32,
    pub name: String,
    pub icon: String,
    pub level: u32,
    #[ts(type = "number")]
    pub points: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Rank {
    pub tier: LeagueTier,
    /// Roman numeral; empty from Master up.
    pub division: String,
    #[ts(type = "number")]
    pub lp: i64,
    pub crest: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Profile {
    pub account: String,
    pub name: String,
    pub tag: String,
    pub level: u32,
    pub icon: String,
    pub region: String,
    pub rank: Option<Rank>,
    pub masteries: Vec<Mastery>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Summoner {
    puuid: String,
    game_name: String,
    tag_line: String,
    summoner_level: u32,
    profile_icon_id: u32,
}

#[derive(Deserialize)]
struct SignedIn {
    puuid: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RegionLocale {
    region: String,
    /// The name players know, like "lan" for LA1.
    #[serde(default)]
    web_region: String,
}

impl RegionLocale {
    fn name(self) -> String {
        if self.web_region.is_empty() { self.region } else { self.web_region.to_uppercase() }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RankedStats {
    queue_map: HashMap<String, QueueEntry>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct QueueEntry {
    tier: String,
    division: String,
    league_points: i64,
    #[serde(default)]
    wins: u32,
    #[serde(default)]
    losses: u32,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ChampionMastery {
    champion_id: u32,
    champion_level: u32,
    champion_points: u64,
    #[serde(default)]
    champion_points_since_last_level: i64,
    #[serde(default)]
    champion_points_until_next_level: i64,
    #[serde(default)]
    mark_required_for_next_level: u32,
    #[serde(default)]
    tokens_earned: u32,
    #[serde(default)]
    highest_grade: String,
    #[serde(default)]
    last_play_time: u64,
}

/// A champion's mastery and how far its next level is.
#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct MasteryProgress {
    pub champion: Asset,
    pub level: u32,
    #[ts(type = "number")]
    pub points: u64,
    /// Points earned in the current level.
    #[ts(type = "number")]
    pub since_level: u64,
    /// Points still needed for the next level.
    #[ts(type = "number")]
    pub until_next: u64,
    /// Marks the next level asks for and those already earned.
    pub marks_needed: u32,
    pub marks: u32,
    /// Best grade ever earned with it, like "S+"; empty when it has none.
    pub best_grade: String,
    /// Unix milliseconds.
    #[ts(type = "number")]
    pub last_played: u64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct OwnedChampion {
    id: u32,
    ownership: Ownership,
    free_to_play: bool,
}

#[derive(Deserialize)]
struct Ownership {
    owned: bool,
}

/// PUUID of the account in a current-summoner payload.
pub fn account(summoner: &Value) -> Result<Option<String>> {
    let signed_in: SignedIn = parse(CURRENT_SUMMONER, summoner)?;
    Ok(Some(signed_in.puuid).filter(|puuid| !puuid.is_empty()))
}

fn rank(stats: &RankedStats) -> Option<Rank> {
    let queue = stats.queue_map.get(SOLO_QUEUE)?;
    Some(Rank {
        tier: LeagueTier::from_client(&queue.tier)?,
        division: if queue.division == NO_DIVISION { String::new() } else { queue.division.clone() },
        lp: queue.league_points,
        crest: format!("{RANKED_CRESTS}/{}.svg", queue.tier.to_lowercase()),
    })
}

pub fn read(lcu: &Lcu, catalog: &Catalog) -> Result<Profile> {
    let summoner: Summoner = lcu.get_as(CURRENT_SUMMONER)?;
    if summoner.puuid.is_empty() {
        return Err(AppError::NoSummoner);
    }
    let region: RegionLocale = lcu.get_as(REGION)?;
    let mut masteries: Vec<ChampionMastery> = lcu.get_as(MASTERIES)?;
    masteries.sort_by_key(|m| Reverse(m.champion_points));
    masteries.truncate(TOP_MASTERIES);
    Ok(Profile {
        account: summoner.puuid,
        name: summoner.game_name,
        tag: summoner.tag_line,
        level: summoner.summoner_level,
        icon: asset_url(&format!("/lol-game-data/assets/v1/profile-icons/{}.jpg", summoner.profile_icon_id)),
        region: region.name(),
        rank: rank(&lcu.get_as(RANKED_STATS)?),
        masteries: masteries
            .into_iter()
            .map(|m| {
                let champion = named(&catalog.champions, m.champion_id);
                Mastery { id: champion.id, name: champion.name, icon: champion.icon, level: m.champion_level, points: m.champion_points }
            })
            .collect(),
    })
}

/// Mastery points of the account per champion.
pub fn read_mastery_points(lcu: &Lcu) -> Result<HashMap<u32, u64>> {
    let masteries: Vec<ChampionMastery> = lcu.get_as(MASTERIES)?;
    Ok(masteries.into_iter().map(|m| (m.champion_id, m.champion_points)).collect())
}

/// Every playable champion the account has mastery on, most points first; special modes' champions are left out.
pub fn read_masteries(lcu: &Lcu, catalog: &Catalog) -> Result<Vec<MasteryProgress>> {
    let mut masteries: Vec<ChampionMastery> = lcu.get_as(MASTERIES)?;
    masteries.retain(|m| catalog.champions.contains_key(&m.champion_id));
    masteries.sort_by_key(|m| Reverse(m.champion_points));
    Ok(masteries.into_iter().map(|m| mastery_progress(m, catalog)).collect())
}

fn mastery_progress(mastery: ChampionMastery, catalog: &Catalog) -> MasteryProgress {
    MasteryProgress {
        champion: catalog.champion(mastery.champion_id),
        level: mastery.champion_level,
        points: mastery.champion_points,
        since_level: mastery.champion_points_since_last_level.max(0) as u64,
        until_next: mastery.champion_points_until_next_level.max(0) as u64,
        marks_needed: mastery.mark_required_for_next_level,
        marks: mastery.tokens_earned,
        best_grade: mastery.highest_grade,
        last_played: mastery.last_play_time,
    }
}

/// Champions the account owns or has free this week.
pub fn read_available_champions(lcu: &Lcu) -> Result<HashSet<u32>> {
    available_champions(&lcu.get(OWNED_CHAMPIONS)?)
}

/// The champions of an owned champions list, as the client answers or announces it.
pub fn available_champions(list: &Value) -> Result<HashSet<u32>> {
    let champions: Vec<OwnedChampion> = parse(OWNED_CHAMPIONS, list)?;
    Ok(champions.into_iter().filter(|c| c.ownership.owned || c.free_to_play).map(|c| c.id).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn ranked(queue: Value) -> RankedStats {
        parse(RANKED_STATS, &json!({ "queueMap": { SOLO_QUEUE: queue } })).unwrap()
    }

    #[test]
    fn reads_solo_queue_rank() {
        let solo = rank(&ranked(json!({ "tier": "PLATINUM", "division": "II", "leaguePoints": 18 }))).unwrap();
        assert_eq!((solo.tier, solo.division.as_str(), solo.lp), (LeagueTier::Platinum, "II", 18));
        assert!(solo.crest.ends_with("/platinum.svg"));
        let master = rank(&ranked(json!({ "tier": "MASTER", "division": "NA", "leaguePoints": 120 }))).unwrap();
        assert_eq!(master.division, "");
        assert!(rank(&ranked(json!({ "tier": "NONE", "division": "NA", "leaguePoints": 0 }))).is_none());
        assert!(matches!(parse::<RankedStats>(RANKED_STATS, &json!({ "queueMap": { SOLO_QUEUE: { "tier": "GOLD" } } })), Err(AppError::ClientFormat(_))));
    }

    #[test]
    fn counts_lp_across_divisions_and_tiers() {
        let at = |tier, division: &str, lp| Standing { tier, division: division.into(), lp, wins: 0, losses: 0 };
        assert_eq!(at(LeagueTier::Gold, "II", 40).lp_since(&at(LeagueTier::Gold, "II", 21)), 19);
        assert_eq!(at(LeagueTier::Gold, "I", 5).lp_since(&at(LeagueTier::Gold, "II", 85)), 20);
        assert_eq!(at(LeagueTier::Silver, "I", 75).lp_since(&at(LeagueTier::Gold, "IV", 0)), -25);
        assert_eq!(at(LeagueTier::Master, "", 12).lp_since(&at(LeagueTier::Diamond, "I", 90)), 22);
        assert_eq!(at(LeagueTier::Grandmaster, "", 450).lp_since(&at(LeagueTier::Master, "", 430)), 20);
        let stats: RankedStats =
            parse(RANKED_STATS, &json!({ "queueMap": { FLEX_QUEUE: { "tier": "EMERALD", "division": "III", "leaguePoints": 55, "wins": 9, "losses": 7 } } }))
                .unwrap();
        let flex = standing(&stats, RankedQueue::Flex).unwrap();
        assert_eq!((flex.tier, flex.lp, flex.games()), (LeagueTier::Emerald, 55, 16));
        assert!(standing(&stats, RankedQueue::Solo).is_none());
        assert_eq!((RankedQueue::of_queue(420), RankedQueue::of_queue(450)), (Some(RankedQueue::Solo), None));
    }

    #[test]
    fn reads_the_signed_in_account() {
        assert_eq!(account(&json!({ "puuid": "abc", "gameName": "Xyra" })), Ok(Some("abc".into())));
        assert_eq!(account(&json!({ "puuid": "" })), Ok(None));
        assert!(matches!(account(&json!({ "accountId": 1 })), Err(AppError::ClientFormat(_))));
    }

    #[test]
    fn keeps_owned_and_free_champions() {
        let list = json!([
            { "id": 1, "ownership": { "owned": true }, "freeToPlay": false },
            { "id": 2, "ownership": { "owned": false }, "freeToPlay": true },
            { "id": 3, "ownership": { "owned": false }, "freeToPlay": false }
        ]);
        assert_eq!(available_champions(&list), Ok(HashSet::from([1, 2])));
    }
}
