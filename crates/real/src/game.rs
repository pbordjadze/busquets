//! One match as a timeline in metres: who played (and when), every event, and the 360 freeze-frames.
//! The focus side (Busquets' when he played, else the home side) attacks +x in both halves; y grows to
//! its left. Time `t` runs on across the periods (penalty shoot-outs are left out).
use crate::sb;
use pitch::{v2, V2};
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvKind {
    Pass,
    Receipt,
    Carry,
    Shot,
    Pressure,
    Duel,
    Recovery,
    Clearance,
    Interception,
    Keeper,
    Dribble,
    Miscontrol,
    Dispossessed,
    Block,
    Foul,
    Other,
}

impl EvKind {
    fn of(name: &str) -> EvKind {
        match name {
            "Pass" => EvKind::Pass,
            "Ball Receipt*" => EvKind::Receipt,
            "Carry" => EvKind::Carry,
            "Shot" => EvKind::Shot,
            "Pressure" => EvKind::Pressure,
            "Duel" => EvKind::Duel,
            "Ball Recovery" => EvKind::Recovery,
            "Clearance" => EvKind::Clearance,
            "Interception" => EvKind::Interception,
            "Goal Keeper" => EvKind::Keeper,
            "Dribble" => EvKind::Dribble,
            "Miscontrol" => EvKind::Miscontrol,
            "Dispossessed" => EvKind::Dispossessed,
            "Block" => EvKind::Block,
            "Foul Committed" | "Foul Won" => EvKind::Foul,
            _ => EvKind::Other,
        }
    }
    /// Its location is the ball's (not a presser's, not a foul's).
    pub fn on_ball(self) -> bool {
        matches!(
            self,
            EvKind::Pass | EvKind::Receipt | EvKind::Carry | EvKind::Shot | EvKind::Recovery | EvKind::Clearance | EvKind::Interception | EvKind::Keeper | EvKind::Dribble | EvKind::Miscontrol | EvKind::Dispossessed
        )
    }
}

#[derive(Clone, Debug)]
pub struct Person {
    pub id: u32,
    pub name: String,
    /// The name a shirt would carry (the last word, or a known one).
    pub short: String,
    /// 0 the focus side, 1 the other.
    pub side: u8,
    pub jersey: u32,
    /// StatsBomb position ids over time: (from t, position id).
    pub positions: Vec<(f32, u32)>,
    /// On the pitch from, to (s).
    pub on: f32,
    pub off: f32,
}

impl Person {
    pub fn position_at(&self, t: f32) -> u32 {
        let mut p = self.positions.first().map(|p| p.1).unwrap_or(0);
        for &(t0, id) in &self.positions {
            if t0 <= t {
                p = id;
            }
        }
        p
    }
    pub fn playing(&self, t: f32) -> bool {
        t >= self.on && t < self.off
    }
}

#[derive(Clone, Debug)]
pub struct Ev {
    pub t: f32,
    pub period: u8,
    pub kind: EvKind,
    pub side: u8,
    /// The side in possession.
    pub poss: u8,
    pub who: Option<usize>,
    pub at: Option<V2>,
    pub end: Option<V2>,
    pub dur: f32,
    /// A pass completed, a shot on target, a carry kept (true unless StatsBomb gives an outcome).
    pub ok: bool,
    /// A pass's named recipient.
    pub to: Option<usize>,
    pub pressed: bool,
    /// Index into `Game::frames`.
    pub frame: Option<usize>,
    /// A pass: 0 ground, 1 low, 2 high.
    pub height: u8,
    /// A pass from a restart (corner, free kick, throw-in, goal kick, kick-off).
    pub restart: bool,
    /// A goal (a shot's outcome).
    pub goal: bool,
}

#[derive(Clone, Debug)]
pub struct Dot {
    pub at: V2,
    /// 0 the focus side, 1 the other.
    pub side: u8,
    pub actor: bool,
    pub keeper: bool,
}

#[derive(Clone, Debug)]
pub struct Frame {
    pub event: usize,
    /// The camera's visible area, a polygon in metres.
    pub visible: Vec<V2>,
    pub dots: Vec<Dot>,
}

#[derive(Clone, Debug)]
pub struct Game {
    pub teams: [String; 2],
    pub people: Vec<Person>,
    pub events: Vec<Ev>,
    pub frames: Vec<Frame>,
    /// Each period's (start, end) t.
    pub periods: Vec<(f32, f32)>,
    pub end: f32,
    /// Goals: when, and for which side.
    pub goals: Vec<(f32, u8)>,
}

/// StatsBomb's 120 x 80 (from the acting side) to metres (the focus side attacking +x).
pub fn to_m(l: &[f32], flip: bool) -> V2 {
    let p = v2((l[0] - 60.0) * crate::LEN / 120.0, (40.0 - l[1]) * crate::WID / 80.0);
    if flip {
        -p
    } else {
        p
    }
}

fn short_name(full: &str) -> String {
    const KNOWN: [(&str, &str); 6] = [("Sergio Busquets", "Busquets"), ("Lionel Andrés Messi", "Messi"), ("Pedro González López", "Pedri"), ("Pablo Martín Páez Gavira", "Gavi"), ("Jordi Alba", "Alba"), ("Gerard Piqué", "Piqué")];
    for (k, v) in KNOWN {
        if full.starts_with(k) {
            return v.to_string();
        }
    }
    full.split_whitespace().last().unwrap_or(full).to_string()
}

impl Game {
    /// Reads a match: its events, and its 360 frames when there are any. The focus side is the one
    /// `focus` (a player id) played for, else the first listed.
    pub fn read(events: &[u8], frames: Option<&[u8]>, focus: u32) -> anyhow::Result<Game> {
        let raw: Vec<sb::Event> = serde_json::from_slice(events)?;
        let mut raw: Vec<sb::Event> = raw.into_iter().filter(|e| e.period <= 4).collect();
        raw.sort_by(|a, b| (a.period, sb::seconds(&a.timestamp), a.index).partial_cmp(&(b.period, sb::seconds(&b.timestamp), b.index)).unwrap());
        // the sides
        let xis: Vec<&sb::Event> = raw.iter().filter(|e| e.kind.name == "Starting XI").collect();
        anyhow::ensure!(xis.len() == 2, "a match needs two starting elevens");
        let plays_for = |e: &sb::Event| e.tactics.as_ref().map(|t| t.lineup.iter().any(|l| l.player.id == focus)).unwrap_or(false);
        let any_focus = raw.iter().any(|e| e.player.as_ref().map(|p| p.id) == Some(focus));
        let focus_team = if xis.iter().any(|e| plays_for(e)) {
            xis.iter().find(|e| plays_for(e)).unwrap().team.id
        } else if any_focus {
            raw.iter().find(|e| e.player.as_ref().map(|p| p.id) == Some(focus)).unwrap().team.id
        } else {
            xis[0].team.id
        };
        let side_of = |team: u32| if team == focus_team { 0u8 } else { 1u8 };
        let mut teams = [String::new(), String::new()];
        for x in &xis {
            teams[side_of(x.team.id) as usize] = x.team.name.clone();
        }
        // time: each period runs on from the last
        let mut period_len: HashMap<u8, f32> = HashMap::new();
        for e in &raw {
            let s = sb::seconds(&e.timestamp) + e.duration.max(0.0);
            let m = period_len.entry(e.period).or_insert(0.0);
            *m = m.max(s);
        }
        let mut ps: Vec<u8> = period_len.keys().copied().collect();
        ps.sort();
        let mut start = HashMap::new();
        let mut periods = vec![];
        let mut acc = 0.0;
        for p in ps {
            start.insert(p, acc);
            periods.push((acc, acc + period_len[&p]));
            acc += period_len[&p];
        }
        let end = acc;
        let time = |e: &sb::Event| start.get(&e.period).copied().unwrap_or(0.0) + sb::seconds(&e.timestamp);
        // people
        let mut people: Vec<Person> = vec![];
        let mut index: HashMap<u32, usize> = HashMap::new();
        for x in &xis {
            for l in &x.tactics.as_ref().unwrap().lineup {
                index.insert(l.player.id, people.len());
                people.push(Person {
                    id: l.player.id,
                    short: short_name(&l.player.name),
                    name: l.player.name.clone(),
                    side: side_of(x.team.id),
                    jersey: l.jersey_number,
                    positions: vec![(0.0, l.position.id)],
                    on: 0.0,
                    off: end + 1.0,
                });
            }
        }
        for e in &raw {
            let t = time(e);
            match e.kind.name.as_str() {
                "Substitution" => {
                    let (Some(off), Some(rep)) = (&e.player, e.substitution.as_ref().and_then(|s| s.replacement.clone())) else { continue };
                    let pos = e.position.as_ref().map(|p| p.id).unwrap_or(0);
                    if let Some(&i) = index.get(&off.id) {
                        people[i].off = t;
                    }
                    let i = *index.entry(rep.id).or_insert_with(|| {
                        people.push(Person { id: rep.id, short: short_name(&rep.name), name: rep.name.clone(), side: side_of(e.team.id), jersey: 0, positions: vec![], on: t, off: end + 1.0 });
                        people.len() - 1
                    });
                    people[i].on = t;
                    people[i].positions.push((t, pos));
                }
                "Tactical Shift" => {
                    if let Some(tc) = &e.tactics {
                        for l in &tc.lineup {
                            if let Some(&i) = index.get(&l.player.id) {
                                people[i].positions.push((t, l.position.id));
                                if people[i].jersey == 0 {
                                    people[i].jersey = l.jersey_number;
                                }
                            }
                        }
                    }
                }
                "Bad Behaviour" | "Foul Committed" => {
                    // a red card sends them off: StatsBomb puts it under the event's own detail; read it loosely
                }
                _ => {}
            }
        }
        // events
        let mut events = vec![];
        let mut by_uuid: HashMap<&str, usize> = HashMap::new();
        for e in &raw {
            let kind = EvKind::of(&e.kind.name);
            let side = side_of(e.team.id);
            let flip = side == 1;
            let who = e.player.as_ref().and_then(|p| index.get(&p.id).copied());
            let at = e.location.as_ref().filter(|l| l.len() >= 2).map(|l| to_m(l, flip));
            let (mut end_l, mut ok, mut to, mut height, mut goal) = (None, true, None, 0u8, false);
            if let Some(p) = &e.pass {
                end_l = p.end_location.as_ref().map(|l| to_m(l, flip));
                ok = p.outcome.is_none();
                to = p.recipient.as_ref().and_then(|r| index.get(&r.id).copied());
                height = match p.height.as_ref().map(|h| h.id) {
                    Some(2) => 1,
                    Some(3) => 2,
                    _ => 0,
                };
            }
            if let Some(c) = &e.carry {
                end_l = c.end_location.as_ref().map(|l| to_m(l, flip));
            }
            if let Some(s) = &e.shot {
                end_l = s.end_location.as_ref().filter(|l| l.len() >= 2).map(|l| to_m(l, flip));
                goal = s.outcome.as_ref().map(|o| o.name == "Goal").unwrap_or(false);
                ok = goal || s.outcome.as_ref().map(|o| o.name == "Saved").unwrap_or(false);
            }
            let restart = e.pass.as_ref().and_then(|p| p.kind.as_ref()).map(|k| k.name != "Recovery" && k.name != "Interception").unwrap_or(false);
            let poss = e.possession_team.as_ref().map(|t| side_of(t.id)).unwrap_or(side);
            by_uuid.insert(e.id.as_str(), events.len());
            events.push(Ev { t: time(e), period: e.period, kind, side, poss, who, at, end: end_l, dur: e.duration.max(0.0), ok, to, pressed: e.under_pressure, frame: None, height, restart, goal });
        }
        let mut goals: Vec<(f32, u8)> = events.iter().filter(|e| e.goal).map(|e| (e.t, e.side)).collect();
        for e in &raw {
            if e.kind.name == "Own Goal For" {
                goals.push((time(e), side_of(e.team.id)));
            }
        }
        goals.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        // the 360 frames
        let mut out_frames = vec![];
        if let Some(fr) = frames {
            let raw_f: Vec<sb::Frame> = serde_json::from_slice(fr)?;
            for f in raw_f {
                let Some(&ei) = by_uuid.get(f.event_uuid.as_str()) else { continue };
                let flip = events[ei].side == 1;
                let visible = f.visible_area.chunks(2).filter(|c| c.len() == 2).map(|c| to_m(c, flip)).collect();
                let dots = f
                    .freeze_frame
                    .iter()
                    .filter(|d| d.location.len() >= 2)
                    .map(|d| Dot { at: to_m(&d.location, flip), side: if d.teammate { events[ei].side } else { 1 - events[ei].side }, actor: d.actor, keeper: d.keeper })
                    .collect();
                events[ei].frame = Some(out_frames.len());
                out_frames.push(Frame { event: ei, visible, dots });
            }
        }
        Ok(Game { teams, people, events, frames: out_frames, periods, end, goals })
    }

    /// The score at `t`.
    pub fn score(&self, t: f32) -> [u32; 2] {
        let mut s = [0, 0];
        for &(gt, side) in &self.goals {
            if gt <= t {
                s[side as usize] += 1;
            }
        }
        s
    }

    pub fn find(&self, id: u32) -> Option<usize> {
        self.people.iter().position(|p| p.id == id)
    }

    /// The last event at or before `t`.
    pub fn event_at(&self, t: f32) -> usize {
        match self.events.binary_search_by(|e| e.t.partial_cmp(&t).unwrap()) {
            Ok(i) => i,
            Err(i) => i.saturating_sub(1),
        }
    }

    /// The match clock at `t` ("67:12"), as a broadcast shows it.
    pub fn clock(&self, t: f32) -> (u32, u32) {
        let base = [0.0, 45.0, 90.0, 105.0];
        for (k, &(a, b)) in self.periods.iter().enumerate() {
            if t < b || k + 1 == self.periods.len() {
                let s = base.get(k).copied().unwrap_or(0.0) * 60.0 + (t - a).max(0.0);
                return ((s / 60.0) as u32, (s % 60.0) as u32);
            }
        }
        (0, 0)
    }
}
