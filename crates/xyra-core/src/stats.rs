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
const TOP_AUGMENTS: usize = 15;
const RECENT_GAMES: usize = 10;
/// Solo/duo and flex queues.
const RANKED_QUEUES: [u32; 2] = [420, 440];
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
    /// The augment choices Xyra saw in this game, in order.
    #[serde(default)]
    pub offers: Vec<Offer>,
}

/// The cards of one augment choice and the one Xyra recommended.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Offer {
    pub cards: Vec<u32>,
    pub best: Option<u32>,
}

/// For each chosen augment, whether it was the recommended card of its choice; None when Xyra did not see that choice.
pub fn followed(augments: &[u32], offers: &[Offer]) -> Vec<Option<bool>> {
    let mut used = vec![false; offers.len()];
    augments
        .iter()
        .map(|&augment| {
            let index = offers.iter().enumerate().position(|(i, offer)| !used[i] && offer.cards.contains(&augment))?;
            used[index] = true;
            Some(offers[index].best == Some(augment))
        })
        .collect()
}

#[derive(Deserialize)]
pub struct MatchHistory {
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
    #[serde(default)]
    queue_id: u32,
    /// Seconds.
    #[serde(default)]
    game_duration: u32,
    participants: Vec<Participant>,
}

/// A game of any mode from the client's match history.
#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct MatchSummary {
    #[ts(type = "number")]
    pub game_id: u64,
    /// ISO 8601, UTC.
    pub date: String,
    pub champion: Asset,
    pub mode: GameMode,
    pub ranked: bool,
    pub win: bool,
    pub kills: u32,
    pub deaths: u32,
    pub assists: u32,
    pub level: u32,
    /// Minions and monsters killed.
    pub farm: u32,
    pub gold: u32,
    pub damage: u32,
    /// Seconds.
    pub duration: u32,
    /// The items the game ended with, in their slots.
    pub items: Vec<Asset>,
    /// The runes' keystone and secondary tree, and the summoner spells, as the game was played.
    pub keystone: Option<Asset>,
    pub secondary_style: Option<Asset>,
    pub spells: Vec<Asset>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Participant {
    champion_id: u32,
    #[serde(default)]
    spell1_id: u32,
    #[serde(default)]
    spell2_id: u32,
    stats: ParticipantStats,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ParticipantStats {
    win: bool,
    #[serde(default)]
    kills: u32,
    #[serde(default)]
    deaths: u32,
    #[serde(default)]
    assists: u32,
    #[serde(default)]
    champ_level: u32,
    #[serde(default)]
    total_minions_killed: u32,
    #[serde(default)]
    neutral_minions_killed: u32,
    #[serde(default)]
    gold_earned: u32,
    #[serde(default)]
    total_damage_dealt_to_champions: u32,
    #[serde(default)]
    item0: u32,
    #[serde(default)]
    item1: u32,
    #[serde(default)]
    item2: u32,
    #[serde(default)]
    item3: u32,
    #[serde(default)]
    item4: u32,
    #[serde(default)]
    item5: u32,
    #[serde(default)]
    item6: u32,
    /// The keystone rune.
    #[serde(default)]
    perk0: u32,
    #[serde(default)]
    perk_sub_style: u32,
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
pub fn read_history(lcu: &Lcu) -> Result<MatchHistory> {
    lcu.get_as(MATCH_HISTORY)
}

/// The history's games of every mode, newest first.
pub fn recent_matches(history: MatchHistory, catalog: &Catalog) -> Vec<MatchSummary> {
    history
        .games
        .games
        .into_iter()
        .filter_map(|game| {
            let player = game.participants.into_iter().next()?;
            let stats = &player.stats;
            let items = [stats.item0, stats.item1, stats.item2, stats.item3, stats.item4, stats.item5, stats.item6]
                .into_iter()
                .filter(|item| catalog.items.contains_key(item))
                .map(|item| named(&catalog.items, item))
                .collect();
            let known = |assets: &NamedAssets, id: u32| assets.contains_key(&id).then(|| named(assets, id));
            Some(MatchSummary {
                keystone: known(&catalog.runes, stats.perk0),
                secondary_style: known(&catalog.runes, stats.perk_sub_style),
                spells: [player.spell1_id, player.spell2_id].into_iter().filter_map(|spell| known(&catalog.spells, spell)).collect(),
                game_id: game.game_id,
                date: game.game_creation_date,
                champion: catalog.champion(player.champion_id),
                mode: GameMode::from_client(&game.game_mode),
                ranked: RANKED_QUEUES.contains(&game.queue_id),
                win: player.stats.win,
                kills: stats.kills,
                deaths: stats.deaths,
                assists: stats.assists,
                level: stats.champ_level,
                farm: stats.total_minions_killed + stats.neutral_minions_killed,
                gold: stats.gold_earned,
                damage: stats.total_damage_dealt_to_champions,
                duration: game.game_duration,
                items,
            })
        })
        .collect()
}

/// Adds the augment games of the history that are not stored yet and returns how many.
pub fn add_new_games(history: MatchHistory, account: &str, games: &mut Vec<StoredGame>) -> usize {
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
            offers: Vec::new(),
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
    /// Aligned with `augments`: whether each was the card Xyra recommended; None when it did not see that choice.
    pub followed: Vec<Option<bool>>,
}

/// How the player's augment picks went with Xyra's recommendation, over the choices Xyra saw.
#[derive(Debug, Default, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct Following {
    /// Picks of the recommended card, and how many of their games were won.
    pub followed: u32,
    pub followed_wins: u32,
    /// Picks of another card when Xyra recommended one.
    pub ignored: u32,
    pub ignored_wins: u32,
}

fn following<'a>(games: impl IntoIterator<Item = &'a StoredGame>) -> Following {
    let mut tally = Following::default();
    for game in games {
        for choice in followed(&game.augments, &game.offers).into_iter().flatten() {
            let (picks, wins) = if choice { (&mut tally.followed, &mut tally.followed_wins) } else { (&mut tally.ignored, &mut tally.ignored_wins) };
            *picks += 1;
            *wins += game.win as u32;
        }
    }
    tally
}

#[derive(Serialize, TS)]
#[ts(export)]
pub struct StatsSummary {
    pub games: u32,
    pub wins: u32,
    pub champions: Vec<StatRow>,
    pub augments: Vec<StatRow>,
    pub recent: Vec<RecentGame>,
    pub following: Following,
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
    // Riot does not allow win rates of augments, so they are listed by use and never ranked by wins.
    let mut augments = rows(augments, &catalog.augments);
    augments.sort_by(|a, b| b.games.cmp(&a.games).then_with(|| a.name.cmp(&b.name)));
    augments.truncate(TOP_AUGMENTS);
    StatsSummary {
        games: games.len() as u32,
        wins: games.iter().filter(|g| g.win).count() as u32,
        following: following(games.iter().copied()),
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
                followed: followed(&game.augments, &game.offers),
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
            offers: Vec::new(),
        }
    }

    #[test]
    fn matches_each_chosen_augment_with_its_offer() {
        let offers = [Offer { cards: vec![1, 2, 3], best: Some(2) }, Offer { cards: vec![4, 5, 6], best: Some(4) }];
        assert_eq!(followed(&[2, 5, 9], &offers), [Some(true), Some(false), None]);
        assert_eq!(followed(&[2, 2], &offers[..1]), [Some(true), None]);
        let game = |augments: Vec<u32>, win| StoredGame {
            game_id: 1,
            account: String::new(),
            date: String::new(),
            champion: 1,
            mode: GameMode::Mayhem,
            win,
            augments,
            offers: offers.to_vec(),
        };
        let tally = following(&[game(vec![2, 5], true), game(vec![2, 4], false), game(Vec::new(), true)]);
        assert_eq!(tally, Following { followed: 3, followed_wins: 1, ignored: 1, ignored_wins: 1 });
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

    #[test]
    fn summarizes_games_of_every_mode() {
        let history = json!({ "games": { "games": [
            { "gameId": 2, "gameMode": "CLASSIC", "queueId": 420, "gameCreationDate": "2026-09-25T10:00:00.000Z",
              "participants": [{ "championId": 1, "spell1Id": 4, "spell2Id": 14,
                "stats": { "win": true, "kills": 7, "deaths": 2, "assists": 9, "perk0": 8112, "perkSubStyle": 8200 } }] },
            { "gameId": 1, "gameMode": "ARAM", "queueId": 450, "gameCreationDate": "2026-09-24T10:00:00.000Z",
              "participants": [{ "championId": 1, "stats": { "win": false } }] }
        ]}});
        let asset = |name: &str| (name.to_string(), String::new());
        let catalog = Catalog {
            runes: HashMap::from([(8112, asset("Electrocute")), (8200, asset("Sorcery"))]),
            spells: HashMap::from([(4, asset("Flash")), (14, asset("Ignite"))]),
            ..Catalog::default()
        };
        let matches = recent_matches(parse(MATCH_HISTORY, &history).unwrap(), &catalog);
        assert_eq!(
            matches.iter().map(|m| (m.game_id, m.mode, m.ranked, m.win, m.kills)).collect::<Vec<_>>(),
            [(2, GameMode::SummonersRift, true, true, 7), (1, GameMode::Aram, false, false, 0)]
        );
        let names = |assets: &[Asset]| assets.iter().map(|a| a.name.clone()).collect::<Vec<_>>();
        assert_eq!(matches[0].keystone.as_ref().map(|k| k.name.as_str()), Some("Electrocute"));
        assert_eq!(matches[0].secondary_style.as_ref().map(|s| s.name.as_str()), Some("Sorcery"));
        assert_eq!(names(&matches[0].spells), ["Flash", "Ignite"]);
        assert!(matches[1].keystone.is_none() && matches[1].spells.is_empty());
    }
}
