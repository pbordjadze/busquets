//! The world: you, your teammates, the pressers and the ball, stepped on simple rules.
//!
//! - Teammates hold a shape (their home slots, turning slowly, following the ball a little) and offer
//!   an angle: every so often each tries a few spots near home and moves to the one with the most
//!   open lane from the ball.
//! - Pressers split the work: the nearest goes to the ball, the others each stand in the most open
//!   lane left; they react late and a little at random. A ball that passes within reach of one is
//!   poked away, rolls loose, and the nearest of your side collects it.
//! - A teammate with the ball touches it for a moment and passes, most often back to you when your
//!   lane is open. You never have to: with the ball you decide, at your pace.
//! - Time: the world's pace is 1 while the ball moves between others, softens as a pass comes to you,
//!   and all but holds while you have it (`Config::hold_pace`).
//! - Seeing: you see what is in front of you (your facing, toward the ball) and what a glance sweeps;
//!   everyone else you remember where you last saw them (`Player::seen`). The lanes you are shown are
//!   reckoned from what you remember; the ball meets the real pressers.

use crate::lane::{openness, LaneParams, Openness};
use crate::v2::{angle_diff, closest_on_segment, smoothstep, v2, Rng, V2};
use serde::{Deserialize, Serialize};
use std::f32::consts::PI;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    /// Your home, and your teammates' home slots, relative to the shape's centre (y forward), m.
    pub you_home: V2,
    pub shape: Vec<V2>,
    /// Pressers, and where they start (relative to the centre).
    pub pressers: Vec<V2>,
    /// Half the side of the square everyone stays in, m.
    pub area: f32,
    pub pass_speed: f32,
    pub mate_speed: f32,
    pub presser_speed: f32,
    pub reach: f32,
    pub react: f32,
    /// The world's pace while you have the ball (1 = real time).
    pub hold_pace: f32,
    /// World seconds before a pass reaches you that time begins to soften.
    pub soften: f32,
    /// How long a teammate keeps the ball, s (a range).
    pub mate_hold: [f32; 2],
    /// A teammate's leaning toward giving it back to you.
    pub to_you: f32,
    /// How much the shape breathes and turns.
    pub drift: f32,
    /// Your field of view, half-angle, radians; a glance's half-angle.
    pub view: f32,
    pub glance: f32,
    pub seed: u64,
}

impl Default for Config {
    /// A positional box around the pivot: two centre-backs behind, two full-backs wide, two
    /// interiors ahead; three pressers.
    fn default() -> Config {
        Config {
            you_home: v2(0.0, 0.0),
            shape: vec![v2(-7.0, -9.0), v2(7.0, -9.0), v2(-14.0, -1.0), v2(14.0, -1.0), v2(-6.5, 9.0), v2(6.5, 9.0)],
            pressers: vec![v2(-3.0, 5.0), v2(3.5, 5.5), v2(0.0, -4.0)],
            area: 19.0,
            pass_speed: 13.0,
            mate_speed: 5.5,
            presser_speed: 5.2,
            reach: 1.0,
            react: 0.18,
            hold_pace: 0.05,
            soften: 0.7,
            mate_hold: [0.45, 1.1],
            to_you: 3.0,
            drift: 1.0,
            view: 1.05,
            glance: 1.0,
            seed: 7,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Kind {
    You,
    Mate,
    Presser,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Player {
    pub kind: Kind,
    pub pos: V2,
    pub vel: V2,
    /// The direction faced, radians.
    pub facing: f32,
    /// Home slot (relative to the shape's centre).
    pub home: V2,
    /// Where it is going.
    pub target: V2,
    /// Where you last saw it, and how long ago (world seconds).
    pub seen: V2,
    pub seen_age: f32,
    /// Seconds it cannot act (a presser after a poke).
    pub stun: f32,
    phase: f32,
    retarget: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Ball {
    Held { by: usize, t: f32, hold: f32 },
    Flight { from: usize, to: usize, a: V2, b: V2, t: f32, dur: f32, first_time: bool },
    Loose { vel: V2 },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Event {
    /// A pass: who to whom, how open its lane was (truly), its direction (radians) and length.
    Pass { from: usize, to: usize, open: f32, angle: f32, length: f32, first_time: bool, yours: bool },
    Received { by: usize, first_time: bool },
    Poked { by: usize, at: V2 },
    Recovered { by: usize },
    Glance { angle: f32 },
    /// Your pass was waiting for the ball and went the moment it came.
    Queued,
}

/// A pass you could make: to whom, how open it looks (from what you remember) and truly is.
#[derive(Clone, Copy, Debug)]
pub struct Lane {
    pub to: usize,
    pub a: V2,
    pub b: V2,
    pub seen: Openness,
    pub truly: Openness,
}

#[derive(Clone, Debug)]
pub struct World {
    pub cfg: Config,
    pub players: Vec<Player>,
    pub ball: Ball,
    pub ball_pos: V2,
    /// The world's pace now (1 = real time).
    pub pace: f32,
    /// World seconds and real seconds since the start.
    pub t: f64,
    pub real_t: f64,
    /// The shape's centre and turn.
    pub center: V2,
    pub turn: f32,
    /// Passes in a row without a poke; yours among them.
    pub chain: u32,
    pub your_touches: u32,
    /// A glance under way: its direction and real seconds left.
    pub glance: Option<(f32, f32)>,
    /// A pass you asked for before the ball arrived (a direction), played first time.
    pub queued: Option<(V2, f32)>,
    events: Vec<Event>,
    rng: Rng,
}

pub const YOU: usize = 0;

impl World {
    pub fn new(cfg: Config) -> World {
        let mut rng = Rng::new(cfg.seed);
        let mut players = Vec::new();
        let mk = |kind, home: V2, rng: &mut Rng| Player {
            kind,
            pos: home,
            vel: V2::ZERO,
            facing: PI / 2.0,
            home,
            target: home,
            seen: home,
            seen_age: 0.0,
            stun: 0.0,
            phase: rng.f() * 10.0,
            retarget: rng.f() * 0.3,
        };
        players.push(mk(Kind::You, cfg.you_home, &mut rng));
        for h in &cfg.shape {
            players.push(mk(Kind::Mate, *h, &mut rng));
        }
        for h in &cfg.pressers {
            players.push(mk(Kind::Presser, *h, &mut rng));
        }
        // the ball starts with the left centre-back (or the first teammate)
        let by = 1.min(players.len() - 1);
        let ball_pos = players[by].pos;
        let mut w = World {
            cfg,
            players,
            ball: Ball::Held { by, t: 0.0, hold: 0.8 },
            ball_pos,
            pace: 1.0,
            t: 0.0,
            real_t: 0.0,
            center: V2::ZERO,
            turn: 0.0,
            chain: 0,
            your_touches: 0,
            glance: None,
            queued: None,
            events: Vec::new(),
            rng,
        };
        w.face_ball(1.0);
        w
    }

    pub fn you(&self) -> &Player {
        &self.players[YOU]
    }

    pub fn lane_params(&self) -> LaneParams {
        LaneParams { ball_speed: self.cfg.pass_speed, presser_speed: self.cfg.presser_speed, reach: self.cfg.reach, react: self.cfg.react }
    }

    pub fn holder(&self) -> Option<usize> {
        match self.ball {
            Ball::Held { by, .. } => Some(by),
            _ => None,
        }
    }

    /// You have the ball.
    pub fn yours(&self) -> bool {
        self.holder() == Some(YOU)
    }

    /// A pass is on its way to you: world seconds until it arrives.
    pub fn incoming(&self) -> Option<f32> {
        match self.ball {
            Ball::Flight { to: YOU, t, dur, .. } => Some((dur - t).max(0.0)),
            _ => None,
        }
    }

    /// The events since the last call.
    pub fn drain(&mut self) -> Vec<Event> {
        std::mem::take(&mut self.events)
    }

    fn pressers_now(&self) -> Vec<Option<V2>> {
        self.players.iter().filter(|p| p.kind == Kind::Presser).map(|p| (p.stun <= 0.0).then_some(p.pos)).collect()
    }

    fn pressers_seen(&self) -> Vec<Option<V2>> {
        self.players.iter().filter(|p| p.kind == Kind::Presser).map(|p| Some(p.seen)).collect()
    }

    /// The passes open to you now (from the ball, or from you when it is coming).
    pub fn lanes(&self) -> Vec<Lane> {
        let lp = self.lane_params();
        let (now, seen) = (self.pressers_now(), self.pressers_seen());
        let a = if self.yours() { self.ball_pos } else { self.players[YOU].pos };
        self.players
            .iter()
            .enumerate()
            .filter(|(_, p)| p.kind == Kind::Mate)
            .map(|(i, p)| {
                let b = p.pos;
                Lane { to: i, a, b, seen: openness(a, b, &seen, &lp), truly: openness(a, b, &now, &lp) }
            })
            .collect()
    }

    /// Glances toward `angle` (radians, world): you see that side for a moment.
    pub fn glance(&mut self, angle: f32) {
        let fresh = !self.glance.is_some_and(|(a, _)| angle_diff(a, angle).abs() < 0.35);
        self.glance = Some((angle, 0.4));
        if fresh {
            self.events.push(Event::Glance { angle });
        }
    }

    /// A flick in world direction `dir` (strength ~1: a usual pass). With the ball: the pass goes to the
    /// teammate who best answers the direction. With the ball coming: it waits and goes first time.
    /// Without: nothing (the take turns early swipes into glances).
    pub fn flick(&mut self, dir: V2, strength: f32) -> bool {
        if self.incoming().is_some() {
            self.queued = Some((dir.norm(), strength));
            return true;
        }
        if !self.yours() {
            return false;
        }
        match self.choose(dir) {
            Some(to) => {
                self.pass(YOU, to, strength, false);
                true
            }
            None => false,
        }
    }

    /// A tap at a world point: with the ball (or it coming), a pass to the teammate nearest the point;
    /// without, a glance toward it.
    pub fn tap(&mut self, at: V2) -> bool {
        let you = self.players[YOU].pos;
        if self.yours() || self.incoming().is_some() {
            let best = self
                .players
                .iter()
                .enumerate()
                .filter(|(_, p)| p.kind == Kind::Mate)
                .min_by(|a, b| a.1.pos.dist(at).partial_cmp(&b.1.pos.dist(at)).unwrap())
                .map(|(i, p)| (i, p.pos));
            if let Some((i, p)) = best {
                if p.dist(at) < 6.0 {
                    if self.incoming().is_some() {
                        self.queued = Some(((p - you).norm(), 1.0));
                        return true;
                    }
                    self.pass(YOU, i, 1.0, false);
                    return true;
                }
            }
            return false;
        }
        self.glance((at - you).angle());
        true
    }

    /// The teammate a flick in `dir` means: the best of angle and (seen) openness, within 55°.
    pub fn choose(&self, dir: V2) -> Option<usize> {
        let want = dir.angle();
        let mut best = None;
        let mut best_s = 0.0;
        for l in self.lanes() {
            let d = angle_diff((l.b - l.a).angle(), want).abs();
            if d > 0.96 {
                continue;
            }
            let s = d.cos() * (0.55 + 0.45 * l.seen.open);
            if s > best_s {
                best_s = s;
                best = Some(l.to);
            }
        }
        best
    }

    fn pass(&mut self, from: usize, to: usize, strength: f32, first_time: bool) {
        let a = self.ball_pos;
        let recv = &self.players[to];
        let speed = self.cfg.pass_speed * strength.clamp(0.8, 1.35) * if first_time { 1.1 } else { 1.0 };
        let dur0 = a.dist(recv.pos) / speed;
        // lead the receiver a little, and miss by a little more when the lane was tight
        let mut b = recv.pos + recv.vel * (dur0 * 0.5);
        let open = openness(a, b, &self.pressers_now(), &self.lane_params());
        let wobble = (1.0 - open.open) * 0.6 + 0.15;
        b += v2(self.rng.s(), self.rng.s()) * wobble;
        let dur = (a.dist(b) / speed).max(0.12);
        self.ball = Ball::Flight { from, to, a, b, t: 0.0, dur, first_time };
        self.players[to].target = b;
        if from == YOU {
            self.your_touches += 1;
        }
        self.events.push(Event::Pass { from, to, open: open.open, angle: (b - a).angle(), length: a.dist(b), first_time, yours: from == YOU });
    }

    fn face_ball(&mut self, k: f32) {
        for i in 0..self.players.len() {
            let p = &self.players[i];
            let to = self.ball_pos - p.pos;
            if to.len() > 0.3 {
                let want = to.angle();
                let f = p.facing + angle_diff(want, p.facing) * k;
                self.players[i].facing = f;
            }
        }
    }

    /// Time moves by `dt` real seconds.
    pub fn step(&mut self, dt: f32) {
        let dt = dt.clamp(0.0, 0.1);
        self.real_t += dt as f64;
        // the pace: held while yours, softening as it comes
        let target = if self.yours() {
            self.cfg.hold_pace
        } else if let Some(rem) = self.incoming() {
            // softening, not stopping: the ball still arrives in about a real second
            0.3 + 0.7 * smoothstep(0.0, self.cfg.soften, rem)
        } else {
            1.0
        };
        let k = 1.0 - (-dt * if target < self.pace { 10.0 } else { 3.0 }).exp();
        self.pace += (target - self.pace) * k;
        let wdt = dt * self.pace;
        self.t += wdt as f64;

        // the glance and your head
        if let Some((a, left)) = self.glance {
            let left = left - dt;
            self.glance = (left > 0.0).then_some((a, left));
        }
        let you_face = match self.glance {
            Some((a, _)) => a,
            None => {
                if self.yours() {
                    self.players[YOU].facing
                } else {
                    (self.ball_pos - self.players[YOU].pos).angle()
                }
            }
        };
        let f = self.players[YOU].facing;
        self.players[YOU].facing = f + angle_diff(you_face, f) * (1.0 - (-dt * 14.0).exp());

        self.see(wdt);
        self.shape(wdt);
        self.think(wdt);
        self.move_players(wdt);
        self.ball_step(wdt);
        let k = 1.0 - (-wdt * 6.0).exp();
        for i in 1..self.players.len() {
            let p = &self.players[i];
            let to = self.ball_pos - p.pos;
            if to.len() > 0.3 {
                let f = p.facing + angle_diff(to.angle(), p.facing) * k;
                self.players[i].facing = f;
            }
        }
    }

    /// What you see: your view cone, a glance's cone, anyone within a few metres.
    fn see(&mut self, wdt: f32) {
        let you = self.players[YOU].pos;
        let facing = self.players[YOU].facing;
        let glance = self.glance.map(|g| g.0);
        for i in 1..self.players.len() {
            let p = &mut self.players[i];
            let to = p.pos - you;
            let a = to.angle();
            let visible = to.len() < 3.0
                || angle_diff(a, facing).abs() < self.cfg.view
                || glance.is_some_and(|g| angle_diff(a, g).abs() < self.cfg.glance);
            if visible {
                p.seen = p.pos;
                p.seen_age = 0.0;
            } else {
                p.seen_age += wdt;
            }
        }
    }

    /// The shape breathes: its centre leans toward the ball, it turns slowly back and forth.
    fn shape(&mut self, wdt: f32) {
        let t = self.t as f32;
        let d = self.cfg.drift;
        let want = self.ball_pos * 0.25 + v2((t * 0.05).sin() * 2.0, (t * 0.037).cos() * 1.5) * d;
        self.center = self.center.lerp(want, 1.0 - (-wdt * 0.6).exp());
        self.turn = ((t * 0.021).sin() * 0.22 + (t * 0.0083).sin() * 0.3) * d;
    }

    fn home(&self, h: V2) -> V2 {
        let (s, c) = self.turn.sin_cos();
        self.center + v2(h.x * c - h.y * s, h.x * s + h.y * c)
    }

    /// Choosing where to go.
    fn think(&mut self, wdt: f32) {
        let lp = self.lane_params();
        let pressers = self.pressers_now();
        let ball_from = match self.ball {
            Ball::Flight { b, .. } => b,
            _ => self.ball_pos,
        };
        let receiver = match self.ball {
            Ball::Flight { to, .. } => Some(to),
            _ => None,
        };
        let loose_chaser = if let Ball::Loose { .. } = self.ball {
            self.players.iter().enumerate().filter(|(_, p)| p.kind != Kind::Presser).min_by(|a, b| a.1.pos.dist(self.ball_pos).partial_cmp(&b.1.pos.dist(self.ball_pos)).unwrap()).map(|(i, _)| i)
        } else {
            None
        };
        // your side: offer an angle
        for i in 0..self.players.len() {
            if self.players[i].kind == Kind::Presser {
                continue;
            }
            self.players[i].retarget -= wdt;
            if Some(i) == receiver {
                continue;
            }
            if Some(i) == loose_chaser {
                self.players[i].target = self.ball_pos;
                continue;
            }
            if self.holder() == Some(i) {
                self.players[i].target = self.players[i].pos;
                continue;
            }
            if self.players[i].retarget > 0.0 {
                continue;
            }
            let pivot = self.players[i].kind == Kind::You;
            // the pivot looks for space more often and further (it is what the pivot does)
            self.players[i].retarget = if pivot { 0.15 } else { 0.25 + self.rng.f() * 0.2 };
            let home = self.home(self.players[i].home);
            let mut best = (f32::NEG_INFINITY, home);
            let (n, r) = if pivot { (13, 4.5) } else { (9, 2.6) };
            for k in 0..n {
                let c = if k == 0 { home } else { home + V2::from_angle(k as f32 * PI * 2.0 / (n - 1) as f32 + self.rng.s() * 0.3) * (r * (0.5 + 0.5 * self.rng.f()) + 0.6) };
                let open = openness(ball_from, c, &pressers, &lp).open;
                let mut s = open - c.dist(home) * 0.07;
                if self.players[i].kind == Kind::Mate {
                    s += 0.25 * openness(c, self.players[YOU].pos, &pressers, &lp).open;
                }
                // not on top of a teammate
                for (j, q) in self.players.iter().enumerate() {
                    if j != i && q.kind != Kind::Presser {
                        s -= smoothstep(4.0, 0.0, q.pos.dist(c)) * 0.6;
                    }
                }
                if c.dist(ball_from) < 3.5 {
                    s -= 0.8;
                }
                if s > best.0 {
                    best = (s, c);
                }
            }
            self.players[i].target = best.1;
        }

        // pressers: one to the ball, the others into the most open lanes
        let idx: Vec<usize> = (0..self.players.len()).filter(|&i| self.players[i].kind == Kind::Presser).collect();
        if idx.is_empty() {
            return;
        }
        let ball = self.ball_pos;
        let presser_to_ball = idx.iter().copied().filter(|&i| self.players[i].stun <= 0.0).min_by(|&a, &b| self.players[a].pos.dist(ball).partial_cmp(&self.players[b].pos.dist(ball)).unwrap());
        let mut lanes: Vec<(f32, V2, V2)> = Vec::new();
        let holder_pos = ball_from;
        for (j, q) in self.players.iter().enumerate() {
            if q.kind == Kind::Presser || Some(j) == self.holder() || Some(j) == receiver {
                continue;
            }
            let o = openness(holder_pos, q.pos, &pressers, &lp).open;
            lanes.push((o, holder_pos, q.pos));
        }
        let mut taken = vec![false; lanes.len()];
        for &i in &idx {
            let p = self.players[i].clone();
            let p = &p;
            if p.stun > 0.0 {
                let away = p.pos + (p.pos - ball).norm() * 2.0;
                self.players[i].target = away;
                continue;
            }
            if Some(i) == presser_to_ball {
                let mut t = ball;
                if let Ball::Flight { a, b, t: ft, dur, .. } = self.ball {
                    // cut the ball's path if it can be reached, else go to where it is going
                    let (c, u) = closest_on_segment(p.pos, a, b);
                    let ball_at = (u * dur - ft).max(0.0);
                    let need = (p.pos.dist(c) - self.cfg.reach).max(0.0) / self.cfg.presser_speed;
                    t = if need < ball_at { c } else { b };
                }
                let off = (p.pos - t).norm() * 1.1;
                self.players[i].target = t + off;
                continue;
            }
            if p.retarget > 0.0 {
                self.players[i].retarget -= wdt;
                continue;
            }
            self.players[i].retarget = 0.6 + self.rng.f() * 0.5;
            // the most open lane not yet covered, weighted by how near it is
            let mut best: Option<(f32, usize, V2)> = None;
            for (k, (o, a, b)) in lanes.iter().enumerate() {
                if taken[k] {
                    continue;
                }
                let (c, u) = closest_on_segment(p.pos, *a, *b);
                let u = u.clamp(0.3, 0.75);
                let c = a.lerp(*b, u).lerp(c, 0.3);
                let s = *o * 0.7 - p.pos.dist(c) * 0.08 + self.rng.f() * 0.3;
                if best.is_none_or(|bb| s > bb.0) {
                    best = Some((s, k, c));
                }
            }
            if let Some((_, k, c)) = best {
                taken[k] = true;
                self.players[i].target = c + v2(self.rng.s(), self.rng.s()) * 0.5;
            }
        }
    }

    fn move_players(&mut self, wdt: f32) {
        if wdt <= 0.0 {
            return;
        }
        let n = self.players.len();
        let area = self.cfg.area;
        for i in 0..n {
            let (kind, pos, vel, target) = {
                let p = &self.players[i];
                (p.kind, p.pos, p.vel, p.target)
            };
            let mut max = match kind {
                Kind::Presser => self.cfg.presser_speed,
                Kind::Mate => self.cfg.mate_speed,
                Kind::You => self.cfg.mate_speed * 0.95,
            };
            if self.holder() == Some(i) {
                max *= 0.25;
            }
            let to = target - pos;
            let d = to.len();
            let want = to.norm() * (max * smoothstep(0.0, 2.5, d));
            let mut acc = (want - vel) * 3.5;
            // apart
            for j in 0..n {
                if i == j {
                    continue;
                }
                let away = pos - self.players[j].pos;
                let l = away.len();
                if l < 1.6 && l > 1e-4 {
                    acc += away.norm() * (1.6 - l) * 6.0;
                }
            }
            // a little life
            let ph = self.players[i].phase + self.t as f32 * 0.7;
            acc += v2(ph.sin(), (ph * 1.3).cos()) * 0.35;
            let p = &mut self.players[i];
            p.vel = (p.vel + acc * wdt).clamp_len(max * 1.1);
            p.pos += p.vel * wdt;
            p.pos.x = p.pos.x.clamp(-area, area);
            p.pos.y = p.pos.y.clamp(-area, area);
            p.stun = (p.stun - wdt).max(0.0);
        }
    }

    fn ball_step(&mut self, wdt: f32) {
        match self.ball {
            Ball::Held { by, t, hold } => {
                let p = &self.players[by];
                self.ball_pos = p.pos + V2::from_angle(p.facing) * 0.45;
                let t = t + wdt;
                self.ball = Ball::Held { by, t, hold };
                if by != YOU && t > hold {
                    self.mate_decides(by, t);
                }
            }
            Ball::Flight { from, to, a, b, t, dur, first_time } => {
                let t = t + wdt;
                let u = (t / dur).min(1.0);
                self.ball_pos = a.lerp(b, u);
                // the real pressers' feet
                for i in 0..self.players.len() {
                    let p = &self.players[i];
                    if p.kind != Kind::Presser || p.stun > 0.0 || u > 0.93 {
                        continue;
                    }
                    if p.pos.dist(self.ball_pos) < self.cfg.reach * 0.85 {
                        let dir = (b - a).norm();
                        let side = if (p.pos - self.ball_pos).cross(dir) > 0.0 { -1.0 } else { 1.0 };
                        let out = (dir * 0.5 + dir.perp() * side * (0.6 + self.rng.f() * 0.5)).norm();
                        self.ball = Ball::Loose { vel: out * (self.cfg.pass_speed * 0.45) };
                        self.players[i].stun = 1.4;
                        self.chain = 0;
                        self.queued = None;
                        self.events.push(Event::Poked { by: i, at: self.ball_pos });
                        return;
                    }
                }
                if u >= 1.0 {
                    self.ball = Ball::Held { by: to, t: 0.0, hold: self.rng.range(self.cfg.mate_hold[0], self.cfg.mate_hold[1]) };
                    self.chain += 1;
                    self.events.push(Event::Received { by: to, first_time });
                    let _ = from;
                    if to == YOU {
                        if let Some((dir, s)) = self.queued.take() {
                            if let Some(m) = self.choose(dir) {
                                self.events.push(Event::Queued);
                                self.pass(YOU, m, s, true);
                            }
                        }
                    }
                } else {
                    self.ball = Ball::Flight { from, to, a, b, t, dur, first_time };
                }
            }
            Ball::Loose { vel } => {
                self.ball_pos += vel * wdt;
                let vel = vel * (-1.6 * wdt).exp();
                self.ball_pos.x = self.ball_pos.x.clamp(-self.cfg.area, self.cfg.area);
                self.ball_pos.y = self.ball_pos.y.clamp(-self.cfg.area, self.cfg.area);
                self.ball = Ball::Loose { vel };
                let got = self.players.iter().enumerate().filter(|(_, p)| p.kind != Kind::Presser && p.pos.dist(self.ball_pos) < 0.8).map(|(i, _)| i).next();
                if let Some(by) = got {
                    self.ball = Ball::Held { by, t: 0.0, hold: self.rng.range(self.cfg.mate_hold[0], self.cfg.mate_hold[1]) + 0.3 };
                    self.events.push(Event::Recovered { by });
                    // the pressers fall back a step to start again
                    for p in self.players.iter_mut().filter(|p| p.kind == Kind::Presser) {
                        p.stun = p.stun.max(0.5);
                    }
                }
            }
        }
    }

    /// A teammate with the ball passes: to the most open, leaning toward you.
    fn mate_decides(&mut self, by: usize, held: f32) {
        let lp = self.lane_params();
        let pressers = self.pressers_now();
        let from = self.ball_pos;
        let mut best: Option<(f32, usize, f32)> = None;
        for (j, q) in self.players.iter().enumerate() {
            if j == by || q.kind == Kind::Presser || q.pos.dist(from) < 3.0 {
                continue;
            }
            let o = openness(from, q.pos, &pressers, &lp).open;
            let mut s = if j == YOU { o * self.cfg.to_you + if o > 0.3 { 0.5 } else { 0.0 } } else { o } - q.pos.dist(from) * 0.012;
            s += self.rng.f() * 0.25;
            if best.is_none_or(|b| s > b.0) {
                best = Some((s, j, o));
            }
        }
        let Some((_, to, o)) = best else { return };
        if o < 0.25 && held < 2.5 {
            return; // shield it a moment longer
        }
        // wait a beat for the pivot to come free, if it nearly is
        let yours = openness(from, self.players[YOU].pos, &pressers, &lp).open;
        if to != YOU && yours > 0.1 && held < self.cfg.mate_hold[1] + 0.8 && self.rng.f() < 0.9 {
            return;
        }
        self.pass(by, to, 1.0, false);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(w: &mut World, secs: f32) -> Vec<Event> {
        let mut ev = Vec::new();
        let n = (secs * 60.0) as usize;
        for _ in 0..n {
            w.step(1.0 / 60.0);
            ev.extend(w.drain());
        }
        ev
    }

    #[test]
    fn the_ball_comes_to_you_and_time_holds() {
        let mut w = World::new(Config::default());
        let mut came = false;
        for _ in 0..(60 * 40) {
            w.step(1.0 / 60.0);
            if w.yours() {
                came = true;
                break;
            }
        }
        assert!(came, "the ball never came to the pivot in 40 s");
        run(&mut w, 1.0);
        assert!(w.yours(), "it is still yours: nothing takes it");
        assert!(w.pace < 0.1, "time holds while you decide: pace {}", w.pace);
        let before = w.t;
        run(&mut w, 10.0);
        assert!(w.yours());
        assert!(w.t - before < 1.0, "ten real seconds are under one world second while you hold: {}", w.t - before);
    }

    #[test]
    fn a_flick_passes_and_time_returns() {
        let mut w = World::new(Config::default());
        while !w.yours() {
            w.step(1.0 / 60.0);
            assert!(w.real_t < 60.0);
        }
        let lanes = w.lanes();
        let best = lanes.iter().max_by(|a, b| a.seen.open.partial_cmp(&b.seen.open).unwrap()).unwrap();
        let dir = best.b - best.a;
        assert!(w.flick(dir, 1.0));
        assert!(!w.yours());
        run(&mut w, 2.0);
        assert!(w.pace > 0.5, "pace back up: {}", w.pace);
    }

    #[test]
    fn the_world_goes_on_without_you() {
        // play passing at random for two minutes: the ball keeps moving, pokes happen and are recovered
        let mut w = World::new(Config::default());
        let mut passes = 0;
        let mut pokes = 0;
        let mut rng = Rng::new(3);
        for _ in 0..(60 * 120) {
            w.step(1.0 / 60.0);
            if w.yours() && rng.f() < 0.05 {
                let a = rng.f() * std::f32::consts::TAU;
                w.flick(V2::from_angle(a), 1.0);
            }
            for e in w.drain() {
                match e {
                    Event::Pass { .. } => passes += 1,
                    Event::Poked { .. } => pokes += 1,
                    _ => {}
                }
            }
        }
        assert!(passes > 40, "passes {passes}");
        assert!(pokes >= 1, "pokes {pokes}");
        for p in &w.players {
            assert!(p.pos.x.is_finite() && p.pos.x.abs() <= w.cfg.area);
        }
    }

    #[test]
    fn what_you_have_not_seen_you_remember() {
        let mut w = World::new(Config::default());
        run(&mut w, 8.0);
        let behind = w.players.iter().filter(|p| p.kind == Kind::Presser).map(|p| p.seen_age).fold(0.0, f32::max);
        assert!(behind > 0.0, "some presser has gone unseen a while");
        let you = w.players[YOU].pos;
        let (i, _) = w.players.iter().enumerate().filter(|(_, p)| p.kind == Kind::Presser).max_by(|a, b| a.1.seen_age.partial_cmp(&b.1.seen_age).unwrap()).unwrap();
        let a = (w.players[i].pos - you).angle();
        w.glance(a);
        w.step(1.0 / 60.0);
        assert_eq!(w.players[i].seen_age, 0.0);
    }
}
