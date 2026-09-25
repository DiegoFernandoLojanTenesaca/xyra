use base64::Engine;
use reqwest::{blocking::Client, Method};
use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
    time::Duration,
};

const RIOT_METADATA: &str = r"C:\ProgramData\Riot Games\Metadata\league_of_legends.live\league_of_legends.live.product_settings.yaml";
const DEFAULT_INSTALL_DIR: &str = r"C:\Riot Games\League of Legends";
const DEFAULT_LOCALE: &str = "en_US";
const HTTP_TIMEOUT: Duration = Duration::from_secs(5);
const LIVE_GAME_TIMEOUT: Duration = Duration::from_secs(2);
const LIVE_GAME_ENDPOINT: &str = "https://127.0.0.1:2999/liveclientdata/allgamedata";
const BORDERLESS_WINDOW_MODE: &str = "2";
const ASSETS_BASE: &str = "https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default";

/// The client and the game serve self-signed certificates on 127.0.0.1.
pub fn http_client() -> Client {
    Client::builder()
        .danger_accept_invalid_certs(true)
        .timeout(HTTP_TIMEOUT)
        .user_agent("xyra")
        .build()
        .expect("http client")
}

pub struct Installation {
    pub dir: PathBuf,
    pub locale: String,
}

fn quoted(line: &str) -> Option<String> {
    let start = line.find('"')? + 1;
    let len = line[start..].find('"')?;
    Some(line[start..start + len].to_string())
}

pub fn read_installation() -> Installation {
    let text = fs::read_to_string(RIOT_METADATA).unwrap_or_default();
    let mut dir = None;
    let mut locale = None;
    for line in text.lines() {
        if line.starts_with("product_install_full_path:") {
            dir = quoted(line);
        } else if line.starts_with(' ') && line.trim_start().starts_with("locale:") {
            locale = quoted(line);
        }
    }
    Installation {
        dir: PathBuf::from(dir.unwrap_or_else(|| DEFAULT_INSTALL_DIR.into())),
        locale: locale.unwrap_or_else(|| DEFAULT_LOCALE.into()),
    }
}

pub struct Lcu {
    url: String,
    auth: String,
}

impl Lcu {
    pub fn connect(install_dir: &Path) -> Option<Lcu> {
        let text = fs::read_to_string(install_dir.join("lockfile")).ok()?;
        let parts: Vec<&str> = text.split(':').collect();
        if parts.len() < 5 {
            return None;
        }
        let key = base64::engine::general_purpose::STANDARD.encode(format!("riot:{}", parts[3]));
        Some(Lcu { url: format!("https://127.0.0.1:{}", parts[2]), auth: format!("Basic {key}") })
    }

    pub fn get(&self, http: &Client, endpoint: &str) -> Result<Value, String> {
        self.request(http, Method::GET, endpoint, None)
    }

    pub fn request(&self, http: &Client, method: Method, endpoint: &str, body: Option<&Value>) -> Result<Value, String> {
        let mut request = http.request(method, format!("{}{}", self.url, endpoint)).header("Authorization", &self.auth);
        if let Some(body) = body {
            request = request.json(body);
        }
        let response = request.send().map_err(|e| e.to_string())?;
        let status = response.status();
        let text = response.text().unwrap_or_default();
        if !status.is_success() {
            return Err(format!("{status}: {text}"));
        }
        Ok(serde_json::from_str(&text).unwrap_or(Value::Null))
    }
}

pub struct LiveGame {
    pub alias: String,
    pub champion: String,
    pub mode: String,
}

pub fn read_live_game(http: &Client) -> Option<LiveGame> {
    let data: Value = http.get(LIVE_GAME_ENDPOINT).timeout(LIVE_GAME_TIMEOUT).send().ok()?.json().ok()?;
    let me = data["activePlayer"]["riotId"].as_str()?;
    let player = data["allPlayers"].as_array()?.iter().find(|p| p["riotId"].as_str() == Some(me))?;
    Some(LiveGame {
        alias: player["rawChampionName"].as_str()?.rsplit('_').next()?.to_string(),
        champion: player["championName"].as_str()?.to_string(),
        mode: data["gameData"]["gameMode"].as_str()?.to_string(),
    })
}

fn game_cfg(install_dir: &Path) -> PathBuf {
    install_dir.join("Config").join("game.cfg")
}

pub fn is_borderless(install_dir: &Path) -> Option<bool> {
    let text = fs::read_to_string(game_cfg(install_dir)).ok()?;
    let value = text.lines().find_map(|line| line.trim().strip_prefix("WindowMode="))?;
    Some(value.trim() == BORDERLESS_WINDOW_MODE)
}

pub fn set_borderless(install_dir: &Path, http: &Client) -> Result<(), String> {
    if read_live_game(http).is_some() {
        return Err("gameInProgress".into());
    }
    let path = game_cfg(install_dir);
    let text = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let backup = path.with_extension("cfg.before-xyra");
    if !backup.exists() {
        fs::write(&backup, &text).map_err(|e| e.to_string())?;
    }
    let updated: String = text
        .split_inclusive('\n')
        .map(|line| {
            if line.trim_start().starts_with("WindowMode=") {
                let ending = if line.ends_with("\r\n") {
                    "\r\n"
                } else if line.ends_with('\n') {
                    "\n"
                } else {
                    ""
                };
                format!("WindowMode={BORDERLESS_WINDOW_MODE}{ending}")
            } else {
                line.to_string()
            }
        })
        .collect();
    fs::write(&path, updated).map_err(|e| e.to_string())
}

/// Maps a client asset path (/lol-game-data/assets/...) to its CommunityDragon URL, usable without the client.
pub fn asset_url(path: &str) -> String {
    let rest = path.trim_start_matches("/lol-game-data/assets/").to_lowercase();
    format!("{ASSETS_BASE}/{rest}")
}
