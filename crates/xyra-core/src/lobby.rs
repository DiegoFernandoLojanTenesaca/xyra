use crate::{
    errors::Result,
    league::{Lcu, asset_url, parse},
};
use reqwest::Method;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use ts_rs::TS;

pub const LOBBY: &str = "/lol-lobby/v2/lobby";
const INVITATIONS: &str = "/lol-lobby/v2/lobby/invitations";
const SEARCH: &str = "/lol-lobby/v2/lobby/matchmaking/search";
const QUEUES: &str = "/lol-game-queues/v1/queues";
const FRIENDS: &str = "/lol-chat/v1/friends";
const SUMMONER_BY_PUUID: &str = "/lol-summoner/v2/summoners/puuid";
/// The queues Xyra offers, in this order, besides whichever Arena queue is open: Swiftplay, Draft, Solo/Duo, Flex,
/// ARAM and ARAM: Mayhem.
const QUEUE_IDS: [u32; 6] = [480, 400, 420, 440, 450, 2400];
const ARENA_MODE: &str = "CHERRY";
const AVAILABLE: &str = "Available";
const PENDING: &str = "Pending";
/// Friends with these presences are not in the client.
const AWAY_FROM_CLIENT: [&str; 2] = ["offline", "mobile"];

/// A queue the player can open a lobby for, named in the client's language.
#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct LobbyQueue {
    pub id: u32,
    pub name: String,
    pub ranked: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct QueueAnswer {
    id: u32,
    name: String,
    game_mode: String,
    #[serde(default)]
    is_ranked: bool,
    #[serde(default)]
    queue_availability: String,
}

/// What a friend in the client is doing, which says whether they can join now.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub enum FriendStatus {
    Available,
    Away,
    Busy,
    InQueue,
    ChampSelect,
    InGame,
}

impl FriendStatus {
    fn can_join(self) -> bool {
        matches!(self, FriendStatus::Available | FriendStatus::Away)
    }
}

/// A friend in the client, who can be invited when available.
#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct Friend {
    pub puuid: String,
    #[ts(type = "number")]
    pub summoner_id: u64,
    pub name: String,
    pub icon: String,
    pub status: FriendStatus,
    pub can_join: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct FriendAnswer {
    puuid: String,
    #[serde(default)]
    summoner_id: u64,
    #[serde(default)]
    game_name: String,
    #[serde(default)]
    icon: i64,
    availability: String,
    #[serde(default)]
    lol: FriendGame,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FriendGame {
    game_status: Option<String>,
}

impl FriendAnswer {
    fn status(&self) -> FriendStatus {
        match (self.lol.game_status.as_deref(), self.availability.as_str()) {
            (Some("inGame"), _) => FriendStatus::InGame,
            (Some("championSelect"), _) => FriendStatus::ChampSelect,
            (Some("inQueue"), _) => FriendStatus::InQueue,
            (_, "away") => FriendStatus::Away,
            (_, "dnd") => FriendStatus::Busy,
            _ => FriendStatus::Available,
        }
    }
}

/// The player's lobby as the client describes it; names are looked up apart, since the client leaves them empty.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LobbyAnswer {
    game_config: GameConfig,
    #[serde(default)]
    can_start_activity: bool,
    local_member: Member,
    members: Vec<Member>,
    #[serde(default)]
    invitations: Vec<Invitation>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GameConfig {
    queue_id: u32,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Member {
    puuid: String,
    #[serde(default)]
    summoner_icon_id: i64,
    #[serde(default)]
    is_leader: bool,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Invitation {
    #[serde(default)]
    to_puuid: String,
    state: String,
}

impl LobbyAnswer {
    /// Everyone whose name the lobby needs: its members and the friends invited.
    pub fn players(&self) -> impl Iterator<Item = &str> {
        self.members.iter().map(|member| member.puuid.as_str()).chain(self.pending().map(|invitation| invitation.to_puuid.as_str()))
    }

    fn pending(&self) -> impl Iterator<Item = &Invitation> {
        self.invitations.iter().filter(|invitation| invitation.state == PENDING)
    }
}

/// The player's lobby as Xyra shows it.
#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct Lobby {
    pub queue_id: u32,
    pub members: Vec<LobbyMember>,
    /// Names of the friends invited who have not answered yet.
    pub invited: Vec<String>,
    /// PUUIDs of the members and of the friends invited, to tell who can still be invited.
    pub players: Vec<String>,
    pub leader: bool,
    /// The leader can start looking for a match now.
    pub can_search: bool,
    pub searching: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct LobbyMember {
    pub name: String,
    pub icon: String,
    pub leader: bool,
}

fn profile_icon(id: i64) -> String {
    asset_url(&format!("/lol-game-data/assets/v1/profile-icons/{id}.jpg"))
}

pub fn view(answer: &LobbyAnswer, searching: bool, name: impl Fn(&str) -> String) -> Lobby {
    Lobby {
        queue_id: answer.game_config.queue_id,
        members: answer
            .members
            .iter()
            .map(|member| LobbyMember { name: name(&member.puuid), icon: profile_icon(member.summoner_icon_id), leader: member.is_leader })
            .collect(),
        invited: answer.pending().map(|invitation| name(&invitation.to_puuid)).collect(),
        players: answer.players().map(String::from).collect(),
        leader: answer.local_member.is_leader,
        can_search: answer.local_member.is_leader && answer.can_start_activity,
        searching,
    }
}

pub fn parse_lobby(value: &Value) -> Result<LobbyAnswer> {
    parse(LOBBY, value)
}

/// The queues of `QUEUE_IDS` that are open now, then the open Arena queue.
pub fn queues(lcu: &Lcu) -> Result<Vec<LobbyQueue>> {
    Ok(open_queues(lcu.get_as(QUEUES)?))
}

fn open_queues(answers: Vec<QueueAnswer>) -> Vec<LobbyQueue> {
    let open: Vec<QueueAnswer> = answers.into_iter().filter(|queue| queue.queue_availability == AVAILABLE).collect();
    let listed = QUEUE_IDS.iter().filter_map(|id| open.iter().find(|queue| queue.id == *id));
    let arena = open.iter().filter(|queue| queue.game_mode == ARENA_MODE).min_by_key(|queue| queue.id);
    listed.chain(arena).map(|queue| LobbyQueue { id: queue.id, name: queue.name.clone(), ranked: queue.is_ranked }).collect()
}

/// Friends in the client, those who can join first, then by name.
pub fn friends(lcu: &Lcu) -> Result<Vec<Friend>> {
    Ok(online_friends(lcu.get_as(FRIENDS)?))
}

fn online_friends(answers: Vec<FriendAnswer>) -> Vec<Friend> {
    let mut friends: Vec<Friend> = answers
        .into_iter()
        .filter(|friend| !AWAY_FROM_CLIENT.contains(&friend.availability.as_str()))
        .map(|friend| {
            let status = friend.status();
            Friend {
                icon: profile_icon(friend.icon),
                name: friend.game_name,
                puuid: friend.puuid,
                summoner_id: friend.summoner_id,
                can_join: status.can_join(),
                status,
            }
        })
        .collect();
    friends.sort_by_cached_key(|friend| (!friend.can_join, friend.name.to_lowercase()));
    friends
}

/// A player's name, like "Lautish", from their PUUID.
pub fn player_name(lcu: &Lcu, puuid: &str) -> Result<String> {
    Ok(lcu.get(&format!("{SUMMONER_BY_PUUID}/{puuid}"))?["gameName"].as_str().unwrap_or_default().to_string())
}

pub fn create(lcu: &Lcu, queue: u32) -> Result<()> {
    lcu.request(Method::POST, LOBBY, Some(&json!({ "queueId": queue }))).map(drop)
}

pub fn leave(lcu: &Lcu) -> Result<()> {
    lcu.request(Method::DELETE, LOBBY, None).map(drop)
}

pub fn invite(lcu: &Lcu, puuid: &str, summoner_id: u64) -> Result<()> {
    lcu.request(Method::POST, INVITATIONS, Some(&json!([{ "toPuuid": puuid, "toSummonerId": summoner_id }]))).map(drop)
}

/// Starts or stops looking for a match with the lobby.
pub fn search(lcu: &Lcu, start: bool) -> Result<()> {
    lcu.request(if start { Method::POST } else { Method::DELETE }, SEARCH, None).map(drop)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offers_the_open_queues_in_order_with_the_arena_one() {
        let queue = |id, mode: &str, open| QueueAnswer {
            id,
            name: format!("#{id}"),
            game_mode: mode.into(),
            is_ranked: id == 420,
            queue_availability: if open { AVAILABLE } else { "PlatformDisabled" }.into(),
        };
        let offered = open_queues(vec![
            queue(2400, "KIWI", true),
            queue(1750, ARENA_MODE, true),
            queue(1700, ARENA_MODE, false),
            queue(420, "CLASSIC", true),
            queue(440, "CLASSIC", false),
            queue(1090, "TFT", true),
        ]);
        assert_eq!(offered.iter().map(|q| (q.id, q.ranked)).collect::<Vec<_>>(), [(420, true), (2400, false), (1750, false)]);
    }

    #[test]
    fn lists_friends_in_the_client_who_can_join_first() {
        let friends: Vec<FriendAnswer> = serde_json::from_value(json!([
            { "puuid": "a", "summonerId": 1, "gameName": "Zed Main", "icon": 7, "availability": "chat", "lol": { "gameStatus": "outOfGame" } },
            { "puuid": "b", "summonerId": 2, "gameName": "Ahri", "icon": 8, "availability": "dnd", "lol": { "gameStatus": "inGame" } },
            { "puuid": "c", "summonerId": 3, "gameName": "Off", "icon": 9, "availability": "offline", "lol": {} },
            { "puuid": "d", "summonerId": 4, "gameName": "Brb", "icon": 10, "availability": "away", "lol": {} }
        ]))
        .unwrap();
        let online = online_friends(friends);
        assert_eq!(
            online.iter().map(|f| (f.name.as_str(), f.status, f.can_join)).collect::<Vec<_>>(),
            [("Brb", FriendStatus::Away, true), ("Zed Main", FriendStatus::Available, true), ("Ahri", FriendStatus::InGame, false)]
        );
    }

    #[test]
    fn reads_the_lobby_with_pending_invitations() {
        let answer = parse_lobby(&json!({
            "canStartActivity": true,
            "gameConfig": { "queueId": 450, "gameMode": "ARAM" },
            "localMember": { "puuid": "me", "isLeader": true, "summonerIconId": 7144 },
            "members": [{ "puuid": "me", "isLeader": true, "summonerIconId": 7144, "summonerName": "" }],
            "invitations": [
                { "toPuuid": "me", "state": "Accepted" },
                { "toPuuid": "friend", "state": "Pending" },
                { "toPuuid": "other", "state": "Declined" }
            ]
        }))
        .unwrap();
        let lobby = view(&answer, false, |puuid| puuid.to_uppercase());
        assert_eq!((lobby.queue_id, lobby.leader, lobby.can_search), (450, true, true));
        assert_eq!(lobby.members.iter().map(|m| (m.name.as_str(), m.leader)).collect::<Vec<_>>(), [("ME", true)]);
        assert!(lobby.members[0].icon.ends_with("/profile-icons/7144.jpg"));
        assert_eq!((lobby.invited, lobby.players), (vec!["FRIEND".to_string()], vec!["me".to_string(), "friend".to_string()]));
    }
}
