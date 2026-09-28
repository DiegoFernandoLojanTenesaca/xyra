use super::{EngineEvent, Shared};
use crate::{
    overlay::{Labels, OverlayHandle},
    screen::{self, Ocr},
    voice::{self, Voice},
};
use std::{
    collections::HashMap,
    fs::File,
    io::BufWriter,
    sync::Arc,
    thread,
    time::{Duration, Instant},
};
use xyra_core::{
    cards::{self, Candidate, Card, CardTracker, OcrLine, TrackerAction},
    catalog::NamedAssets,
    errors::{AppError, Result},
    i18n,
    model::GameMode,
    opgg::{self, AugmentStat},
    stats::Offer,
};

const WATCH_CARDS: Duration = Duration::from_millis(250);
const WATCH_GAME: Duration = Duration::from_millis(500);
const UNFOCUSED: Duration = Duration::from_secs(1);
const RETRY: Duration = Duration::from_secs(5);
/// OP.GG is asked this many times, this far apart, before the stats of a game give up until the next read.
const STATS_ATTEMPTS: u32 = 4;
const STATS_RETRY: Duration = Duration::from_secs(2);
/// Reads in a row that recognize a row of cards without labeling them before it is logged, to find out why.
const UNLABELED_READS_TO_REPORT: u8 = 12;
/// With "Save screenshots" on, the card area and what the OCR read are kept, to see why cards go unlabeled: this often
/// while card names are on screen, this often otherwise, and at most this many times per game.
const DIAGNOSTIC_WITH_CARDS: Duration = Duration::from_secs(2);
const DIAGNOSTIC_WITHOUT_CARDS: Duration = Duration::from_secs(60);
const DIAGNOSTICS_PER_GAME: u32 = 40;
const DIAGNOSTIC_SUFFIX: &str = "-cards";
/// Where the cards appear, as fractions of the screen: left, top, width, height.
const CARD_ZONE: (f64, f64, f64, f64) = (0.125, 0.1, 0.75, 0.7);
const REFERENCE_HEIGHT: f64 = 1200.0;
const DEMO_CHAMPION: &str = "Brand";
/// Demo cards measured in a real game (docs/cards-background.png): augment id, x offset from center, tier, performance.
const DEMO_CARDS: [(u32, f64, Option<u8>, f64); 3] = [(1211, -410.0, Some(0), 83.0), (1098, 0.0, Some(5), 70.0), (2128, 410.0, Some(0), 91.0)];
const DEMO_ROW: f64 = 0.4175;
const VOICE_TEST: &str = "overlay:voice.right";

/// Left, top, width and height of the card area on a screen of this size.
fn card_zone(width: i32, height: i32) -> (i32, i32, i32, i32) {
    let fraction = |part: f64, of: i32| (part * of as f64) as i32;
    (fraction(CARD_ZONE.0, width), fraction(CARD_ZONE.1, height), fraction(CARD_ZONE.2, width), fraction(CARD_ZONE.3, height))
}

pub fn demo_cards(augments: &NamedAssets, width: i32, height: i32) -> Vec<Card> {
    let scale = height as f64 / REFERENCE_HEIGHT;
    let (center, row) = (width as f64 / 2.0, height as f64 * DEMO_ROW);
    let candidates: Vec<Candidate> = DEMO_CARDS.iter().map(|&(id, dx, ..)| Candidate { ids: vec![id], x: center + dx * scale, y: row }).collect();
    let stats = DEMO_CARDS.iter().filter_map(|&(id, _, tier, performance)| Some((id, AugmentStat { tier: tier?, performance, pick_rate: 0.0 }))).collect();
    cards::rate_cards(&candidates, &stats, augments)
}

/// Reads the augment cards on screen during a game and labels them on the overlay.
pub struct CardReader {
    ocr: Option<Ocr>,
    overlay: Option<OverlayHandle>,
    voice: Option<Voice>,
    width: i32,
    height: i32,
    zone: (i32, i32, i32, i32),
    tracker: CardTracker,
    stats: Option<((u32, GameMode), HashMap<u32, AugmentStat>)>,
    /// The champion and mode whose stats are being read from OP.GG.
    fetching: Option<(u32, GameMode)>,
    /// Cards on screen waiting for the stats to label them.
    waiting: Option<Vec<Candidate>>,
    unlabeled_reads: u8,
    diagnosed_at: Option<Instant>,
    diagnostics: u32,
    next_read: Option<Instant>,
    cards: Vec<Card>,
    rounds: Vec<Vec<Card>>,
    round_open: bool,
}

impl CardReader {
    pub fn new(shared: &Arc<Shared>) -> CardReader {
        let (width, height) = screen::primary_size();
        let report = {
            let shared = Arc::clone(shared);
            move |e| shared.log_error("overlay", e)
        };
        CardReader {
            ocr: Ocr::new(&shared.installation.locale),
            overlay: OverlayHandle::spawn(width, height, report).map_err(|e| shared.log_error("overlay", e)).ok(),
            voice: Voice::new().map_err(|e| shared.log_error("voice", e)).ok(),
            width,
            height,
            zone: card_zone(width, height),
            tracker: CardTracker::default(),
            stats: None,
            fetching: None,
            waiting: None,
            unlabeled_reads: 0,
            diagnosed_at: None,
            diagnostics: 0,
            next_read: None,
            cards: Vec::new(),
            rounds: Vec::new(),
            round_open: false,
        }
    }

    pub fn ocr_language(&self) -> Option<String> {
        self.ocr.as_ref().map(|o| o.language.clone())
    }

    pub fn cards(&self) -> &[Card] {
        &self.cards
    }

    pub fn rounds(&self) -> &[Vec<Card>] {
        &self.rounds
    }

    pub fn new_game(&mut self) {
        self.rounds.clear();
        self.round_open = false;
        self.diagnostics = 0;
    }

    /// The choices of this game, to keep with it once the match history lists it.
    pub fn offers(&self) -> Vec<Offer> {
        self.rounds.iter().map(|round| Offer { cards: round.iter().map(|c| c.id).collect(), best: round.iter().find(|c| c.best).map(|c| c.id) }).collect()
    }

    /// A reroll or a new read of the open choice replaces it; cards shown after a hide start a new choice.
    fn record_round(&mut self, cards: &[Card]) {
        let same_cards = |round: &Vec<Card>| round.iter().map(|c| c.id).eq(cards.iter().map(|c| c.id));
        match self.rounds.last_mut() {
            Some(last) if self.round_open || same_cards(last) => *last = cards.to_vec(),
            _ => self.rounds.push(cards.to_vec()),
        }
        self.round_open = true;
    }

    pub fn next_read(&self) -> Option<Instant> {
        self.next_read
    }

    pub fn start(&mut self) {
        if self.next_read.is_none() && self.ocr.is_some() {
            self.next_read = Some(Instant::now());
        }
    }

    pub fn stop(&mut self) {
        self.next_read = None;
        self.hide();
    }

    fn hide(&mut self) {
        if (self.tracker.is_active() || !self.cards.is_empty())
            && let Some(overlay) = &self.overlay
        {
            overlay.hide();
        }
        self.tracker.reset();
        self.cards.clear();
        self.waiting = None;
        self.round_open = false;
    }

    pub fn read(&mut self, champion: u32, mode: GameMode, shared: &Arc<Shared>) {
        self.fetch_stats(champion, mode, shared);
        self.next_read = Some(Instant::now() + self.read_screen(champion, mode, shared));
    }

    /// Asks OP.GG for the champion's augment stats in the background, so reading the screen never waits on the network;
    /// they arrive as `EngineEvent::AugmentStats`.
    fn fetch_stats(&mut self, champion: u32, mode: GameMode, shared: &Arc<Shared>) {
        let key = (champion, mode);
        if self.stats.as_ref().is_some_and(|(read, _)| *read == key) || self.fetching == Some(key) {
            return;
        }
        self.fetching = Some(key);
        let shared = Arc::clone(shared);
        thread::spawn(move || {
            let mut result = opgg::fetch_augments(&shared.web, champion, mode);
            for _ in 1..STATS_ATTEMPTS {
                if result.is_ok() {
                    break;
                }
                thread::sleep(STATS_RETRY);
                result = opgg::fetch_augments(&shared.web, champion, mode);
            }
            let stats = result.map_err(|e| shared.log_error("OP.GG augments", e)).ok();
            shared.send(EngineEvent::AugmentStats { champion, mode, stats });
        });
    }

    /// Keeps the stats OP.GG answered and labels the cards that were waiting for them.
    pub fn set_stats(&mut self, champion: u32, mode: GameMode, stats: Option<HashMap<u32, AugmentStat>>, shared: &Shared) {
        self.fetching = None;
        let Some(stats) = stats else { return };
        let catalog = shared.catalog();
        if let Some(candidates) = self.waiting.take() {
            self.show(cards::rate_cards(&candidates, &stats, &catalog.augments), champion, shared);
        }
        self.stats = Some(((champion, mode), stats));
    }

    fn read_screen(&mut self, champion: u32, mode: GameMode, shared: &Shared) -> Duration {
        let Some(ocr) = &self.ocr else { return RETRY };
        if !screen::is_game_visible() {
            self.hide();
            return UNFOCUSED;
        }
        let (x, y, width, height) = self.zone;
        let read = screen::capture(x, y, width, height).and_then(|pixels| Ok((ocr.read(&pixels, width, height)?, pixels)));
        let (lines, pixels) = match read {
            Ok(read) => read,
            Err(e) => {
                shared.log_error("screen reading", e);
                return RETRY;
            }
        };
        let catalog = shared.catalog();
        let found = cards::find_candidates(&lines, &catalog.augment_names, x as f64, y as f64);
        let row = cards::card_row(&found, self.height as f64);
        if shared.config().record_screenshots
            && let Err(e) = self.save_diagnostic(&pixels, &lines, &found, shared)
        {
            shared.log_error("cards diagnostic", e);
        }
        self.report_unlabeled(&row, shared);
        match self.tracker.observe(row, &found) {
            TrackerAction::Idle => {}
            TrackerAction::Hide => self.hide(),
            TrackerAction::Show(candidates) => match self.stats.as_ref().filter(|(read, _)| *read == (champion, mode)) {
                Some((_, stats)) => {
                    let cards = cards::rate_cards(&candidates, stats, &catalog.augments);
                    self.show(cards, champion, shared);
                }
                None => self.waiting = Some(candidates),
            },
        }
        if self.tracker.is_active() { WATCH_CARDS } else { WATCH_GAME }
    }

    /// Logs, once per stretch, a row of cards Xyra keeps recognizing without labeling, with a screenshot when enabled.
    fn report_unlabeled(&mut self, row: &[Candidate], shared: &Shared) {
        if row.is_empty() || !self.cards.is_empty() || self.waiting.is_some() {
            self.unlabeled_reads = 0;
            return;
        }
        self.unlabeled_reads = self.unlabeled_reads.saturating_add(1);
        if self.unlabeled_reads != UNLABELED_READS_TO_REPORT {
            return;
        }
        let catalog = shared.catalog();
        let names: Vec<String> = row.iter().map(|card| format!("{} @ {:.0},{:.0}", cards::name_of(&card.ids, &catalog.augments), card.x, card.y)).collect();
        shared.log_error("cards seen but not labeled", names.join(" | "));
        if shared.config().record_screenshots
            && let Err(e) = self.save_screenshot(shared)
        {
            shared.log_error("screenshot", e);
        }
    }

    fn show(&mut self, cards: Vec<Card>, champion: u32, shared: &Shared) {
        self.record_round(&cards);
        let (config, language) = (shared.config(), shared.language());
        if let Some(overlay) = &self.overlay {
            overlay.show(Labels { cards: cards.clone(), champion: shared.catalog().champion(champion).name, config: config.clone(), language });
        }
        if config.voice
            && let (Some(voice), Some(phrase)) = (&self.voice, voice::best_card_phrase(&cards, language))
            && let Err(e) = voice.speak(&phrase, language)
        {
            shared.log_error("voice", e);
        }
        if config.record_screenshots
            && let Err(e) = self.save_screenshot(shared)
        {
            shared.log_error("screenshot", e);
        }
        self.cards = cards;
    }

    /// Keeps the card area and every line the OCR read, with the names it matched, now and then during a game.
    fn save_diagnostic(&mut self, bgra: &[u8], lines: &[OcrLine], found: &[Candidate], shared: &Shared) -> Result<()> {
        let every = if found.is_empty() { DIAGNOSTIC_WITHOUT_CARDS } else { DIAGNOSTIC_WITH_CARDS };
        if self.diagnostics >= DIAGNOSTICS_PER_GAME || self.diagnosed_at.is_some_and(|at| at.elapsed() < every) {
            return Ok(());
        }
        self.diagnosed_at = Some(Instant::now());
        self.diagnostics += 1;
        let (_, _, width, height) = self.zone;
        let image = shared.storage.screenshot_named(DIAGNOSTIC_SUFFIX, "png")?;
        write_png(&image, bgra, width, height)?;
        let catalog = shared.catalog();
        let mut text = format!("zone {:?} screen {}x{}\n", self.zone, self.width, self.height);
        for line in lines {
            text.push_str(&format!("line {:.0},{:.0}-{:.0},{:.0} {}\n", line.x0, line.y0, line.x1, line.y1, line.text));
        }
        for card in found {
            text.push_str(&format!("found {} @ {:.0},{:.0}\n", cards::name_of(&card.ids, &catalog.augments), card.x, card.y));
        }
        std::fs::write(image.with_extension("txt"), text)?;
        Ok(())
    }

    fn save_screenshot(&self, shared: &Shared) -> Result<()> {
        let bgra = screen::capture(0, 0, self.width, self.height).map_err(AppError::platform)?;
        write_png(&shared.storage.screenshot_path()?, &bgra, self.width, self.height)
    }

    pub fn demo(&self, shared: &Shared) {
        if let Some(overlay) = &self.overlay {
            let cards = demo_cards(&shared.catalog().augments, self.width, self.height);
            overlay.demo(Labels { cards, champion: DEMO_CHAMPION.into(), config: shared.config(), language: shared.language() });
        }
    }

    pub fn test_voice(&self, shared: &Shared) {
        let language = shared.language();
        if let Some(voice) = &self.voice
            && let Err(e) = voice.speak(&i18n::t(language, VOICE_TEST), language)
        {
            shared.log_error("voice", e);
        }
    }
}

fn write_png(path: &std::path::Path, bgra: &[u8], width: i32, height: i32) -> Result<()> {
    let rgba: Vec<u8> = bgra.as_chunks::<4>().0.iter().flat_map(|p| [p[2], p[1], p[0], u8::MAX]).collect();
    let mut encoder = png::Encoder::new(BufWriter::new(File::create(path)?), width as u32, height as u32);
    encoder.set_color(png::ColorType::Rgba);
    encoder.write_header().and_then(|mut writer| writer.write_image_data(&rgba)).map_err(AppError::platform)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{collections::HashMap, env, fs, io::BufReader, thread};
    use windows::Win32::UI::HiDpi::{DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetProcessDpiAwarenessContext};
    use xyra_core::{cards::normalize, catalog::Catalog};

    const DEFAULT_PROBE_SECONDS: u64 = 300;

    const CAPTURE: &str = "../docs/cards-background.png";
    const CARDS: [(&str, u32); 3] = [("Juguito de Hechicería", 1), ("Inventor Supremo", 2), ("Interés en Llamas", 3)];

    #[test]
    #[ignore = "reads the live screen for XYRA_PROBE_SECONDS while a game shows augment cards"]
    fn probes_the_live_screen() {
        unsafe { SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) }.unwrap();
        screen::init_thread().unwrap();
        let seconds: u64 = env::var("XYRA_PROBE_SECONDS").ok().and_then(|s| s.parse().ok()).unwrap_or(DEFAULT_PROBE_SECONDS);
        let catalog: Catalog = serde_json::from_str(&fs::read_to_string(env::var("XYRA_CATALOG").unwrap()).unwrap()).unwrap();
        let (width, height) = screen::primary_size();
        let (x, y, zone_width, zone_height) = card_zone(width, height);
        let ocr = Ocr::new("es_MX").expect("a Windows OCR language");
        let mut tracker = CardTracker::default();
        let end = Instant::now() + Duration::from_secs(seconds);
        while Instant::now() < end {
            let started = Instant::now();
            let lines = ocr.read(&screen::capture(x, y, zone_width, zone_height).unwrap(), zone_width, zone_height).unwrap();
            let found = cards::find_candidates(&lines, &catalog.augment_names, x as f64, y as f64);
            let row = cards::card_row(&found, height as f64);
            let visible = screen::is_game_visible();
            let action = tracker.observe(row.clone(), &found);
            if !found.is_empty() || action != TrackerAction::Idle {
                let names: Vec<String> = found.iter().map(|c| format!("{:?}@({:.0},{:.0})", c.ids, c.x, c.y)).collect();
                let texts: Vec<&str> = lines.iter().map(|l| l.text.as_str()).collect();
                println!(
                    "{:?} visible={visible} found={names:?} row={} action={action:?} ocr={}ms lines={texts:?}",
                    Instant::now(),
                    row.len(),
                    started.elapsed().as_millis()
                );
            }
            thread::sleep(WATCH_GAME);
        }
    }

    #[test]
    #[ignore = "needs the Windows OCR of a language installed"]
    fn reads_every_card_of_a_real_capture() {
        screen::init_thread().unwrap();
        let mut reader = png::Decoder::new(BufReader::new(File::open(CAPTURE).unwrap())).read_info().unwrap();
        let mut pixels = vec![0; reader.output_buffer_size().unwrap()];
        let info = reader.next_frame(&mut pixels).unwrap();
        let channels = info.color_type.samples();
        let (x, y, width, height) = card_zone(info.width as i32, info.height as i32);
        let bgra: Vec<u8> = (y..y + height)
            .flat_map(|row| (x..x + width).map(move |column| (row as usize * info.width as usize + column as usize) * channels))
            .flat_map(|at| [pixels[at + 2], pixels[at + 1], pixels[at], u8::MAX])
            .collect();
        let lines = Ocr::new("es_MX").expect("a Windows OCR language").read(&bgra, width, height).unwrap();
        let names: HashMap<String, Vec<u32>> = CARDS.iter().map(|&(name, id)| (normalize(name), vec![id])).collect();
        let row = cards::card_row(&cards::find_candidates(&lines, &names, x as f64, y as f64), info.height as f64);
        assert_eq!(row.iter().map(|c| c.ids[0]).collect::<Vec<_>>(), [1, 2, 3]);
    }
}
