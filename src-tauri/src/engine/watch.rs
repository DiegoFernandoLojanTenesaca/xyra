use super::{EngineEvent, Shared};
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::sync::Arc;
use xyra_core::league::Installation;

/// Tells the engine when the client starts or stops (its lockfile) and when the game settings file changes.
pub fn start(shared: &Arc<Shared>) -> notify::Result<RecommendedWatcher> {
    let target = Arc::clone(shared);
    let mut watcher = notify::recommended_watcher(move |result: notify::Result<Event>| match result {
        Ok(event) => {
            for name in event.paths.iter().filter_map(|path| path.file_name()?.to_str()) {
                if Installation::is_lockfile(name) {
                    target.send(EngineEvent::LockfileChanged);
                } else if Installation::is_game_config(name) {
                    target.send(EngineEvent::GameConfigChanged);
                }
            }
        }
        Err(error) => target.log_error("file watcher", error),
    })?;
    watcher.watch(&shared.installation.dir, RecursiveMode::NonRecursive)?;
    watcher.watch(&shared.installation.config_dir(), RecursiveMode::NonRecursive)?;
    Ok(watcher)
}
