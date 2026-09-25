use crate::model::GameMode;
use serde::Deserialize;
use serde_json::Value;

pub const SESSION: &str = "/lol-gameflow/v1/session";
const PLAYER_LISTS: [&str; 3] = ["teamOne", "teamTwo", "playerChampionSelections"];

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

pub fn parse(session: &Value, account: Option<&str>) -> Gameflow {
    let data = &session["gameData"];
    let champion = account.and_then(|me| {
        PLAYER_LISTS
            .iter()
            .flat_map(|list| data[*list].as_array().into_iter().flatten())
            .filter(|player| player["puuid"].as_str() == Some(me))
            .find_map(|player| player["championId"].as_u64().filter(|&id| id > 0))
            .map(|id| id as u32)
    });
    Gameflow {
        phase: GameflowPhase::deserialize(&session["phase"]).unwrap_or(GameflowPhase::Unknown),
        mode: GameMode::from_client(data["queue"]["gameMode"].as_str().unwrap_or_default()),
        champion,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn reads_phase_mode_and_own_champion() {
        let session = json!({
            "phase": "InProgress",
            "gameData": {
                "queue": { "gameMode": "KIWI" },
                "teamOne": [{ "puuid": "other", "championId": 1 }, { "puuid": "me", "championId": 103 }],
                "teamTwo": []
            }
        });
        let flow = parse(&session, Some("me"));
        assert_eq!(flow, Gameflow { phase: GameflowPhase::InProgress, mode: GameMode::Mayhem, champion: Some(103) });
        assert!(flow.phase.is_in_game());
        assert_eq!(parse(&session, None).champion, None);
        assert_eq!(parse(&json!({ "phase": "Brand new" }), None).phase, GameflowPhase::Unknown);
    }
}
