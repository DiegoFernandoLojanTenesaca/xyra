use super::Shared;
use std::{
    collections::{HashMap, HashSet},
    time::{Duration, Instant},
};
use xyra_core::{
    champ_select::ChampSelectSession,
    model::{ChampSelect, ChampionInfo, GameMode, Matchup, Position},
    opgg::MATCHUPS_SHOWN,
};

/// How long a pick must stay before its runes are imported.
const AUTO_RUNES_SETTLE: Duration = Duration::from_secs(3);

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

    fn is_locked(&self, shared: &Shared, champion: u32) -> bool {
        self.pickable.as_ref().map_or_else(|| !shared.is_available(champion), |pickable| !pickable.contains(&champion))
    }

    pub fn view(&self, shared: &Shared, mode: GameMode) -> Option<ChampSelect> {
        let session = self.session.as_ref()?;
        let info = |id| ChampionInfo { locked: self.is_locked(shared, id), ..shared.champion_info(id) };
        let counter = session
            .position
            .and_then(|position| session.enemies.iter().find_map(|&enemy| Some((enemy, self.counter_picks.get(&(enemy, position))?.as_ref()?))));
        let (lane_opponent, counter_picks) = match counter {
            Some((enemy, picks)) => {
                let pickable = picks.iter().filter(|m| !self.is_locked(shared, m.champion.id) && !session.enemies.contains(&m.champion.id));
                (Some(shared.champion_info(enemy)), pickable.take(MATCHUPS_SHOWN).cloned().collect())
            }
            None => (None, Vec::new()),
        };
        Some(ChampSelect {
            mode,
            position: session.position,
            champion: session.champion.map(info),
            bench: session.bench.iter().map(|&id| info(id)).collect(),
            lane_opponent,
            counter_picks,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session(champion: Option<u32>, enemies: Vec<u32>) -> ChampSelectSession {
        ChampSelectSession { champion, bench: Vec::new(), enemies, position: Some(Position::Mid) }
    }

    #[test]
    fn asks_counter_picks_once_per_enemy_and_only_on_the_rift() {
        let mut tracker = ChampSelectTracker::default();
        assert!(tracker.update(session(None, vec![157]), None, GameMode::Mayhem, false).is_empty());
        assert_eq!(tracker.update(session(None, vec![157]), None, GameMode::SummonersRift, false), [(157, Position::Mid)]);
        assert_eq!(tracker.update(session(None, vec![157, 777]), None, GameMode::SummonersRift, false), [(777, Position::Mid)]);
        tracker.set_counter_picks(1, Position::Top, Some(Vec::new()));
        assert!(tracker.counter_picks.is_empty());
    }

    #[test]
    fn imports_runes_once_the_pick_settles() {
        let mut tracker = ChampSelectTracker::default();
        tracker.update(session(Some(103), Vec::new()), None, GameMode::Mayhem, true);
        let deadline = tracker.rune_deadline().unwrap();
        assert_eq!(tracker.take_due_rune_import(deadline - Duration::from_millis(1)), None);
        assert_eq!(tracker.take_due_rune_import(deadline), Some(103));
        tracker.update(session(Some(103), Vec::new()), None, GameMode::Mayhem, true);
        assert!(tracker.rune_deadline().is_none());
        tracker.clear();
        assert_eq!(tracker.last_champion(), Some(103));
        assert!(!tracker.is_active());
    }
}
