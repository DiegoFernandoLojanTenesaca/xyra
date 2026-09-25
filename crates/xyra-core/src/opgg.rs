use crate::{
    catalog::{Catalog, NamedAssets},
    model::{Asset, Build, BuildMode, RunePage},
};
use reqwest::blocking::Client;
use serde_json::Value;
use std::collections::HashMap;

const API: &str = "https://lol-api-champion.op.gg/api";
const MIN_ARENA_GAMES: f64 = 20.0;
const SITUATIONAL_ITEMS: usize = 6;
const DEFAULT_RIFT_POSITION: &str = "mid";
/// Arena tier cutoffs by percentile of average placement: top 10 % = S, next 20 % = A, 30 % = B, 20 % = C, rest = D.
const ARENA_PERCENTILES: [f64; 4] = [0.1, 0.3, 0.6, 0.8];

#[derive(Clone, Copy)]
pub struct AugmentStat {
    /// 0 = S … 6 = F.
    pub tier: u8,
    pub performance: f64,
    pub pick_rate: f64,
}

fn fetch_json(http: &Client, url: &str) -> Result<Value, String> {
    http.get(url).send().and_then(|r| r.error_for_status()).and_then(|r| r.json()).map_err(|e| e.to_string())
}

pub fn fetch_mayhem_augments(http: &Client, champion: u32) -> Result<HashMap<u32, AugmentStat>, String> {
    let value = fetch_json(http, &format!("{API}/contents/stats/champions/{champion}/aram-augments"))?;
    Ok(value["data"]
        .as_array()
        .ok_or("noData")?
        .iter()
        .filter_map(|augment| {
            let stat = AugmentStat {
                tier: augment["tier"].as_u64()? as u8,
                performance: augment["performance"].as_f64().unwrap_or(0.0),
                pick_rate: augment["popular"].as_f64().unwrap_or(0.0),
            };
            Some((augment["id"].as_u64()? as u32, stat))
        })
        .collect())
}

pub fn fetch_arena_augments(http: &Client, champion: u32) -> Result<HashMap<u32, AugmentStat>, String> {
    let value = fetch_json(http, &format!("{API}/global/champions/arena/{champion}"))?;
    let mut placements: Vec<(u32, f64, f64)> = value["data"]["augment_group"]
        .as_array()
        .ok_or("noData")?
        .iter()
        .flat_map(|group| group["augments"].as_array().cloned().unwrap_or_default())
        .filter_map(|augment| {
            let games = augment["play"].as_f64()?;
            if games < MIN_ARENA_GAMES {
                return None;
            }
            let average_place = augment["total_place"].as_f64()? / games;
            Some((augment["id"].as_u64()? as u32, average_place, augment["pick_rate"].as_f64().unwrap_or(0.0) * 100.0))
        })
        .collect();
    placements.sort_by(|a, b| a.1.total_cmp(&b.1));
    Ok(tiers_by_percentile(&placements))
}

fn tiers_by_percentile(sorted: &[(u32, f64, f64)]) -> HashMap<u32, AugmentStat> {
    let total = sorted.len().max(1) as f64;
    sorted
        .iter()
        .enumerate()
        .map(|(index, &(id, average_place, pick_rate))| {
            let percentile = index as f64 / total;
            let tier = ARENA_PERCENTILES.iter().position(|&cut| percentile < cut).unwrap_or(ARENA_PERCENTILES.len()) as u8;
            (id, AugmentStat { tier, performance: 100.0 - average_place * 10.0, pick_rate })
        })
        .collect()
}

/// ARAM: Mayhem champion tier list: id -> (tier 1 = best … 5, rank).
pub fn fetch_champion_tiers(http: &Client) -> Result<HashMap<u32, (u8, u32)>, String> {
    let value = fetch_json(http, &format!("{API}/contents/tiers?type=aram_mayhem"))?;
    Ok(value["data"]
        .as_array()
        .ok_or("noData")?
        .iter()
        .filter_map(|c| Some((c["id"].as_u64()? as u32, (c["tier"].as_u64()? as u8, c["rank"].as_u64()? as u32))))
        .collect())
}

/// ARAM builds are the ones OP.GG also shows for ARAM: Mayhem; Rift builds default to the most played position.
pub fn fetch_build(http: &Client, champion: u32, mode: BuildMode, position: Option<&str>, catalog: &Catalog) -> Result<Build, String> {
    let base = format!("{API}/global/champions");
    if mode == BuildMode::Aram {
        return Ok(parse_build(&fetch_json(http, &format!("{base}/aram/{champion}/none"))?["data"], champion, catalog));
    }
    let requested = position.unwrap_or(DEFAULT_RIFT_POSITION);
    let mut build = parse_build(&fetch_json(http, &format!("{base}/ranked/{champion}/{requested}"))?["data"], champion, catalog);
    if let (None, Some(main)) = (position, build.positions.first().cloned()) {
        if main != requested {
            build = parse_build(&fetch_json(http, &format!("{base}/ranked/{champion}/{main}"))?["data"], champion, catalog);
        }
    }
    build.position = Some(build.positions.first().cloned().filter(|_| position.is_none()).unwrap_or_else(|| requested.to_string()));
    Ok(build)
}

fn asset(assets: &NamedAssets, id: u32) -> Asset {
    let (name, icon) = assets.get(&id).cloned().unwrap_or_else(|| (format!("#{id}"), String::new()));
    Asset { id, name, icon }
}

fn ids(value: &Value) -> Vec<u32> {
    value.as_array().into_iter().flatten().filter_map(|x| x.as_u64()).map(|x| x as u32).collect()
}

fn strings(value: &Value) -> Vec<String> {
    value.as_array().into_iter().flatten().filter_map(|x| x.as_str().map(String::from)).collect()
}

/// OP.GG lists are sorted by games played.
fn most_played<'a>(data: &'a Value, key: &str) -> &'a Value {
    &data[key][0]
}

fn win_rate(entry: &Value) -> f64 {
    let (wins, games) = (entry["win"].as_f64().unwrap_or(0.0), entry["play"].as_f64().unwrap_or(0.0));
    if games > 0.0 {
        100.0 * wins / games
    } else {
        0.0
    }
}

pub(crate) fn parse_build(data: &Value, champion: u32, catalog: &Catalog) -> Build {
    let item = |id| asset(&catalog.items, id);
    let rune = |id| asset(&catalog.runes, id);
    let runes = most_played(data, "runes");
    let boots = ids(&most_played(data, "boots")["ids"]);
    let core = ids(&most_played(data, "core_items")["ids"]);
    let mut situational: Vec<u32> = Vec::new();
    for entry in data["last_items"].as_array().into_iter().flatten() {
        for id in ids(&entry["ids"]) {
            if !core.contains(&id) && !boots.contains(&id) && !situational.contains(&id) {
                situational.push(id);
            }
        }
    }
    situational.truncate(SITUATIONAL_ITEMS);
    let stats = &data["summary"]["average_stats"];
    Build {
        champion,
        runes: RunePage {
            primary_style: rune(runes["primary_page_id"].as_u64().unwrap_or(0) as u32),
            secondary_style: rune(runes["secondary_page_id"].as_u64().unwrap_or(0) as u32),
            primary: ids(&runes["primary_rune_ids"]).into_iter().map(rune).collect(),
            secondary: ids(&runes["secondary_rune_ids"]).into_iter().map(rune).collect(),
            shards: ids(&runes["stat_mod_ids"]).into_iter().map(rune).collect(),
            win_rate: win_rate(runes),
            pick_rate: runes["pick_rate"].as_f64().unwrap_or(0.0) * 100.0,
        },
        spells: ids(&most_played(data, "summoner_spells")["ids"]).into_iter().map(|id| asset(&catalog.spells, id)).collect(),
        starting_items: ids(&most_played(data, "starter_items")["ids"]).into_iter().map(item).collect(),
        boots: boots.into_iter().map(item).collect(),
        core_items: core.into_iter().map(item).collect(),
        situational_items: situational.into_iter().map(item).collect(),
        skill_order: strings(&most_played(data, "skills")["order"]),
        skill_priority: strings(&most_played(data, "skill_masteries")["ids"]),
        win_rate: stats["win_rate"].as_f64().unwrap_or(0.0) * 100.0,
        games: stats["play"].as_u64().unwrap_or(0) as u32,
        position: None,
        positions: data["summary"]["positions"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|p| p["name"].as_str().map(str::to_lowercase))
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arena_tiers_follow_percentiles() {
        let sorted: Vec<(u32, f64, f64)> = (0..10).map(|i| (i, 2.0 + i as f64 * 0.3, 1.0)).collect();
        let tiers = tiers_by_percentile(&sorted);
        assert_eq!(tiers[&0].tier, 0);
        assert_eq!(tiers[&2].tier, 1);
        assert_eq!(tiers[&5].tier, 2);
        assert_eq!(tiers[&9].tier, 4);
        assert!(tiers[&0].performance > tiers[&9].performance);
    }
}
