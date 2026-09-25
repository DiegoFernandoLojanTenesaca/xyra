use crate::{
    catalog::{Catalog, named},
    errors::{AppError, Result},
    league::{Lcu, asset_url, parse},
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
const OWNED_CHAMPIONS: &str = "/lol-champions/v1/owned-champions-minimal";
const SOLO_QUEUE: &str = "RANKED_SOLO_5x5";
const NO_DIVISION: &str = "NA";
const TOP_MASTERIES: usize = 5;
const RANKED_CRESTS: &str = "https://raw.communitydragon.org/latest/plugins/rcp-fe-lol-static-assets/global/default/images/ranked-mini-crests";

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, TS)]
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
struct RegionLocale {
    region: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RankedStats {
    queue_map: HashMap<String, RankedQueue>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RankedQueue {
    tier: String,
    division: String,
    league_points: i64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ChampionMastery {
    champion_id: u32,
    champion_level: u32,
    champion_points: u64,
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
        region: region.region,
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

/// Champions the account owns or has free this week.
pub fn read_available_champions(lcu: &Lcu) -> Result<HashSet<u32>> {
    let champions: Vec<OwnedChampion> = lcu.get_as(OWNED_CHAMPIONS)?;
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
        let champions: Vec<OwnedChampion> = parse(OWNED_CHAMPIONS, &list).unwrap();
        let available: HashSet<u32> = champions.into_iter().filter(|c| c.ownership.owned || c.free_to_play).map(|c| c.id).collect();
        assert_eq!(available, HashSet::from([1, 2]));
    }
}
