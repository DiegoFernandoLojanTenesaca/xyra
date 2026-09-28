use crate::commands::App;
use discord_rich_presence::{DiscordIpc, DiscordIpcClient, activity};
use std::{
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tauri::{AppHandle, Manager};
use xyra_core::{
    config::Config,
    i18n,
    model::{EngineState, Phase},
};

/// The Discord application "Xyra"; its name is what Discord shows after "Playing". Empty turns the presence off.
const APP_ID: &str = "1554231605860499517";
/// Discord takes one update every 15 seconds at most.
const REFRESH: Duration = Duration::from_secs(15);
/// The logo uploaded to the Discord application's art assets.
const LOGO: &str = "xyra";

/// What Xyra shows on the player's Discord profile.
#[derive(Clone, Debug, PartialEq)]
struct Presence {
    /// What the player is doing, which also restarts the elapsed time when it changes.
    doing: Doing,
    details: String,
    state: Option<String>,
    image: Option<String>,
    image_text: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Doing {
    Playing,
    Selecting,
    Searching,
    InLobby,
    Idle,
}

/// The presence for the player's state; None hides it.
fn presence(state: &EngineState, config: &Config) -> Option<Presence> {
    if !config.discord_presence {
        return None;
    }
    let t = |key: &str| i18n::t(&state.language, key);
    let with = |key: &str, values: &[(&str, &str)]| i18n::t_with(&state.language, key, values);
    let champion = state.game.as_ref().and_then(|game| game.champion.as_ref()).or(state.champ_select.as_ref().and_then(|select| select.champion.as_ref()));
    let details = config.discord_details;
    let (doing, text, line) = match state.phase {
        Phase::InGame => {
            let game = state.game.as_ref()?;
            let mode = t(&format!("common:modes.{}", i18n::variant_key(game.mode)));
            let text = match champion {
                Some(champion) if details => with("discord:playing", &[("mode", &mode), ("champion", &champion.name)]),
                _ => mode,
            };
            let line = state.live.as_ref().filter(|_| details).map(|live| {
                let (kills, deaths, assists, level) = (live.kills.to_string(), live.deaths.to_string(), live.assists.to_string(), live.level.to_string());
                with("discord:stats", &[("kills", &kills), ("deaths", &deaths), ("assists", &assists), ("level", &level)])
            });
            (Doing::Playing, text, line)
        }
        Phase::ChampSelect => (Doing::Selecting, t("discord:selecting"), champion.filter(|_| details).map(|champion| champion.name.clone())),
        _ if state.lobby.as_ref().is_some_and(|lobby| lobby.searching) => (Doing::Searching, t("discord:searching"), None),
        _ if state.lobby.is_some() => (Doing::InLobby, t("discord:lobby"), None),
        _ if config.discord_idle => (Doing::Idle, t("discord:idle"), None),
        _ => return None,
    };
    let shown = champion.filter(|_| details && matches!(doing, Doing::Playing | Doing::Selecting));
    Some(Presence {
        doing,
        details: text,
        state: line,
        image: shown.map(|champion| champion.icon.clone()).filter(|icon| !icon.is_empty()),
        image_text: shown.map(|champion| champion.name.clone()),
    })
}

/// Keeps the Discord presence in step with Xyra, apart from everything else: without Discord or with it off, it
/// simply waits.
pub fn start(app: AppHandle) {
    if APP_ID.is_empty() {
        return;
    }
    thread::spawn(move || {
        let mut client: Option<DiscordIpcClient> = None;
        let mut shown: Option<Presence> = None;
        let mut since = (Doing::Idle, now());
        loop {
            let shared = app.state::<App>();
            let wanted = presence(&shared.state.lock().unwrap(), &shared.config());
            if let Some(presence) = &wanted
                && presence.doing != since.0
            {
                since = (presence.doing, now());
            }
            if wanted != shown {
                shown = match &wanted {
                    Some(presence) => send(&mut client, presence, since.1).then_some(presence.clone()),
                    None => {
                        if let Some(mut connected) = client.take() {
                            let _ = connected.clear_activity();
                            let _ = connected.close();
                        }
                        None
                    }
                };
            }
            thread::sleep(REFRESH);
        }
    });
}

fn now() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |elapsed| elapsed.as_secs() as i64)
}

/// Sends the presence, connecting first when needed; false when Discord is not there, to try again later.
fn send(client: &mut Option<DiscordIpcClient>, presence: &Presence, since: i64) -> bool {
    if client.is_none() {
        let mut connecting = DiscordIpcClient::new(APP_ID);
        if connecting.connect().is_err() {
            return false;
        }
        *client = Some(connecting);
    }
    let Some(connected) = client.as_mut() else { return false };
    let mut assets = activity::Assets::new().small_image(LOGO).small_text("Xyra");
    if let Some(image) = &presence.image {
        assets = assets.large_image(image);
    } else {
        assets = assets.large_image(LOGO);
    }
    if let Some(text) = &presence.image_text {
        assets = assets.large_text(text);
    }
    let mut activity = activity::Activity::new().details(&presence.details).assets(assets).timestamps(activity::Timestamps::new().start(since));
    if let Some(state) = &presence.state {
        activity = activity.state(state);
    }
    if connected.set_activity(activity).is_err() {
        let _ = connected.close();
        *client = None;
        return false;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use xyra_core::model::{Asset, ChampionInfo, CurrentGame, GameMode, LiveStats};

    fn state(phase: Phase) -> EngineState {
        EngineState {
            phase,
            account: None,
            game: None,
            champ_select: None,
            build_mode: None,
            cards: Vec::new(),
            rounds: Vec::new(),
            borderless: None,
            client_locale: "es_MX".into(),
            language: "es".into(),
            ocr_language: None,
            version: "1.0.0".into(),
            tips: None,
            live: None,
            ready_check: false,
            lobby: None,
        }
    }

    fn jinx() -> ChampionInfo {
        ChampionInfo::new(Asset { id: 222, name: "Jinx".into(), icon: "https://cdn/jinx.png".into() }, None, false)
    }

    #[test]
    fn shows_the_game_and_hides_what_the_player_turned_off() {
        let mut config = Config { discord_presence: true, ..Config::default() };
        let mut playing = state(Phase::InGame);
        playing.game = Some(CurrentGame { mode: GameMode::Mayhem, champion: Some(jinx()) });
        playing.live = Some(LiveStats { kills: 5, deaths: 2, assists: 7, farm: 40, gold: 900, level: 11 });
        let shown = presence(&playing, &config).unwrap();
        assert_eq!((shown.doing, shown.details.as_str(), shown.state.as_deref()), (Doing::Playing, "ARAM: Caos · Jinx", Some("5/2/7 · Nivel 11")));
        assert_eq!(shown.image.as_deref(), Some("https://cdn/jinx.png"));
        config.discord_details = false;
        let plain = presence(&playing, &config).unwrap();
        assert_eq!((plain.details.as_str(), plain.state, plain.image), ("ARAM: Caos", None, None));
        assert_eq!(presence(&state(Phase::Client), &config).map(|p| p.doing), Some(Doing::Idle));
        config.discord_idle = false;
        assert!(presence(&state(Phase::Client), &config).is_none());
        config.discord_presence = false;
        assert!(presence(&playing, &config).is_none());
    }
}
