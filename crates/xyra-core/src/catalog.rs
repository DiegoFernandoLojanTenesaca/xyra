use crate::{
    cards::normalize,
    errors::{AppError, Result},
    league::{Lcu, asset_url},
    model::{Asset, Rarity},
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, ops::Range};

const PLAYABLE_CHAMPION_IDS: Range<i64> = 1..10_000;
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

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct NamedEntry {
    id: u32,
    name: String,
    icon_path: String,
}

#[derive(Deserialize)]
struct RuneStyles {
    styles: Vec<NamedEntry>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AugmentEntry {
    id: u32,
    #[serde(rename = "nameTRA")]
    name: String,
    augment_small_icon_path: String,
    rarity: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ChampionEntry {
    id: i64,
    name: String,
    square_portrait_path: String,
}

fn assets(entries: Vec<NamedEntry>) -> NamedAssets {
    entries.into_iter().map(|entry| (entry.id, (entry.name, asset_url(&entry.icon_path)))).collect()
}

impl Catalog {
    pub fn read(lcu: &Lcu) -> Result<Catalog> {
        let mut catalog = Catalog::default();
        let augments: Vec<AugmentEntry> = lcu.get_as(AUGMENTS)?;
        for augment in augments.into_iter().filter(|a| !a.name.is_empty()) {
            catalog.augment_names.entry(normalize(&augment.name)).or_default().push(augment.id);
            if let Some(rarity) = Rarity::from_client(&augment.rarity) {
                catalog.rarity.insert(augment.id, rarity);
            }
            catalog.augments.insert(augment.id, (augment.name, asset_url(&augment.augment_small_icon_path)));
        }
        let champions: Vec<ChampionEntry> = lcu.get_as(CHAMPIONS)?;
        for champion in champions.into_iter().filter(|c| PLAYABLE_CHAMPION_IDS.contains(&c.id)) {
            catalog.champions.insert(champion.id as u32, (champion.name, asset_url(&champion.square_portrait_path)));
        }
        if catalog.augment_names.is_empty() || catalog.champions.is_empty() {
            return Err(AppError::EmptyCatalog);
        }
        catalog.items = assets(lcu.get_as(ITEMS)?);
        catalog.runes = assets(lcu.get_as(RUNES)?);
        catalog.runes.extend(assets(lcu.get_as::<RuneStyles>(RUNE_STYLES)?.styles));
        catalog.spells = assets(lcu.get_as(SPELLS)?);
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
    use crate::league::parse;
    use serde_json::json;

    #[test]
    fn reads_asset_lists_and_placeholders() {
        let styles = json!({ "styles": [{ "id": 8100, "name": "Domination", "iconPath": "/lol-game-data/assets/v1/perk-images/Styles/7200_Domination.png" }] });
        let assets = assets(parse::<RuneStyles>(RUNE_STYLES, &styles).unwrap().styles);
        assert_eq!(assets[&8100].0, "Domination");
        assert!(assets[&8100].1.ends_with("/v1/perk-images/styles/7200_domination.png"));
        assert_eq!(named(&assets, 9), Asset { id: 9, name: "#9".into(), icon: String::new() });
        assert!(Catalog::default().is_empty());
        assert!(matches!(parse::<Vec<NamedEntry>>(ITEMS, &json!([{ "id": 1001, "name": "Boots" }])), Err(AppError::ClientFormat(_))));
    }
}
