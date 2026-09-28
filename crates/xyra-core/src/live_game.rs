use crate::{
    catalog::Catalog,
    errors::{AppError, Result},
    league::local_http,
    model::{Asset, Build, GameTips, ItemTip, LiveStats},
    web::Client,
};
use reqwest::Url;
use serde::Deserialize;
use std::collections::HashMap;

const ACTIVE_PLAYER: &str = "https://127.0.0.1:2999/liveclientdata/activeplayer";
const PLAYER_ITEMS: &str = "https://127.0.0.1:2999/liveclientdata/playeritems";
const PLAYER_SCORES: &str = "https://127.0.0.1:2999/liveclientdata/playerscores";
const SKILL_KEYS: [&str; 4] = ["Q", "W", "E", "R"];

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ActivePlayer {
    abilities: HashMap<String, Ability>,
    current_gold: f64,
    level: u32,
    riot_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Ability {
    #[serde(default)]
    ability_level: u32,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Scores {
    kills: u32,
    deaths: u32,
    assists: u32,
    creep_score: u32,
}

#[derive(Deserialize)]
struct OwnedItem {
    #[serde(rename = "itemID")]
    item_id: u32,
}

/// What the game reports about the player, through Riot's Live Client Data API on this PC.
#[derive(Clone, Debug, PartialEq)]
pub struct PlayerSnapshot {
    pub skill_points_spent: u32,
    pub items: Vec<u32>,
    pub live: LiveStats,
}

/// The client for the game's local API, which uses the same Riot certificate as the League client.
pub fn client() -> Client {
    local_http()
}

/// Errors while the game loads, before the API answers.
pub fn read(http: &Client) -> Result<PlayerSnapshot> {
    let player: ActivePlayer = http.get(ACTIVE_PLAYER).send()?.error_for_status()?.json().map_err(|e| AppError::client_format(ACTIVE_PLAYER, e))?;
    let of_player = |endpoint| Url::parse_with_params(endpoint, [("riotId", &player.riot_id)]).map_err(AppError::client);
    let items: Vec<OwnedItem> = http.get(of_player(PLAYER_ITEMS)?).send()?.error_for_status()?.json().map_err(|e| AppError::client_format(PLAYER_ITEMS, e))?;
    let scores: Scores = http.get(of_player(PLAYER_SCORES)?).send()?.error_for_status()?.json().map_err(|e| AppError::client_format(PLAYER_SCORES, e))?;
    Ok(PlayerSnapshot {
        skill_points_spent: SKILL_KEYS.iter().filter_map(|key| player.abilities.get(*key)).map(|a| a.ability_level).sum(),
        items: items.into_iter().map(|item| item.item_id).collect(),
        live: LiveStats {
            kills: scores.kills,
            deaths: scores.deaths,
            assists: scores.assists,
            farm: scores.creep_score,
            gold: player.current_gold.max(0.0) as u32,
            level: player.level,
        },
    })
}

/// The skill to level when a point is free, following the build's order, and the first item of the build not bought yet.
pub fn tips(player: &PlayerSnapshot, build: &Build, catalog: &Catalog) -> GameTips {
    let skill = (player.live.level > player.skill_points_spent)
        .then(|| build.skill_order.get(player.skill_points_spent as usize))
        .flatten()
        .filter(|key| SKILL_KEYS.contains(&key.as_str()))
        .cloned();
    let order = build.core_items.first().into_iter().chain(build.boots.first()).chain(build.core_items.iter().skip(1)).chain(&build.situational_items);
    let next_item = order.filter(|item| !player.items.contains(&item.id)).map(|item| item_tip(item, player.live.gold, catalog)).next();
    GameTips { skill, next_item }
}

fn item_tip(item: &Asset, gold: u32, catalog: &Catalog) -> ItemTip {
    let price = catalog.item_prices.get(&item.id).copied();
    ItemTip { item: item.clone(), price, missing: price.map(|price| price.saturating_sub(gold)) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::RunePage;

    fn asset(id: u32) -> Asset {
        Asset { id, name: format!("#{id}"), icon: String::new() }
    }

    fn build() -> Build {
        Build {
            champion: 1,
            runes: RunePage {
                primary_style: asset(0),
                secondary_style: asset(0),
                primary: Vec::new(),
                secondary: Vec::new(),
                shards: Vec::new(),
                win_rate: 0.0,
                pick_rate: 0.0,
            },
            spells: Vec::new(),
            starting_items: Vec::new(),
            boots: vec![asset(3020)],
            core_items: vec![asset(6655), asset(3089), asset(3157)],
            situational_items: vec![asset(3135)],
            skill_order: ["Q", "E", "W", "Q"].map(String::from).to_vec(),
            skill_priority: Vec::new(),
            win_rate: 0.0,
            games: 0,
            position: None,
            positions: Vec::new(),
            strong_against: Vec::new(),
            weak_against: Vec::new(),
        }
    }

    fn snapshot(level: u32, skill_points_spent: u32, gold: u32, items: Vec<u32>) -> PlayerSnapshot {
        PlayerSnapshot { skill_points_spent, items, live: LiveStats { gold, level, ..LiveStats::default() } }
    }

    #[test]
    fn suggests_the_next_skill_and_the_first_item_missing() {
        let catalog = Catalog { item_prices: HashMap::from([(6655, 2800), (3020, 1100)]), ..Catalog::default() };
        let player = snapshot(3, 2, 1000, Vec::new());
        let advice = tips(&player, &build(), &catalog);
        assert_eq!(advice.skill.as_deref(), Some("W"));
        let next = advice.next_item.unwrap();
        assert_eq!((next.item.id, next.price, next.missing), (6655, Some(2800), Some(1800)));

        let player = snapshot(3, 3, 1500, vec![6655]);
        let advice = tips(&player, &build(), &catalog);
        assert_eq!(advice.skill, None);
        assert_eq!(advice.next_item.map(|n| (n.item.id, n.missing)), Some((3020, Some(0))));

        let player = snapshot(18, 17, 0, vec![6655, 3020, 3089, 3157, 3135]);
        let advice = tips(&player, &build(), &catalog);
        assert_eq!((advice.skill, advice.next_item), (None, None));
    }
}
