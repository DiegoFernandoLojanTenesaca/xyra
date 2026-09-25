use crate::{errors::Result, league::Lcu, model::Position};
use serde_json::Value;
use std::collections::HashSet;

pub const SESSION: &str = "/lol-champ-select/v1/session";
const PICKABLE: &str = "/lol-champ-select/v1/pickable-champion-ids";

#[derive(Clone, Debug, PartialEq)]
pub struct ChampSelectSession {
    pub champion: Option<u32>,
    pub bench: Vec<u32>,
    pub enemies: Vec<u32>,
    pub position: Option<Position>,
}

fn champion_ids(list: &Value) -> Vec<u32> {
    list.as_array().into_iter().flatten().filter_map(|entry| entry["championId"].as_u64()).filter(|&id| id > 0).map(|id| id as u32).collect()
}

pub fn parse(session: &Value) -> Option<ChampSelectSession> {
    let me = session["localPlayerCellId"].as_i64()?;
    let cell = session["myTeam"].as_array()?.iter().find(|member| member["cellId"].as_i64() == Some(me))?;
    Some(ChampSelectSession {
        champion: cell["championId"].as_u64().filter(|&id| id > 0).map(|id| id as u32),
        bench: champion_ids(&session["benchChampions"]),
        enemies: champion_ids(&session["theirTeam"]),
        position: Position::from_client(cell["assignedPosition"].as_str().unwrap_or_default()),
    })
}

/// Champions the player can pick or take from the bench in the current champion select.
pub fn read_pickable(lcu: &Lcu) -> Result<HashSet<u32>> {
    Ok(serde_json::from_value(lcu.get(PICKABLE)?)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn reads_own_cell_bench_and_enemies() {
        let session = json!({
            "localPlayerCellId": 2,
            "myTeam": [{ "cellId": 1, "championId": 7 }, { "cellId": 2, "championId": 157, "assignedPosition": "middle" }],
            "theirTeam": [{ "championId": 777 }, { "championId": 0 }],
            "benchChampions": [{ "championId": 22 }, { "championId": 51 }]
        });
        let parsed = parse(&session).unwrap();
        assert_eq!(parsed, ChampSelectSession { champion: Some(157), bench: vec![22, 51], enemies: vec![777], position: Some(Position::Mid) });
        assert!(parse(&json!({})).is_none());
    }
}
