use crate::{commands::App, engine::Shared};
use qrcode::{QrCode, render::svg};
use serde::Serialize;
use serde_json::{Value, json};
use std::{
    io::Cursor,
    net::{IpAddr, Ipv4Addr, UdpSocket},
    sync::Arc,
    thread,
    time::Duration,
};
use tauri::{AppHandle, Manager};
use tiny_http::{Header, Method, Request, Response, Server};
use xyra_core::{
    errors::{AppError, Result},
    i18n, matchmaking,
    model::{ImportTarget, PhoneLink},
};

pub const PORT: u16 = 47811;
/// A state request waits this long for a change before answering with the same state.
const LONG_POLL: Duration = Duration::from_secs(20);
/// Connecting a UDP socket sends nothing and only picks the interface that reaches the address: the home network
/// through a public address, and Tailscale through its DNS address when it is running.
const ROUTE_PROBES: [(Ipv4Addr, u16); 2] = [(Ipv4Addr::new(8, 8, 8, 8), 80), (Ipv4Addr::new(100, 100, 100, 100), 53)];
const PAIRING_URI: &str = "xyra://pair";
const QR_SIZE: u32 = 240;
const PAGE: &str = include_str!("phone.html");
const TOKENS: &str = include_str!("../../design/tokens.json");

/// Serves Xyra to phones on the same network while the player keeps it on.
pub struct PhoneServer {
    server: Arc<Server>,
}

impl Drop for PhoneServer {
    fn drop(&mut self) {
        self.server.unblock();
    }
}

/// Starts or stops the server to match the settings.
pub fn follow_config(app: &AppHandle) {
    let shared = Arc::clone(&app.state::<App>());
    let wanted = shared.config().phone_link;
    let mut phone = shared.phone.lock().unwrap();
    if wanted && phone.is_none() {
        match start(&shared) {
            Ok(server) => *phone = Some(server),
            Err(e) => shared.log_error("phone link", e),
        }
    } else if !wanted {
        *phone = None;
    }
}

fn start(shared: &Arc<Shared>) -> Result<PhoneServer> {
    let server = Arc::new(Server::http((Ipv4Addr::UNSPECIFIED, PORT)).map_err(AppError::platform)?);
    let (listener, shared) = (Arc::clone(&server), Arc::clone(shared));
    thread::spawn(move || {
        for request in listener.incoming_requests() {
            let shared = Arc::clone(&shared);
            thread::spawn(move || handle(request, &shared));
        }
    });
    Ok(PhoneServer { server })
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

fn handle(request: Request, shared: &Shared) {
    let url = request.url().to_string();
    let (path, query) = url.split_once('?').unwrap_or((&url, ""));
    let param = |name: &str| query.split('&').find_map(|pair| pair.strip_prefix(name)?.strip_prefix('='));
    let response = match (request.method(), path) {
        (Method::Get, "/") => page(shared),
        _ if !authorized(param("t"), shared) => text(403, "forbidden"),
        (Method::Get, "/api/state") => state(shared, param("after").and_then(|after| after.parse().ok())),
        (Method::Get, "/api/stats") => json_response(&shared.stats_summary()),
        (Method::Post, "/api/accept") => outcome(shared.lcu().and_then(|lcu| matchmaking::accept_if_waiting(&lcu)).map(drop)),
        (Method::Post, "/api/import") => outcome(import(shared, param("target").unwrap_or_default())),
        _ => text(404, "not found"),
    };
    if let Err(e) = request.respond(response) {
        shared.log_error("phone link answer", e);
    }
}

fn authorized(token: Option<&str>, shared: &Shared) -> bool {
    token.is_some_and(|token| shared.config().is_phone_token(token))
}

/// Answers at once for a first request, then waits until the state changes past `after`.
fn state(shared: &Shared, after: Option<u64>) -> Response<Cursor<Vec<u8>>> {
    let (version, state) = shared.wait_for_state(after, LONG_POLL);
    json_response(&json!({ "version": version, "state": state }))
}

/// Imports part of the build of the champion picked in the current champion select.
fn import(shared: &Shared, target: &str) -> Result<()> {
    let target: ImportTarget = serde_json::from_value(Value::String(target.into())).map_err(|_| AppError::NoData)?;
    let select = shared.state.lock().unwrap().champ_select.clone().ok_or(AppError::NotInChampSelect)?;
    let champion = select.champion.ok_or(AppError::NotInChampSelect)?;
    shared.import_build(champion.id, select.mode.build_mode(), select.position, target)
}

fn page(shared: &Shared) -> Response<Cursor<Vec<u8>>> {
    let language = shared.language();
    let mut texts = i18n::namespace(language, "phone").unwrap_or_else(|| json!({}));
    texts["errors"] = i18n::namespace(language, "errors").unwrap_or(Value::Null);
    let html = PAGE.replace("\"__TEXTS__\"", &texts.to_string()).replace("\"__TOKENS__\"", TOKENS).replace("__LANG__", language);
    with_type(Response::from_string(html), "text/html; charset=utf-8")
}

fn outcome(result: Result<()>) -> Response<Cursor<Vec<u8>>> {
    match result {
        Ok(()) => json_response(&json!({ "ok": true })),
        Err(e) => json_response(&json!({ "ok": false, "error": e })).with_status_code(409),
    }
}

fn json_response(value: &impl Serialize) -> Response<Cursor<Vec<u8>>> {
    with_type(Response::from_string(serde_json::to_string(value).unwrap_or_default()), "application/json")
}

fn text(status: u16, body: &str) -> Response<Cursor<Vec<u8>>> {
    Response::from_string(body).with_status_code(status)
}

fn with_type(response: Response<Cursor<Vec<u8>>>, content_type: &str) -> Response<Cursor<Vec<u8>>> {
    match Header::from_bytes("Content-Type", content_type) {
        Ok(header) => response.with_header(header),
        Err(()) => response,
    }
}
