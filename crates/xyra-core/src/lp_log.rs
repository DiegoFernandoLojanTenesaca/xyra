use crate::{
    catalog::{Catalog, named},
    model::Asset,
    profile::{RankedQueue, Standing},
};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// The log keeps this many ranked games, the oldest leaving first.
const KEPT: usize = 200;

/// A ranked game and where the player stood before and after it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LpChange {
    pub account: String,
    pub game_id: Option<u64>,
    /// Unix seconds.
    pub ended_at: u64,
    pub queue: RankedQueue,
    pub champion: Option<u32>,
    pub before: Standing,
    pub after: Standing,
}

/// A ranked game of the log as the app shows it.
#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct LpGame {
    /// Unix seconds.
    #[ts(type = "number")]
    pub ended_at: u64,
    pub queue: RankedQueue,
    pub champion: Option<Asset>,
    pub win: bool,
    /// LP gained, negative when lost.
    #[ts(type = "number")]
    pub delta: i64,
    pub after: Standing,
}

/// Adds a finished game to the log, once, dropping the oldest ones past its size.
pub fn record(log: &mut Vec<LpChange>, change: LpChange) {
    if change.game_id.is_some() && log.iter().any(|known| known.game_id == change.game_id) {
        return;
    }
    log.push(change);
    let extra = log.len().saturating_sub(KEPT);
    log.drain(..extra);
}

/// The account's ranked games, newest first.
pub fn games(log: &[LpChange], account: Option<&str>, catalog: &Catalog) -> Vec<LpGame> {
    log.iter()
        .rev()
        .filter(|change| account.is_none_or(|account| change.account == account))
        .map(|change| LpGame {
            ended_at: change.ended_at,
            queue: change.queue,
            champion: change.champion.map(|id| named(&catalog.champions, id)),
            win: change.after.wins > change.before.wins,
            delta: change.after.lp_since(&change.before),
            after: change.after.clone(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profile::LeagueTier;

    fn change(game_id: u64, account: &str, before_lp: i64, after_lp: i64, won: bool) -> LpChange {
        let standing = |lp, wins| Standing { tier: LeagueTier::Gold, division: "II".into(), lp, wins, losses: 0 };
        LpChange {
            account: account.into(),
            game_id: Some(game_id),
            ended_at: game_id,
            queue: RankedQueue::Solo,
            champion: None,
            before: standing(before_lp, 5),
            after: standing(after_lp, if won { 6 } else { 5 }),
        }
    }

    #[test]
    fn keeps_each_game_once_newest_first_for_its_account() {
        let mut log = Vec::new();
        record(&mut log, change(1, "me", 10, 31, true));
        record(&mut log, change(1, "me", 10, 31, true));
        record(&mut log, change(2, "other", 50, 30, false));
        record(&mut log, change(3, "me", 31, 12, false));
        let mine = games(&log, Some("me"), &Catalog::default());
        assert_eq!(mine.iter().map(|g| (g.ended_at, g.delta, g.win)).collect::<Vec<_>>(), [(3, -19, false), (1, 21, true)]);
        for game_id in 4..=KEPT as u64 + 5 {
            record(&mut log, change(game_id, "me", 0, 20, true));
        }
        assert_eq!(log.len(), KEPT);
        assert!(log.iter().all(|kept| kept.game_id != Some(1)));
    }
}
