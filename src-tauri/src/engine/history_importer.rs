use super::Shared;
use xyra_core::{league::Lcu, model::GameMode, stats};

#[derive(Default)]
pub struct HistoryImporter {
    pending: bool,
}

impl HistoryImporter {
    pub fn expect_games_of(&mut self, mode: GameMode) {
        self.pending = mode.has_augments();
    }

    pub fn cancel(&mut self) {
        self.pending = false;
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

    pub fn import(&mut self, lcu: &Lcu, account: &str, shared: &Shared) -> bool {
        let mut games = shared.games.lock().unwrap();
        let added = match stats::import_recent(lcu, account, &mut games) {
            Ok(added) => added,
            Err(e) => {
                shared.log_error("match history", e);
                0
            }
        };
        if added > 0 {
            self.pending = false;
            if let Err(e) = shared.storage.save_games(&games) {
                shared.log_error("save stats", e);
            }
        }
        added > 0
    }
}
