use crate::{
    model::{Quality, grade},
    opgg::AugmentStat,
};
use serde::Serialize;
use std::collections::HashMap;
use ts_rs::TS;
use unicode_normalization::UnicodeNormalization;

const MIN_NAME_CHARS: usize = 4;
const MIN_NAME_SIMILARITY: f64 = 0.8;
const ROW_TOLERANCE: f64 = 0.03;
const MIN_ROW_CARDS: usize = 2;
const GOOD_TIER: u8 = 2;
const REROLL_TIER: u8 = 4;
const STILL_TOLERANCE_PX: f64 = 8.0;
/// Two names closer than this horizontally belong to the same card slot.
const SLOT_TOLERANCE_PX: f64 = 100.0;
const MISSES_TO_HIDE: u8 = 3;

pub struct OcrLine {
    pub text: String,
    pub x0: f64,
    pub x1: f64,
    pub y0: f64,
    pub y1: f64,
}

impl OcrLine {
    fn center(&self) -> f64 {
        (self.x0 + self.x1) / 2.0
    }

    /// `below` continues this line: it starts right under it, around the same center.
    fn continued_by(&self, below: &OcrLine) -> bool {
        let gap = below.y0 - self.y1;
        let height = self.y1 - self.y0;
        (-height / 2.0..height).contains(&gap) && (self.center() - below.center()).abs() < (self.x1 - self.x0).max(below.x1 - below.x0) / 2.0
    }
}

pub fn normalize(text: &str) -> String {
    let kept: String = text.to_lowercase().nfkd().filter(|c| c.is_alphanumeric() || *c == ' ').collect();
    kept.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// `ids` holds several augments when Arena and ARAM: Mayhem share a name.
#[derive(Clone, Debug, PartialEq)]
pub struct Candidate {
    pub ids: Vec<u32>,
    pub x: f64,
    pub y: f64,
}

/// Card names read in single lines, or in two stacked lines when the name wraps.
pub fn find_candidates(lines: &[OcrLine], names: &HashMap<String, Vec<u32>>, dx: f64, dy: f64) -> Vec<Candidate> {
    let wrapped =
        lines.iter().flat_map(|top| lines.iter().filter(|below| top.continued_by(below)).map(move |below| (top, format!("{} {}", top.text, below.text))));
    lines
        .iter()
        .map(|line| (line, line.text.clone()))
        .chain(wrapped)
        .filter_map(|(line, raw)| {
            let text = normalize(&raw);
            if text.chars().count() < MIN_NAME_CHARS {
                return None;
            }
            let (name, similarity) = names.keys().map(|known| (known, strsim::normalized_levenshtein(&text, known))).max_by(|a, b| a.1.total_cmp(&b.1))?;
            (similarity >= MIN_NAME_SIMILARITY).then(|| Candidate { ids: names[name].clone(), x: dx + line.center(), y: dy + line.y1 })
        })
        .collect()
}

pub fn card_row(candidates: &[Candidate], screen_height: f64) -> Vec<Candidate> {
    let mut sorted = candidates.to_vec();
    sorted.sort_by(|a, b| a.x.total_cmp(&b.x));
    let mut largest: Vec<Candidate> = Vec::new();
    for anchor in candidates {
        let mut row: Vec<Candidate> = Vec::new();
        for other in &sorted {
            if (other.y - anchor.y).abs() < screen_height * ROW_TOLERANCE && !row.iter().any(|r| r.ids == other.ids) {
                row.push(other.clone());
            }
        }
        if row.len() > largest.len() {
            largest = row;
        }
    }
    if largest.len() >= MIN_ROW_CARDS { largest } else { Vec::new() }
}

#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct Card {
    pub id: u32,
    pub name: String,
    pub icon: String,
    /// 0 = S … 6 = F; None when OP.GG has no games of this augment with the champion.
    pub tier: Option<u8>,
    pub quality: Quality,
    pub grade: String,
    pub performance: f64,
    pub rank: u32,
    pub best: bool,
    pub reroll: bool,
    pub x: f64,
    pub y: f64,
}

pub fn rate_cards(candidates: &[Candidate], stats: &HashMap<u32, AugmentStat>, augments: &HashMap<u32, (String, String)>) -> Vec<Card> {
    let mut cards: Vec<Card> = candidates
        .iter()
        .map(|candidate| {
            let id = candidate.ids.iter().copied().find(|i| stats.contains_key(i)).unwrap_or(candidate.ids[0]);
            let stat = stats.get(&id);
            let (name, icon) = augments.get(&id).cloned().unwrap_or_default();
            Card {
                id,
                name,
                icon,
                tier: stat.map(|s| s.tier),
                quality: Quality::of(stat.map(|s| s.tier)),
                grade: grade(stat.map(|s| s.tier)),
                performance: stat.map_or(0.0, |s| s.performance),
                rank: 0,
                best: false,
                reroll: false,
                x: candidate.x,
                y: candidate.y,
            }
        })
        .collect();
    let mut order: Vec<usize> = (0..cards.len()).collect();
    order.sort_by(|&a, &b| {
        let (x, y) = (&cards[a], &cards[b]);
        (x.tier.is_none(), x.tier.unwrap_or(0)).cmp(&(y.tier.is_none(), y.tier.unwrap_or(0))).then(y.performance.total_cmp(&x.performance))
    });
    for (position, &index) in order.iter().enumerate() {
        cards[index].rank = position as u32 + 1;
    }
    let has_good = cards.iter().any(|c| c.tier.is_some_and(|t| t <= GOOD_TIER));
    for card in &mut cards {
        card.best = card.rank == 1 && card.tier.is_some();
        card.reroll = has_good && card.tier.is_none_or(|t| t >= REROLL_TIER);
    }
    cards
}

#[derive(Debug, PartialEq)]
pub enum TrackerAction {
    Idle,
    Show(Vec<Candidate>),
    Hide,
}

/// Follows the card row across reads that may miss names: still cards replace the shown card of their slot.
#[derive(Default)]
pub struct CardTracker {
    shown: Vec<Candidate>,
    previous: Vec<Candidate>,
    misses: u8,
}

fn same_card(a: &Candidate, b: &Candidate) -> bool {
    a.ids == b.ids && (a.x - b.x).abs() < STILL_TOLERANCE_PX && (a.y - b.y).abs() < STILL_TOLERANCE_PX
}

fn same_slot(a: &Candidate, b: &Candidate) -> bool {
    (a.x - b.x).abs() < SLOT_TOLERANCE_PX
}

impl CardTracker {
    /// `row` is the card row of this read and `found` every card name the read recognized.
    pub fn observe(&mut self, row: Vec<Candidate>, found: &[Candidate]) -> TrackerAction {
        let confirmed: Vec<Candidate> = row.into_iter().filter(|card| self.previous.iter().any(|seen| same_card(seen, card))).collect();
        let shown_visible = found.iter().any(|card| self.shown.iter().any(|shown| same_card(shown, card)));
        self.previous = found.to_vec();
        if confirmed.is_empty() && !shown_visible {
            if self.shown.is_empty() {
                return TrackerAction::Idle;
            }
            self.misses += 1;
            if self.misses < MISSES_TO_HIDE {
                return TrackerAction::Idle;
            }
            *self = CardTracker::default();
            return TrackerAction::Hide;
        }
        self.misses = 0;
        let needed = if self.shown.is_empty() { MIN_ROW_CARDS } else { 1 };
        if confirmed.len() < needed {
            return TrackerAction::Idle;
        }
        let mut merged: Vec<Candidate> = self.shown.iter().filter(|shown| !confirmed.iter().any(|card| same_slot(card, shown))).cloned().collect();
        merged.extend(confirmed);
        merged.sort_by(|a, b| a.x.total_cmp(&b.x));
        if merged.iter().map(|c| &c.ids).eq(self.shown.iter().map(|c| &c.ids)) {
            return TrackerAction::Idle;
        }
        self.shown = merged.clone();
        TrackerAction::Show(merged)
    }

    pub fn is_active(&self) -> bool {
        !self.shown.is_empty() || !self.previous.is_empty()
    }

    pub fn reset(&mut self) {
        *self = CardTracker::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(text: &str, x: f64, y: f64) -> OcrLine {
        OcrLine { text: text.into(), x0: x, x1: x + 120.0, y0: y, y1: y + 20.0 }
    }

    fn candidate(id: u32, x: f64) -> Candidate {
        Candidate { ids: vec![id], x, y: 500.0 }
    }

    #[test]
    fn normalizes_names() {
        assert_eq!(normalize("¡Tirador Mágico!"), "tirador magico");
        assert_eq!(normalize("  Interés   en Llamas "), "interes en llamas");
    }

    #[test]
    fn finds_filters_and_rates_cards() {
        let names: HashMap<String, Vec<u32>> =
            [(normalize("¡BONK!"), vec![2111]), (normalize("Tirador Mágico"), vec![129, 1129]), (normalize("Golpe Místico"), vec![1058])].into();
        let ocr = [
            line("BONK!", 100.0, 480.0),
            line("TIRADOR MAGICO", 500.0, 482.0),
            line("Golpe Mistico", 900.0, 481.0),
            line("Golpe Místico", 300.0, 900.0),
            line("Tienda", 50.0, 480.0),
        ];
        let row = card_row(&find_candidates(&ocr, &names, 0.0, 0.0), 1200.0);
        assert_eq!(row.iter().map(|c| c.ids.clone()).collect::<Vec<_>>(), vec![vec![2111], vec![129, 1129], vec![1058]]);

        let stats: HashMap<u32, AugmentStat> =
            [(2111, AugmentStat { tier: 1, performance: 91.0, pick_rate: 1.0 }), (1129, AugmentStat { tier: 0, performance: 92.0, pick_rate: 1.0 })].into();
        let rated = rate_cards(&row, &stats, &HashMap::new());
        let summary: Vec<_> = rated.iter().map(|c| (c.id, c.tier, c.rank, c.best, c.reroll)).collect();
        assert_eq!(summary, vec![(2111, Some(1), 2, false, false), (1129, Some(0), 1, true, false), (1058, None, 3, false, true)]);
    }

    #[test]
    fn single_name_is_not_a_row() {
        let names: HashMap<String, Vec<u32>> = [(normalize("Golpe Místico"), vec![1058])].into();
        assert!(card_row(&find_candidates(&[line("Golpe Mistico", 0.0, 0.0)], &names, 0.0, 0.0), 1200.0).is_empty());
    }

    fn observe(tracker: &mut CardTracker, cards: &[Candidate]) -> TrackerAction {
        let row = card_row(cards, 1200.0);
        tracker.observe(row, cards)
    }

    #[test]
    fn tracker_shows_still_cards_and_hides_after_three_empty_reads() {
        let mut tracker = CardTracker::default();
        assert_eq!(observe(&mut tracker, &[candidate(1, 380.0), candidate(2, 900.0)]), TrackerAction::Idle);
        assert_eq!(observe(&mut tracker, &[candidate(1, 400.0), candidate(2, 920.0)]), TrackerAction::Idle);
        let still = [candidate(1, 402.0), candidate(2, 921.0)];
        assert_eq!(observe(&mut tracker, &still), TrackerAction::Show(still.to_vec()));
        assert_eq!(observe(&mut tracker, &still), TrackerAction::Idle);
        assert_eq!(observe(&mut tracker, &[]), TrackerAction::Idle);
        assert_eq!(observe(&mut tracker, &still), TrackerAction::Idle);
        assert_eq!(observe(&mut tracker, &[]), TrackerAction::Idle);
        assert_eq!(observe(&mut tracker, &[]), TrackerAction::Idle);
        assert_eq!(observe(&mut tracker, &[]), TrackerAction::Hide);
        assert!(!tracker.is_active());
    }

    #[test]
    fn tracker_keeps_labels_through_partial_reads() {
        let mut tracker = CardTracker::default();
        let row = [candidate(1, 400.0), candidate(2, 900.0), candidate(3, 1400.0)];
        observe(&mut tracker, &row);
        assert_eq!(observe(&mut tracker, &row), TrackerAction::Show(row.to_vec()));
        for _ in 0..5 {
            assert_eq!(observe(&mut tracker, &[candidate(3, 1400.0)]), TrackerAction::Idle);
            assert_eq!(observe(&mut tracker, &[candidate(1, 400.0), candidate(3, 1400.0)]), TrackerAction::Idle);
        }
        assert!(tracker.is_active());
    }

    #[test]
    fn tracker_relabels_a_rerolled_card_even_when_reads_are_partial() {
        let mut tracker = CardTracker::default();
        let row = [candidate(1, 400.0), candidate(2, 900.0), candidate(3, 1400.0)];
        observe(&mut tracker, &row);
        observe(&mut tracker, &row);
        let rerolled = [candidate(1, 400.0), candidate(4, 900.0)];
        assert_eq!(observe(&mut tracker, &rerolled), TrackerAction::Idle);
        assert_eq!(observe(&mut tracker, &rerolled), TrackerAction::Show(vec![candidate(1, 400.0), candidate(4, 900.0), candidate(3, 1400.0)]));
    }

    #[test]
    fn a_shown_name_elsewhere_does_not_keep_the_labels() {
        let mut tracker = CardTracker::default();
        let row = [candidate(1, 400.0), candidate(2, 900.0)];
        observe(&mut tracker, &row);
        observe(&mut tracker, &row);
        for _ in 0..2 {
            assert_eq!(observe(&mut tracker, &[candidate(1, 1700.0)]), TrackerAction::Idle);
        }
        assert_eq!(observe(&mut tracker, &[candidate(1, 1700.0)]), TrackerAction::Hide);
    }

    #[test]
    fn joins_names_that_wrap_to_two_lines() {
        let names: HashMap<String, Vec<u32>> = [(normalize("¡Comienza a Emocionarte!"), vec![7])].into();
        let lines = [
            OcrLine { text: "¡Comienza a".into(), x0: 100.0, x1: 220.0, y0: 480.0, y1: 500.0 },
            OcrLine { text: "Emocionarte!".into(), x0: 105.0, x1: 215.0, y0: 504.0, y1: 524.0 },
        ];
        let found = find_candidates(&lines, &names, 0.0, 0.0);
        assert_eq!(found, [Candidate { ids: vec![7], x: 160.0, y: 500.0 }]);
    }
}
