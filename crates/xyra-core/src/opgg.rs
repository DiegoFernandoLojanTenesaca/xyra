use crate::{
    catalog::{Catalog, named},
    errors::{AppError, Result},
    model::{AugmentRow, Build, BuildMode, GameMode, Matchup, Position, Quality, RunePage, grade},
};
pub use reqwest::blocking::Client;
use serde_json::Value;
use std::{collections::HashMap, sync::Once, time::Duration};

const API: &str = "https://lol-api-champion.op.gg/api";
const HTTP_TIMEOUT: Duration = Duration::from_secs(10);
const MIN_ARENA_GAMES: f64 = 20.0;
const SITUATIONAL_ITEMS: usize = 6;
const DEFAULT_RIFT_POSITION: Position = Position::Mid;
/// Arena tier cutoffs by percentile of average placement: top 10 % = S, next 20 % = A, 30 % = B, 20 % = C, rest = D.
const ARENA_PERCENTILES: [f64; 4] = [0.1, 0.3, 0.6, 0.8];
/// A matchup counts once it holds this share of the champion's games in the position.
const MIN_MATCHUP_SHARE: f64 = 0.01;
pub const MATCHUPS_SHOWN: usize = 5;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AugmentStat {
    /// 0 = S … 6 = F.
    pub tier: u8,
    pub performance: f64,
    pub pick_rate: f64,
}

/// HTTP client for OP.GG, verified against the system's certificate authorities.
pub fn client() -> Client {
    static CRYPTO: Once = Once::new();
    CRYPTO.call_once(|| rustls::crypto::ring::default_provider().install_default().expect("first TLS crypto provider"));
    Client::builder().timeout(HTTP_TIMEOUT).build().expect("OP.GG HTTP client")
}

fn fetch_json(http: &Client, path: &str) -> Result<Value> {
    Ok(http.get(format!("{API}/{path}")).send()?.error_for_status()?.json()?)
}

pub fn fetch_augments(http: &Client, champion: u32, mode: GameMode) -> Result<HashMap<u32, AugmentStat>> {
    if mode == GameMode::Arena { fetch_arena_augments(http, champion) } else { fetch_mayhem_augments(http, champion) }
}

fn fetch_mayhem_augments(http: &Client, champion: u32) -> Result<HashMap<u32, AugmentStat>> {
    let value = fetch_json(http, &format!("contents/stats/champions/{champion}/aram-augments"))?;
    Ok(value["data"]
        .as_array()
        .ok_or(AppError::NoData)?
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

fn fetch_arena_augments(http: &Client, champion: u32) -> Result<HashMap<u32, AugmentStat>> {
    let value = fetch_json(http, &format!("global/champions/arena/{champion}"))?;
    let mut placements: Vec<(u32, f64, f64)> = value["data"]["augment_group"]
        .as_array()
        .ok_or(AppError::NoData)?
        .iter()
        .flat_map(|group| group["augments"].as_array().cloned().unwrap_or_default())
        .filter_map(|augment| {
            let games = augment["play"].as_f64().filter(|&games| games >= MIN_ARENA_GAMES)?;
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

/// Augment tier list of a champion, best first, for the augments the catalog knows.
pub fn augment_rows(stats: HashMap<u32, AugmentStat>, catalog: &Catalog) -> Vec<AugmentRow> {
    let mut rows: Vec<AugmentRow> = stats
        .into_iter()
        .filter_map(|(id, stat)| {
            let (name, icon) = catalog.augments.get(&id)?.clone();
            Some(AugmentRow {
                id,
                name,
                icon,
                rarity: catalog.rarity.get(&id).copied(),
                tier: stat.tier,
                quality: Quality::of(Some(stat.tier)),
                grade: grade(Some(stat.tier)),
                performance: stat.performance,
                pick_rate: stat.pick_rate,
            })
        })
        .collect();
    rows.sort_by(|a, b| a.tier.cmp(&b.tier).then(b.performance.total_cmp(&a.performance)));
    rows
}

/// ARAM: Mayhem champion tier list: id -> (tier 1 = best … 5, rank).
pub fn fetch_champion_tiers(http: &Client) -> Result<HashMap<u32, (u8, u32)>> {
    let value = fetch_json(http, "contents/tiers?type=aram_mayhem")?;
    Ok(value["data"]
        .as_array()
        .ok_or(AppError::NoData)?
        .iter()
        .filter_map(|c| Some((c["id"].as_u64()? as u32, (c["tier"].as_u64()? as u8, c["rank"].as_u64()? as u32))))
        .collect())
}

fn fetch_rift(http: &Client, champion: u32, position: Position) -> Result<Value> {
    Ok(fetch_json(http, &format!("global/champions/ranked/{champion}/{}", position.opgg()))?["data"].take())
}

fn main_position(data: &Value) -> Option<Position> {
    data["summary"]["positions"][0]["name"].as_str().and_then(Position::from_opgg)
}

/// ARAM builds are the ones OP.GG also shows for ARAM: Mayhem; Rift builds default to the most played position.
pub fn fetch_build(http: &Client, champion: u32, mode: BuildMode, position: Option<Position>, catalog: &Catalog) -> Result<Build> {
    if mode == BuildMode::Aram {
        let data = fetch_json(http, &format!("global/champions/aram/{champion}/none"))?;
        return Ok(parse_build(&data["data"], champion, catalog));
    }
    let requested = position.unwrap_or(DEFAULT_RIFT_POSITION);
    let mut data = fetch_rift(http, champion, requested)?;
    let played = position.or_else(|| main_position(&data)).unwrap_or(requested);
    if played != requested {
        data = fetch_rift(http, champion, played)?;
    }
    let mut build = parse_build(&data, champion, catalog);
    build.position = Some(played);
    Ok(build)
}

/// Opponents of the champion in the position with enough games: (id, games, the champion's win rate).
fn matchups(data: &Value) -> Vec<(u32, u32, f64)> {
    let games = data["summary"]["average_stats"]["play"].as_f64().unwrap_or(0.0);
    data["counters"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|counter| {
            let (played, won) = (counter["play"].as_f64()?, counter["win"].as_f64()?);
            let id = counter["champion_id"].as_u64()? as u32;
            (played > 0.0 && played >= games * MIN_MATCHUP_SHARE).then_some((id, played as u32, 100.0 * won / played))
        })
        .collect()
}

fn to_matchups(list: impl Iterator<Item = (u32, u32, f64)>, catalog: &Catalog) -> Vec<Matchup> {
    list.take(MATCHUPS_SHOWN).map(|(id, games, win_rate)| Matchup { champion: catalog.champion(id), games, win_rate }).collect()
}

/// Every champion that faced `enemy` in `position`, best counter first, with its win rate; None when `enemy` mainly plays elsewhere.
pub fn fetch_counter_picks(http: &Client, enemy: u32, position: Position, catalog: &Catalog) -> Result<Option<Vec<Matchup>>> {
    let data = fetch_rift(http, enemy, position)?;
    if main_position(&data) != Some(position) {
        return Ok(None);
    }
    let mut picks: Vec<(u32, u32, f64)> = matchups(&data).into_iter().map(|(id, games, win_rate)| (id, games, 100.0 - win_rate)).collect();
    picks.sort_by(|a, b| b.2.total_cmp(&a.2));
    Ok(Some(picks.into_iter().map(|(id, games, win_rate)| Matchup { champion: catalog.champion(id), games, win_rate }).collect()))
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
    if games > 0.0 { 100.0 * wins / games } else { 0.0 }
}

pub(crate) fn parse_build(data: &Value, champion: u32, catalog: &Catalog) -> Build {
    let item = |id| named(&catalog.items, id);
    let rune = |id| named(&catalog.runes, id);
    let runes = most_played(data, "runes");
    let boots = ids(&most_played(data, "boots")["ids"]);
    let core = ids(&most_played(data, "core_items")["ids"]);
    let mut situational: Vec<u32> = Vec::new();
    for id in data["last_items"].as_array().into_iter().flatten().flat_map(|entry| ids(&entry["ids"])) {
        if !core.contains(&id) && !boots.contains(&id) && !situational.contains(&id) {
            situational.push(id);
        }
    }
    situational.truncate(SITUATIONAL_ITEMS);
    let mut opponents = matchups(data);
    opponents.sort_by(|a, b| b.2.total_cmp(&a.2));
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
        spells: ids(&most_played(data, "summoner_spells")["ids"]).into_iter().map(|id| named(&catalog.spells, id)).collect(),
        starting_items: ids(&most_played(data, "starter_items")["ids"]).into_iter().map(item).collect(),
        boots: boots.into_iter().map(item).collect(),
        core_items: core.into_iter().map(item).collect(),
        situational_items: situational.into_iter().map(item).collect(),
        skill_order: strings(&most_played(data, "skills")["order"]),
        skill_priority: strings(&most_played(data, "skill_masteries")["ids"]),
        win_rate: stats["win_rate"].as_f64().unwrap_or(0.0) * 100.0,
        games: stats["play"].as_u64().unwrap_or(0) as u32,
        position: None,
        positions: data["summary"]["positions"].as_array().into_iter().flatten().filter_map(|p| p["name"].as_str().and_then(Position::from_opgg)).collect(),
        strong_against: to_matchups(opponents.iter().copied(), catalog),
        weak_against: to_matchups(opponents.iter().rev().copied(), catalog),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

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

    #[test]
    fn matchups_skip_rare_opponents_and_rank_both_ends() {
        let data = json!({
            "summary": { "average_stats": { "play": 1000 }, "positions": [{ "name": "MID" }] },
            "counters": [
                { "champion_id": 1, "play": 100, "win": 60 },
                { "champion_id": 2, "play": 100, "win": 40 },
                { "champion_id": 3, "play": 5, "win": 5 }
            ]
        });
        assert_eq!(main_position(&data), Some(Position::Mid));
        let build = parse_build(&data, 157, &Catalog::default());
        assert_eq!(build.strong_against.iter().map(|m| m.champion.id).collect::<Vec<_>>(), [1, 2]);
        assert_eq!(build.weak_against[0].champion.id, 2);
        assert_eq!(build.weak_against[0].win_rate, 40.0);
        assert_eq!(build.positions, [Position::Mid]);
    }
}
