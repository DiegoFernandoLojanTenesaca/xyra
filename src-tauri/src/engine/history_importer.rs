use super::{EngineEvent, Shared};
use std::{collections::HashMap, sync::Arc, thread, time::Duration};
use xyra_core::{
    errors::Result,
    league::Lcu,
    model::GameMode,
    stats::{self, MatchHistory, Offer, StoredGame},
};

/// Right after signing in the client can take longer than a request allows to bring the history from Riot, so it is
/// asked this many times, this far apart.
const READ_ATTEMPTS: u32 = 3;
const READ_RETRY: Duration = Duration::from_secs(3);
/// The history lists the last 20 games, so choices of older games waiting for theirs will never find it.
const PENDING_GAMES: usize = 20;

#[derive(Default)]
pub struct HistoryImporter {
    pending: bool,
    reading: bool,
    /// The augment choices Xyra saw in each game, until the match history lists that game.
    offers: HashMap<u64, Vec<Offer>>,
}

impl HistoryImporter {
    /// Picks up the choices kept on disk, of games that ended before Xyra last closed.
    pub fn restore(shared: &Shared) -> HistoryImporter {
        let offers = shared.storage.load_pending_offers().unwrap_or_else(|e| {
            shared.log_error("pending offers", e);
            HashMap::new()
        });
        HistoryImporter { offers, ..HistoryImporter::default() }
    }

    /// Waits for the game that just ended, to store the augment choices Xyra saw with it; returns whether there were
    /// choices to keep.
    pub fn expect_game(&mut self, game: Option<u64>, mode: GameMode, offers: Vec<Offer>) -> bool {
        if !mode.has_augments() {
            return false;
        }
        self.pending = true;
        let Some(game) = game.filter(|_| !offers.is_empty()) else { return false };
        self.offers.insert(game, offers);
        while self.offers.len() > PENDING_GAMES
            && let Some(&oldest) = self.offers.keys().min()
        {
            self.offers.remove(&oldest);
        }
        true
    }

    /// Keeps the choices still waiting on disk, so a restart does not lose them.
    pub fn save_offers(&self, shared: &Shared) {
        if let Err(e) = shared.storage.save_pending_offers(&self.offers) {
            shared.log_error("save pending offers", e);
        }
    }

    /// Stops asking the history during champion select; the choices wait for the next import.
    pub fn cancel(&mut self) {
        self.pending = false;
    }

    /// Gives each stored game the choices Xyra saw in it; returns whether any found its game.
    fn attach_offers(&mut self, games: &mut [StoredGame]) -> bool {
        let mut attached = false;
        for game in games.iter_mut().filter(|game| game.offers.is_empty()) {
            if let Some(offers) = self.offers.remove(&game.game_id) {
                game.offers = offers;
                attached = true;
            }
        }
        attached
    }

    pub fn is_pending(&self) -> bool {
        self.pending
    }

    pub fn claim_unowned(&self, account: &str, shared: &Shared) {
        let mut games = shared.games.lock().unwrap();
        if stats::claim_unowned(&mut games, account)
            && let Err(e) = shared.storage.save_games(&games)
        {
            shared.log_error("save stats", e);
        }
    }

    /// Reads the match history in the background, so the engine never waits on the client; it comes back as an event.
    pub fn read(&mut self, lcu: &Lcu, account: &str, shared: &Arc<Shared>) {
        if self.reading {
            return;
        }
        self.reading = true;
        let (lcu, account, shared) = (lcu.clone(), account.to_owned(), Arc::clone(shared));
        thread::spawn(move || {
            let mut history = stats::read_history(&lcu);
            for _ in 1..READ_ATTEMPTS {
                if history.is_ok() {
                    break;
                }
                thread::sleep(READ_RETRY);
                history = stats::read_history(&lcu);
            }
            shared.send(EngineEvent::History { account, history });
        });
    }

    /// Stores the account's new games of a history just read; returns whether any stored game changed.
    pub fn store(&mut self, account: &str, history: Result<MatchHistory>, shared: &Shared) -> bool {
        self.reading = false;
        let history = match history {
            Ok(history) => history,
            Err(e) => {
                shared.log_error("match history", e);
                return false;
            }
        };
        let mut games = shared.games.lock().unwrap();
        let added = stats::add_new_games(history, account, &mut games);
        let attached = self.attach_offers(&mut games);
        if attached {
            self.save_offers(shared);
        }
        if added == 0 && !attached {
            return false;
        }
        if added > 0 {
            self.pending = false;
        }
        if let Err(e) = shared.storage.save_games(&games) {
            shared.log_error("save stats", e);
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn game(game_id: u64) -> StoredGame {
        StoredGame {
            game_id,
            account: "me".into(),
            date: String::new(),
            champion: 1,
            mode: GameMode::Mayhem,
            win: true,
            augments: Vec::new(),
            offers: Vec::new(),
        }
    }

    #[test]
    fn keeps_each_game_choices_until_its_game_is_stored() {
        let mut importer = HistoryImporter::default();
        let offer = |best| vec![Offer { cards: vec![1, 2, 3], best: Some(best) }];
        importer.expect_game(Some(10), GameMode::Mayhem, offer(1));
        importer.cancel();
        importer.expect_game(Some(11), GameMode::Mayhem, offer(2));
        let mut games = vec![game(11), game(9)];
        assert!(importer.attach_offers(&mut games));
        assert_eq!((games[0].offers[0].best, games[1].offers.len()), (Some(2), 0));
        let mut later = vec![game(12), game(11), game(10)];
        assert!(importer.attach_offers(&mut later));
        assert_eq!(later[2].offers[0].best, Some(1));
        assert!(!importer.expect_game(Some(13), GameMode::SummonersRift, offer(3)));
        assert!(!importer.is_pending() || importer.offers.is_empty());
    }

    #[test]
    fn keeps_only_the_games_the_history_can_still_list() {
        let mut importer = HistoryImporter::default();
        for game in 1..=PENDING_GAMES as u64 + 2 {
            assert!(importer.expect_game(Some(game), GameMode::Mayhem, vec![Offer { cards: vec![1, 2, 3], best: None }]));
        }
        assert_eq!(importer.offers.len(), PENDING_GAMES);
        assert!(!importer.offers.contains_key(&1) && !importer.offers.contains_key(&2) && importer.offers.contains_key(&3));
    }
}
