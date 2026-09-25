use crate::{
    model::{grade, Quality},
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
const MISSES_TO_HIDE: u8 = 2;

pub struct OcrLine {
    pub text: String,
    pub x0: f64,
    pub x1: f64,
    pub y1: f64,
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

pub fn find_candidates(lines: &[OcrLine], names: &HashMap<String, Vec<u32>>, dx: f64, dy: f64) -> Vec<Candidate> {
    lines
        .iter()
        .filter_map(|line| {
            let text = normalize(&line.text);
            if text.chars().count() < MIN_NAME_CHARS {
                return None;
            }
            let (name, similarity) = names
                .keys()
                .map(|known| (known, strsim::normalized_levenshtein(&text, known)))
                .max_by(|a, b| a.1.total_cmp(&b.1))?;
            (similarity >= MIN_NAME_SIMILARITY).then(|| Candidate { ids: names[name].clone(), x: dx + (line.x0 + line.x1) / 2.0, y: dy + line.y1 })
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
    if largest.len() >= MIN_ROW_CARDS {
        largest
    } else {
        Vec::new()
    }
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
        (x.tier.is_none(), x.tier.unwrap_or(0))
            .cmp(&(y.tier.is_none(), y.tier.unwrap_or(0)))
            .then(y.performance.total_cmp(&x.performance))
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

/// Shows labels once the cards stop moving between two reads and hides them after two empty reads.
#[derive(Default)]
pub struct CardTracker {
    shown: Vec<Vec<u32>>,
    pending: Option<Vec<Candidate>>,
    misses: u8,
}

fn same_cards(a: &[Candidate], b: &[Candidate]) -> bool {
    a.len() == b.len()
        && a.iter()
            .zip(b)
            .all(|(a, b)| a.ids == b.ids && (a.x - b.x).abs() < STILL_TOLERANCE_PX && (a.y - b.y).abs() < STILL_TOLERANCE_PX)
}

impl CardTracker {
    pub fn observe(&mut self, candidates: Vec<Candidate>) -> TrackerAction {
        if candidates.is_empty() {
            self.pending = None;
            if self.shown.is_empty() {
                return TrackerAction::Idle;
            }
            self.misses += 1;
            if self.misses < MISSES_TO_HIDE {
                return TrackerAction::Idle;
            }
            self.shown.clear();
            return TrackerAction::Hide;
        }
        self.misses = 0;
        let ids: Vec<Vec<u32>> = candidates.iter().map(|c| c.ids.clone()).collect();
        if ids == self.shown {
            return TrackerAction::Idle;
        }
        if self.pending.as_deref().is_some_and(|p| same_cards(p, &candidates)) {
            self.pending = None;
            self.shown = ids;
            return TrackerAction::Show(candidates);
        }
        self.pending = Some(candidates);
        TrackerAction::Idle
    }

    pub fn is_active(&self) -> bool {
        !self.shown.is_empty() || self.pending.is_some()
    }

    pub fn reset(&mut self) {
        *self = CardTracker::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(text: &str, x: f64, y: f64) -> OcrLine {
        OcrLine { text: text.into(), x0: x, x1: x + 120.0, y1: y + 20.0 }
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
        let names: HashMap<String, Vec<u32>> = [
            (normalize("¡BONK!"), vec![2111]),
            (normalize("Tirador Mágico"), vec![129, 1129]),
            (normalize("Golpe Místico"), vec![1058]),
        ]
        .into();
        let ocr = [
            line("BONK!", 100.0, 480.0),
            line("TIRADOR MAGICO", 500.0, 482.0),
            line("Golpe Mistico", 900.0, 481.0),
            line("Golpe Místico", 300.0, 900.0),
            line("Tienda", 50.0, 480.0),
        ];
        let row = card_row(&find_candidates(&ocr, &names, 0.0, 0.0), 1200.0);
        assert_eq!(row.iter().map(|c| c.ids.clone()).collect::<Vec<_>>(), vec![vec![2111], vec![129, 1129], vec![1058]]);

        let stats: HashMap<u32, AugmentStat> = [
            (2111, AugmentStat { tier: 1, performance: 91.0, pick_rate: 1.0 }),
            (1129, AugmentStat { tier: 0, performance: 92.0, pick_rate: 1.0 }),
        ]
        .into();
        let rated = rate_cards(&row, &stats, &HashMap::new());
        let summary: Vec<_> = rated.iter().map(|c| (c.id, c.tier, c.rank, c.best, c.reroll)).collect();
        assert_eq!(summary, vec![(2111, Some(1), 2, false, false), (1129, Some(0), 1, true, false), (1058, None, 3, false, true)]);
    }

    #[test]
    fn single_name_is_not_a_row() {
        let names: HashMap<String, Vec<u32>> = [(normalize("Golpe Místico"), vec![1058])].into();
        assert!(card_row(&find_candidates(&[line("Golpe Mistico", 0.0, 0.0)], &names, 0.0, 0.0), 1200.0).is_empty());
    }

    #[test]
    fn tracker_waits_for_still_cards_and_hides_quickly() {
        let mut tracker = CardTracker::default();
        assert_eq!(tracker.observe(vec![candidate(1, 380.0), candidate(2, 900.0)]), TrackerAction::Idle);
        assert_eq!(tracker.observe(vec![candidate(1, 400.0), candidate(2, 920.0)]), TrackerAction::Idle);
        let still = vec![candidate(1, 402.0), candidate(2, 921.0)];
        assert_eq!(tracker.observe(still.clone()), TrackerAction::Show(still.clone()));
        assert_eq!(tracker.observe(still.clone()), TrackerAction::Idle);
        assert_eq!(tracker.observe(vec![]), TrackerAction::Idle);
        assert_eq!(tracker.observe(still.clone()), TrackerAction::Idle);
        assert_eq!(tracker.observe(vec![]), TrackerAction::Idle);
        assert_eq!(tracker.observe(vec![]), TrackerAction::Hide);
        assert!(!tracker.is_active());
        tracker.observe(still.clone());
        tracker.observe(still);
        let rerolled = vec![candidate(1, 402.0), candidate(3, 921.0)];
        assert_eq!(tracker.observe(rerolled.clone()), TrackerAction::Idle);
        assert_eq!(tracker.observe(rerolled.clone()), TrackerAction::Show(rerolled));
    }
}
