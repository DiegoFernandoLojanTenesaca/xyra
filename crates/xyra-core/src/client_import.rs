use crate::{
    errors::{AppError, Result},
    i18n,
    league::Lcu,
    model::{Asset, Build},
    profile::CURRENT_SUMMONER,
};
use reqwest::Method;
use serde_json::{Value, json};

/// Prefix of the rune pages and item sets Xyra creates and replaces.
const PREFIX: &str = "Xyra · ";
const RUNE_PAGES: &str = "/lol-perks/v1/pages";
const MAX_PAGES_ERROR: &str = "max";
/// Start of the per-champion item set UUID ("Xyra" in hex).
const ITEM_SET_UID_PREFIX: &str = "58797261-0000-4000-8000-";

fn ids(assets: &[Asset]) -> impl Iterator<Item = u32> + '_ {
    assets.iter().map(|a| a.id)
}

pub fn rune_page(build: &Build, champion: &str) -> Value {
    let runes = &build.runes;
    json!({
        "name": format!("{PREFIX}{champion}"),
        "primaryStyleId": runes.primary_style.id,
        "subStyleId": runes.secondary_style.id,
        "selectedPerkIds": ids(&runes.primary).chain(ids(&runes.secondary)).chain(ids(&runes.shards)).collect::<Vec<_>>(),
        "current": true,
    })
}

pub fn item_set(build: &Build, champion: &str, language: &str) -> Value {
    let block = |key: &str, items: &[Asset]| {
        json!({
            "type": i18n::t(language, key),
            "items": items.iter().map(|i| json!({ "id": i.id.to_string(), "count": 1 })).collect::<Vec<_>>(),
        })
    };
    json!({
        "title": format!("{PREFIX}{champion}"),
        "associatedChampions": [build.champion],
        "associatedMaps": [],
        "blocks": [
            block("build:blocks.starting", &build.starting_items),
            block("build:blocks.boots", &build.boots),
            block("build:blocks.core", &build.core_items),
            block("build:blocks.situational", &build.situational_items),
        ],
        "map": "any",
        "mode": "any",
        "preferredItemSlots": [],
        "sortrank": 0,
        "startedFrom": "blank",
        "type": "custom",
        "uid": format!("{ITEM_SET_UID_PREFIX}{:012}", build.champion),
    })
}

fn upsert_item_set(sets: &mut Value, set: Value, champion: u32) {
    if !sets["itemSets"].is_array() {
        sets["itemSets"] = json!([]);
    }
    let list = sets["itemSets"].as_array_mut().expect("itemSets array");
    list.retain(|s| {
        let ours = s["title"].as_str().is_some_and(|t| t.starts_with(PREFIX));
        let same_champion = s["associatedChampions"].as_array().is_some_and(|a| a.iter().any(|c| c.as_u64() == Some(champion as u64)));
        !(ours && same_champion)
    });
    list.push(set);
}

pub fn import_runes(lcu: &Lcu, build: &Build, champion: &str) -> Result<()> {
    for page in lcu.get(RUNE_PAGES)?.as_array().into_iter().flatten() {
        let ours = page["name"].as_str().is_some_and(|n| n.starts_with(PREFIX));
        if ours && page["isDeletable"] == true {
            lcu.request(Method::DELETE, &format!("{RUNE_PAGES}/{}", page["id"]), None)?;
        }
    }
    lcu.request(Method::POST, RUNE_PAGES, Some(&rune_page(build, champion))).map(drop).map_err(|e| match e {
        AppError::Client(detail) if detail.to_lowercase().contains(MAX_PAGES_ERROR) => AppError::NoFreeRunePage,
        other => other,
    })
}

pub fn import_items(lcu: &Lcu, build: &Build, champion: &str, language: &str) -> Result<()> {
    let id = lcu.get(CURRENT_SUMMONER)?["summonerId"].as_u64().ok_or(AppError::NoSummoner)?;
    let endpoint = format!("/lol-item-sets/v1/item-sets/{id}/sets");
    let mut sets = lcu.get(&endpoint)?;
    upsert_item_set(&mut sets, item_set(build, champion, language), build.champion);
    lcu.request(Method::PUT, &endpoint, Some(&sets)).map(drop)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{catalog::Catalog, opgg};

    fn build() -> Build {
        let data = opgg::decode("fixture", include_str!("fixtures/opgg-aram-ahri.json")).unwrap();
        opgg::parse_build(&data, 103, &Catalog::default()).unwrap()
    }

    #[test]
    fn parses_opgg_build() {
        let build = build();
        let ids = |assets: &[Asset]| assets.iter().map(|a| a.id).collect::<Vec<_>>();
        assert_eq!(ids(&build.core_items), [6655, 4646, 4645]);
        assert_eq!(ids(&build.boots), [3020]);
        assert_eq!(ids(&build.situational_items), [3089]);
        assert_eq!(build.runes.primary_style.id, 8100);
        assert_eq!(build.runes.primary.len() + build.runes.secondary.len() + build.runes.shards.len(), 9);
        assert_eq!(build.skill_order.len(), 15);
        assert_eq!(build.boots[0].name, "#3020");
        assert!(build.positions.is_empty());
    }

    #[test]
    fn replaces_only_its_own_item_set() {
        let build = build();
        let page = rune_page(&build, "Ahri");
        assert_eq!(page["name"], "Xyra · Ahri");
        assert_eq!(page["selectedPerkIds"].as_array().unwrap().len(), 9);

        let mut sets = json!({ "itemSets": [
            { "title": "Xyra · Ahri", "associatedChampions": [103] },
            { "title": "Mi set", "associatedChampions": [103] },
            { "title": "Xyra · Lux", "associatedChampions": [99] },
        ]});
        upsert_item_set(&mut sets, item_set(&build, "Ahri", "es"), 103);
        let titles: Vec<&str> = sets["itemSets"].as_array().unwrap().iter().map(|s| s["title"].as_str().unwrap()).collect();
        assert_eq!(titles, ["Mi set", "Xyra · Lux", "Xyra · Ahri"]);
        assert_eq!(sets["itemSets"][2]["blocks"][2]["items"][0]["id"], "6655");
        assert_eq!(sets["itemSets"][2]["blocks"][2]["type"], "Núcleo");
    }
}
