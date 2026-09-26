use super::Shared;
use xyra_core::{
    league::Lcu,
    model::GameMode,
    stats::{self, Offer},
};

#[derive(Default)]
pub struct HistoryImporter {
    pending: bool,
    offers: Vec<Offer>,
}

impl HistoryImporter {
    /// Waits for the game that just ended, to store the augment choices Xyra saw with it.
    pub fn expect_games_of(&mut self, mode: GameMode, offers: Vec<Offer>) {
        self.pending = mode.has_augments();
        self.offers = offers;
    }

    pub fn cancel(&mut self) {
        self.pending = false;
        self.offers.clear();
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

    /// Reads the match history before locking the stored games, so the stats screen never waits on the client.
    pub fn import(&mut self, lcu: &Lcu, account: &str, shared: &Shared) -> bool {
        let history = match stats::read_history(lcu) {
            Ok(history) => history,
            Err(e) => {
                shared.log_error("match history", e);
                return false;
            }
        };
        let mut games = shared.games.lock().unwrap();
        if stats::add_new_games(history, account, &mut games) == 0 {
            return false;
        }
        self.pending = false;
        if let Some(newest) = games.first_mut() {
            newest.offers = std::mem::take(&mut self.offers);
        }
        if let Err(e) = shared.storage.save_games(&games) {
            shared.log_error("save stats", e);
        }
        true
    }
}
