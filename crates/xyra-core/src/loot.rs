use crate::{
    errors::{AppError, Result},
    league::{Lcu, asset_url},
};
use reqwest::Method;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use ts_rs::TS;

const PLAYER_LOOT: &str = "/lol-loot/v1/player-loot";
const RECIPES: &str = "/lol-loot/v1/recipes/initial-item";
const GRANTS: &str = "/lol-rewards/v1/grants";
const BLUE_ESSENCE: &str = "CURRENCY_champion";
const ORANGE_ESSENCE: &str = "CURRENCY_cosmetic";
const MYTHIC_ESSENCE: &str = "CURRENCY_mythic";
const KEY: &str = "MATERIAL_key";
const KEY_FRAGMENT: &str = "MATERIAL_key_fragment";
const KEY_FORGE: &str = "MATERIAL_key_fragment_forge";
const FRAGMENTS_PER_KEY: u32 = 3;
const CHEST: &str = "CHEST";
const CHAMPION_SHARD: &str = "CHAMPION_RENTAL";
const OPEN_RECIPE: &str = "OPEN";
const OWNED: &str = "OWNED";
const PENDING_SELECTION: &str = "PENDING_SELECTION";

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LootItem {
    loot_id: String,
    #[serde(rename = "type")]
    kind: String,
    count: u32,
    #[serde(default)]
    localized_name: String,
    #[serde(default)]
    item_desc: String,
    #[serde(default)]
    tile_path: String,
    #[serde(default)]
    item_status: String,
    #[serde(default)]
    disenchant_value: u64,
    #[serde(default)]
    disenchant_recipe_name: String,
}

impl LootItem {
    fn view(&self, count: u32) -> LootItemView {
        let name = [&self.localized_name, &self.item_desc].into_iter().find(|name| !name.is_empty()).cloned().unwrap_or_else(|| self.loot_id.clone());
        LootItemView { name, icon: if self.tile_path.is_empty() { String::new() } else { asset_url(&self.tile_path) }, count }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Recipe {
    recipe_name: String,
    #[serde(rename = "type")]
    kind: String,
    slots: Vec<RecipeSlot>,
}

#[derive(Clone, Debug, Deserialize)]
struct RecipeSlot {}

/// An item of the loot as the app shows it.
#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct LootItemView {
    pub name: String,
    pub icon: String,
    pub count: u32,
}

/// What Xyra can do with the loot; only these, never skin shards, emotes or eternals.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub enum LootActionKind {
    /// Capsules, orbs and mystery items that open without a key.
    OpenFree,
    ForgeKeys,
    OpenChests,
    /// Champion shards of champions the player already owns, for blue essence.
    DisenchantChampionShards,
}

impl LootActionKind {
    /// Whether it gives something up for good, so the app asks before doing it.
    pub fn is_irreversible(self) -> bool {
        self == LootActionKind::DisenchantChampionShards
    }
}

/// One step of an action: a recipe crafted with these items, this many times.
#[derive(Clone, Debug, PartialEq)]
struct Craft {
    recipe: String,
    items: Vec<String>,
    times: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct LootAction {
    pub kind: LootActionKind,
    /// How many times it crafts, like keys forged or chests opened.
    pub times: u32,
    /// What it uses.
    pub items: Vec<LootItemView>,
    /// Blue essence it gives, for disenchants.
    #[ts(type = "number")]
    pub essence: u64,
    #[serde(skip)]
    crafts: Vec<Craft>,
}

/// A reward the client keeps until the player picks it.
#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct PendingReward {
    pub grant_id: String,
    pub group_id: String,
    /// How many choices the player takes.
    pub picks: u32,
    pub choices: Vec<RewardChoice>,
}

#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct RewardChoice {
    pub id: String,
    pub name: String,
    pub icon: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct LootSummary {
    #[ts(type = "number")]
    pub blue_essence: u64,
    #[ts(type = "number")]
    pub orange_essence: u64,
    #[ts(type = "number")]
    pub mythic_essence: u64,
    pub chests: u32,
    pub keys: u32,
    pub key_fragments: u32,
    /// Skin shards stay for the client's own reroll and upgrade.
    pub skin_shards: u32,
    pub actions: Vec<LootAction>,
    pub rewards: Vec<PendingReward>,
}

/// What an action got, and why it stopped when it did not finish.
#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct LootOutcome {
    pub gained: Vec<LootItemView>,
    pub stopped: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Grant {
    info: GrantInfo,
    reward_group: RewardGroup,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GrantInfo {
    id: String,
    status: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RewardGroup {
    id: String,
    #[serde(default)]
    rewards: Vec<Reward>,
    selection_strategy_config: Option<SelectionConfig>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SelectionConfig {
    max_selections_allowed: u32,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Reward {
    id: String,
    #[serde(default)]
    localizations: HashMap<String, String>,
    #[serde(default)]
    media: HashMap<String, String>,
}

#[derive(Deserialize)]
struct CraftAnswer {
    #[serde(default)]
    added: Vec<LootDelta>,
    #[serde(default)]
    redeemed: Vec<LootDelta>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LootDelta {
    delta_count: i64,
    player_loot: LootItem,
}

fn count(loot: &[LootItem], id: &str) -> u32 {
    loot.iter().filter(|item| item.loot_id == id).map(|item| item.count).sum()
}

/// The actions the loot allows; `recipes` gives the recipes of an item, from the client.
fn plan(loot: &[LootItem], mut recipes: impl FnMut(&str) -> Result<Vec<Recipe>>) -> Result<Vec<LootAction>> {
    let mut actions = Vec::new();
    let keys = count(loot, KEY);
    let mut free = LootAction { kind: LootActionKind::OpenFree, times: 0, items: Vec::new(), essence: 0, crafts: Vec::new() };
    let mut chests = LootAction { kind: LootActionKind::OpenChests, times: 0, items: Vec::new(), essence: 0, crafts: Vec::new() };
    let mut keys_left = keys;
    for chest in loot.iter().filter(|item| item.kind == CHEST && item.count > 0) {
        let Some(open) = recipes(&chest.loot_id)?.into_iter().find(|recipe| recipe.kind == OPEN_RECIPE) else { continue };
        match open.slots.len() {
            1 => {
                free.times += chest.count;
                free.items.push(chest.view(chest.count));
                free.crafts.push(Craft { recipe: open.recipe_name, items: vec![chest.loot_id.clone()], times: chest.count });
            }
            2 if keys_left > 0 => {
                let times = chest.count.min(keys_left);
                keys_left -= times;
                chests.times += times;
                chests.items.push(chest.view(times));
                chests.crafts.push(Craft { recipe: open.recipe_name, items: vec![chest.loot_id.clone(), KEY.into()], times });
            }
            _ => {}
        }
    }
    let fragments = count(loot, KEY_FRAGMENT);
    if fragments >= FRAGMENTS_PER_KEY {
        let times = fragments / FRAGMENTS_PER_KEY;
        let fragment = loot.iter().find(|item| item.loot_id == KEY_FRAGMENT).map(|item| item.view(times * FRAGMENTS_PER_KEY));
        actions.push(LootAction {
            kind: LootActionKind::ForgeKeys,
            times,
            items: fragment.into_iter().collect(),
            essence: 0,
            crafts: vec![Craft { recipe: KEY_FORGE.into(), items: vec![KEY_FRAGMENT.into()], times }],
        });
    }
    for action in [free, chests] {
        if action.times > 0 {
            actions.push(action);
        }
    }
    let shards: Vec<&LootItem> = loot
        .iter()
        .filter(|item| item.kind == CHAMPION_SHARD && item.item_status == OWNED && item.count > 0 && !item.disenchant_recipe_name.is_empty())
        .collect();
    if !shards.is_empty() {
        actions.push(LootAction {
            kind: LootActionKind::DisenchantChampionShards,
            times: shards.iter().map(|shard| shard.count).sum(),
            items: shards.iter().map(|shard| shard.view(shard.count)).collect(),
            essence: shards.iter().map(|shard| shard.disenchant_value * u64::from(shard.count)).sum(),
            crafts: shards
                .iter()
                .map(|shard| Craft { recipe: shard.disenchant_recipe_name.clone(), items: vec![shard.loot_id.clone()], times: shard.count })
                .collect(),
        });
    }
    Ok(actions)
}

fn pending_rewards(grants: Vec<Grant>) -> Vec<PendingReward> {
    grants
        .into_iter()
        .filter(|grant| grant.info.status == PENDING_SELECTION && !grant.reward_group.rewards.is_empty())
        .map(|grant| PendingReward {
            picks: grant.reward_group.selection_strategy_config.map_or(1, |config| config.max_selections_allowed.max(1)),
            grant_id: grant.info.id,
            group_id: grant.reward_group.id,
            choices: grant
                .reward_group
                .rewards
                .into_iter()
                .map(|reward| RewardChoice {
                    name: reward.localizations.get("title").cloned().unwrap_or_default(),
                    icon: reward.media.get("iconUrl").map(|icon| if icon.starts_with('/') { asset_url(icon) } else { icon.clone() }).unwrap_or_default(),
                    id: reward.id,
                })
                .collect(),
        })
        .collect()
}

fn read_loot(lcu: &Lcu) -> Result<Vec<LootItem>> {
    lcu.get_as(PLAYER_LOOT)
}

fn read_recipes(lcu: &Lcu, loot_id: &str) -> Result<Vec<Recipe>> {
    lcu.get_as(&format!("{RECIPES}/{loot_id}"))
}

/// The player's essences, chests and keys, what Xyra can do with them and the rewards waiting to be picked.
pub fn summary(lcu: &Lcu) -> Result<LootSummary> {
    let loot = read_loot(lcu)?;
    let actions = plan(&loot, |id| read_recipes(lcu, id))?;
    let free: u32 = actions.iter().filter(|action| action.kind == LootActionKind::OpenFree).map(|action| action.times).sum();
    let chests = loot.iter().filter(|item| item.kind == CHEST).map(|item| item.count).sum::<u32>() - free;
    Ok(LootSummary {
        blue_essence: count(&loot, BLUE_ESSENCE).into(),
        orange_essence: count(&loot, ORANGE_ESSENCE).into(),
        mythic_essence: count(&loot, MYTHIC_ESSENCE).into(),
        chests,
        keys: count(&loot, KEY),
        key_fragments: count(&loot, KEY_FRAGMENT),
        skin_shards: loot.iter().filter(|item| item.kind.starts_with("SKIN")).map(|item| item.count).sum(),
        actions,
        rewards: pending_rewards(lcu.get_as(GRANTS)?),
    })
}

/// Runs an action on the loot as it is now, recipe by recipe; a failure stops it and says what was done.
pub fn run(lcu: &Lcu, kind: LootActionKind) -> Result<LootOutcome> {
    let loot = read_loot(lcu)?;
    let action = plan(&loot, |id| read_recipes(lcu, id))?.into_iter().find(|action| action.kind == kind).ok_or(AppError::NoData)?;
    let mut gained: Vec<LootItemView> = Vec::new();
    for craft in action.crafts {
        let url = format!("/lol-loot/v1/recipes/{}/craft?repeat={}", craft.recipe, craft.times);
        let answer = match lcu.request(Method::POST, &url, Some(&json!(craft.items))) {
            Ok(answer) => answer,
            Err(e) => return Ok(LootOutcome { gained, stopped: Some(e.to_string()) }),
        };
        let answer: CraftAnswer = serde_json::from_value(answer).map_err(|e| AppError::client_format(RECIPES, e))?;
        for delta in answer.added.into_iter().chain(answer.redeemed).filter(|delta| delta.delta_count > 0) {
            gained.push(delta.player_loot.view(delta.delta_count as u32));
        }
    }
    Ok(LootOutcome { gained, stopped: None })
}

/// Takes the chosen rewards of a pending grant.
pub fn claim(lcu: &Lcu, grant_id: &str, group_id: &str, choices: &[String]) -> Result<()> {
    let body = json!({ "grantId": grant_id, "rewardGroupId": group_id, "selections": choices });
    lcu.request(Method::POST, &format!("{GRANTS}/{grant_id}/select"), Some(&body)).map(drop)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    fn loot() -> Vec<LootItem> {
        serde_json::from_value(json!([
            { "lootId": "CHEST_187", "type": "CHEST", "count": 1, "localizedName": "Gesto Misterioso" },
            { "lootId": "CHEST_generic", "type": "CHEST", "count": 12, "localizedName": "Cofre Hextech" },
            { "lootId": "MATERIAL_key", "type": "MATERIAL", "count": 2 },
            { "lootId": "MATERIAL_key_fragment", "type": "MATERIAL", "count": 7 },
            { "lootId": "CHAMPION_RENTAL_99", "type": "CHAMPION_RENTAL", "count": 2, "itemDesc": "Lux", "itemStatus": "OWNED",
              "disenchantValue": 90, "disenchantRecipeName": "CHAMPION_RENTAL_disenchant" },
            { "lootId": "CHAMPION_RENTAL_1", "type": "CHAMPION_RENTAL", "count": 1, "itemDesc": "Annie", "itemStatus": "NONE",
              "disenchantValue": 90, "disenchantRecipeName": "CHAMPION_RENTAL_disenchant" },
            { "lootId": "CHAMPION_SKIN_RENTAL_79008", "type": "SKIN_RENTAL", "count": 1 },
            { "lootId": "CURRENCY_champion", "type": "CURRENCY", "count": 61588 }
        ]))
        .unwrap()
    }

    fn recipes(id: &str) -> Result<Vec<Recipe>> {
        let slots: Value = if id == "CHEST_generic" { json!([{}, {}]) } else { json!([{}]) };
        Ok(serde_json::from_value(json!([{ "recipeName": format!("{id}_OPEN"), "type": "OPEN", "slots": slots }])).unwrap())
    }

    #[test]
    fn plans_only_what_the_loot_allows_and_never_skin_shards() {
        let actions = plan(&loot(), recipes).unwrap();
        let by_kind = |kind| actions.iter().find(|action| action.kind == kind).unwrap();
        assert_eq!(actions.len(), 4);
        assert_eq!((by_kind(LootActionKind::ForgeKeys).times, by_kind(LootActionKind::ForgeKeys).crafts[0].times), (2, 2));
        let free = by_kind(LootActionKind::OpenFree);
        assert_eq!((free.times, free.crafts[0].items.clone()), (1, vec!["CHEST_187".to_string()]));
        let chests = by_kind(LootActionKind::OpenChests);
        assert_eq!((chests.times, chests.crafts[0].items.clone()), (2, vec!["CHEST_generic".to_string(), KEY.to_string()]));
        let shards = by_kind(LootActionKind::DisenchantChampionShards);
        assert_eq!((shards.times, shards.essence, shards.items[0].name.as_str()), (2, 180, "Lux"));
        assert!(shards.kind.is_irreversible() && !free.kind.is_irreversible());
        assert!(actions.iter().flat_map(|action| &action.crafts).all(|craft| !craft.items.iter().any(|item| item.contains("SKIN"))));
    }

    #[test]
    fn lists_rewards_waiting_to_be_picked() {
        let grants: Vec<Grant> = serde_json::from_value(json!([
            { "info": { "id": "g1", "status": "PENDING_SELECTION" },
              "rewardGroup": { "id": "r1", "selectionStrategyConfig": { "maxSelectionsAllowed": 1, "minSelectionsAllowed": 1 },
                "rewards": [{ "id": "a", "localizations": { "title": "750 de Esencia Azul" }, "media": {} }] } },
            { "info": { "id": "g2", "status": "FULFILLED" }, "rewardGroup": { "id": "r2", "rewards": [{ "id": "b" }] } }
        ]))
        .unwrap();
        let pending = pending_rewards(grants);
        assert_eq!(pending.len(), 1);
        assert_eq!((pending[0].grant_id.as_str(), pending[0].picks, pending[0].choices[0].name.as_str()), ("g1", 1, "750 de Esencia Azul"));
    }
}
