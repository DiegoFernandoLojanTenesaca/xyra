//! Smoke test against the running League client and OP.GG.

use reqwest::Method;
use std::{
    env,
    path::PathBuf,
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};
use xyra_core::{
    catalog::Catalog,
    champ_select, client_import,
    errors::AppError,
    game_settings, gameflow,
    league::{Installation, Lcu},
    model::BuildMode,
    opgg, profile, stats,
};

const DEFAULT_DIR: &str = r"C:\Riot Games\League of Legends";
const EVERY_EVENT: &str = "";
const EVENT_WAIT: Duration = Duration::from_secs(10);
const AHRI: u32 = 103;
const RUNE_PAGES: &str = "/lol-perks/v1/pages";

fn installation() -> Installation {
    Installation { dir: PathBuf::from(env::var("XYRA_LEAGUE_DIR").unwrap_or(DEFAULT_DIR.into())), locale: "en_US".into() }
}

#[test]
#[ignore]
fn talks_to_the_running_client() {
    let lcu = Lcu::connect(&installation()).expect("League client running");
    let summoner = lcu.get(profile::CURRENT_SUMMONER).expect("current summoner over verified TLS");
    let account = profile::account(&summoner).expect("signed in");
    println!("account {account}");

    let flow = gameflow::parse(&lcu.get(gameflow::SESSION).unwrap_or_default(), Some(&account));
    println!("gameflow {:?} in {:?}", flow.phase, flow.mode);

    let available = profile::read_available_champions(&lcu).expect("owned champions");
    assert!(!available.is_empty());
    println!("{} champions available", available.len());

    let catalog = Catalog::read(&lcu).expect("game data catalog");
    println!("{} champions, {} augments", catalog.champions.len(), catalog.augments.len());

    let settings = game_settings::read(&lcu).expect("game settings");
    println!("game settings {settings:?}");

    let profile = profile::read(&lcu, &catalog).expect("profile");
    println!("profile {} #{} level {}", profile.name, profile.tag, profile.level);

    let mut games = Vec::new();
    println!("{} recent games with augments", stats::import_recent(&lcu, &account, &mut games).expect("match history"));

    if let Ok(pickable) = champ_select::read_pickable(&lcu) {
        println!("{} pickable champions in champion select", pickable.len());
    }

    let (sender, events) = mpsc::channel();
    let listener = lcu.clone();
    thread::spawn(move || listener.listen(&[EVERY_EVENT], |event| drop(sender.send(event.uri))));
    let started = Instant::now();
    thread::sleep(Duration::from_secs(1));
    let flipped = settings.iter().find(|s| s.option == game_settings::GameOption::FlippedMinimap).expect("minimap side").value;
    game_settings::update(&lcu, game_settings::GameOption::FlippedMinimap, flipped).expect("rewrite a setting to raise an event");
    let uri = events.recv_timeout(EVENT_WAIT).expect("an event over the verified WebSocket");
    println!("first event {uri} after {:?}", started.elapsed());

    let http = opgg::client();
    let build = opgg::fetch_build(&http, AHRI, BuildMode::Aram, None, &catalog).expect("OP.GG build");
    assert_eq!(build.runes.primary.len() + build.runes.secondary.len() + build.runes.shards.len(), 9);

    if env::var("XYRA_SMOKE_WRITE").is_ok() {
        match client_import::import_runes(&lcu, &build, "Smoke") {
            Ok(()) => {
                let pages = lcu.get(RUNE_PAGES).unwrap();
                let page = pages.as_array().unwrap().iter().find(|p| p["name"] == "Xyra · Smoke").expect("page created").clone();
                lcu.request(Method::DELETE, &format!("{RUNE_PAGES}/{}", page["id"]), None).expect("page removed");
                println!("rune page imported and removed");
            }
            Err(AppError::NoFreeRunePage) => println!("every rune page is taken: the import was refused without touching them"),
            Err(error) => panic!("rune import failed: {error}"),
        }
    }
}
