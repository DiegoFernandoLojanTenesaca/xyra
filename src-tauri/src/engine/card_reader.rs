use super::Shared;
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
    time::{Duration, Instant},
};
use xyra_core::{
    cards::{self, Candidate, Card, CardTracker, TrackerAction},
    catalog::NamedAssets,
    errors::{AppError, Result},
    i18n,
    model::GameMode,
    opgg::{self, AugmentStat},
};

const WATCH_CARDS: Duration = Duration::from_millis(250);
const WATCH_GAME: Duration = Duration::from_millis(700);
const UNFOCUSED: Duration = Duration::from_secs(1);
const RETRY: Duration = Duration::from_secs(5);
/// Where the cards appear, as fractions of the screen: left, top, width, height.
const CARD_ZONE: (f64, f64, f64, f64) = (0.125, 0.1, 0.75, 0.7);
const REFERENCE_HEIGHT: f64 = 1200.0;
const DEMO_CHAMPION: &str = "Brand";
/// Demo cards measured in a real game (docs/cards-background.png): augment id, x offset from center, tier, performance.
const DEMO_CARDS: [(u32, f64, Option<u8>, f64); 3] = [(1211, -410.0, Some(0), 83.0), (1098, 0.0, Some(5), 70.0), (2128, 410.0, Some(0), 91.0)];
const DEMO_ROW: f64 = 0.4175;
const VOICE_TEST: &str = "overlay:voice.right";

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
    next_read: Option<Instant>,
    cards: Vec<Card>,
}

impl CardReader {
    pub fn new(shared: &Arc<Shared>) -> CardReader {
        let (width, height) = screen::primary_size();
        let fraction = |part: f64, of: i32| (part * of as f64) as i32;
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
            zone: (fraction(CARD_ZONE.0, width), fraction(CARD_ZONE.1, height), fraction(CARD_ZONE.2, width), fraction(CARD_ZONE.3, height)),
            tracker: CardTracker::default(),
            stats: None,
            next_read: None,
            cards: Vec::new(),
        }
    }

    pub fn ocr_language(&self) -> Option<String> {
        self.ocr.as_ref().map(|o| o.language.clone())
    }

    pub fn cards(&self) -> &[Card] {
        &self.cards
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
    }

    pub fn read(&mut self, champion: u32, mode: GameMode, shared: &Shared) {
        self.next_read = Some(Instant::now() + self.read_once(champion, mode, shared));
    }

    /// Returns how long to wait before the next read.
    fn read_once(&mut self, champion: u32, mode: GameMode, shared: &Shared) -> Duration {
        let stats = match self.stats.take() {
            Some((key, stats)) if key == (champion, mode) => stats,
            _ => match opgg::fetch_augments(&shared.opgg, champion, mode) {
                Ok(stats) => stats,
                Err(e) => {
                    shared.log_error("OP.GG augments", e);
                    return RETRY;
                }
            },
        };
        let wait = self.read_screen(champion, &stats, shared);
        self.stats = Some(((champion, mode), stats));
        wait
    }

    fn read_screen(&mut self, champion: u32, stats: &HashMap<u32, AugmentStat>, shared: &Shared) -> Duration {
        let Some(ocr) = &self.ocr else { return RETRY };
        if !screen::is_game_focused() {
            self.hide();
            return UNFOCUSED;
        }
        let (x, y, width, height) = self.zone;
        let lines = match screen::capture(x, y, width, height).and_then(|pixels| ocr.read(&pixels, width, height)) {
            Ok(lines) => lines,
            Err(e) => {
                shared.log_error("screen reading", e);
                return RETRY;
            }
        };
        let catalog = shared.catalog();
        let candidates = cards::card_row(&cards::find_candidates(&lines, &catalog.augment_names, x as f64, y as f64), self.height as f64);
        match self.tracker.observe(candidates) {
            TrackerAction::Idle => {}
            TrackerAction::Hide => self.hide(),
            TrackerAction::Show(candidates) => self.show(cards::rate_cards(&candidates, stats, &catalog.augments), champion, shared),
        }
        if self.tracker.is_active() { WATCH_CARDS } else { WATCH_GAME }
    }

    fn show(&mut self, cards: Vec<Card>, champion: u32, shared: &Shared) {
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

    fn save_screenshot(&self, shared: &Shared) -> Result<()> {
        let bgra = screen::capture(0, 0, self.width, self.height).map_err(AppError::platform)?;
        let rgba: Vec<u8> = bgra.as_chunks::<4>().0.iter().flat_map(|p| [p[2], p[1], p[0], u8::MAX]).collect();
        let file = File::create(shared.storage.screenshot_path()?)?;
        let mut encoder = png::Encoder::new(BufWriter::new(file), self.width as u32, self.height as u32);
        encoder.set_color(png::ColorType::Rgba);
        encoder.write_header().and_then(|mut writer| writer.write_image_data(&rgba)).map_err(AppError::platform)
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
