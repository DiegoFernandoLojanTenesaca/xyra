use crate::{
    errors::Result,
    league::{Lcu, asset_url},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use ts_rs::TS;

const CHALLENGES: &str = "/lol-challenges/v1/challenges/local-player";
const SUMMARY: &str = "/lol-challenges/v1/summary-player-data/local-player";
/// How many of the challenges closest to their next level are shown.
const CLOSEST: usize = 12;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SummaryAnswer {
    overall_challenge_level: String,
    #[serde(default)]
    total_challenge_score: u32,
    #[serde(default)]
    points_until_next_rank: u32,
    #[serde(default)]
    position_percentile: f64,
    #[serde(default)]
    category_progress: Vec<CategoryAnswer>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CategoryAnswer {
    category: String,
    level: String,
    current: u32,
    max: u32,
    #[serde(default)]
    position_percentile: f64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ChallengeAnswer {
    id: u64,
    name: String,
    #[serde(default)]
    description: String,
    category: String,
    current_level: String,
    #[serde(default)]
    next_level: String,
    current_value: f64,
    #[serde(default)]
    current_threshold: f64,
    #[serde(default)]
    next_threshold: f64,
    #[serde(default)]
    is_capstone: bool,
    #[serde(default)]
    is_reverse_direction: bool,
    #[serde(default)]
    retire_timestamp: u64,
    #[serde(default)]
    level_to_icon_path: HashMap<String, String>,
}

/// The account's challenges: its overall level, each category and those about to level up.
#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct Challenges {
    /// Riot's tier names in capitals, like "DIAMOND".
    pub level: String,
    pub points: u32,
    pub points_to_next: u32,
    /// Share of players ahead, 0-100.
    pub top_percent: f64,
    pub categories: Vec<ChallengeCategory>,
    /// Closest to their next level first.
    pub closest: Vec<ChallengeProgress>,
}

#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct ChallengeCategory {
    /// Like "TEAMWORK".
    pub category: String,
    pub level: String,
    pub points: u32,
    pub max: u32,
    pub top_percent: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct ChallengeProgress {
    #[ts(type = "number")]
    pub id: u64,
    pub name: String,
    pub description: String,
    pub category: String,
    pub level: String,
    pub next_level: String,
    pub value: f64,
    pub next_value: f64,
    /// How much of the way from the current level to the next one is done, 0-100.
    pub progress: f64,
    /// The token of the next level.
    pub icon: String,
}

/// Reads the account's challenges from the client.
pub fn read(lcu: &Lcu) -> Result<Challenges> {
    let summary: SummaryAnswer = lcu.get_as(SUMMARY)?;
    let challenges: HashMap<String, ChallengeAnswer> = lcu.get_as(CHALLENGES)?;
    Ok(summarize(summary, challenges.into_values().collect()))
}

fn summarize(summary: SummaryAnswer, challenges: Vec<ChallengeAnswer>) -> Challenges {
    let mut closest: Vec<ChallengeProgress> = challenges.into_iter().filter_map(progress).collect();
    closest.sort_by(|a, b| b.progress.total_cmp(&a.progress));
    closest.truncate(CLOSEST);
    Challenges {
        level: summary.overall_challenge_level,
        points: summary.total_challenge_score,
        points_to_next: summary.points_until_next_rank,
        top_percent: summary.position_percentile,
        categories: summary
            .category_progress
            .into_iter()
            .map(|c| ChallengeCategory { category: c.category, level: c.level, points: c.current, max: c.max, top_percent: c.position_percentile })
            .collect(),
        closest,
    }
}

/// An active challenge still climbing, with how far its next level is; None for finished, retired, grouping or
/// countdown challenges.
fn progress(challenge: ChallengeAnswer) -> Option<ChallengeProgress> {
    let span = challenge.next_threshold - challenge.current_threshold;
    let climbing = !challenge.next_level.is_empty() && !challenge.is_capstone && !challenge.is_reverse_direction && challenge.retire_timestamp == 0;
    if !climbing || span <= 0.0 || challenge.current_value >= challenge.next_threshold {
        return None;
    }
    Some(ChallengeProgress {
        icon: challenge.level_to_icon_path.get(&challenge.next_level).map(|path| asset_url(path)).unwrap_or_default(),
        progress: ((challenge.current_value - challenge.current_threshold) / span * 100.0).clamp(0.0, 100.0),
        id: challenge.id,
        name: challenge.name,
        description: challenge.description,
        category: challenge.category,
        level: challenge.current_level,
        next_level: challenge.next_level,
        value: challenge.current_value,
        next_value: challenge.next_threshold,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::league::parse;
    use serde_json::json;

    fn challenge(id: u64, value: f64, next_level: &str, retired: u64) -> serde_json::Value {
        json!({
            "id": id, "name": format!("C{id}"), "category": "TEAMWORK", "currentLevel": "GOLD", "nextLevel": next_level,
            "currentValue": value, "currentThreshold": 10.0, "nextThreshold": 20.0, "isCapstone": false,
            "isReverseDirection": false, "retireTimestamp": retired,
            "levelToIconPath": { "PLATINUM": "/lol-game-data/assets/ASSETS/Challenges/Config/1/Tokens/PLATINUM.png" }
        })
    }

    #[test]
    fn keeps_the_climbing_challenges_closest_first() {
        let summary: SummaryAnswer =
            parse(SUMMARY, &json!({ "overallChallengeLevel": "DIAMOND", "totalChallengeScore": 100, "categoryProgress": [] })).unwrap();
        let answers: HashMap<String, ChallengeAnswer> = parse(
            CHALLENGES,
            &json!({ "1": challenge(1, 12.0, "PLATINUM", 0), "2": challenge(2, 19.0, "PLATINUM", 0), "3": challenge(3, 19.0, "", 0), "4": challenge(4, 19.0, "PLATINUM", 5) }),
        )
        .unwrap();
        let challenges = summarize(summary, answers.into_values().collect());
        assert_eq!(challenges.closest.iter().map(|c| (c.id, c.progress.round())).collect::<Vec<_>>(), [(2, 90.0), (1, 20.0)]);
        assert!(challenges.closest[0].icon.ends_with("/assets/challenges/config/1/tokens/platinum.png"));
    }
}
