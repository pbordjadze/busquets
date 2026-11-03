//! StatsBomb open data, the parts we read (https://github.com/statsbomb/open-data, its doc/ specs).
//! Locations are StatsBomb's 120 x 80 pitch, from the acting team's side (it attacks x = 120; y grows
//! to its right).
use serde::Deserialize;

#[derive(Deserialize, Clone, Debug)]
pub struct Named {
    pub id: u32,
    pub name: String,
}

#[derive(Deserialize, Clone, Debug, Default)]
pub struct Tactics {
    #[serde(default)]
    pub formation: u32,
    #[serde(default)]
    pub lineup: Vec<LineupEntry>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct LineupEntry {
    pub player: Named,
    pub position: Named,
    #[serde(default)]
    pub jersey_number: u32,
}

#[derive(Deserialize, Clone, Debug, Default)]
pub struct PassDetail {
    pub recipient: Option<Named>,
    pub end_location: Option<Vec<f32>>,
    pub outcome: Option<Named>,
    pub height: Option<Named>,
    pub body_part: Option<Named>,
    pub technique: Option<Named>,
    #[serde(rename = "type")]
    pub kind: Option<Named>,
    #[serde(default)]
    pub switch: bool,
    #[serde(default)]
    pub cross: bool,
    #[serde(default)]
    pub through_ball: bool,
}

#[derive(Deserialize, Clone, Debug, Default)]
pub struct EndDetail {
    pub end_location: Option<Vec<f32>>,
    pub outcome: Option<Named>,
}

#[derive(Deserialize, Clone, Debug, Default)]
pub struct SubDetail {
    pub replacement: Option<Named>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct Event {
    pub id: String,
    pub index: u32,
    pub period: u8,
    pub timestamp: String,
    #[serde(rename = "type")]
    pub kind: Named,
    pub team: Named,
    pub possession_team: Option<Named>,
    pub player: Option<Named>,
    pub position: Option<Named>,
    pub location: Option<Vec<f32>>,
    #[serde(default)]
    pub duration: f32,
    #[serde(default)]
    pub under_pressure: bool,
    pub play_pattern: Option<Named>,
    pub pass: Option<PassDetail>,
    pub carry: Option<EndDetail>,
    pub shot: Option<EndDetail>,
    pub substitution: Option<SubDetail>,
    pub tactics: Option<Tactics>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct Dot {
    pub teammate: bool,
    pub actor: bool,
    pub keeper: bool,
    pub location: Vec<f32>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct Frame {
    pub event_uuid: String,
    #[serde(default)]
    pub visible_area: Vec<f32>,
    pub freeze_frame: Vec<Dot>,
}

/// "hh:mm:ss.mmm" to seconds.
pub fn seconds(ts: &str) -> f32 {
    let mut s = 0.0f64;
    for part in ts.split(':') {
        s = s * 60.0 + part.parse::<f64>().unwrap_or(0.0);
    }
    s as f32
}
