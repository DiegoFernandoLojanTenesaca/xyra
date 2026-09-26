use super::{EngineEvent, Shared};
use std::{
    sync::Arc,
    thread,
    time::{Duration, Instant},
};
use xyra_core::{
    errors::AppError,
    live_game,
    model::{Build, BuildMode, GameTips, Position},
    web::Client,
};

/// The game's local API has no events, so it is read on this interval while a game with builds runs.
const POLL_EVERY: Duration = Duration::from_secs(2);

/// Suggests the next skill and item of the running game from the champion's build.
pub struct GameTipsReader {
    http: Client,
    game: Option<FollowedGame>,
}

struct FollowedGame {
    champion: u32,
    build: Option<Build>,
    next_poll: Instant,
    tips: Option<GameTips>,
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

    /// Starts on a new champion, fetching its build in the background, and stops when there is no game to follow.
    pub fn follow(&mut self, target: Option<(u32, BuildMode, Option<Position>)>, shared: &Arc<Shared>) {
        let Some((champion, mode, position)) = target else {
            self.game = None;
            return;
        };
        if self.game.as_ref().is_some_and(|game| game.champion == champion) {
            return;
        }
        self.game = Some(FollowedGame { champion, build: None, next_poll: Instant::now() + POLL_EVERY, tips: None });
        let shared = Arc::clone(shared);
        thread::spawn(move || match shared.fetch_build(champion, mode, position) {
            Ok(build) => shared.send(EngineEvent::GameBuild(Box::new(build))),
            Err(e) => shared.log_error("OP.GG build for game tips", e),
        });
    }

    pub fn set_build(&mut self, build: Build) {
        if let Some(game) = self.game.as_mut().filter(|game| game.champion == build.champion) {
            game.build = Some(build);
        }
    }

    pub fn poll(&mut self, shared: &Shared) {
        let Some(game) = self.game.as_mut() else { return };
        game.next_poll = Instant::now() + POLL_EVERY;
        let Some(build) = &game.build else { return };
        match live_game::read(&self.http) {
            Ok(player) => game.tips = Some(live_game::tips(&player, build, &shared.catalog())),
            Err(e @ AppError::ClientFormat(_)) => shared.log_error("game tips", e),
            Err(_) => {}
        }
    }
}
