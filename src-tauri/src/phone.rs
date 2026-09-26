use crate::{
    commands::{App, apply_config},
    engine::Shared,
};
use base64::{Engine, engine::general_purpose::STANDARD};
use qrcode::{QrCode, render::svg};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use std::{
    io::Cursor,
    net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream, UdpSocket},
    os::windows::process::CommandExt,
    process::Command,
    sync::Arc,
    thread::{self, JoinHandle},
    time::Duration,
};
use tauri::{AppHandle, Manager};
use tiny_http::{Header, Method, Request, Response, Server};
use xyra_core::{
    errors::{AppError, Result},
    matchmaking,
    model::{BuildMode, GameMode, ImportTarget, PcInfo, PhoneLink, PhoneSettings, Position},
    opgg,
};

pub const PORT: u16 = 47811;
/// A state request waits this long for a change before answering with the same state.
const LONG_POLL: Duration = Duration::from_secs(20);
/// Connecting a UDP socket sends nothing and only picks the interface that reaches the address: the home network
/// through a public address, and Tailscale through its DNS address when it is running.
const ROUTE_PROBES: [(Ipv4Addr, u16); 2] = [(Ipv4Addr::new(8, 8, 8, 8), 80), (Ipv4Addr::new(100, 100, 100, 100), 53)];
const PAIRING_URI: &str = "xyra://pair";
const QR_SIZE: u32 = 240;
const FIREWALL_RULE: &str = "Xyra (celular)";
const CREATE_NO_WINDOW: u32 = 0x0800_0000;
const WAKE_TIMEOUT: Duration = Duration::from_millis(500);
const WAKE_PAUSE: Duration = Duration::from_millis(50);
/// Up to two seconds for tiny_http's accept thread to let the port go.
const WAKE_ATTEMPTS: usize = 40;

/// Serves Xyra to the phone app on the same network while the player keeps it on.
pub struct PhoneServer {
    port: u16,
    server: Option<Arc<Server>>,
    listener: Option<JoinHandle<()>>,
}

impl Drop for PhoneServer {
    /// Frees the port so the link can start again: tiny_http wakes its accept thread by connecting to the address it
    /// listens on, which fails for 0.0.0.0 on Windows, so it is woken through localhost until the port refuses.
    fn drop(&mut self) {
        if let Some(server) = self.server.take() {
            server.unblock();
            if let Some(listener) = self.listener.take() {
                listener.join().ok();
            }
            drop(server);
        }
        let address = SocketAddr::from((Ipv4Addr::LOCALHOST, self.port));
        for _ in 0..WAKE_ATTEMPTS {
            if TcpStream::connect_timeout(&address, WAKE_TIMEOUT).is_err() {
                break;
            }
            thread::sleep(WAKE_PAUSE);
        }
    }
}

/// Starts or stops the server to match the settings.
pub fn follow_config(app: &AppHandle) {
    let shared = Arc::clone(&app.state::<App>());
    let wanted = shared.config().phone_link;
    let mut phone = shared.phone.lock().unwrap();
    if wanted && phone.is_none() {
        match start(app) {
            Ok(server) => *phone = Some(server),
            Err(e) => shared.log_error("phone link", e),
        }
    } else if !wanted {
        *phone = None;
    }
}

fn start(app: &AppHandle) -> Result<PhoneServer> {
    let server = Arc::new(Server::http((Ipv4Addr::UNSPECIFIED, PORT)).map_err(AppError::platform)?);
    let (requests, app) = (Arc::clone(&server), app.clone());
    let listener = thread::spawn(move || {
        for request in requests.incoming_requests() {
            let app = app.clone();
            thread::spawn(move || handle(request, &app));
        }
    });
    Ok(PhoneServer { port: PORT, server: Some(server), listener: Some(listener) })
}

/// The QR code the Xyra phone app scans to pair; None while the link is off.
pub fn link(shared: &Shared) -> Result<Option<PhoneLink>> {
    let config = shared.config();
    if !config.phone_link {
        return Ok(None);
    }
    if shared.phone.lock().unwrap().is_none() {
        return Err(AppError::PhoneLinkUnavailable);
    }
    let hosts = local_addresses().iter().map(IpAddr::to_string).collect::<Vec<_>>().join(",");
    let pairing = format!("{PAIRING_URI}?hosts={hosts}&port={PORT}&token={}", config.phone_token);
    let qr = QrCode::new(&pairing).map_err(AppError::platform)?.render::<svg::Color>().min_dimensions(QR_SIZE, QR_SIZE).build();
    Ok(Some(PhoneLink { qr }))
}

/// Asks Windows, through its administrator prompt, to let phones in: marks the network that reaches the internet as
/// private and allows Xyra's port on private networks only.
pub fn prepare_windows() -> Result<()> {
    let program = std::env::current_exe()?.display().to_string().replace('\'', "''");
    let script = format!(
        "$route = Get-NetRoute -DestinationPrefix 0.0.0.0/0 | Sort-Object RouteMetric | Select-Object -First 1; \
         Set-NetConnectionProfile -InterfaceIndex $route.InterfaceIndex -NetworkCategory Private; \
         Get-NetFirewallRule -DisplayName '{FIREWALL_RULE}' -ErrorAction SilentlyContinue | Remove-NetFirewallRule; \
         New-NetFirewallRule -DisplayName '{FIREWALL_RULE}' -Direction Inbound -Program '{program}' -Protocol TCP -LocalPort {PORT} -Action Allow -Profile Private | Out-Null"
    );
    let encoded = STANDARD.encode(script.encode_utf16().flat_map(u16::to_le_bytes).collect::<Vec<u8>>());
    let elevate = format!("Start-Process powershell -Verb RunAs -Wait -WindowStyle Hidden -ArgumentList '-NoProfile','-EncodedCommand','{encoded}'");
    let status = Command::new("powershell").args(["-NoProfile", "-Command", &elevate]).creation_flags(CREATE_NO_WINDOW).status()?;
    if status.success() { Ok(()) } else { Err(AppError::WindowsNotPrepared) }
}

/// This PC's addresses on the home network and on Tailscale, the ones a phone can reach.
fn local_addresses() -> Vec<IpAddr> {
    let mut addresses = Vec::new();
    for probe in ROUTE_PROBES {
        let address = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0)).and_then(|socket| socket.connect(probe).and_then(|()| socket.local_addr()));
        if let Ok(address) = address
            && !addresses.contains(&address.ip())
        {
            addresses.push(address.ip());
        }
    }
    addresses
}

/// Query parameters of a request, already split; values are plain ASCII the app sends unencoded.
struct Query<'a>(&'a str);

impl Query<'_> {
    fn get(&self, name: &str) -> Option<&str> {
        self.0.split('&').find_map(|pair| pair.strip_prefix(name)?.strip_prefix('='))
    }

    fn parse<T: DeserializeOwned>(&self, name: &str) -> Option<T> {
        serde_json::from_value(Value::String(self.get(name)?.into())).ok()
    }

    fn number<T: std::str::FromStr>(&self, name: &str) -> Option<T> {
        self.get(name)?.parse().ok()
    }
}

fn handle(request: Request, app: &AppHandle) {
    let shared = Arc::clone(&app.state::<App>());
    let url = request.url().to_string();
    let (path, query) = url.split_once('?').unwrap_or((&url, ""));
    let query = Query(query);
    let response = match (request.method(), path) {
        _ if !query.get("t").is_some_and(|token| shared.config().is_phone_token(token)) => text(403, "forbidden"),
        (Method::Get, "/api/pc") => json_response(&pc_info(&shared)),
        (Method::Get, "/api/state") => {
            let (version, state) = shared.wait_for_state(query.number("after"), LONG_POLL);
            json_response(&json!({ "version": version, "state": state }))
        }
        (Method::Get, "/api/stats") => json_response(&shared.stats_summary()),
        (Method::Get, "/api/champions") => json_response(&shared.champions()),
        (Method::Get, "/api/meta") => answer(shared.meta()),
        (Method::Get, "/api/build") => answer(build(&shared, &query)),
        (Method::Get, "/api/augments") => answer(augments(&shared, &query)),
        (Method::Get, "/api/settings") => json_response(&PhoneSettings::from(&shared.config())),
        (Method::Post, "/api/settings") => answer(change_settings(app, &shared, &query)),
        (Method::Post, "/api/accept") => answer(shared.lcu().and_then(|lcu| matchmaking::accept_if_waiting(&lcu))),
        (Method::Post, "/api/import") => answer(import(&shared, &query)),
        _ => text(404, "not found"),
    };
    if let Err(e) = request.respond(response) {
        shared.log_error("phone link answer", e);
    }
}

fn pc_info(shared: &Shared) -> PcInfo {
    PcInfo { name: std::env::var("COMPUTERNAME").unwrap_or_default(), version: env!("CARGO_PKG_VERSION").into(), language: shared.language().into() }
}

fn build(shared: &Shared, query: &Query) -> Result<xyra_core::model::Build> {
    let champion = query.number("champion").ok_or(AppError::NoData)?;
    let mode: BuildMode = query.parse("mode").unwrap_or(BuildMode::Aram);
    shared.fetch_build(champion, mode, query.parse::<Position>("position"))
}

fn augments(shared: &Shared, query: &Query) -> Result<Vec<xyra_core::model::AugmentRow>> {
    let champion = query.number("champion").ok_or(AppError::NoData)?;
    let mode: GameMode = query.parse("mode").unwrap_or(GameMode::Mayhem);
    Ok(opgg::augment_rows(opgg::fetch_augments(&shared.web, champion, mode)?, &shared.catalog()))
}

/// Applies the settings the phone sends; any the request leaves out keep their value.
fn change_settings(app: &AppHandle, shared: &Shared, query: &Query) -> Result<PhoneSettings> {
    let mut config = shared.config();
    if let Some(on) = query.parse("auto_accept") {
        config.auto_accept = on;
    }
    if let Some(seconds) = query.number("accept_delay_seconds") {
        config.accept_delay_seconds = seconds;
    }
    if let Some(on) = query.parse("auto_import_build") {
        config.auto_import_build = on;
    }
    if let Some(on) = query.parse("paused") {
        config.paused = on;
    }
    if let Some(order) = query.parse("champion_order") {
        config.champion_order = order;
    }
    Ok(PhoneSettings::from(&apply_config(app, shared, config)?))
}

/// Imports part of the build of the champion picked in the current champion select.
fn import(shared: &Shared, query: &Query) -> Result<()> {
    let target: ImportTarget = query.parse("target").ok_or(AppError::NoData)?;
    let select = shared.state.lock().unwrap().champ_select.clone().ok_or(AppError::NotInChampSelect)?;
    let champion = select.champion.ok_or(AppError::NotInChampSelect)?;
    shared.import_build(champion.id, select.mode.build_mode(), select.position, target)
}

fn answer(result: Result<impl Serialize>) -> Response<Cursor<Vec<u8>>> {
    match result {
        Ok(value) => json_response(&json!({ "ok": true, "value": value })),
        Err(e) => json_response(&json!({ "ok": false, "error": e })),
    }
}

fn json_response(value: &impl Serialize) -> Response<Cursor<Vec<u8>>> {
    with_headers(Response::from_string(serde_json::to_string(value).unwrap_or_default()))
}

fn text(status: u16, body: &str) -> Response<Cursor<Vec<u8>>> {
    with_headers(Response::from_string(body).with_status_code(status))
}

/// The app runs from its own origin, so every answer allows being read from any origin; the token guards access.
fn with_headers(response: Response<Cursor<Vec<u8>>>) -> Response<Cursor<Vec<u8>>> {
    [("Content-Type", "application/json; charset=utf-8"), ("Access-Control-Allow-Origin", "*")]
        .into_iter()
        .filter_map(|(name, value)| Header::from_bytes(name, value).ok())
        .fold(response, Response::with_header)
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_PORT: u16 = 47_899;

    #[test]
    fn frees_the_port_when_the_link_stops() {
        for _ in 0..3 {
            let server = Arc::new(Server::http((Ipv4Addr::UNSPECIFIED, TEST_PORT)).expect("the port is free again"));
            let requests = Arc::clone(&server);
            let listener = thread::spawn(move || for _ in requests.incoming_requests() {});
            drop(PhoneServer { port: TEST_PORT, server: Some(server), listener: Some(listener) });
        }
    }
}
