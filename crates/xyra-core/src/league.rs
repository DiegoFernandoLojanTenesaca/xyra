use crate::{
    errors::{AppError, Result},
    game_settings::WindowMode,
};
use base64::Engine;
use reqwest::{Method, blocking::Client, header::AUTHORIZATION};
use rustls::{
    ClientConfig, ClientConnection, RootCertStore, StreamOwned,
    pki_types::{CertificateDer, ServerName, pem::PemObject},
};
use serde_json::{Value, json};
use std::{
    fs,
    net::{IpAddr, Ipv4Addr, TcpStream},
    path::PathBuf,
    sync::{Arc, OnceLock},
    time::Duration,
};
use tungstenite::{Message, client::IntoClientRequest};

/// Root of the certificates the League client and game serve on 127.0.0.1.
const RIOT_ROOT_CERTIFICATE: &[u8] = include_bytes!("../riotgames.pem");
const HOST: Ipv4Addr = Ipv4Addr::LOCALHOST;
const HTTP_TIMEOUT: Duration = Duration::from_secs(5);
const LOCKFILE: &str = "lockfile";
const CONFIG_DIR: &str = "Config";
const GAME_CONFIG: &str = "game.cfg";
const GAME_CONFIG_BACKUP: &str = "cfg.before-xyra";
const WINDOW_MODE_KEY: &str = "WindowMode=";
const EVENT_PREFIX: &str = "OnJsonApiEvent";
const WAMP_SUBSCRIBE: u8 = 5;
const WAMP_EVENT: u8 = 8;
const DELETE_EVENT: &str = "Delete";
const ASSETS_BASE: &str = "https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default";

pub struct Installation {
    pub dir: PathBuf,
    pub locale: String,
}

impl Installation {
    pub fn lockfile(&self) -> PathBuf {
        self.dir.join(LOCKFILE)
    }

    pub fn config_dir(&self) -> PathBuf {
        self.dir.join(CONFIG_DIR)
    }

    pub fn game_config(&self) -> PathBuf {
        self.config_dir().join(GAME_CONFIG)
    }

    pub fn is_lockfile(name: &str) -> bool {
        name == LOCKFILE
    }

    pub fn is_game_config(name: &str) -> bool {
        name == GAME_CONFIG
    }
}

fn local_tls() -> Arc<ClientConfig> {
    static CONFIG: OnceLock<Arc<ClientConfig>> = OnceLock::new();
    CONFIG
        .get_or_init(|| {
            let mut roots = RootCertStore::empty();
            roots.add(CertificateDer::from_pem_slice(RIOT_ROOT_CERTIFICATE).expect("embedded Riot certificate")).expect("valid Riot certificate");
            let config = ClientConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
                .with_safe_default_protocol_versions()
                .expect("TLS protocol versions")
                .with_root_certificates(roots)
                .with_no_client_auth();
            Arc::new(config)
        })
        .clone()
}

fn local_http() -> Client {
    Client::builder().tls_backend_preconfigured((*local_tls()).clone()).timeout(HTTP_TIMEOUT).build().expect("local HTTP client")
}

/// A change of a client resource; `data` is None when the resource was deleted.
pub struct LcuEvent {
    pub uri: String,
    pub data: Option<Value>,
}

fn event_name(endpoint: &str) -> String {
    format!("{EVENT_PREFIX}{}", endpoint.replace('/', "_"))
}

fn parse_event(text: &str) -> Option<LcuEvent> {
    let message: Value = serde_json::from_str(text).ok()?;
    if message[0] != WAMP_EVENT {
        return None;
    }
    let payload = &message[2];
    let deleted = payload["eventType"] == DELETE_EVENT;
    Some(LcuEvent { uri: payload["uri"].as_str()?.into(), data: (!deleted).then(|| payload["data"].clone()) })
}

/// The League client's local API, authenticated with the lockfile it writes while running.
#[derive(Clone)]
pub struct Lcu {
    port: u16,
    auth: String,
    http: Client,
}

impl Lcu {
    pub fn connect(installation: &Installation) -> Result<Lcu> {
        let text = fs::read_to_string(installation.lockfile()).map_err(|_| AppError::ClientClosed)?;
        Lcu::from_lockfile(&text).ok_or(AppError::ClientClosed)
    }

    fn from_lockfile(text: &str) -> Option<Lcu> {
        let mut parts = text.split(':').skip(2);
        let port = parts.next()?.parse().ok()?;
        let password = parts.next()?;
        let key = base64::engine::general_purpose::STANDARD.encode(format!("riot:{password}"));
        Some(Lcu { port, auth: format!("Basic {key}"), http: local_http() })
    }

    pub fn same_session(&self, other: &Lcu) -> bool {
        self.port == other.port && self.auth == other.auth
    }

    pub fn get(&self, endpoint: &str) -> Result<Value> {
        self.request(Method::GET, endpoint, None)
    }

    pub fn request(&self, method: Method, endpoint: &str, body: Option<&Value>) -> Result<Value> {
        let mut request = self.http.request(method, format!("https://{HOST}:{}{endpoint}", self.port)).header(AUTHORIZATION, &self.auth);
        if let Some(body) = body {
            request = request.json(body);
        }
        let response = request.send().map_err(AppError::client)?;
        let status = response.status();
        let text = response.text().map_err(AppError::client)?;
        if !status.is_success() {
            return Err(AppError::Client(format!("{status}: {text}")));
        }
        if text.is_empty() {
            return Ok(Value::Null);
        }
        serde_json::from_str(&text).map_err(AppError::client)
    }

    /// Streams the changes of `endpoints` until the client closes.
    pub fn listen(&self, endpoints: &[&str], mut on_event: impl FnMut(LcuEvent)) -> Result<()> {
        let tcp = TcpStream::connect((HOST, self.port)).map_err(AppError::client)?;
        let tls = ClientConnection::new(local_tls(), ServerName::from(IpAddr::V4(HOST))).map_err(AppError::client)?;
        let mut request = format!("wss://{HOST}:{}/", self.port).into_client_request().map_err(AppError::client)?;
        request.headers_mut().insert(AUTHORIZATION, self.auth.parse().map_err(AppError::client)?);
        let (mut socket, _) = tungstenite::client(request, StreamOwned::new(tls, tcp)).map_err(AppError::client)?;
        for endpoint in endpoints {
            socket.send(Message::text(json!([WAMP_SUBSCRIBE, event_name(endpoint)]).to_string())).map_err(AppError::client)?;
        }
        loop {
            match socket.read() {
                Ok(Message::Text(text)) => {
                    if let Some(event) = parse_event(text.as_str()) {
                        on_event(event);
                    }
                }
                Ok(Message::Close(_)) | Err(tungstenite::Error::ConnectionClosed) => return Ok(()),
                Ok(_) => {}
                Err(e) => return Err(AppError::client(e)),
            }
        }
    }
}

pub fn is_borderless(installation: &Installation) -> Option<bool> {
    let text = fs::read_to_string(installation.game_config()).ok()?;
    let value = text.lines().find_map(|line| line.trim().strip_prefix(WINDOW_MODE_KEY))?;
    Some(value.trim().parse().ok() == Some(WindowMode::Borderless.code()))
}

/// Edits game.cfg directly, for when the client is closed; keeps a backup of the original file.
pub fn set_borderless(installation: &Installation) -> Result<()> {
    let path = installation.game_config();
    let text = fs::read_to_string(&path)?;
    let backup = path.with_extension(GAME_CONFIG_BACKUP);
    if !backup.exists() {
        fs::write(&backup, &text)?;
    }
    let updated: String = text
        .split_inclusive('\n')
        .map(|line| match line.trim_start().starts_with(WINDOW_MODE_KEY) {
            true => format!("{WINDOW_MODE_KEY}{}{}", WindowMode::Borderless.code(), &line[line.trim_end().len()..]),
            false => line.to_string(),
        })
        .collect();
    Ok(fs::write(&path, updated)?)
}

/// Maps a client asset path (/lol-game-data/assets/...) to its CommunityDragon URL, usable without the client.
pub fn asset_url(path: &str) -> String {
    let rest = path.trim_start_matches("/lol-game-data/assets/").to_lowercase();
    format!("{ASSETS_BASE}/{rest}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_lockfile_and_events() {
        let lcu = Lcu::from_lockfile("LeagueClient:1234:59499:secret:https").unwrap();
        assert_eq!(lcu.port, 59499);
        assert_eq!(lcu.auth, "Basic cmlvdDpzZWNyZXQ=");
        assert!(Lcu::from_lockfile("broken").is_none());
        assert_eq!(event_name("/lol-gameflow/v1/session"), "OnJsonApiEvent_lol-gameflow_v1_session");

        let update = parse_event(r#"[8,"OnJsonApiEvent",{"data":"Lobby","eventType":"Update","uri":"/lol-gameflow/v1/gameflow-phase"}]"#).unwrap();
        assert_eq!((update.uri.as_str(), update.data), ("/lol-gameflow/v1/gameflow-phase", Some(json!("Lobby"))));
        let deleted = parse_event(r#"[8,"OnJsonApiEvent",{"data":null,"eventType":"Delete","uri":"/lol-champ-select/v1/session"}]"#).unwrap();
        assert!(deleted.data.is_none());
        assert!(parse_event(r#"[0,"welcome"]"#).is_none());
    }

    #[test]
    fn switches_game_config_to_borderless_keeping_the_rest() {
        let dir = std::env::temp_dir().join("xyra-league-test");
        fs::create_dir_all(dir.join(CONFIG_DIR)).unwrap();
        let installation = Installation { dir: dir.clone(), locale: "es_MX".into() };
        fs::write(installation.game_config(), "[General]\r\nWindowMode=0\r\nWidth=1920\r\n").unwrap();
        assert_eq!(is_borderless(&installation), Some(false));
        set_borderless(&installation).unwrap();
        assert_eq!(fs::read_to_string(installation.game_config()).unwrap(), "[General]\r\nWindowMode=2\r\nWidth=1920\r\n");
        assert_eq!(is_borderless(&installation), Some(true));
        fs::remove_dir_all(dir).unwrap();
    }
}
