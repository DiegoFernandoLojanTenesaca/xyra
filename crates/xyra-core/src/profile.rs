use crate::{
    catalog::{Catalog, named},
    errors::{AppError, Result},
    league::{Lcu, asset_url},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{cmp::Reverse, collections::HashSet};
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

/// PUUID of the account in a current-summoner payload.
pub fn account(summoner: &Value) -> Option<String> {
    summoner["puuid"].as_str().filter(|puuid| !puuid.is_empty()).map(String::from)
}

fn rank(stats: &Value) -> Option<Rank> {
    let queue = &stats["queueMap"][SOLO_QUEUE];
    let tier_code = queue["tier"].as_str()?;
    let division = queue["division"].as_str().filter(|d| *d != NO_DIVISION).unwrap_or_default();
    Some(Rank {
        tier: LeagueTier::from_client(tier_code)?,
        division: division.into(),
        lp: queue["leaguePoints"].as_i64().unwrap_or(0),
        crest: format!("{RANKED_CRESTS}/{}.svg", tier_code.to_lowercase()),
    })
}

pub fn read(lcu: &Lcu, catalog: &Catalog) -> Result<Profile> {
    let summoner = lcu.get(CURRENT_SUMMONER)?;
    let region = lcu.get(REGION)?["region"].as_str().unwrap_or_default().to_string();
    let mut masteries: Vec<Mastery> = lcu
        .get(MASTERIES)?
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|m| {
            let champion = named(&catalog.champions, m["championId"].as_u64()? as u32);
            Some(Mastery {
                id: champion.id,
                name: champion.name,
                icon: champion.icon,
                level: m["championLevel"].as_u64().unwrap_or(0) as u32,
                points: m["championPoints"].as_u64().unwrap_or(0),
            })
        })
        .collect();
    masteries.sort_by_key(|m| Reverse(m.points));
    masteries.truncate(TOP_MASTERIES);
    Ok(Profile {
        account: account(&summoner).ok_or(AppError::NoSummoner)?,
        name: summoner["gameName"].as_str().unwrap_or_default().into(),
        tag: summoner["tagLine"].as_str().unwrap_or_default().into(),
        level: summoner["summonerLevel"].as_u64().unwrap_or(0) as u32,
        icon: asset_url(&format!("/lol-game-data/assets/v1/profile-icons/{}.jpg", summoner["profileIconId"].as_u64().unwrap_or(0))),
        region,
        rank: rank(&lcu.get(RANKED_STATS)?),
        masteries,
    })
}

/// Champions the account owns or has free this week.
pub fn read_available_champions(lcu: &Lcu) -> Result<HashSet<u32>> {
    Ok(lcu
        .get(OWNED_CHAMPIONS)?
        .as_array()
        .into_iter()
        .flatten()
        .filter(|c| c["ownership"]["owned"] == true || c["freeToPlay"] == true)
        .filter_map(|c| c["id"].as_u64().map(|id| id as u32))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn reads_solo_queue_rank() {
        let stats = json!({ "queueMap": { "RANKED_SOLO_5x5": { "tier": "PLATINUM", "division": "II", "leaguePoints": 18 } } });
        let solo = rank(&stats).unwrap();
        assert_eq!((solo.tier, solo.division.as_str(), solo.lp), (LeagueTier::Platinum, "II", 18));
        assert!(solo.crest.ends_with("/platinum.svg"));
        assert!(rank(&json!({ "queueMap": { "RANKED_SOLO_5x5": { "tier": "NONE" } } })).is_none());
        assert_eq!(account(&json!({ "puuid": "abc" })), Some("abc".into()));
        assert_eq!(account(&json!({ "puuid": "" })), None);
    }
}
