use crate::{
    catalog::{Catalog, NamedAssets},
    i18n,
    lol::Lcu,
    model::Asset,
};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::{cmp::Reverse, collections::HashMap, fs, path::Path};
use ts_rs::TS;

const MATCH_HISTORY: &str = "/lol-match-history/v1/products/lol/current-summoner/matches?begIndex=0&endIndex=19";
const TRACKED_MODES: [&str; 2] = ["KIWI", "CHERRY"];
const MIN_AUGMENT_GAMES: u32 = 2;
const TOP_AUGMENTS: usize = 15;
const RECENT_GAMES: usize = 10;
const AUGMENT_SLOTS: std::ops::RangeInclusive<u32> = 1..=6;

#[derive(Clone, Serialize, Deserialize)]
pub struct StoredGame {
    pub game_id: u64,
    pub date: String,
    pub champion: u32,
    pub mode: String,
    pub win: bool,
    pub augments: Vec<u32>,
}

pub fn load(path: &Path) -> Vec<StoredGame> {
    fs::read_to_string(path).ok().and_then(|text| serde_json::from_str(&text).ok()).unwrap_or_default()
}

pub fn save(path: &Path, games: &[StoredGame]) -> Result<(), String> {
    fs::write(path, serde_json::to_string(games).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
}

/// Adds the ARAM: Mayhem and Arena games from the recent match history that are not stored yet.
pub fn import_recent(lcu: &Lcu, http: &Client, games: &mut Vec<StoredGame>) -> Result<usize, String> {
    let history = lcu.get(http, MATCH_HISTORY)?;
    let mut added = 0;
    for game in history["games"]["games"].as_array().into_iter().flatten() {
        let mode = game["gameMode"].as_str().unwrap_or_default();
        let Some(id) = game["gameId"].as_u64() else { continue };
        if !TRACKED_MODES.contains(&mode) || games.iter().any(|g| g.game_id == id) {
            continue;
        }
        let player = &game["participants"][0];
        let stats = &player["stats"];
        let Some(champion) = player["championId"].as_u64() else { continue };
        games.push(StoredGame {
            game_id: id,
            date: game["gameCreationDate"].as_str().unwrap_or_default().chars().take(16).collect(),
            champion: champion as u32,
            mode: mode.into(),
            win: stats["win"].as_bool().unwrap_or(false),
            augments: AUGMENT_SLOTS
                .filter_map(|slot| stats[format!("playerAugment{slot}")].as_u64())
                .filter(|&a| a > 0)
                .map(|a| a as u32)
                .collect(),
        });
        added += 1;
    }
    games.sort_by_key(|g| Reverse(g.game_id));
    Ok(added)
}

#[derive(Serialize, TS)]
#[ts(export)]
pub struct StatRow {
    pub id: u32,
    pub name: String,
    pub icon: String,
    pub games: u32,
    pub wins: u32,
}

#[derive(Serialize, TS)]
#[ts(export)]
pub struct RecentGame {
    #[ts(type = "number")]
    pub game_id: u64,
    pub date: String,
    pub champion: String,
    pub icon: String,
    pub mode: String,
    pub win: bool,
    pub augments: Vec<Asset>,
}

#[derive(Serialize, TS)]
#[ts(export)]
pub struct StatsSummary {
    pub games: u32,
    pub wins: u32,
    pub champions: Vec<StatRow>,
    pub augments: Vec<StatRow>,
    pub recent: Vec<RecentGame>,
}

fn named(assets: &NamedAssets, id: u32) -> (String, String) {
    assets.get(&id).cloned().unwrap_or_else(|| (format!("#{id}"), String::new()))
}

fn rows(tally: HashMap<u32, (u32, u32)>, assets: &NamedAssets) -> Vec<StatRow> {
    tally
        .into_iter()
        .map(|(id, (games, wins))| {
            let (name, icon) = named(assets, id);
            StatRow { id, name, icon, games, wins }
        })
        .collect()
}

pub fn summarize(games: &[StoredGame], catalog: &Catalog) -> StatsSummary {
    let mut champions: HashMap<u32, (u32, u32)> = HashMap::new();
    let mut augments: HashMap<u32, (u32, u32)> = HashMap::new();
    for game in games {
        let entry = champions.entry(game.champion).or_default();
        entry.0 += 1;
        entry.1 += game.win as u32;
        for &augment in &game.augments {
            let entry = augments.entry(augment).or_default();
            entry.0 += 1;
            entry.1 += game.win as u32;
        }
    }
    let mut champions = rows(champions, &catalog.champions);
    champions.sort_by_key(|r| (Reverse(r.games), Reverse(r.wins)));
    let mut augments: Vec<StatRow> = rows(augments, &catalog.augments).into_iter().filter(|r| r.games >= MIN_AUGMENT_GAMES).collect();
    augments.sort_by(|a, b| (b.wins as f64 / b.games as f64).total_cmp(&(a.wins as f64 / a.games as f64)).then(b.games.cmp(&a.games)));
    augments.truncate(TOP_AUGMENTS);
    StatsSummary {
        games: games.len() as u32,
        wins: games.iter().filter(|g| g.win).count() as u32,
        champions,
        augments,
        recent: games
            .iter()
            .take(RECENT_GAMES)
            .map(|game| {
                let (champion, icon) = named(&catalog.champions, game.champion);
                RecentGame {
                    game_id: game.game_id,
                    date: game.date.replace('T', " "),
                    champion,
                    icon,
                    mode: game.mode.clone(),
                    win: game.win,
                    augments: game
                        .augments
                        .iter()
                        .map(|&id| {
                            let (name, icon) = named(&catalog.augments, id);
                            Asset { id, name, icon }
                        })
                        .collect(),
                }
            })
            .collect(),
    }
}

fn csv_field(text: String) -> String {
    format!("\"{}\"", text.replace('"', "\"\""))
}

pub fn to_csv(games: &[StoredGame], catalog: &Catalog, language: &str) -> String {
    let t = |key: &str| i18n::t(language, key);
    let header = ["stats:csv.date", "stats:csv.mode", "stats:csv.champion", "stats:csv.result", "stats:csv.augments"].map(t).map(csv_field);
    let mut csv = header.join(",") + "\r\n";
    for game in games {
        let augments: Vec<String> = game.augments.iter().map(|&id| named(&catalog.augments, id).0).collect();
        let row = [
            game.date.replace('T', " "),
            t(&format!("common:modes.{}", game.mode)),
            named(&catalog.champions, game.champion).0,
            t(if game.win { "stats:victory" } else { "stats:defeat" }),
            augments.join(" | "),
        ];
        csv += &row.map(csv_field).join(",");
        csv += "\r\n";
    }
    csv
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csv_escapes_quotes_and_names_assets() {
        let mut catalog = Catalog::default();
        catalog.champions.insert(1, ("Annie".into(), String::new()));
        catalog.augments.insert(7, ("Ojo \"de\" halcón, raro".into(), String::new()));
        let game = StoredGame { game_id: 1, date: "2026-09-25T10:00".into(), champion: 1, mode: "KIWI".into(), win: true, augments: vec![7, 9] };
        let csv = to_csv(&[game], &catalog, "es");
        assert_eq!(csv.lines().nth(1).unwrap(), r#""2026-09-25 10:00","ARAM: Caos","Annie","Victoria","Ojo ""de"" halcón, raro | #9""#);
    }
}
