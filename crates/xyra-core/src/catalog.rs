use crate::{cards::normalize, lol};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs, path::Path};

const PLAYABLE_CHAMPION_IDS: std::ops::Range<i64> = 1..10_000;

/// Excludes special-mode variants the client also lists (e.g. "Jade_Ahri", id 60103).
fn is_playable_champion(id: i64) -> bool {
    PLAYABLE_CHAMPION_IDS.contains(&id)
}

pub type NamedAssets = HashMap<u32, (String, String)>;

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Catalog {
    pub augments: NamedAssets,
    pub augment_names: HashMap<String, Vec<u32>>,
    pub aliases: HashMap<String, u32>,
    pub champions: NamedAssets,
    pub rarity: HashMap<u32, String>,
    pub items: NamedAssets,
    pub runes: NamedAssets,
    pub spells: NamedAssets,
}

fn read_asset_list(lcu: &lol::Lcu, http: &Client, endpoint: &str) -> NamedAssets {
    let value = lcu.get(http, endpoint).unwrap_or_default();
    let list = value.as_array().or_else(|| value["styles"].as_array()).cloned().unwrap_or_default();
    list.iter()
        .filter_map(|entry| {
            let id = entry["id"].as_u64()? as u32;
            Some((id, (entry["name"].as_str()?.to_string(), lol::asset_url(entry["iconPath"].as_str().unwrap_or_default()))))
        })
        .collect()
}

impl Catalog {
    pub fn read(lcu: &lol::Lcu, http: &Client) -> Result<Catalog, String> {
        let mut catalog = Catalog::default();
        for augment in lcu.get(http, "/lol-game-data/assets/v1/cherry-augments.json")?.as_array().ok_or("augments")? {
            let (Some(id), Some(name)) = (augment["id"].as_u64(), augment["nameTRA"].as_str()) else { continue };
            if name.is_empty() {
                continue;
            }
            let icon = lol::asset_url(augment["augmentSmallIconPath"].as_str().unwrap_or_default());
            catalog.augment_names.entry(normalize(name)).or_default().push(id as u32);
            catalog.augments.insert(id as u32, (name.to_string(), icon));
            catalog.rarity.insert(id as u32, augment["rarity"].as_str().unwrap_or_default().to_string());
        }
        for champion in lcu.get(http, "/lol-game-data/assets/v1/champion-summary.json")?.as_array().ok_or("champions")? {
            let (Some(id), Some(name), Some(alias)) = (champion["id"].as_i64(), champion["name"].as_str(), champion["alias"].as_str()) else {
                continue;
            };
            if !is_playable_champion(id) {
                continue;
            }
            let icon = lol::asset_url(champion["squarePortraitPath"].as_str().unwrap_or_default());
            catalog.aliases.insert(alias.to_string(), id as u32);
            catalog.champions.insert(id as u32, (name.to_string(), icon));
        }
        if catalog.augment_names.is_empty() {
            return Err("emptyCatalog".into());
        }
        catalog.items = read_asset_list(lcu, http, "/lol-game-data/assets/v1/items.json");
        catalog.runes = read_asset_list(lcu, http, "/lol-game-data/assets/v1/perks.json");
        catalog.runes.extend(read_asset_list(lcu, http, "/lol-game-data/assets/v1/perkstyles.json"));
        catalog.spells = read_asset_list(lcu, http, "/lol-game-data/assets/v1/summoner-spells.json");
        Ok(catalog)
    }

    pub fn load(path: &Path) -> Option<Catalog> {
        serde_json::from_str(&fs::read_to_string(path).ok()?).ok()
    }

    pub fn save(&self, path: &Path) -> Result<(), String> {
        fs::write(path, serde_json::to_string(self).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
    }
}
