use crate::{
    errors::Result,
    league::{Lcu, parse as parse_client},
    model::{ChampSelect, ChampionInfo, GameMode, Matchup, Position},
    opgg::MATCHUPS_SHOWN,
};
use serde::Deserialize;
use serde_json::Value;
use std::{
    collections::{HashMap, HashSet},
    time::{Duration, Instant},
};

pub const SESSION: &str = "/lol-champ-select/v1/session";
const PICKABLE: &str = "/lol-champ-select/v1/pickable-champion-ids";
/// How long a pick must stay before its runes are imported.
const AUTO_RUNES_SETTLE: Duration = Duration::from_secs(3);

#[derive(Clone, Debug, PartialEq)]
pub struct ChampSelectSession {
    pub champion: Option<u32>,
    pub bench: Vec<u32>,
    pub enemies: Vec<u32>,
    pub position: Option<Position>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Session {
    local_player_cell_id: i64,
    my_team: Vec<Member>,
    their_team: Vec<Pick>,
    bench_champions: Vec<Pick>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Member {
    cell_id: i64,
    champion_id: u32,
    assigned_position: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Pick {
    champion_id: u32,
}

fn picked(id: u32) -> Option<u32> {
    (id > 0).then_some(id)
}

fn champion_ids(picks: &[Pick]) -> Vec<u32> {
    picks.iter().filter_map(|pick| picked(pick.champion_id)).collect()
}

pub fn parse(session: &Value) -> Result<Option<ChampSelectSession>> {
    if session.is_null() {
        return Ok(None);
    }
    let session: Session = parse_client(SESSION, session)?;
    let Some(cell) = session.my_team.iter().find(|member| member.cell_id == session.local_player_cell_id) else { return Ok(None) };
    Ok(Some(ChampSelectSession {
        champion: picked(cell.champion_id),
        bench: champion_ids(&session.bench_champions),
        enemies: champion_ids(&session.their_team),
        position: Position::from_client(&cell.assigned_position),
    }))
}

/// Champions the player can pick or take from the bench in the current champion select.
pub fn read_pickable(lcu: &Lcu) -> Result<HashSet<u32>> {
    lcu.get_as(PICKABLE)
}

/// The current champion select: what can be picked, counter picks and the runes to import.
#[derive(Default)]
pub struct ChampSelectTracker {
    session: Option<ChampSelectSession>,
    pickable: Option<HashSet<u32>>,
    last_champion: Option<u32>,
    counter_picks: HashMap<(u32, Position), Option<Vec<Matchup>>>,
    requested: HashSet<(u32, Position)>,
    rune_import: Option<(u32, Instant)>,
    runes_imported_for: Option<u32>,
}

impl ChampSelectTracker {
    pub fn is_active(&self) -> bool {
        self.session.is_some()
    }

    /// The champion picked last, which is the one the game starts with.
    pub fn last_champion(&self) -> Option<u32> {
        self.last_champion
    }

    pub fn position(&self) -> Option<Position> {
        self.session.as_ref().and_then(|s| s.position)
    }

    pub fn clear(&mut self) {
        *self = ChampSelectTracker { last_champion: self.last_champion, ..ChampSelectTracker::default() };
    }

    /// Returns the (enemy, position) pairs whose counter picks are still unknown.
    pub fn update(&mut self, session: ChampSelectSession, pickable: Option<HashSet<u32>>, mode: GameMode, auto_runes: bool) -> Vec<(u32, Position)> {
        let changed = self.session.as_ref().map(|s| s.champion) != Some(session.champion);
        if session.champion.is_some() {
            self.last_champion = session.champion;
        }
        if auto_runes && changed {
            self.rune_import = session.champion.filter(|&c| Some(c) != self.runes_imported_for).map(|c| (c, Instant::now() + AUTO_RUNES_SETTLE));
        }
        let wanted = match (mode, session.position) {
            (GameMode::SummonersRift, Some(position)) => {
                session.enemies.iter().map(|&enemy| (enemy, position)).filter(|key| self.requested.insert(*key)).collect()
            }
            _ => Vec::new(),
        };
        self.session = Some(session);
        self.pickable = pickable;
        wanted
    }

    pub fn set_counter_picks(&mut self, enemy: u32, position: Position, picks: Option<Vec<Matchup>>) {
        if self.requested.contains(&(enemy, position)) {
            self.counter_picks.insert((enemy, position), picks);
        }
    }

    pub fn rune_deadline(&self) -> Option<Instant> {
        self.rune_import.map(|r| r.1)
    }

    pub fn take_due_rune_import(&mut self, now: Instant) -> Option<u32> {
        let (champion, at) = self.rune_import?;
        (at <= now).then(|| {
            self.rune_import = None;
            self.runes_imported_for = Some(champion);
            champion
        })
    }

    pub fn view(&self, mode: GameMode, info: impl Fn(u32) -> ChampionInfo) -> Option<ChampSelect> {
        let session = self.session.as_ref()?;
        let pickable_info = |id| {
            let champion = info(id);
            let locked = self.pickable.as_ref().map_or(champion.locked, |pickable| !pickable.contains(&id));
            champion.with_lock(locked)
        };
        let counter = session
            .position
            .and_then(|position| session.enemies.iter().find_map(|&enemy| Some((enemy, self.counter_picks.get(&(enemy, position))?.as_ref()?))));
        let (lane_opponent, counter_picks) = match counter {
            Some((enemy, picks)) => {
                let pickable = picks.iter().filter(|m| !pickable_info(m.champion.id).locked && !session.enemies.contains(&m.champion.id));
                (Some(info(enemy)), pickable.take(MATCHUPS_SHOWN).cloned().collect())
            }
            None => (None, Vec::new()),
        };
        let champion = session.champion.map(pickable_info);
        let bench: Vec<ChampionInfo> = session.bench.iter().map(|&id| pickable_info(id)).collect();
        Some(ChampSelect { mode, position: session.position, bench_pick: bench_pick(champion.as_ref(), &bench), champion, bench, lane_opponent, counter_picks })
    }
}

fn bench_pick(champion: Option<&ChampionInfo>, bench: &[ChampionInfo]) -> Option<ChampionInfo> {
    let best = bench.iter().filter(|c| c.recommendable).min_by_key(|c| c.rank)?;
    champion.and_then(|c| c.rank).is_none_or(|yours| best.rank < Some(yours)).then(|| best.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{errors::AppError, model::Asset};
    use serde_json::json;

    fn session(champion: Option<u32>, bench: Vec<u32>, enemies: Vec<u32>) -> ChampSelectSession {
        ChampSelectSession { champion, bench, enemies, position: Some(Position::Mid) }
    }

    fn info(id: u32) -> ChampionInfo {
        let rank = HashMap::from([(1, 40), (2, 10), (3, 5), (4, 20)]);
        ChampionInfo::new(Asset { id, name: format!("#{id}"), icon: String::new() }, rank.get(&id).map(|&r| (1, r)), false)
    }

    #[test]
    fn reads_own_cell_bench_and_enemies() {
        let raw = json!({
            "localPlayerCellId": 2,
            "myTeam": [{ "cellId": 1, "championId": 7, "assignedPosition": "" }, { "cellId": 2, "championId": 157, "assignedPosition": "middle" }],
            "theirTeam": [{ "championId": 777 }, { "championId": 0 }],
            "benchChampions": [{ "championId": 22 }, { "championId": 51 }]
        });
        let parsed = parse(&raw).unwrap().unwrap();
        assert_eq!(parsed, session(Some(157), vec![22, 51], vec![777]));
        assert_eq!(parse(&Value::Null).unwrap(), None);
        assert!(matches!(parse(&json!({ "myTeam": [] })), Err(AppError::ClientFormat(_))));
    }

    #[test]
    fn asks_counter_picks_once_per_enemy_and_only_on_the_rift() {
        let mut tracker = ChampSelectTracker::default();
        assert!(tracker.update(session(None, Vec::new(), vec![157]), None, GameMode::Mayhem, false).is_empty());
        assert_eq!(tracker.update(session(None, Vec::new(), vec![157]), None, GameMode::SummonersRift, false), [(157, Position::Mid)]);
        assert_eq!(tracker.update(session(None, Vec::new(), vec![157, 777]), None, GameMode::SummonersRift, false), [(777, Position::Mid)]);
        tracker.set_counter_picks(1, Position::Top, Some(Vec::new()));
        assert!(tracker.counter_picks.is_empty());
    }

    #[test]
    fn imports_runes_once_the_pick_settles() {
        let mut tracker = ChampSelectTracker::default();
        tracker.update(session(Some(103), Vec::new(), Vec::new()), None, GameMode::Mayhem, true);
        let deadline = tracker.rune_deadline().unwrap();
        assert_eq!(tracker.take_due_rune_import(deadline - Duration::from_millis(1)), None);
        assert_eq!(tracker.take_due_rune_import(deadline), Some(103));
        tracker.update(session(Some(103), Vec::new(), Vec::new()), None, GameMode::Mayhem, true);
        assert!(tracker.rune_deadline().is_none());
        tracker.clear();
        assert_eq!(tracker.last_champion(), Some(103));
        assert!(!tracker.is_active());
    }

    #[test]
    fn picks_the_best_takeable_bench_champion_only_when_it_beats_yours() {
        let mut tracker = ChampSelectTracker::default();
        tracker.update(session(Some(1), vec![2, 3, 9], Vec::new()), Some(HashSet::from([1, 2, 9])), GameMode::Mayhem, false);
        let view = tracker.view(GameMode::Mayhem, info).unwrap();
        assert!(view.bench[1].locked && !view.bench[1].recommendable);
        assert_eq!(view.bench_pick.map(|c| c.id), Some(2));

        tracker.update(session(Some(3), vec![2, 4], Vec::new()), None, GameMode::Mayhem, false);
        assert_eq!(tracker.view(GameMode::Mayhem, info).unwrap().bench_pick, None);

        tracker.update(session(None, vec![4, 2], Vec::new()), None, GameMode::Mayhem, false);
        assert_eq!(tracker.view(GameMode::Mayhem, info).unwrap().bench_pick.map(|c| c.id), Some(2));
    }
}
