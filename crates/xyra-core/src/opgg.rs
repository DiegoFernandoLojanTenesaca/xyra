use crate::{
    catalog::{Catalog, named},
    errors::{AppError, Result},
    model::{AugmentRow, Build, BuildMode, GameMode, Matchup, Meta, MetaChampion, Position, PositionMeta, Quality, RunePage, grade},
    web::Client,
};
use serde::{Deserialize, de::DeserializeOwned};
use std::collections::HashMap;

const API: &str = "https://lol-api-champion.op.gg/api";
const MIN_ARENA_GAMES: f64 = 20.0;
const SITUATIONAL_ITEMS: usize = 6;
const DEFAULT_RIFT_POSITION: Position = Position::Mid;
/// Arena tier cutoffs by percentile of average placement: top 10 % = S, next 20 % = A, 30 % = B, 20 % = C, rest = D.
const ARENA_PERCENTILES: [f64; 4] = [0.1, 0.3, 0.6, 0.8];
/// A matchup counts once it holds this share of the champion's games in the position.
const MIN_MATCHUP_SHARE: f64 = 0.01;
pub const MATCHUPS_SHOWN: usize = 5;
const RANKED_LIST: &str = "global/champions/ranked";
/// Game version 16.x is the public patch 26.x.
const PATCH_YEAR_OFFSET: u32 = 10;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AugmentStat {
    /// 0 = S … 6 = F.
    pub tier: u8,
    pub performance: f64,
    pub pick_rate: f64,
}

#[derive(Deserialize)]
struct Response<T> {
    data: Option<T>,
}

pub(crate) fn decode<T: DeserializeOwned>(source: &str, text: &str) -> Result<T> {
    let response: Response<T> = serde_json::from_str(text).map_err(|e| AppError::opgg_format(source, e))?;
    response.data.ok_or(AppError::NoData)
}

#[derive(Deserialize)]
struct MayhemAugment {
    id: u32,
    tier: Option<u8>,
    performance: f64,
    popular: f64,
}

#[derive(Deserialize)]
struct ArenaData {
    augment_group: Vec<ArenaGroup>,
}

#[derive(Deserialize)]
struct ArenaGroup {
    augments: Vec<ArenaAugment>,
}

#[derive(Deserialize)]
struct ArenaAugment {
    id: u32,
    play: f64,
    total_place: f64,
    pick_rate: f64,
}

#[derive(Deserialize)]
struct ChampionTier {
    id: u32,
    tier: u8,
    rank: u32,
}

#[derive(Deserialize)]
struct RankedList {
    data: Vec<RankedChampion>,
    meta: RankedListInfo,
}

#[derive(Deserialize)]
struct RankedListInfo {
    version: String,
}

#[derive(Deserialize)]
struct RankedChampion {
    id: u32,
    positions: Option<Vec<RankedPosition>>,
}

#[derive(Deserialize)]
struct RankedPosition {
    name: String,
    stats: RankedStats,
}

#[derive(Deserialize)]
struct RankedStats {
    win_rate: f64,
    pick_rate: f64,
    ban_rate: f64,
    tier_data: TierData,
}

#[derive(Deserialize)]
struct TierData {
    tier: u8,
    rank: u32,
    rank_prev_patch: Option<u32>,
}

#[derive(Deserialize)]
struct ModeChampion {
    id: u32,
    #[serde(default)]
    is_rip: bool,
    average_stats: Option<ModeStats>,
}

/// ARAM answers a win rate; Arena answers games and top-four finishes, which it calls wins.
#[derive(Deserialize)]
struct ModeStats {
    win_rate: Option<f64>,
    win: Option<f64>,
    play: Option<f64>,
    pick_rate: Option<f64>,
    ban_rate: Option<f64>,
    tier_data: TierData,
}

#[derive(Deserialize)]
pub(crate) struct BuildData {
    summary: Summary,
    runes: Vec<RuneStats>,
    summoner_spells: Vec<IdSet>,
    starter_items: Vec<IdSet>,
    boots: Vec<IdSet>,
    core_items: Vec<IdSet>,
    last_items: Vec<IdSet>,
    skills: Vec<SkillOrder>,
    skill_masteries: Vec<SkillPriority>,
    counters: Vec<Counter>,
}

#[derive(Deserialize)]
struct Summary {
    average_stats: AverageStats,
    positions: Option<Vec<PositionStats>>,
}

#[derive(Deserialize)]
struct AverageStats {
    play: u32,
    win_rate: f64,
}

#[derive(Deserialize)]
struct PositionStats {
    name: String,
}

#[derive(Deserialize)]
struct RuneStats {
    primary_page_id: u32,
    secondary_page_id: u32,
    primary_rune_ids: Vec<u32>,
    secondary_rune_ids: Vec<u32>,
    stat_mod_ids: Vec<u32>,
    play: f64,
    win: f64,
    pick_rate: f64,
}

#[derive(Deserialize)]
struct IdSet {
    ids: Vec<u32>,
}

#[derive(Deserialize)]
struct SkillOrder {
    order: Vec<String>,
}

#[derive(Deserialize)]
struct SkillPriority {
    ids: Vec<String>,
}

#[derive(Deserialize)]
struct Counter {
    champion_id: u32,
    play: f64,
    win: f64,
}

fn fetch<T: DeserializeOwned>(http: &Client, path: &str) -> Result<T> {
    decode(path, &http.get(format!("{API}/{path}")).send()?.error_for_status()?.text()?)
}

pub fn fetch_augments(http: &Client, champion: u32, mode: GameMode) -> Result<HashMap<u32, AugmentStat>> {
    if mode == GameMode::Arena {
        Ok(arena_stats(fetch(http, &format!("global/champions/arena/{champion}"))?))
    } else {
        Ok(mayhem_stats(fetch(http, &format!("contents/stats/champions/{champion}/aram-augments"))?))
    }
}

fn mayhem_stats(augments: Vec<MayhemAugment>) -> HashMap<u32, AugmentStat> {
    augments
        .into_iter()
        .filter_map(|augment| Some((augment.id, AugmentStat { tier: augment.tier?, performance: augment.performance, pick_rate: augment.popular })))
        .collect()
}

fn arena_stats(data: ArenaData) -> HashMap<u32, AugmentStat> {
    let mut placements: Vec<(u32, f64, f64)> = data
        .augment_group
        .into_iter()
        .flat_map(|group| group.augments)
        .filter(|augment| augment.play >= MIN_ARENA_GAMES)
        .map(|augment| (augment.id, augment.total_place / augment.play, augment.pick_rate * 100.0))
        .collect();
    placements.sort_by(|a, b| a.1.total_cmp(&b.1));
    tiers_by_percentile(&placements)
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

/// Ranked tier list of every position in the current patch.
pub fn fetch_meta(http: &Client, catalog: &Catalog) -> Result<Meta> {
    let text = http.get(format!("{API}/{RANKED_LIST}")).send()?.error_for_status()?.text()?;
    let list: RankedList = serde_json::from_str(&text).map_err(|e| AppError::opgg_format(RANKED_LIST, e))?;
    Ok(meta(list, catalog))
}

fn meta(list: RankedList, catalog: &Catalog) -> Meta {
    let mut positions: Vec<PositionMeta> = Position::ALL.map(|position| PositionMeta { position, champions: Vec::new() }).to_vec();
    for champion in list.data {
        for played in champion.positions.into_iter().flatten() {
            let Some(slot) = Position::from_opgg(&played.name).and_then(|position| positions.iter_mut().find(|p| p.position == position)) else { continue };
            let (stats, tier) = (&played.stats, &played.stats.tier_data);
            slot.champions.push(MetaChampion {
                champion: catalog.champion(champion.id),
                tier: tier.tier,
                rank: tier.rank,
                win_rate: stats.win_rate * 100.0,
                pick_rate: stats.pick_rate * 100.0,
                ban_rate: stats.ban_rate * 100.0,
                trend: tier.rank_prev_patch.map(|previous| previous as i32 - tier.rank as i32),
            });
        }
    }
    positions.iter_mut().for_each(|p| p.champions.sort_by_key(|c| c.rank));
    Meta { patch: patch_name(&list.meta.version), positions }
}

/// Champion tier list of ARAM or Arena, best first; Arena's win rate is its share of top-four finishes.
pub fn fetch_mode_champions(http: &Client, mode: GameMode, catalog: &Catalog) -> Result<Vec<MetaChampion>> {
    let path = match mode {
        GameMode::Aram => "global/champions/aram",
        GameMode::Arena => "global/champions/arena",
        _ => return Err(AppError::NoData),
    };
    Ok(mode_champions(fetch(http, path)?, catalog))
}

fn mode_champions(list: Vec<ModeChampion>, catalog: &Catalog) -> Vec<MetaChampion> {
    let mut champions: Vec<MetaChampion> = list
        .into_iter()
        .filter(|champion| !champion.is_rip && catalog.champions.contains_key(&champion.id))
        .filter_map(|champion| {
            let stats = champion.average_stats?;
            let win_rate = stats.win_rate.or_else(|| Some(stats.win? / stats.play?.max(1.0)))?;
            let tier = &stats.tier_data;
            Some(MetaChampion {
                champion: catalog.champion(champion.id),
                tier: tier.tier,
                rank: tier.rank,
                win_rate: win_rate * 100.0,
                pick_rate: stats.pick_rate.unwrap_or_default() * 100.0,
                ban_rate: stats.ban_rate.unwrap_or_default() * 100.0,
                trend: tier.rank_prev_patch.map(|previous| previous as i32 - tier.rank as i32),
            })
        })
        .collect();
    champions.sort_by_key(|c| c.rank);
    champions
}

/// "16.19" becomes "26.19"; anything else is kept as it came.
fn patch_name(version: &str) -> String {
    match version.split_once('.').and_then(|(major, minor)| Some((major.parse::<u32>().ok()?, minor))) {
        Some((major, minor)) => format!("{}.{minor}", major + PATCH_YEAR_OFFSET),
        None => version.to_string(),
    }
}

/// ARAM: Mayhem champion tier list: id -> (tier 1 = best … 5, rank).
pub fn fetch_champion_tiers(http: &Client) -> Result<HashMap<u32, (u8, u32)>> {
    let tiers: Vec<ChampionTier> = fetch(http, "contents/tiers?type=aram_mayhem")?;
    Ok(tiers.into_iter().map(|c| (c.id, (c.tier, c.rank))).collect())
}

fn fetch_rift(http: &Client, champion: u32, position: Position) -> Result<BuildData> {
    fetch(http, &format!("global/champions/ranked/{champion}/{}", position.opgg()))
}

fn main_position(data: &BuildData) -> Option<Position> {
    data.summary.positions.as_deref()?.first().and_then(|p| Position::from_opgg(&p.name))
}

/// ARAM builds are the ones OP.GG also shows for ARAM: Mayhem; Rift builds default to the most played position.
pub fn fetch_build(http: &Client, champion: u32, mode: BuildMode, position: Option<Position>, catalog: &Catalog) -> Result<Build> {
    if mode == BuildMode::Aram {
        return parse_build(&fetch(http, &format!("global/champions/aram/{champion}/none"))?, champion, catalog);
    }
    let requested = position.unwrap_or(DEFAULT_RIFT_POSITION);
    let mut data = fetch_rift(http, champion, requested)?;
    let played = position.or_else(|| main_position(&data)).unwrap_or(requested);
    if played != requested {
        data = fetch_rift(http, champion, played)?;
    }
    let mut build = parse_build(&data, champion, catalog)?;
    build.position = Some(played);
    Ok(build)
}

/// Opponents of the champion in the position with enough games: (id, games, the champion's win rate).
fn matchups(data: &BuildData) -> Vec<(u32, u32, f64)> {
    let games = f64::from(data.summary.average_stats.play);
    data.counters
        .iter()
        .filter(|counter| counter.play > 0.0 && counter.play >= games * MIN_MATCHUP_SHARE)
        .map(|counter| (counter.champion_id, counter.play as u32, 100.0 * counter.win / counter.play))
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

/// OP.GG lists are sorted by games played.
fn most_played(sets: &[IdSet]) -> Vec<u32> {
    sets.first().map(|set| set.ids.clone()).unwrap_or_default()
}

pub(crate) fn parse_build(data: &BuildData, champion: u32, catalog: &Catalog) -> Result<Build> {
    let item = |id| named(&catalog.items, id);
    let rune = |id| named(&catalog.runes, id);
    let runes = data.runes.first().ok_or(AppError::NoData)?;
    let boots = most_played(&data.boots);
    let core = most_played(&data.core_items);
    let mut situational: Vec<u32> = Vec::new();
    for id in data.last_items.iter().flat_map(|set| set.ids.iter().copied()) {
        if !core.contains(&id) && !boots.contains(&id) && !situational.contains(&id) {
            situational.push(id);
        }
    }
    situational.truncate(SITUATIONAL_ITEMS);
    let mut opponents = matchups(data);
    opponents.sort_by(|a, b| b.2.total_cmp(&a.2));
    let stats = &data.summary.average_stats;
    Ok(Build {
        champion,
        runes: RunePage {
            primary_style: rune(runes.primary_page_id),
            secondary_style: rune(runes.secondary_page_id),
            primary: runes.primary_rune_ids.iter().copied().map(rune).collect(),
            secondary: runes.secondary_rune_ids.iter().copied().map(rune).collect(),
            shards: runes.stat_mod_ids.iter().copied().map(rune).collect(),
            win_rate: if runes.play > 0.0 { 100.0 * runes.win / runes.play } else { 0.0 },
            pick_rate: runes.pick_rate * 100.0,
        },
        spells: most_played(&data.summoner_spells).into_iter().map(|id| named(&catalog.spells, id)).collect(),
        starting_items: most_played(&data.starter_items).into_iter().map(item).collect(),
        boots: boots.into_iter().map(item).collect(),
        core_items: core.into_iter().map(item).collect(),
        situational_items: situational.into_iter().map(item).collect(),
        skill_order: data.skills.first().map(|s| s.order.clone()).unwrap_or_default(),
        skill_priority: data.skill_masteries.first().map(|s| s.ids.clone()).unwrap_or_default(),
        win_rate: stats.win_rate * 100.0,
        games: stats.play,
        position: None,
        positions: data.summary.positions.iter().flatten().filter_map(|p| Position::from_opgg(&p.name)).collect(),
        strong_against: to_matchups(opponents.iter().copied(), catalog),
        weak_against: to_matchups(opponents.iter().rev().copied(), catalog),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    #[test]
    fn groups_the_ranked_list_by_position_best_first() {
        let list: RankedList = serde_json::from_value(json!({
            "meta": { "version": "16.19" },
            "data": [
                { "id": 1, "positions": [
                    { "name": "MID", "stats": { "win_rate": 0.52, "pick_rate": 0.05, "ban_rate": 0.1, "tier_data": { "tier": 1, "rank": 2, "rank_prev_patch": 5 } } },
                    { "name": "TOP", "stats": { "win_rate": 0.48, "pick_rate": 0.01, "ban_rate": 0.1, "tier_data": { "tier": 4, "rank": 40, "rank_prev_patch": null } } }
                ] },
                { "id": 2, "positions": [{ "name": "MID", "stats": { "win_rate": 0.55, "pick_rate": 0.08, "ban_rate": 0.2, "tier_data": { "tier": 1, "rank": 1, "rank_prev_patch": 1 } } }] },
                { "id": 3, "positions": null }
            ]
        }))
        .unwrap();
        let meta = meta(list, &Catalog::default());
        assert_eq!(meta.patch, "26.19");
        let mid = &meta.positions.iter().find(|p| p.position == Position::Mid).unwrap().champions;
        assert_eq!(mid.iter().map(|c| (c.champion.id, c.trend)).collect::<Vec<_>>(), [(2, Some(0)), (1, Some(3))]);
        assert_eq!(mid[1].win_rate.round(), 52.0);
        let top = &meta.positions.iter().find(|p| p.position == Position::Top).unwrap().champions;
        assert_eq!(top[0].trend, None);
        assert_eq!(patch_name("latest"), "latest");
    }

    #[test]
    fn reads_aram_and_arena_lists_best_first() {
        let catalog = Catalog { champions: [1, 2, 3].map(|id| (id, (format!("C{id}"), String::new()))).into(), ..Catalog::default() };
        let list: Vec<ModeChampion> = serde_json::from_value(json!([
            { "id": 1, "average_stats": { "win_rate": 0.51, "pick_rate": 0.06, "ban_rate": null, "tier_data": { "tier": 2, "rank": 9, "rank_prev_patch": 4 } } },
            { "id": 2, "average_stats": { "win": 60.0, "play": 100.0, "pick_rate": 0.1, "ban_rate": 0.05, "tier_data": { "tier": 1, "rank": 1, "rank_prev_patch": null } } },
            { "id": 3, "is_rip": true, "average_stats": null },
            { "id": 60086, "average_stats": { "win_rate": 0.5, "tier_data": { "tier": 1, "rank": 2, "rank_prev_patch": null } } }
        ]))
        .unwrap();
        let champions = mode_champions(list, &catalog);
        assert_eq!(champions.iter().map(|c| (c.champion.id, c.trend)).collect::<Vec<_>>(), [(2, None), (1, Some(-5))]);
        assert_eq!((champions[0].win_rate.round(), champions[1].ban_rate), (60.0, 0.0));
    }

    fn ranked(runes: Value, counters: Value) -> BuildData {
        let response = json!({ "data": {
            "summary": { "average_stats": { "play": 1000, "win_rate": 0.5 }, "positions": [{ "name": "MID" }] },
            "runes": runes,
            "summoner_spells": [],
            "starter_items": [],
            "boots": [],
            "core_items": [],
            "last_items": [],
            "skills": [],
            "skill_masteries": [],
            "counters": counters
        }});
        decode("ranked", &response.to_string()).unwrap()
    }

    fn one_rune_page() -> Value {
        json!([{ "primary_page_id": 8100, "secondary_page_id": 8000, "primary_rune_ids": [], "secondary_rune_ids": [], "stat_mod_ids": [], "play": 10, "win": 5, "pick_rate": 0.1 }])
    }

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
        let counters = json!([
            { "champion_id": 1, "play": 100, "win": 60 },
            { "champion_id": 2, "play": 100, "win": 40 },
            { "champion_id": 3, "play": 5, "win": 5 }
        ]);
        let data = ranked(one_rune_page(), counters);
        assert_eq!(main_position(&data), Some(Position::Mid));
        let build = parse_build(&data, 157, &Catalog::default()).unwrap();
        assert_eq!(build.strong_against.iter().map(|m| m.champion.id).collect::<Vec<_>>(), [1, 2]);
        assert_eq!(build.weak_against[0].champion.id, 2);
        assert_eq!(build.weak_against[0].win_rate, 40.0);
        assert_eq!(build.positions, [Position::Mid]);
        assert_eq!((build.win_rate, build.games, build.runes.win_rate), (50.0, 1000, 50.0));
    }

    #[test]
    fn a_changed_answer_is_an_error_never_a_zero() {
        assert_eq!(decode::<Vec<MayhemAugment>>("augments", r#"{"data":null}"#).err(), Some(AppError::NoData));
        let renamed = decode::<Vec<MayhemAugment>>("augments", r#"{"data":[{"id":1,"tier":0,"score":90,"popular":2}]}"#);
        assert!(matches!(renamed, Err(AppError::OpggFormat(detail)) if detail.starts_with("augments: ")));
        let unranked = decode::<Vec<MayhemAugment>>("augments", r#"{"data":[{"id":1,"tier":null,"performance":90,"popular":2}]}"#).unwrap();
        assert!(mayhem_stats(unranked).is_empty());
        assert_eq!(parse_build(&ranked(json!([]), json!([])), 157, &Catalog::default()).err(), Some(AppError::NoData));
    }
}
