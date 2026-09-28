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
    sync::{
        Arc,
        mpsc::{self, RecvTimeoutError, Sender},
    },
    thread::{self, JoinHandle},
    time::Duration,
    time::{Instant, SystemTime, UNIX_EPOCH},
};
use tauri::{AppHandle, Manager};
use tiny_http::{Header, Method, Request, Response, Server};
use xyra_core::{
    config::{new_phone_token, same_secret},
    errors::{AppError, Result},
    lobby, matchmaking,
    model::{BuildMode, GameMode, ImportTarget, PcInfo, PhoneDevice, PhoneDeviceView, PhoneLink, PhonePermissions, PhoneSettings, Position},
    opgg,
};

pub const PORT: u16 = 47811;
/// A state request waits this long for a change before answering with the same state.
const LONG_POLL: Duration = Duration::from_secs(20);
/// Connecting a UDP socket sends nothing and only picks the interface that reaches the address: the home network,
/// through a public address.
const ROUTE_PROBE: (Ipv4Addr, u16) = (Ipv4Addr::new(8, 8, 8, 8), 80);
/// How often the PC introduces itself on the home network while the link is on.
const ANNOUNCE_EVERY: Duration = Duration::from_secs(30);
/// The discard port: the packets only make the PC known on the network, nobody needs to read them.
const DISCARD_PORT: u16 = 9;
/// Host numbers of a /24 home network, without its network and broadcast addresses.
const HOSTS: std::ops::RangeInclusive<u8> = 1..=254;
const PAIRING_URI: &str = "xyra://pair";
const QR_SIZE: u32 = 240;
const FIREWALL_RULE: &str = "Xyra (celular)";
const CREATE_NO_WINDOW: u32 = 0x0800_0000;
const WAKE_TIMEOUT: Duration = Duration::from_millis(500);
const DEVICE_ID_LENGTH: usize = 12;
const DEVICE_NAME_LENGTH: usize = 40;
const WAKE_PAUSE: Duration = Duration::from_millis(50);
/// Up to two seconds for tiny_http's accept thread to let the port go.
const WAKE_ATTEMPTS: usize = 40;

/// Serves Xyra to the phone app on the same network while the player keeps it on.
pub struct PhoneServer {
    port: u16,
    server: Option<Arc<Server>>,
    listener: Option<JoinHandle<()>>,
    /// Dropping the sender stops the announcements.
    announcer: Option<(Sender<()>, JoinHandle<()>)>,
}

impl Drop for PhoneServer {
    /// Frees the port so the link can start again: tiny_http wakes its accept thread by connecting to the address it
    /// listens on, which fails for 0.0.0.0 on Windows, so it is woken through localhost until the port refuses.
    fn drop(&mut self) {
        if let Some((stop, announcer)) = self.announcer.take() {
            drop(stop);
            announcer.join().ok();
        }
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
    let (stop, stopped) = mpsc::channel::<()>();
    let announcer = thread::spawn(move || {
        while {
            announce();
            matches!(stopped.recv_timeout(ANNOUNCE_EVERY), Err(RecvTimeoutError::Timeout))
        } {}
    });
    Ok(PhoneServer { port: PORT, server: Some(server), listener: Some(listener), announcer: Some((stop, announcer)) })
}

/// Some routers keep a phone from reaching the PC until the PC has sent something on the network, so it sends an empty
/// packet to every address of the home network: that makes the router and each device learn where the PC is.
fn announce() {
    let Some(IpAddr::V4(own)) = lan_address() else { return };
    let Ok(socket) = UdpSocket::bind((own, 0)) else { return };
    let [a, b, c, me] = own.octets();
    for host in HOSTS.filter(|&host| host != me) {
        socket.send_to(&[], (Ipv4Addr::new(a, b, c, host), DISCARD_PORT)).ok();
    }
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
    announce();
    let host = lan_address().map(|address| address.to_string()).unwrap_or_default();
    let pairing = format!("{PAIRING_URI}?hosts={host}&port={PORT}&token={}", config.phone_token);
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

/// This PC's address on the home network, the one a phone can reach.
fn lan_address() -> Option<IpAddr> {
    let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0)).ok()?;
    socket.connect(ROUTE_PROBE).ok()?;
    Some(socket.local_addr().ok()?.ip())
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
    let device = query.get("t").and_then(|token| paired(&shared, token));
    let response = match (request.method(), path, device) {
        (Method::Post, "/api/pair", _) => answer(pair(&shared, &query)),
        (_, _, None) => text(403, "forbidden"),
        (method, path, Some(device)) => {
            shared.phones_seen.lock().unwrap().insert(device.id.clone(), Instant::now());
            route(app, &shared, method, path, &query, &device)
        }
    };
    if let Err(e) = request.respond(response) {
        shared.log_error("phone link answer", e);
    }
}

fn route(app: &AppHandle, shared: &Shared, method: &Method, path: &str, query: &Query, device: &PhoneDevice) -> Response<Cursor<Vec<u8>>> {
    let allowed = |permitted: bool| if permitted { Ok(()) } else { Err(AppError::PhoneNotAllowed) };
    let can = device.permissions;
    match (method, path) {
        (Method::Get, "/api/pc") => json_response(&pc_info(shared, can)),
        (Method::Get, "/api/state") => {
            let (version, state) = shared.wait_for_state(query.number("after"), LONG_POLL);
            json_response(&json!({ "version": version, "state": state }))
        }
        (Method::Get, "/api/stats") => json_response(&shared.stats_summary()),
        (Method::Get, "/api/champions") => json_response(&shared.champions()),
        (Method::Get, "/api/meta") => answer(shared.meta()),
        (Method::Get, "/api/patch") => answer(shared.patch_changes()),
        (Method::Get, "/api/matches") => answer(shared.recent_matches()),
        (Method::Get, "/api/masteries") => answer(shared.masteries()),
        (Method::Get, "/api/challenges") => answer(shared.challenges()),
        (Method::Get, "/api/build") => answer(build(shared, query)),
        (Method::Get, "/api/augments") => answer(augments(shared, query)),
        (Method::Get, "/api/settings") => json_response(&PhoneSettings::from(&shared.config())),
        (Method::Post, "/api/settings") => answer(allowed(can.settings).and_then(|()| change_settings(app, shared, query))),
        (Method::Post, "/api/accept") => answer(allowed(can.accept).and_then(|()| shared.lcu()).and_then(|lcu| matchmaking::accept_if_waiting(&lcu))),
        (Method::Post, "/api/decline") => answer(allowed(can.accept).and_then(|()| shared.lcu()).and_then(|lcu| matchmaking::decline_if_waiting(&lcu))),
        (Method::Post, "/api/import") => answer(allowed(can.import).and_then(|()| import(shared, query))),
        (Method::Post, "/api/bench") => answer(allowed(can.bench).and_then(|()| shared.take_bench_pick(query.number("champion")))),
        (Method::Get, "/api/lobby") => answer(shared.lcu().and_then(|lcu| Ok(json!({ "queues": lobby::queues(&lcu)?, "friends": lobby::friends(&lcu)? })))),
        (Method::Post, "/api/lobby/create") => {
            answer(allowed(can.lobby).and_then(|()| lobby::create(&shared.lcu()?, query.number("queue").ok_or(AppError::NoData)?)))
        }
        (Method::Post, "/api/lobby/invite") => answer(allowed(can.lobby).and_then(|()| {
            let (puuid, summoner) = (query.get("puuid").ok_or(AppError::NoData)?, query.number("summoner").ok_or(AppError::NoData)?);
            lobby::invite(&shared.lcu()?, puuid, summoner)
        })),
        (Method::Post, "/api/lobby/search") => answer(allowed(can.lobby).and_then(|()| lobby::search(&shared.lcu()?, true))),
        (Method::Post, "/api/lobby/cancel") => answer(allowed(can.lobby).and_then(|()| lobby::search(&shared.lcu()?, false))),
        (Method::Post, "/api/lobby/leave") => answer(allowed(can.lobby).and_then(|()| lobby::leave(&shared.lcu()?))),
        _ => text(404, "not found"),
    }
}

/// The paired phone that owns `token`.
fn paired(shared: &Shared, token: &str) -> Option<PhoneDevice> {
    shared.phones.lock().unwrap().iter().find(|device| same_secret(token, &device.token)).cloned()
}

/// Pairs a phone that scanned the QR code: it gets its own token, so it can be disconnected alone.
fn pair(shared: &Shared, query: &Query) -> Result<Value> {
    if !query.get("t").is_some_and(|code| shared.config().is_phone_token(code)) {
        return Err(AppError::PhoneNotAllowed);
    }
    let name: String = decode(query.get("name").unwrap_or_default()).chars().take(DEVICE_NAME_LENGTH).collect();
    let paired_at = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |time| time.as_secs());
    let device =
        PhoneDevice { id: new_phone_token()[..DEVICE_ID_LENGTH].into(), name, token: new_phone_token(), paired_at, permissions: PhonePermissions::default() };
    let token = device.token.clone();
    let mut phones = shared.phones.lock().unwrap();
    phones.push(device);
    shared.storage.save_phones(&phones)?;
    Ok(json!({ "token": token }))
}

/// Percent-decodes a query value; invalid escapes are kept as they came.
fn decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let escaped = (bytes[i] == b'%').then(|| value.get(i + 1..i + 3).and_then(|hex| u8::from_str_radix(hex, 16).ok())).flatten();
        match (bytes[i], escaped) {
            (_, Some(byte)) => {
                decoded.push(byte);
                i += 3;
                continue;
            }
            (b'+', None) => decoded.push(b' '),
            (byte, None) => decoded.push(byte),
        }
        i += 1;
    }
    String::from_utf8_lossy(&decoded).into_owned()
}

/// The paired phones, with how long ago each one asked for something.
pub fn devices(shared: &Shared) -> Vec<PhoneDeviceView> {
    let seen = shared.phones_seen.lock().unwrap();
    shared
        .phones
        .lock()
        .unwrap()
        .iter()
        .map(|device| PhoneDeviceView {
            id: device.id.clone(),
            name: device.name.clone(),
            paired_at: device.paired_at,
            seen_ago: seen.get(&device.id).map(|at| at.elapsed().as_secs()),
            permissions: device.permissions,
        })
        .collect()
}

/// Changes or removes a paired phone, saving the list.
pub fn change_device(shared: &Shared, id: &str, permissions: Option<PhonePermissions>) -> Result<Vec<PhoneDeviceView>> {
    {
        let mut phones = shared.phones.lock().unwrap();
        match permissions {
            Some(permissions) => phones.iter_mut().filter(|device| device.id == id).for_each(|device| device.permissions = permissions),
            None => phones.retain(|device| device.id != id),
        }
        shared.storage.save_phones(&phones)?;
    }
    Ok(devices(shared))
}

fn pc_info(shared: &Shared, permissions: PhonePermissions) -> PcInfo {
    PcInfo {
        name: std::env::var("COMPUTERNAME").unwrap_or_default(),
        version: env!("CARGO_PKG_VERSION").into(),
        language: shared.language().into(),
        permissions,
    }
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

/// The value of `target` that imports runes, items and spells at once.
const WHOLE_BUILD: &str = "all";

/// Imports the build, or part of it, of the champion picked in the current champion select.
fn import(shared: &Shared, query: &Query) -> Result<()> {
    let select = shared.state.lock().unwrap().champ_select.clone().ok_or(AppError::NotInChampSelect)?;
    let champion = select.champion.ok_or(AppError::NotInChampSelect)?;
    let mode = select.mode.build_mode();
    if query.get("target") == Some(WHOLE_BUILD) {
        return shared.import_whole_build(champion.id, mode, select.position);
    }
    let target: ImportTarget = query.parse("target").ok_or(AppError::NoData)?;
    shared.import_build(champion.id, mode, select.position, target)
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
    fn decodes_the_phone_name() {
        assert_eq!(decode("Infinix%20X6816C"), "Infinix X6816C");
        assert_eq!(decode("Pixel+8%C3%B1"), "Pixel 8ñ");
        assert_eq!(decode("100%"), "100%");
    }

    #[test]
    fn frees_the_port_when_the_link_stops() {
        for _ in 0..3 {
            let server = Arc::new(Server::http((Ipv4Addr::UNSPECIFIED, TEST_PORT)).expect("the port is free again"));
            let requests = Arc::clone(&server);
            let listener = thread::spawn(move || for _ in requests.incoming_requests() {});
            drop(PhoneServer { port: TEST_PORT, server: Some(server), listener: Some(listener), announcer: None });
        }
    }
}
