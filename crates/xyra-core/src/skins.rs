use crate::{
    config::SkinChoice,
    errors::Result,
    league::{Lcu, asset_url},
    profile::CURRENT_SUMMONER,
};
use reqwest::Method;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use ts_rs::TS;

const CAROUSEL: &str = "/lol-champ-select/v1/skin-carousel-skins";
const MY_SELECTION: &str = "/lol-champ-select/v1/session/my-selection";
/// Right after a champion is assigned the carousel may still list the one before; it is read again this often.
const CAROUSEL_ATTEMPTS: u32 = 5;
const CAROUSEL_RETRY: Duration = Duration::from_secs(1);
/// A champion's base skin id is the champion id times this.
const SKINS_PER_CHAMPION: u32 = 1000;

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CarouselSkin {
    id: u32,
    champion_id: u32,
    #[serde(default)]
    unlocked: bool,
    #[serde(default)]
    disabled: bool,
    #[serde(default)]
    child_skins: Vec<CarouselChroma>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CarouselChroma {
    id: u32,
    #[serde(default)]
    unlocked: bool,
    #[serde(default)]
    disabled: bool,
}

/// A skin the player owns, with the chromas they own of it.
#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct OwnedSkin {
    pub id: u32,
    pub name: String,
    pub icon: String,
    pub chromas: Vec<OwnedChroma>,
}

#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct OwnedChroma {
    pub id: u32,
    pub name: String,
    /// CSS color of its swatch.
    pub color: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct InventorySkin {
    id: u32,
    name: String,
    #[serde(default)]
    tile_path: String,
    ownership: Ownership,
    #[serde(default)]
    chromas: Vec<InventoryChroma>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct InventoryChroma {
    id: u32,
    name: String,
    #[serde(default)]
    colors: Vec<String>,
    ownership: Ownership,
}

#[derive(Deserialize)]
struct Ownership {
    owned: bool,
}

/// The skins and chromas the player can wear now: owned, not the base skin, and chromas only when asked for.
fn wearable(carousel: &[CarouselSkin], champion: u32, chromas: bool) -> Vec<u32> {
    carousel
        .iter()
        .filter(|skin| skin.champion_id == champion && skin.unlocked && !skin.disabled)
        .flat_map(|skin| {
            let own = (skin.id != champion * SKINS_PER_CHAMPION).then_some(skin.id);
            let colors = skin.child_skins.iter().filter(move |chroma| chromas && chroma.unlocked && !chroma.disabled).map(|chroma| chroma.id);
            own.into_iter().chain(colors)
        })
        .collect()
}

/// Whether the player can wear `id` now, base skin and chromas included.
fn owns(carousel: &[CarouselSkin], champion: u32, id: u32) -> bool {
    carousel
        .iter()
        .filter(|skin| skin.champion_id == champion && skin.unlocked && !skin.disabled)
        .any(|skin| skin.id == id || skin.child_skins.iter().any(|chroma| chroma.id == id && chroma.unlocked && !chroma.disabled))
}

/// One of `options`, spread by the clock: good enough to vary the skin from game to game.
fn any(options: &[u32]) -> Option<u32> {
    let seed = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |elapsed| elapsed.subsec_nanos() as usize);
    options.get(seed % options.len().max(1)).copied()
}

fn pick(carousel: &[CarouselSkin], champion: u32, choice: SkinChoice, favorite: Option<u32>, chromas: bool) -> Option<u32> {
    let random = || any(&wearable(carousel, champion, chromas));
    match choice {
        SkinChoice::Off => None,
        SkinChoice::Random => random(),
        SkinChoice::Favorite => favorite.filter(|&id| owns(carousel, champion, id)).or_else(random),
    }
}

/// Puts on the champion the skin the player's choice asks for, among the ones they own; returns the one it chose.
pub fn dress(lcu: &Lcu, champion: u32, choice: SkinChoice, favorite: Option<u32>, chromas: bool) -> Result<Option<u32>> {
    for _ in 0..CAROUSEL_ATTEMPTS {
        let carousel: Vec<CarouselSkin> = lcu.get_as(CAROUSEL)?;
        if carousel.iter().any(|skin| skin.champion_id == champion) {
            let chosen = pick(&carousel, champion, choice, favorite, chromas);
            if let Some(id) = chosen {
                lcu.request(Method::PATCH, MY_SELECTION, Some(&json!({ "selectedSkinId": id })))?;
            }
            return Ok(chosen);
        }
        thread::sleep(CAROUSEL_RETRY);
    }
    Ok(None)
}

/// The skins of a champion the player owns, to pick a favorite outside champion select.
pub fn owned(lcu: &Lcu, champion: u32) -> Result<Vec<OwnedSkin>> {
    let summoner = lcu.get(CURRENT_SUMMONER)?["summonerId"].as_u64().unwrap_or_default();
    let skins: Vec<InventorySkin> = lcu.get_as(&format!("/lol-champions/v1/inventories/{summoner}/champions/{champion}/skins"))?;
    Ok(owned_skins(skins))
}

fn owned_skins(skins: Vec<InventorySkin>) -> Vec<OwnedSkin> {
    skins
        .into_iter()
        .filter(|skin| skin.ownership.owned)
        .map(|skin| OwnedSkin {
            id: skin.id,
            name: skin.name,
            icon: if skin.tile_path.is_empty() { String::new() } else { asset_url(&skin.tile_path) },
            chromas: skin
                .chromas
                .into_iter()
                .filter(|chroma| chroma.ownership.owned)
                .map(|chroma| OwnedChroma { id: chroma.id, name: chroma.name, color: chroma.colors.first().cloned().unwrap_or_default() })
                .collect(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn carousel() -> Vec<CarouselSkin> {
        serde_json::from_value(json!([
            { "id": 99000, "championId": 99, "unlocked": true, "childSkins": [] },
            { "id": 99001, "championId": 99, "unlocked": false, "childSkins": [] },
            { "id": 99007, "championId": 99, "unlocked": true, "childSkins": [
                { "id": 99009, "unlocked": true }, { "id": 99010, "unlocked": false }
            ] },
            { "id": 99017, "championId": 99, "unlocked": true, "disabled": true, "childSkins": [] }
        ]))
        .unwrap()
    }

    #[test]
    fn dresses_only_in_skins_the_player_owns() {
        let carousel = carousel();
        assert_eq!(wearable(&carousel, 99, false), [99007]);
        assert_eq!(wearable(&carousel, 99, true), [99007, 99009]);
        assert!(wearable(&carousel, 1, true).is_empty());
        assert_eq!(pick(&carousel, 99, SkinChoice::Random, None, false), Some(99007));
        assert_eq!(pick(&carousel, 99, SkinChoice::Favorite, Some(99000), false), Some(99000));
        assert_eq!(pick(&carousel, 99, SkinChoice::Favorite, Some(99001), false), Some(99007));
        assert_eq!(pick(&carousel, 99, SkinChoice::Favorite, Some(99010), false), Some(99007));
        assert_eq!(pick(&carousel, 99, SkinChoice::Off, Some(99007), true), None);
    }

    #[test]
    fn lists_owned_skins_with_their_owned_chromas() {
        let skins: Vec<InventorySkin> = serde_json::from_value(json!([
            { "id": 99000, "name": "Lux", "tilePath": "/lol-game-data/assets/Lux.jpg", "ownership": { "owned": true }, "chromas": [] },
            { "id": 99001, "name": "Brujita Lux", "ownership": { "owned": false }, "chromas": [] },
            { "id": 99008, "name": "Lux Emperatriz Lunar", "ownership": { "owned": true }, "chromas": [
                { "id": 99009, "name": "Rubí", "colors": ["#D33528"], "ownership": { "owned": true } },
                { "id": 99010, "name": "Zafiro", "colors": ["#2756CE"], "ownership": { "owned": false } }
            ] }
        ]))
        .unwrap();
        let owned = owned_skins(skins);
        assert_eq!(owned.iter().map(|skin| skin.id).collect::<Vec<_>>(), [99000, 99008]);
        assert!(owned[0].icon.ends_with("/lux.jpg"));
        assert_eq!(owned[1].chromas, [OwnedChroma { id: 99009, name: "Rubí".into(), color: "#D33528".into() }]);
    }
}
