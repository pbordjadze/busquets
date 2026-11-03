//! Eleven-a-side agents that play on from any real moment: the what-ifs.
//!
//! Each player keeps their slot in the side's shape (track::shape_at: the shape slides with the ball
//! and squeezes without it), and when their side has the ball tries a few spots near it for the most
//! open lane from the ball. Without the ball, the nearest goes to it and the next covers the most
//! dangerous lane. The holder keeps it for a moment (their tempo), then passes to the teammate who
//! best answers what they value (an open lane, ground gained, a length they like, how much risk they
//! take), carries into space, or shoots when near enough. A pass meets the real opponents: the lane
//! model (pitch::lane) that decides whether it is open decides whether it is cut out.
use crate::track::{clamp_pitch, shape_at};
use pitch::lane::{openness, LaneParams};
use pitch::v2::Rng;
use pitch::{v2, V2};

/// What a player values on the ball; the defaults are a generic player.
#[derive(Clone, Copy, Debug)]
pub struct Traits {
    /// Seconds on the ball before releasing (mean); one-touch share.
    pub hold: f32,
    pub one_touch: f32,
    /// Weights: an open lane, ground gained (m), and the length they like (m) with its spread.
    pub w_open: f32,
    pub w_gain: f32,
    pub pref_len: f32,
    pub len_spread: f32,
    /// How much they want the lane open before they will play it (0..1).
    pub caution: f32,
    /// The choice's randomness.
    pub temp: f32,
}

impl Default for Traits {
    fn default() -> Traits {
        Traits { hold: 1.6, one_touch: 0.12, w_open: 2.0, w_gain: 0.06, pref_len: 18.0, len_spread: 14.0, caution: 0.35, temp: 0.35 }
    }
}

/// A change to the real match, from the moment you play on.
#[derive(Clone, Copy, Debug, Default)]
pub struct Tweak {
    /// Each side's shape pushed up (+, toward the other goal) or dropped, m.
    pub line: [f32; 2],
    /// How hard each side presses (1 = as usual).
    pub press: [f32; 2],
}

#[derive(Clone, Debug)]
pub struct Agent {
    pub side: u8,
    pub position: u32,
    pub pos: V2,
    pub vel: V2,
    pub traits: Traits,
    /// Which real person this is (into Game::people).
    pub person: usize,
    target: V2,
    retarget: f32,
}

#[derive(Clone, Copy, Debug)]
pub enum SimBall {
    Held { by: usize, t: f32, hold: f32 },
    Flight { from: usize, to: Option<usize>, a: V2, b: V2, t: f32, dur: f32, shot: bool },
    Loose { pos: V2, vel: V2 },
}

#[derive(Clone, Copy, Debug)]
pub enum SimEvent {
    Pass { from: usize, to: usize, a: V2, b: V2 },
    Cut { by: usize, at: V2 },
    Shot { by: usize, goal: bool },
}

#[derive(Clone)]
pub struct Sim {
    pub agents: Vec<Agent>,
    pub ball: SimBall,
    pub ball_pos: V2,
    pub t: f32,
    pub tweak: Tweak,
    pub events: Vec<SimEvent>,
    pub goals: [u32; 2],
    rng: Rng,
}

const RUN: f32 = 7.0;
const PASS_SPEED: f32 = 15.0;

fn lane_params() -> LaneParams {
    LaneParams { ball_speed: PASS_SPEED, presser_speed: 6.0, reach: 1.2, react: 0.25 }
}

impl Sim {
    /// From a real moment: positions and velocities from the tracks, the ball with whoever is nearest
    /// it (within 2.5 m), else loose.
    pub fn from_moment(g: &crate::Game, tr: &crate::Tracks, t: f32, tweak: Tweak, traits: &dyn Fn(usize) -> Traits, seed: u64) -> Sim {
        let mut agents = vec![];
        for (p, pe) in g.people.iter().enumerate() {
            if !pe.playing(t) {
                continue;
            }
            let pos = tr.pos_at(p, t);
            let vel = (tr.pos_at(p, t + 0.2) - tr.pos_at(p, t - 0.2)) * 2.5;
            agents.push(Agent { side: pe.side, position: pe.position_at(t), pos, vel, traits: traits(p), person: p, target: pos, retarget: 0.0 });
        }
        let b = tr.ball_at(t);
        let near = agents.iter().enumerate().map(|(i, a)| (i, a.pos.dist(b))).min_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
        let ball = match near {
            Some((i, d)) if d < 2.5 => SimBall::Held { by: i, t: 0.0, hold: 0.6 },
            _ => SimBall::Loose { pos: b, vel: (tr.ball_at(t + 0.2) - tr.ball_at(t - 0.2)) * 2.5 },
        };
        Sim { agents, ball, ball_pos: b, t, tweak, events: vec![], goals: [0, 0], rng: Rng::new(seed) }
    }

    pub fn holder(&self) -> Option<usize> {
        match self.ball {
            SimBall::Held { by, .. } => Some(by),
            _ => None,
        }
    }

    fn poss_side(&self) -> Option<u8> {
        match self.ball {
            SimBall::Held { by, .. } => Some(self.agents[by].side),
            SimBall::Flight { from, .. } => Some(self.agents[from].side),
            SimBall::Loose { .. } => None,
        }
    }

    fn opponents(&self, side: u8) -> Vec<Option<V2>> {
        self.agents.iter().map(|a| if a.side != side { Some(a.pos) } else { None }).collect()
    }

    /// The holder's options, scored by what they value (pass candidates only).
    pub fn options(&self, by: usize) -> Vec<(usize, f32)> {
        let h = &self.agents[by];
        let opp = self.opponents(h.side);
        let sg = if h.side == 0 { 1.0 } else { -1.0 };
        let tr = h.traits;
        let lp = lane_params();
        self.agents
            .iter()
            .enumerate()
            .filter(|(i, a)| *i != by && a.side == h.side)
            .map(|(i, a)| {
                let o = openness(h.pos, a.pos, &opp, &lp).open;
                let len = h.pos.dist(a.pos);
                let gain = (a.pos.x - h.pos.x) * sg;
                let s = tr.w_open * (o - tr.caution) + tr.w_gain * gain - ((len - tr.pref_len) / tr.len_spread).powi(2) * 0.5;
                (i, s)
            })
            .collect()
    }

    pub fn step(&mut self, dt: f32) {
        self.t += dt;
        let poss = self.poss_side();
        // where the ball will be
        let ball_now = self.ball_pos;
        // off the ball
        let n = self.agents.len();
        let opp_pos: [Vec<Option<V2>>; 2] = [self.opponents(0), self.opponents(1)];
        // each side's chasers: the nearest to the ball, then the next
        let mut chasers = [vec![], vec![]];
        for side in 0..2u8 {
            if poss == Some(side) {
                continue;
            }
            let mut d: Vec<(usize, f32)> = (0..n).filter(|&i| self.agents[i].side == side && self.agents[i].position != 1).map(|i| (i, self.agents[i].pos.dist(ball_now))).collect();
            d.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
            let k = if self.tweak.press[side as usize] > 1.2 { 3 } else { 2 };
            chasers[side as usize] = d.iter().take(k).map(|x| x.0).collect();
        }
        let holder = self.holder();
        for i in 0..n {
            let a = &self.agents[i];
            let side = a.side as usize;
            let mut target;
            let mut speed = RUN;
            if Some(i) == holder {
                target = a.pos + v2(if a.side == 0 { 1.0 } else { -1.0 }, 0.0) * 1.5;
                speed = 4.0;
            } else if let Some(k) = chasers[side].iter().position(|&c| c == i) {
                let press = self.tweak.press[side];
                if k == 0 {
                    target = ball_now;
                    speed = RUN * (0.85 + 0.15 * press);
                } else {
                    // cover: between the ball and their own goal, a step off
                    let goal = v2(if a.side == 0 { -52.5 } else { 52.5 }, 0.0);
                    target = ball_now.lerp(goal, 0.25 / press);
                }
            } else {
                let mut home = shape_at(a.position, a.side, ball_now, poss == Some(a.side));
                home.x += self.tweak.line[side] * if a.side == 0 { 1.0 } else { -1.0 } * if a.position == 1 { 0.3 } else { 1.0 };
                target = a.target;
                if poss == Some(a.side) && a.position != 1 {
                    // find space: every so often try spots near home for the most open lane
                    if a.retarget <= 0.0 {
                        let mut best = (home, -1.0);
                        for k in 0..6 {
                            let ang = k as f32 * 1.047 + self.t;
                            let q = clamp_pitch(home + V2::from_angle(ang) * 4.0);
                            let o = openness(ball_now, q, &opp_pos[side], &lane_params()).open;
                            if o > best.1 {
                                best = (q, o);
                            }
                        }
                        target = best.0;
                    }
                } else {
                    target = home;
                }
                if (target - home).len() > 8.0 {
                    target = home;
                }
            }
            let a = &mut self.agents[i];
            if a.retarget <= 0.0 {
                a.retarget = 0.8 + 0.4 * ((i * 7919) % 10) as f32 / 10.0;
            }
            a.retarget -= dt;
            a.target = target;
            let want = (target - a.pos).clamp_len(speed);
            let acc = 6.0;
            a.vel = a.vel + (want - a.vel).clamp_len(acc * dt);
            a.pos = clamp_pitch(a.pos + a.vel * dt);
        }
        // the ball
        match self.ball {
            SimBall::Held { by, t, hold } => {
                let t = t + dt;
                let h = &self.agents[by];
                self.ball_pos = h.pos + h.vel.norm() * 0.5;
                // pressed hard: let it go sooner
                let pressure = self.agents.iter().filter(|a| a.side != h.side && a.pos.dist(h.pos) < 2.0).count();
                if pressure > 0 && t > 0.3 {
                    // a tackle: the nearest presser takes it, sometimes
                    if self.rng.f() < dt * 0.6 * pressure as f32 {
                        let tk = (0..n).filter(|&j| self.agents[j].side != h.side).min_by(|&a, &b| self.agents[a].pos.dist(h.pos).partial_cmp(&self.agents[b].pos.dist(h.pos)).unwrap()).unwrap();
                        self.events.push(SimEvent::Cut { by: tk, at: h.pos });
                        self.ball = SimBall::Held { by: tk, t: 0.0, hold: self.draw_hold(tk) };
                        return;
                    }
                }
                if t >= hold {
                    self.decide(by);
                } else {
                    self.ball = SimBall::Held { by, t, hold };
                }
            }
            SimBall::Flight { from, to, a, b, t, dur, shot } => {
                let t = t + dt;
                let u = (t / dur).min(1.0);
                self.ball_pos = a.lerp(b, u);
                // cut out: the lane model, on the real positions now
                if !shot {
                    let side = self.agents[from].side;
                    for j in 0..n {
                        let q = &self.agents[j];
                        if q.side != side && q.pos.dist(self.ball_pos) < 1.3 && u > 0.08 {
                            self.events.push(SimEvent::Cut { by: j, at: self.ball_pos });
                            self.ball = SimBall::Held { by: j, t: 0.0, hold: self.draw_hold(j) };
                            return;
                        }
                    }
                }
                if u >= 1.0 {
                    if shot {
                        let side = self.agents[from].side as usize;
                        let goal = self.rng.f() < 0.3;
                        self.events.push(SimEvent::Shot { by: from, goal });
                        if goal {
                            self.goals[side] += 1;
                        }
                        // the keeper has it either way
                        let gk = (0..n).find(|&j| self.agents[j].side as usize != side && self.agents[j].position == 1).unwrap_or(0);
                        self.ball = SimBall::Held { by: gk, t: 0.0, hold: 2.0 };
                    } else if let Some(to) = to {
                        self.ball = SimBall::Held { by: to, t: 0.0, hold: self.draw_hold(to) };
                    } else {
                        self.ball = SimBall::Loose { pos: b, vel: (b - a).norm() * 3.0 };
                    }
                } else {
                    self.ball = SimBall::Flight { from, to, a, b, t, dur, shot };
                }
            }
            SimBall::Loose { pos, vel } => {
                let pos = pos + vel * dt;
                let vel = vel * (1.0 - 1.2 * dt).max(0.0);
                self.ball_pos = clamp_pitch(pos);
                let near = (0..n).min_by(|&a, &b| self.agents[a].pos.dist(pos).partial_cmp(&self.agents[b].pos.dist(pos)).unwrap()).unwrap();
                if self.agents[near].pos.dist(pos) < 1.0 {
                    self.ball = SimBall::Held { by: near, t: 0.0, hold: self.draw_hold(near) };
                } else {
                    self.ball = SimBall::Loose { pos: self.ball_pos, vel };
                }
            }
        }
    }

    fn draw_hold(&mut self, by: usize) -> f32 {
        let tr = self.agents[by].traits;
        if self.rng.f() < tr.one_touch {
            0.15
        } else {
            (tr.hold * (0.5 + self.rng.f())).max(0.3)
        }
    }

    fn decide(&mut self, by: usize) {
        let h = self.agents[by].clone();
        let sg = if h.side == 0 { 1.0 } else { -1.0 };
        let goal = v2(52.5 * sg, 0.0);
        // a shot, when near and central enough
        if h.pos.dist(goal) < 22.0 && (h.pos.y.abs() < 14.0) && self.rng.f() < 0.5 {
            let aim = goal + v2(0.0, self.rng.s() * 3.0);
            self.ball = SimBall::Flight { from: by, to: None, a: h.pos, b: aim, t: 0.0, dur: h.pos.dist(aim) / 24.0, shot: true };
            return;
        }
        let opts = self.options(by);
        if opts.is_empty() {
            return;
        }
        let temp = h.traits.temp.max(0.05);
        let m = opts.iter().map(|o| o.1).fold(f32::MIN, f32::max);
        let ws: Vec<f32> = opts.iter().map(|o| ((o.1 - m) / temp).exp()).collect();
        let sum: f32 = ws.iter().sum();
        let mut r = self.rng.f() * sum;
        let mut pick = opts[0].0;
        for (o, w) in opts.iter().zip(&ws) {
            r -= w;
            if r <= 0.0 {
                pick = o.0;
                break;
            }
        }
        let to = &self.agents[pick];
        // lead the receiver a little
        let b = clamp_pitch(to.pos + to.vel * 0.4);
        let dur = h.pos.dist(b) / PASS_SPEED;
        self.events.push(SimEvent::Pass { from: by, to: pick, a: h.pos, b });
        self.ball = SimBall::Flight { from: by, to: Some(pick), a: h.pos, b, t: 0.0, dur: dur.max(0.2), shot: false };
    }
}
