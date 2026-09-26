use crate::{errors::Result, league::parse as parse_client, model::GameMode};
use serde::Deserialize;
use serde_json::Value;

pub const SESSION: &str = "/lol-gameflow/v1/session";

#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
pub enum GameflowPhase {
    None,
    Lobby,
    Matchmaking,
    CheckedIntoTournament,
    ReadyCheck,
    ChampSelect,
    GameStart,
    FailedToLaunch,
    InProgress,
    Reconnect,
    WaitingForStats,
    PreEndOfGame,
    EndOfGame,
    TerminatedInError,
    #[serde(other)]
    Unknown,
}

impl GameflowPhase {
    pub fn is_in_game(self) -> bool {
        matches!(self, GameflowPhase::GameStart | GameflowPhase::InProgress | GameflowPhase::Reconnect)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Gameflow {
    pub phase: GameflowPhase,
    pub mode: GameMode,
    /// The account's champion in the current game, once the client lists it.
    pub champion: Option<u32>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Session {
    phase: GameflowPhase,
    game_data: GameData,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GameData {
    queue: Queue,
    #[serde(default)]
    team_one: Vec<Player>,
    #[serde(default)]
    team_two: Vec<Player>,
    #[serde(default)]
    player_champion_selections: Vec<Player>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Queue {
    game_mode: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Player {
    puuid: Option<String>,
    #[serde(default)]
    champion_id: u32,
}

pub fn parse(session: &Value, account: Option<&str>) -> Result<Gameflow> {
    let session: Session = parse_client(SESSION, session)?;
    let data = session.game_data;
    let players = data.team_one.iter().chain(&data.team_two).chain(&data.player_champion_selections);
    let champion = account.and_then(|me| players.filter(|p| p.puuid.as_deref() == Some(me)).map(|p| p.champion_id).find(|&id| id > 0));
    Ok(Gameflow { phase: session.phase, mode: GameMode::from_client(&data.queue.game_mode), champion })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::errors::AppError;
    use serde_json::json;

    fn session(phase: &str) -> Value {
        json!({
            "phase": phase,
            "gameData": {
                "queue": { "gameMode": "KIWI" },
                "teamOne": [{ "puuid": "other", "championId": 1 }, { "puuid": "me", "championId": 103 }],
                "teamTwo": [],
                "playerChampionSelections": [{ "championId": 1 }]
            }
        })
    }

    #[test]
    fn reads_phase_mode_and_own_champion() {
        let flow = parse(&session("InProgress"), Some("me")).unwrap();
        assert_eq!(flow, Gameflow { phase: GameflowPhase::InProgress, mode: GameMode::Mayhem, champion: Some(103) });
        assert!(flow.phase.is_in_game());
        assert_eq!(parse(&session("InProgress"), None).unwrap().champion, None);
        assert_eq!(parse(&session("Brand new"), None).unwrap().phase, GameflowPhase::Unknown);
        assert!(matches!(parse(&json!({ "phase": "Lobby" }), None), Err(AppError::ClientFormat(_))));
    }
}
