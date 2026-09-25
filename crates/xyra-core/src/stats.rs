use crate::{
    catalog::{Catalog, NamedAssets, named},
    errors::Result,
    i18n,
    league::Lcu,
    model::{Asset, GameMode},
};
use serde::{Deserialize, Serialize};
use std::{cmp::Reverse, collections::HashMap, fs, path::Path};
use ts_rs::TS;

const MATCH_HISTORY: &str = "/lol-match-history/v1/products/lol/current-summoner/matches?begIndex=0&endIndex=19";
pub const END_OF_GAME: &str = "/lol-end-of-game/v1/eog-stats-block";
const MIN_AUGMENT_GAMES: u32 = 2;
const TOP_AUGMENTS: usize = 15;
const RECENT_GAMES: usize = 10;
const UTF8_BOM: char = '\u{feff}';
const CSV_LINE_END: &str = "\r\n";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StoredGame {
    pub game_id: u64,
    /// PUUID of the account that played it.
    #[serde(default)]
    pub account: String,
    /// ISO 8601, UTC.
    #[serde(alias = "fecha")]
    pub date: String,
    #[serde(alias = "campeon")]
    pub champion: u32,
    #[serde(alias = "modo")]
    pub mode: GameMode,
    #[serde(alias = "victoria")]
    pub win: bool,
    #[serde(alias = "aumentos")]
    pub augments: Vec<u32>,
}

#[derive(Deserialize)]
struct MatchHistory {
    games: HistoryPage,
}

#[derive(Deserialize)]
struct HistoryPage {
    games: Vec<HistoryGame>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct HistoryGame {
    game_id: u64,
    game_mode: String,
    game_creation_date: String,
    participants: Vec<Participant>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Participant {
    champion_id: u32,
    stats: ParticipantStats,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ParticipantStats {
    win: bool,
    player_augment_1: Option<u32>,
    player_augment_2: Option<u32>,
    player_augment_3: Option<u32>,
    player_augment_4: Option<u32>,
    player_augment_5: Option<u32>,
    player_augment_6: Option<u32>,
}

impl ParticipantStats {
    fn augments(&self) -> Vec<u32> {
        [self.player_augment_1, self.player_augment_2, self.player_augment_3, self.player_augment_4, self.player_augment_5, self.player_augment_6]
            .into_iter()
            .flatten()
            .filter(|&augment| augment > 0)
            .collect()
    }
}

/// Adds the ARAM: Mayhem and Arena games of the account's recent match history that are not stored yet.
pub fn import_recent(lcu: &Lcu, account: &str, games: &mut Vec<StoredGame>) -> Result<usize> {
    Ok(add_new_games(lcu.get_as(MATCH_HISTORY)?, account, games))
}

fn add_new_games(history: MatchHistory, account: &str, games: &mut Vec<StoredGame>) -> usize {
    let before = games.len();
    for game in history.games.games {
        let mode = GameMode::from_client(&game.game_mode);
        let Some(player) = game.participants.first() else { continue };
        if !mode.has_augments() || games.iter().any(|g| g.game_id == game.game_id) {
            continue;
        }
        games.push(StoredGame {
            game_id: game.game_id,
            account: account.into(),
            champion: player.champion_id,
            mode,
            win: player.stats.win,
            augments: player.stats.augments(),
            date: game.game_creation_date,
        });
    }
    games.sort_by_key(|g| Reverse(g.game_id));
    games.len() - before
}

/// Games stored before accounts were tracked belong to the first account seen.
pub fn claim_unowned(games: &mut [StoredGame], account: &str) -> bool {
    let mut claimed = false;
    for game in games.iter_mut().filter(|g| g.account.is_empty()) {
        game.account = account.into();
        claimed = true;
    }
    claimed
}

#[derive(Serialize, TS)]
#[ts(export)]
pub struct StatRow {
    pub id: u32,
    pub name: String,
    pub icon: String,
    pub games: u32,
    pub wins: u32,
}

#[derive(Serialize, TS)]
#[ts(export)]
pub struct RecentGame {
    #[ts(type = "number")]
    pub game_id: u64,
    /// ISO 8601, UTC.
    pub date: String,
    pub champion: Asset,
    pub mode: GameMode,
    pub win: bool,
    pub augments: Vec<Asset>,
}

#[derive(Serialize, TS)]
#[ts(export)]
pub struct StatsSummary {
    pub games: u32,
    /// Games an augment needs before it is ranked.
    pub min_augment_games: u32,
    pub wins: u32,
    pub champions: Vec<StatRow>,
    pub augments: Vec<StatRow>,
    pub recent: Vec<RecentGame>,
}

fn count(tally: &mut HashMap<u32, (u32, u32)>, id: u32, win: bool) {
    let entry = tally.entry(id).or_default();
    entry.0 += 1;
    entry.1 += win as u32;
}

fn rows(tally: HashMap<u32, (u32, u32)>, assets: &NamedAssets) -> Vec<StatRow> {
    tally
        .into_iter()
        .map(|(id, (games, wins))| {
            let asset = named(assets, id);
            StatRow { id, name: asset.name, icon: asset.icon, games, wins }
        })
        .collect()
}

/// Summary of the games of `account`, or of every game when no account is signed in.
pub fn summarize(games: &[StoredGame], account: Option<&str>, catalog: &Catalog) -> StatsSummary {
    let games: Vec<&StoredGame> = games.iter().filter(|g| account.is_none_or(|a| g.account == a)).collect();
    let mut champions: HashMap<u32, (u32, u32)> = HashMap::new();
    let mut augments: HashMap<u32, (u32, u32)> = HashMap::new();
    for game in &games {
        count(&mut champions, game.champion, game.win);
        for &augment in &game.augments {
            count(&mut augments, augment, game.win);
        }
    }
    let mut champions = rows(champions, &catalog.champions);
    champions.sort_by_key(|r| (Reverse(r.games), Reverse(r.wins)));
    let mut augments: Vec<StatRow> = rows(augments, &catalog.augments).into_iter().filter(|r| r.games >= MIN_AUGMENT_GAMES).collect();
    augments.sort_by(|a, b| (b.wins as f64 / b.games as f64).total_cmp(&(a.wins as f64 / a.games as f64)).then(b.games.cmp(&a.games)));
    augments.truncate(TOP_AUGMENTS);
    StatsSummary {
        games: games.len() as u32,
        min_augment_games: MIN_AUGMENT_GAMES,
        wins: games.iter().filter(|g| g.win).count() as u32,
        champions,
        augments,
        recent: games
            .iter()
            .take(RECENT_GAMES)
            .map(|game| RecentGame {
                game_id: game.game_id,
                date: game.date.clone(),
                champion: catalog.champion(game.champion),
                mode: game.mode,
                win: game.win,
                augments: game.augments.iter().map(|&id| named(&catalog.augments, id)).collect(),
            })
            .collect(),
    }
}

fn csv_field(text: String) -> String {
    format!("\"{}\"", text.replace('"', "\"\""))
}

pub fn to_csv(games: &[StoredGame], catalog: &Catalog, language: &str) -> String {
    let t = |key: &str| i18n::t(language, key);
    let header = ["stats:csv.date", "stats:csv.mode", "stats:csv.champion", "stats:csv.result", "stats:csv.augments"].map(t).map(csv_field);
    let mut csv = header.join(",") + CSV_LINE_END;
    for game in games {
        let augments: Vec<String> = game.augments.iter().map(|&id| named(&catalog.augments, id).name).collect();
        let row = [
            game.date.clone(),
            t(&format!("common:modes.{}", i18n::variant_key(game.mode))),
            catalog.champion(game.champion).name,
            t(if game.win { "stats:victory" } else { "stats:defeat" }),
            augments.join(" | "),
        ];
        csv += &row.map(csv_field).join(",");
        csv += CSV_LINE_END;
    }
    csv
}

/// Writes the CSV with a UTF-8 byte order mark.
pub fn write_csv(path: &Path, games: &[StoredGame], catalog: &Catalog, language: &str) -> Result<()> {
    Ok(fs::write(path, format!("{UTF8_BOM}{}", to_csv(games, catalog, language)))?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{errors::AppError, league::parse};
    use serde_json::json;

    fn game(id: u64, account: &str, win: bool) -> StoredGame {
        StoredGame {
            game_id: id,
            account: account.into(),
            date: "2026-09-25T10:00:00.000Z".into(),
            champion: 1,
            mode: GameMode::Mayhem,
            win,
            augments: vec![7, 9],
        }
    }

    #[test]
    fn csv_escapes_quotes_and_names_assets() {
        let mut catalog = Catalog::default();
        catalog.champions.insert(1, ("Annie".into(), String::new()));
        catalog.augments.insert(7, ("Ojo \"de\" halcón, raro".into(), String::new()));
        let csv = to_csv(&[game(1, "me", true)], &catalog, "es");
        assert_eq!(csv.lines().nth(1).unwrap(), r#""2026-09-25T10:00:00.000Z","ARAM: Caos","Annie","Victoria","Ojo ""de"" halcón, raro | #9""#);
    }

    #[test]
    fn summarizes_one_account_and_reads_legacy_games() {
        let mut games = vec![game(3, "me", true), game(2, "other", false), game(1, "", false)];
        assert!(claim_unowned(&mut games, "me"));
        let summary = summarize(&games, Some("me"), &Catalog::default());
        assert_eq!((summary.games, summary.wins), (2, 1));
        assert_eq!(summarize(&games, None, &Catalog::default()).games, 3);

        let legacy: StoredGame =
            serde_json::from_str(r#"{"game_id":5,"fecha":"2026-09-01T20:15","campeon":103,"modo":"KIWI","victoria":true,"aumentos":[1]}"#).unwrap();
        assert_eq!((legacy.mode, legacy.account.as_str(), legacy.champion), (GameMode::Mayhem, "", 103));
    }

    #[test]
    fn imports_new_augment_games_and_refuses_a_changed_history() {
        let history = json!({ "games": { "games": [
            { "gameId": 11, "gameMode": "KIWI", "gameCreationDate": "2026-09-25T10:00:00.000Z",
              "participants": [{ "championId": 103, "stats": { "win": true, "playerAugment1": 7, "playerAugment2": 0, "playerAugment3": 9 } }] },
            { "gameId": 10, "gameMode": "CLASSIC", "gameCreationDate": "2026-09-25T09:00:00.000Z",
              "participants": [{ "championId": 1, "stats": { "win": false } }] },
            { "gameId": 3, "gameMode": "CHERRY", "gameCreationDate": "2026-09-24T09:00:00.000Z",
              "participants": [{ "championId": 1, "stats": { "win": false } }] }
        ]}});
        let mut games = vec![game(3, "me", true)];
        assert_eq!(add_new_games(parse(MATCH_HISTORY, &history).unwrap(), "me", &mut games), 1);
        assert_eq!((games[0].game_id, games[0].champion, games[0].win, games[0].augments.as_slice()), (11, 103, true, [7, 9].as_slice()));
        assert!(games[1].win);

        let renamed = json!({ "games": { "games": [{ "gameId": 12, "gameMode": "KIWI", "gameCreationDate": "", "participants": [{ "championId": 1, "stats": { "victory": true } }] }] }});
        assert!(matches!(parse::<MatchHistory>(MATCH_HISTORY, &renamed), Err(AppError::ClientFormat(_))));
    }
}
