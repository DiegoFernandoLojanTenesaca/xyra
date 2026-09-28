use super::{EngineEvent, Shared};
use std::{
    sync::Arc,
    thread,
    time::{Duration, Instant},
};
use xyra_core::{
    errors::AppError,
    live_game,
    model::{Build, BuildMode, GameTips, LiveStats, Position},
    web::Client,
};

/// The game's local API has no events, so it is read on this interval while a game with builds runs.
const POLL_EVERY: Duration = Duration::from_secs(2);

/// Follows the player's numbers in the running game and suggests the next skill and item from the champion's build.
pub struct GameTipsReader {
    http: Client,
    game: Option<FollowedGame>,
}

struct FollowedGame {
    champion: u32,
    build: Option<Arc<Build>>,
    next_poll: Instant,
    tips: Option<GameTips>,
    live: Option<LiveStats>,
    /// A read of the game's API is under way.
    reading: bool,
}

impl GameTipsReader {
    pub fn new() -> GameTipsReader {
        GameTipsReader { http: live_game::client(), game: None }
    }

    pub fn next_poll(&self) -> Option<Instant> {
        self.game.as_ref().map(|game| game.next_poll)
    }

    pub fn tips(&self) -> Option<&GameTips> {
        self.game.as_ref()?.tips.as_ref()
    }

    pub fn live(&self) -> Option<&LiveStats> {
        self.game.as_ref()?.live.as_ref()
    }

    /// Starts on a new champion, fetching its build in the background, and stops when there is no game to follow.
    pub fn follow(&mut self, target: Option<(u32, BuildMode, Option<Position>)>, shared: &Arc<Shared>) {
        let Some((champion, mode, position)) = target else {
            self.game = None;
            return;
        };
        if self.game.as_ref().is_some_and(|game| game.champion == champion) {
            return;
        }
        self.game = Some(FollowedGame { champion, build: None, next_poll: Instant::now() + POLL_EVERY, tips: None, live: None, reading: false });
        let shared = Arc::clone(shared);
        thread::spawn(move || match shared.fetch_build(champion, mode, position) {
            Ok(build) => shared.send(EngineEvent::GameBuild(Box::new(build))),
            Err(e) => shared.log_error("OP.GG build for game tips", e),
        });
    }

    pub fn set_build(&mut self, build: Build) {
        if let Some(game) = self.game.as_mut().filter(|game| game.champion == build.champion) {
            game.build = Some(Arc::new(build));
        }
    }

    /// Reads the game's API in the background: while the game loads it refuses for seconds, which would hold the engine
    /// and the card reading; the numbers and tips come back as `EngineEvent::GameRead`.
    pub fn poll(&mut self, shared: &Arc<Shared>) {
        let Some(game) = self.game.as_mut() else { return };
        game.next_poll = Instant::now() + POLL_EVERY;
        if game.reading {
            return;
        }
        game.reading = true;
        let (http, shared, champion, build) = (self.http.clone(), Arc::clone(shared), game.champion, game.build.clone());
        thread::spawn(move || {
            let read = match live_game::read(&http) {
                Ok(player) => Some((player.live.clone(), build.map(|build| live_game::tips(&player, &build, &shared.catalog())))),
                Err(e @ AppError::ClientFormat(_)) => {
                    shared.log_error("game tips", e);
                    None
                }
                Err(_) => None,
            };
            shared.send(EngineEvent::GameRead { champion, read });
        });
    }

    /// Keeps what a read found; a failed one leaves the last numbers and tips shown.
    pub fn set_read(&mut self, champion: u32, read: Option<(LiveStats, Option<GameTips>)>) {
        let Some(game) = self.game.as_mut().filter(|game| game.champion == champion) else { return };
        game.reading = false;
        if let Some((live, tips)) = read {
            game.live = Some(live);
            if tips.is_some() {
                game.tips = tips;
            }
        }
    }
}
