//! What makes a player that player, from their events: where they receive, how often and how they
//! pass, their tempo (receipt to release), how much risk they take, how they play under pressure.
//! Measured the same way for a reference group (other central midfielders in the same matches), so
//! what is his shows as a difference.
use crate::game::{EvKind, Game};

#[derive(Clone, Debug, Default)]
pub struct Profile {
    pub minutes: f32,
    pub receipts: usize,
    pub passes: usize,
    pub completed: usize,
    /// Receipt to his next pass in the same move, seconds (those under 6 s).
    pub release: Vec<f32>,
    pub lengths: Vec<f32>,
    /// Pass direction relative to the attack: forward (> +45 deg), sideways, back (> 135 deg).
    pub forward: usize,
    pub sideways: usize,
    pub back: usize,
    /// Passes gaining 10 m or more toward goal.
    pub progressive: usize,
    pub lofted: usize,
    pub pressed: usize,
    pub pressed_completed: usize,
    /// Where he receives: x from his own goal line (m), y from the middle (m).
    pub receive_x: Vec<f32>,
    pub receive_y: Vec<f32>,
    pub carries: usize,
    pub carry_m: f32,
    pub dispossessed: usize,
}

impl Profile {
    pub fn add(&mut self, g: &Game, who: &dyn Fn(usize) -> bool) {
        // minutes on the pitch
        for (i, p) in g.people.iter().enumerate() {
            if who(i) {
                self.minutes += (p.off.min(g.end) - p.on).max(0.0) / 60.0;
            }
        }
        let evs = &g.events;
        for (k, e) in evs.iter().enumerate() {
            let Some(w) = e.who else { continue };
            if !who(w) {
                continue;
            }
            let sg = if e.side == 0 { 1.0 } else { -1.0 };
            match e.kind {
                EvKind::Receipt => {
                    self.receipts += 1;
                    if let Some(at) = e.at {
                        self.receive_x.push(at.x * sg + 52.5);
                        self.receive_y.push(at.y * sg);
                    }
                    // his next pass in the same move
                    for f in evs[k + 1..].iter().take(12) {
                        if f.who == Some(w) && f.kind == EvKind::Pass {
                            let dt = f.t - e.t;
                            if dt < 6.0 {
                                self.release.push(dt);
                            }
                            break;
                        }
                        if f.who.is_some() && f.who != Some(w) && f.kind.on_ball() {
                            break;
                        }
                    }
                }
                EvKind::Pass if !e.restart => {
                    let (Some(a), Some(b)) = (e.at, e.end) else { continue };
                    self.passes += 1;
                    if e.ok {
                        self.completed += 1;
                    }
                    let d = (b - a) * sg;
                    self.lengths.push(d.len());
                    let ang = d.y.atan2(d.x).abs().to_degrees();
                    if ang < 45.0 {
                        self.forward += 1;
                    } else if ang > 135.0 {
                        self.back += 1;
                    } else {
                        self.sideways += 1;
                    }
                    if d.x >= 10.0 {
                        self.progressive += 1;
                    }
                    if e.height > 0 {
                        self.lofted += 1;
                    }
                    if e.pressed {
                        self.pressed += 1;
                        if e.ok {
                            self.pressed_completed += 1;
                        }
                    }
                }
                EvKind::Carry => {
                    if let (Some(a), Some(b)) = (e.at, e.end) {
                        self.carries += 1;
                        self.carry_m += a.dist(b);
                    }
                }
                EvKind::Dispossessed | EvKind::Miscontrol => self.dispossessed += 1,
                _ => {}
            }
        }
    }

    fn median(v: &[f32]) -> f32 {
        if v.is_empty() {
            return 0.0;
        }
        let mut s = v.to_vec();
        s.sort_by(|a, b| a.partial_cmp(b).unwrap());
        s[s.len() / 2]
    }

    /// The numbers that describe him, by name, as one row.
    pub fn row(&self) -> Vec<(&'static str, f32)> {
        let p = self.passes.max(1) as f32;
        let per90 = 90.0 / self.minutes.max(1.0);
        let quick = self.release.iter().filter(|&&t| t < 1.2).count() as f32 / self.release.len().max(1) as f32;
        vec![
            ("passes per 90", self.passes as f32 * per90),
            ("receipts per 90", self.receipts as f32 * per90),
            ("completion %", 100.0 * self.completed as f32 / p),
            ("completion under pressure %", 100.0 * self.pressed_completed as f32 / self.pressed.max(1) as f32),
            ("passes under pressure %", 100.0 * self.pressed as f32 / p),
            ("median receipt-to-release s", Self::median(&self.release)),
            ("released within 1.2 s %", 100.0 * quick),
            ("median pass length m", Self::median(&self.lengths)),
            ("forward %", 100.0 * self.forward as f32 / p),
            ("sideways %", 100.0 * self.sideways as f32 / p),
            ("back %", 100.0 * self.back as f32 / p),
            ("progressive (10 m+) %", 100.0 * self.progressive as f32 / p),
            ("lofted %", 100.0 * self.lofted as f32 / p),
            ("median receive x from own goal m", Self::median(&self.receive_x)),
            ("median |receive y| from middle m", Self::median(&self.receive_y.iter().map(|y| y.abs()).collect::<Vec<_>>())),
            ("carry m per 90", self.carry_m * per90),
            ("lost (dispossessed+miscontrol) per 90", self.dispossessed as f32 * per90),
        ]
    }

    pub fn median_release(&self) -> f32 {
        Self::median(&self.release)
    }
    pub fn quick_share(&self) -> f32 {
        self.release.iter().filter(|&&t| t < 0.7).count() as f32 / self.release.len().max(1) as f32
    }
}
