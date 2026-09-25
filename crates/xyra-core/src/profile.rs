use crate::{
    catalog::NamedAssets,
    lol::{asset_url, Lcu},
};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};
use ts_rs::TS;

const TOP_MASTERIES: usize = 5;
const NO_DIVISION: &str = "NA";
const RANKED_CRESTS: &str = "https://raw.communitydragon.org/latest/plugins/rcp-fe-lol-static-assets/global/default/images/ranked-mini-crests";

#[derive(Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Mastery {
    pub id: u32,
    pub name: String,
    pub icon: String,
    pub level: u32,
    #[ts(type = "number")]
    pub points: u64,
}

#[derive(Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Rank {
    /// "IRON" … "CHALLENGER"
    pub tier: String,
    /// Empty from Master up.
    pub division: String,
    pub lp: i64,
    pub crest: String,
}

#[derive(Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Profile {
    pub name: String,
    pub tag: String,
    pub level: u32,
    pub icon: String,
    pub region: String,
    pub rank: Option<Rank>,
    pub masteries: Vec<Mastery>,
}

pub fn read(lcu: &Lcu, http: &Client, champions: &NamedAssets) -> Result<Profile, String> {
    let summoner = lcu.get(http, "/lol-summoner/v1/current-summoner")?;
    let region = lcu.get(http, "/riotclient/region-locale").ok().and_then(|r| r["region"].as_str().map(String::from));
    let rank = lcu.get(http, "/lol-ranked/v1/current-ranked-stats").ok().and_then(|r| {
        let queue = &r["queueMap"]["RANKED_SOLO_5x5"];
        let tier = queue["tier"].as_str().filter(|t| !t.is_empty() && *t != "NONE")?;
        let division = queue["division"].as_str().filter(|d| *d != NO_DIVISION).unwrap_or_default();
        Some(Rank { tier: tier.into(), division: division.into(), lp: queue["leaguePoints"].as_i64().unwrap_or(0), crest: format!("{RANKED_CRESTS}/{}.svg", tier.to_lowercase()) })
    });
    let mut masteries: Vec<Mastery> = lcu
        .get(http, "/lol-champion-mastery/v1/local-player/champion-mastery")
        .ok()
        .and_then(|m| m.as_array().cloned())
        .unwrap_or_default()
        .iter()
        .filter_map(|m| {
            let id = m["championId"].as_u64()? as u32;
            let (name, icon) = champions.get(&id).cloned().unwrap_or_else(|| (format!("#{id}"), String::new()));
            Some(Mastery { id, name, icon, level: m["championLevel"].as_u64().unwrap_or(0) as u32, points: m["championPoints"].as_u64().unwrap_or(0) })
        })
        .collect();
    masteries.sort_by(|a, b| b.points.cmp(&a.points));
    masteries.truncate(TOP_MASTERIES);
    Ok(Profile {
        name: summoner["gameName"].as_str().or(summoner["displayName"].as_str()).unwrap_or_default().into(),
        tag: summoner["tagLine"].as_str().unwrap_or_default().into(),
        level: summoner["summonerLevel"].as_u64().unwrap_or(0) as u32,
        icon: asset_url(&format!("/lol-game-data/assets/v1/profile-icons/{}.jpg", summoner["profileIconId"].as_u64().unwrap_or(0))),
        region: region.unwrap_or_default(),
        rank,
        masteries,
    })
}

pub fn load(path: &Path) -> Option<Profile> {
    serde_json::from_str(&fs::read_to_string(path).ok()?).ok()
}

pub fn save(path: &Path, profile: &Profile) -> Result<(), String> {
    fs::write(path, serde_json::to_string(profile).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
}
