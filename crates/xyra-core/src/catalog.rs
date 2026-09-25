use crate::{
    cards::normalize,
    errors::{AppError, Result},
    league::{Lcu, asset_url},
    model::{Asset, Rarity},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::HashMap, ops::Range};

const PLAYABLE_CHAMPION_IDS: Range<u64> = 1..10_000;
const AUGMENTS: &str = "/lol-game-data/assets/v1/cherry-augments.json";
const CHAMPIONS: &str = "/lol-game-data/assets/v1/champion-summary.json";
const ITEMS: &str = "/lol-game-data/assets/v1/items.json";
const RUNES: &str = "/lol-game-data/assets/v1/perks.json";
const RUNE_STYLES: &str = "/lol-game-data/assets/v1/perkstyles.json";
const SPELLS: &str = "/lol-game-data/assets/v1/summoner-spells.json";

pub type NamedAssets = HashMap<u32, (String, String)>;

/// Names and icons of the game data, read from the client in its language.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Catalog {
    pub augments: NamedAssets,
    pub augment_names: HashMap<String, Vec<u32>>,
    pub champions: NamedAssets,
    pub rarity: HashMap<u32, Rarity>,
    pub items: NamedAssets,
    pub runes: NamedAssets,
    pub spells: NamedAssets,
}

fn asset_list(value: &Value) -> NamedAssets {
    let list = value.as_array().or_else(|| value["styles"].as_array()).cloned().unwrap_or_default();
    list.iter()
        .filter_map(|entry| {
            let id = entry["id"].as_u64()? as u32;
            Some((id, (entry["name"].as_str()?.to_string(), asset_url(entry["iconPath"].as_str().unwrap_or_default()))))
        })
        .collect()
}

impl Catalog {
    pub fn read(lcu: &Lcu) -> Result<Catalog> {
        let mut catalog = Catalog::default();
        for augment in lcu.get(AUGMENTS)?.as_array().into_iter().flatten() {
            let (Some(id), Some(name)) = (augment["id"].as_u64().map(|id| id as u32), augment["nameTRA"].as_str().filter(|n| !n.is_empty())) else {
                continue;
            };
            catalog.augment_names.entry(normalize(name)).or_default().push(id);
            catalog.augments.insert(id, (name.to_string(), asset_url(augment["augmentSmallIconPath"].as_str().unwrap_or_default())));
            if let Some(rarity) = Rarity::from_client(augment["rarity"].as_str().unwrap_or_default()) {
                catalog.rarity.insert(id, rarity);
            }
        }
        for champion in lcu.get(CHAMPIONS)?.as_array().into_iter().flatten() {
            let (Some(id), Some(name)) = (champion["id"].as_u64().filter(|id| PLAYABLE_CHAMPION_IDS.contains(id)), champion["name"].as_str()) else {
                continue;
            };
            catalog.champions.insert(id as u32, (name.to_string(), asset_url(champion["squarePortraitPath"].as_str().unwrap_or_default())));
        }
        if catalog.augment_names.is_empty() || catalog.champions.is_empty() {
            return Err(AppError::EmptyCatalog);
        }
        catalog.items = asset_list(&lcu.get(ITEMS)?);
        catalog.runes = asset_list(&lcu.get(RUNES)?);
        catalog.runes.extend(asset_list(&lcu.get(RUNE_STYLES)?));
        catalog.spells = asset_list(&lcu.get(SPELLS)?);
        Ok(catalog)
    }

    pub fn is_empty(&self) -> bool {
        self.champions.is_empty()
    }

    pub fn champion(&self, id: u32) -> Asset {
        named(&self.champions, id)
    }
}

/// The asset, or a placeholder named by its id when the catalog does not know it.
pub fn named(assets: &NamedAssets, id: u32) -> Asset {
    let (name, icon) = assets.get(&id).cloned().unwrap_or_else(|| (format!("#{id}"), String::new()));
    Asset { id, name, icon }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn reads_asset_lists_and_placeholders() {
        let styles = json!({ "styles": [{ "id": 8100, "name": "Domination", "iconPath": "/lol-game-data/assets/v1/perk-images/Styles/7200_Domination.png" }] });
        let assets = asset_list(&styles);
        assert_eq!(assets[&8100].0, "Domination");
        assert!(assets[&8100].1.ends_with("/v1/perk-images/styles/7200_domination.png"));
        assert_eq!(named(&assets, 9), Asset { id: 9, name: "#9".into(), icon: String::new() });
        assert!(Catalog::default().is_empty());
    }
}
