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

#[derive(Default)]
pub struct HistoryImporter {
    pending: bool,
    reading: bool,
    /// The augment choices Xyra saw in each game, until the match history lists that game.
    offers: HashMap<u64, Vec<Offer>>,
}

impl HistoryImporter {
    /// Waits for the game that just ended, to store the augment choices Xyra saw with it.
    pub fn expect_game(&mut self, game: Option<u64>, mode: GameMode, offers: Vec<Offer>) {
        if !mode.has_augments() {
            return;
        }
        self.pending = true;
        if let Some(game) = game.filter(|_| !offers.is_empty()) {
            self.offers.insert(game, offers);
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
        importer.expect_game(Some(13), GameMode::SummonersRift, offer(3));
        assert!(!importer.is_pending() || importer.offers.is_empty());
    }
}
