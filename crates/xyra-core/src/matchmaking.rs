use crate::{errors::Result, league::Lcu};
use reqwest::Method;
use serde::Deserialize;

const READY_CHECK: &str = "/lol-matchmaking/v1/ready-check";
const ACCEPT: &str = "/lol-matchmaking/v1/ready-check/accept";

#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
enum ReadyCheckState {
    InProgress,
    #[serde(other)]
    Other,
}

#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
enum PlayerResponse {
    None,
    #[serde(other)]
    Answered,
}

#[derive(Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReadyCheck {
    state: ReadyCheckState,
    player_response: PlayerResponse,
}

impl ReadyCheck {
    /// The queue popped and the player has neither accepted nor declined yet.
    fn is_waiting(&self) -> bool {
        self.state == ReadyCheckState::InProgress && self.player_response == PlayerResponse::None
    }
}

/// Accepts the found match unless the player already answered it; returns whether it accepted.
pub fn accept_if_waiting(lcu: &Lcu) -> Result<bool> {
    let check: ReadyCheck = lcu.get_as(READY_CHECK)?;
    if !check.is_waiting() {
        return Ok(false);
    }
    lcu.request(Method::POST, ACCEPT, None)?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::league::parse;
    use serde_json::json;

    fn check(state: &str, response: &str) -> ReadyCheck {
        parse(READY_CHECK, &json!({ "state": state, "playerResponse": response, "timer": 3.0 })).unwrap()
    }

    #[test]
    fn waits_only_for_an_unanswered_ready_check() {
        assert!(check("InProgress", "None").is_waiting());
        assert!(!check("InProgress", "Declined").is_waiting());
        assert!(!check("InProgress", "Accepted").is_waiting());
        assert!(!check("EveryoneReady", "None").is_waiting());
        assert!(!check("Invalid", "None").is_waiting());
    }
}
