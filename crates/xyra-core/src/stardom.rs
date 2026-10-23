use crate::{
    errors::{AppError, Result},
    league::{Lcu, asset_url},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use ts_rs::TS;

/// ARAM: Mayhem's progression of the season, in the client's language; empty when no season runs.
const HUB: &str = "/lol-game-data/assets/v1/kiwi-hub.json";
const MISSIONS: &str = "/lol-missions/v1/missions";
const COMPLETED: &str = "COMPLETED";
const PENDING: &str = "PENDING";
/// Fame missions are told apart by their series name, like "Mayhem_S3_Series", which changes with each season.
const FAME_SERIES: &str = "Mayhem_";

fn progress_endpoint(track: &str) -> String {
    format!("/lol-progression/v1/groups/{track}/instanceData")
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Hub {
    progress_track: Track,
    modes_objective_group_list: GroupList,
    #[serde(default)]
    main_progress_milestone_ranks: Vec<RankName>,
}

#[derive(Deserialize)]
struct Track {
    id: String,
}

#[derive(Deserialize)]
struct GroupList {
    #[serde(rename = "ModesObjectiveGroups")]
    groups: Vec<Group>,
}

#[derive(Deserialize)]
struct Group {
    #[serde(rename = "displayData")]
    display: GroupDisplay,
    #[serde(rename = "ModesObjectiveList", default)]
    objectives: Vec<Objective>,
    #[serde(default)]
    locked: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GroupDisplay {
    name: String,
    #[serde(default)]
    logo_image_path: String,
    progress_track: Track,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Objective {
    title: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    missions: Vec<Track>,
    next_level_objective: Option<Box<Objective>>,
}

#[derive(Deserialize)]
struct RankName {
    rank: String,
}

#[derive(Default, Deserialize)]
struct Progress {
    #[serde(default)]
    counters: Vec<Counter>,
    #[serde(default)]
    milestones: Vec<Milestone>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Counter {
    counter_value: u32,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Milestone {
    trigger_value: u32,
}

impl Progress {
    fn value(&self) -> u32 {
        self.counters.first().map_or(0, |counter| counter.counter_value)
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Mission {
    id: String,
    status: String,
    #[serde(default)]
    series_name: String,
    #[serde(default)]
    objectives: Vec<MissionObjective>,
    #[serde(default)]
    rewards: Vec<MissionReward>,
    #[serde(default)]
    end_time: u64,
}

#[derive(Deserialize)]
struct MissionObjective {
    #[serde(default)]
    description: String,
    #[serde(default)]
    progress: MissionProgress,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MissionProgress {
    current_progress: u32,
    total_count: u32,
}

#[derive(Deserialize)]
struct MissionReward {
    #[serde(default)]
    description: String,
}

/// The player's road to stardom in ARAM: Mayhem: fame, rank and how far they went with each band.
#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct Stardom {
    pub fame: u32,
    pub rank: String,
    /// The next rank and the fame it starts at; None at the top.
    pub next: Option<StardomRank>,
    pub bands: Vec<Band>,
    /// Pending missions that give fame, like the daily one.
    pub missions: Vec<FameMission>,
}

#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct StardomRank {
    pub name: String,
    pub fame: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct Band {
    pub name: String,
    pub logo: String,
    pub milestones: u32,
    pub total: u32,
    pub locked: bool,
    pub auditions: Vec<Audition>,
}

/// An audition mission at the level the player is on.
#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct Audition {
    pub title: String,
    pub description: String,
    pub state: AuditionState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub enum AuditionState {
    Done,
    Open,
    Locked,
}

#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct FameMission {
    pub text: String,
    pub progress: u32,
    pub total: u32,
    pub reward: String,
    /// Milliseconds since the epoch; 0 when it does not expire.
    pub ends_at: u64,
}

/// The player's progress, or None when ARAM: Mayhem has no season running.
pub fn read(lcu: &Lcu) -> Result<Option<Stardom>> {
    let hubs: Vec<Hub> = match lcu.get_as(HUB) {
        Ok(hubs) => hubs,
        // Between seasons the file is missing or empty.
        Err(AppError::Client(_) | AppError::ClientFormat(_)) => return Ok(None),
        Err(e) => return Err(e),
    };
    let Some(hub) = hubs.into_iter().next() else { return Ok(None) };
    let main: Progress = lcu.get_as(&progress_endpoint(&hub.progress_track.id))?;
    let bands = hub
        .modes_objective_group_list
        .groups
        .iter()
        .map(|group| lcu.get_as(&progress_endpoint(&group.display.progress_track.id)).unwrap_or_default())
        .collect();
    let missions: Vec<Mission> = lcu.get_as(MISSIONS)?;
    Ok(Some(stardom(hub, &main, bands, &missions)))
}

fn stardom(hub: Hub, main: &Progress, bands: Vec<Progress>, missions: &[Mission]) -> Stardom {
    let status: HashMap<&str, &str> = missions.iter().map(|mission| (mission.id.as_str(), mission.status.as_str())).collect();
    let mut thresholds: Vec<u32> = main.milestones.iter().map(|milestone| milestone.trigger_value).collect();
    thresholds.sort_unstable();
    thresholds.dedup();
    let fame = main.value();
    let reached = thresholds.iter().filter(|&&threshold| threshold <= fame).count();
    let names: Vec<String> = hub.main_progress_milestone_ranks.into_iter().map(|rank| rank.rank).collect();
    Stardom {
        fame,
        rank: names.get(reached).cloned().unwrap_or_default(),
        next: thresholds.get(reached).zip(names.get(reached + 1)).map(|(&fame, name)| StardomRank { name: name.clone(), fame }),
        bands: hub
            .modes_objective_group_list
            .groups
            .into_iter()
            .zip(bands)
            .map(|(group, progress)| Band {
                name: group.display.name,
                logo: if group.display.logo_image_path.is_empty() { String::new() } else { asset_url(&group.display.logo_image_path) },
                milestones: progress.value(),
                total: progress.milestones.len() as u32,
                locked: group.locked,
                auditions: group.objectives.iter().map(|objective| audition(objective, &status)).collect(),
            })
            .collect(),
        missions: missions
            .iter()
            .filter(|mission| mission.series_name.starts_with(FAME_SERIES) && mission.status == PENDING)
            .filter_map(|mission| {
                let objective = mission.objectives.first()?;
                Some(FameMission {
                    text: objective.description.clone(),
                    progress: objective.progress.current_progress,
                    total: objective.progress.total_count,
                    reward: mission.rewards.first().map(|reward| reward.description.clone()).unwrap_or_default(),
                    ends_at: mission.end_time,
                })
            })
            .collect(),
    }
}

/// The first level of an objective the player has not completed, or the last one when every level is done.
fn audition(objective: &Objective, status: &HashMap<&str, &str>) -> Audition {
    let state_of = |level: &Objective| {
        let states: Vec<&str> = level.missions.iter().map(|mission| status.get(mission.id.as_str()).copied().unwrap_or_default()).collect();
        if !states.is_empty() && states.iter().all(|&state| state == COMPLETED) {
            AuditionState::Done
        } else if states.contains(&PENDING) {
            AuditionState::Open
        } else {
            AuditionState::Locked
        }
    };
    let mut level = objective;
    while state_of(level) == AuditionState::Done
        && let Some(next) = &level.next_level_objective
        && state_of(next) != AuditionState::Locked
    {
        level = next;
    }
    Audition { title: level.title.clone(), description: level.description.clone(), state: state_of(level) }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HUB_JSON: &str = r#"[{"progressTrack":{"id":"main"},"mainProgressMilestoneRanks":[{"rank":"Principiante"},{"rank":"Reconocido"},{"rank":"Fama Discreta"}],
        "modesObjectiveGroupList":{"ModesObjectiveGroups":[{"displayData":{"name":"Pentakill","logoImagePath":"/lol-game-data/assets/ASSETS/UX/P.png","progressTrack":{"id":"p"}},"locked":false,
        "ModesObjectiveList":[{"title":"Baila","description":"d","missions":[{"id":"m1"}],"nextLevelObjective":null},
        {"title":"Acierta 45 tiros","missions":[{"id":"m2"}],"nextLevelObjective":{"title":"Acierta 65 tiros","missions":[{"id":"m3"}],"nextLevelObjective":null}},
        {"title":"Nieve 8","missions":[{"id":"m4"}],"nextLevelObjective":{"title":"Nieve 12","missions":[{"id":"m5"}],"nextLevelObjective":null}}]}]}}]"#;

    fn progress(value: u32, triggers: &[u32]) -> Progress {
        Progress { counters: vec![Counter { counter_value: value }], milestones: triggers.iter().map(|&trigger_value| Milestone { trigger_value }).collect() }
    }

    #[test]
    fn reads_fame_bands_and_missions() {
        let hub: Vec<Hub> = serde_json::from_str(HUB_JSON).unwrap();
        let missions: Vec<Mission> = serde_json::from_str(
            r#"[{"id":"m1","status":"COMPLETED"},{"id":"m2","status":"COMPLETED"},{"id":"m3","status":"PENDING"},{"id":"m4","status":"COMPLETED"},{"id":"m5","status":"NOT_STARTED"},
            {"id":"f","status":"PENDING","seriesName":"Mayhem_S3_Series","endTime":5,"objectives":[{"description":"Juega como Tirador","progress":{"currentProgress":0,"totalCount":1}}],"rewards":[{"description":"50 de Fama"}]},
            {"id":"o","status":"PENDING","seriesName":"TFT18_Series","objectives":[{"description":"TFT"}]}]"#,
        )
        .unwrap();
        let main = progress(1988, &[1000, 1000, 3200, 3200, 5800]);
        let result = stardom(hub.into_iter().next().unwrap(), &main, vec![progress(5, &[1, 2, 3, 4, 5, 6])], &missions);
        assert_eq!((result.fame, result.rank.as_str()), (1988, "Reconocido"));
        assert_eq!(result.next, Some(StardomRank { name: "Fama Discreta".into(), fame: 3200 }));
        let band = &result.bands[0];
        assert_eq!((band.name.as_str(), band.milestones, band.total, band.locked), ("Pentakill", 5, 6, false));
        let auditions: Vec<(&str, AuditionState)> = band.auditions.iter().map(|a| (a.title.as_str(), a.state)).collect();
        assert_eq!(auditions, [("Baila", AuditionState::Done), ("Acierta 65 tiros", AuditionState::Open), ("Nieve 8", AuditionState::Done)]);
        assert_eq!(result.missions, [FameMission { text: "Juega como Tirador".into(), progress: 0, total: 1, reward: "50 de Fama".into(), ends_at: 5 }]);
    }

    #[test]
    fn the_top_rank_has_no_next() {
        let hub: Vec<Hub> = serde_json::from_str(HUB_JSON).unwrap();
        let result = stardom(hub.into_iter().next().unwrap(), &progress(9000, &[1000, 3200]), vec![], &[]);
        assert_eq!((result.rank.as_str(), result.next), ("Fama Discreta", None));
    }
}
