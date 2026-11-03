//! The world: two teams of agents and a ball, stepped on simple rules. With a human it is a game (you
//! are one player, usually the pivot); without one it plays itself, for watching and tinkering.
//!
//! - **Agents.** Every player has a few traits (`Traits`): how often it scans, how hard it presses, how
//!   much risk it takes in a pass, how far it goes to offer an angle, how often it runs in behind, how
//!   strongly it reads a passer's body. The team with the ball keeps a breathing shape and each player
//!   tries a few spots near home for the most open line from the ball; the team without it sends one
//!   to the ball and the rest into the most open lanes.
//! - **The ball rolls.** A pass is a direction and a pace; the ball slows on the grass (`friction`).
//!   Whoever can reach it first takes it: a teammate if it is slow enough to control (`control`), a
//!   presser if it gets there sooner. Too soft and a presser cuts it out; too firm and it runs past
//!   the runner or out; just right and it meets the runner in stride (`Event::Received::grace`).
//!   `predict` plays a pass forward on the same rule: the agents choose with it, the takes draw it.
//! - **Feints.** Pressers read the passer's body (where it faces, where it glances) and commit to a
//!   guess (`Player::lean`, `commit`): they drift toward that lane, and a committed presser is slow to
//!   react the other way. Look one way, let the world move a moment (a look with the ball costs world
//!   time), release the other.
//! - **Time.** With a human the world's pace is 1 while the ball moves between others, softens as a
//!   pass comes to you, and all but holds while you have it (`Config::hold_pace`). Never a clock: but a
//!   ball held long in world time can be taken.
//! - **Seeing.** You see your view cone and what a glance sweeps; everyone else you remember where you
//!   last saw them (`Player::seen`). What you are shown is reckoned from what you remember; the ball
//!   meets the real players.
//! - **Possession.** In a rondo (`swap: false`) a presser only pokes the ball away and your side
//!   recovers it. With `swap` the team that wins the ball keeps it, and with `goals` an attack that
//!   reaches the box makes a chance and the ball restarts with the other side.

use crate::lane::Openness;
use crate::v2::{angle_diff, closest_on_segment, smoothstep, v2, Rng, V2};
use serde::{Deserialize, Serialize};
use std::f32::consts::PI;

/// A player's character, each 0..1 (`speed` a multiplier).
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct Traits {
    /// How often it looks around (agents: how quick and sure it is on the ball).
    pub scan: f32,
    /// How hard it presses (speed when closing, how close it stands off).
    pub press: f32,
    /// How much risk it takes in a pass (tight lanes, through balls).
    pub risk: f32,
    /// How far it goes to offer an angle.
    pub offer: f32,
    /// How often it runs in behind.
    pub run: f32,
    /// How strongly it reads a passer's body (and so how much a feint fools it).
    pub read: f32,
    pub speed: f32,
}

impl Default for Traits {
    fn default() -> Traits {
        Traits { scan: 0.5, press: 0.6, risk: 0.4, offer: 0.6, run: 0.25, read: 0.6, speed: 1.0 }
    }
}

/// One player at the start: team (0 or 1), where it is, its home in the shape (relative to the
/// shape's centre; team 0 attacks +x), its traits.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Spec {
    pub team: u8,
    pub pos: V2,
    pub home: V2,
    pub traits: Traits,
}

impl Spec {
    pub fn new(team: u8, home: V2) -> Spec {
        Spec { team, pos: home, home, traits: Traits::default() }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    pub players: Vec<Spec>,
    /// The player you are, if any (None: the world plays itself).
    pub human: Option<usize>,
    /// Half the pitch: half-length (x), half-width (y), metres; the centre spot at the origin.
    pub half: V2,
    /// A won ball changes possession (else pressers only poke it away: a rondo).
    pub swap: bool,
    /// Attacks that reach the box make a chance and restart with the other side.
    pub goals: bool,
    /// How much the shapes follow the ball, and breathe and turn.
    pub follow: f32,
    pub drift: f32,
    /// A player's running speed, m/s (times its `speed` trait).
    pub run_speed: f32,
    /// The ball's slowing on the grass, m/s².
    pub friction: f32,
    /// The fastest ball a player can bring under control, m/s.
    pub control: f32,
    /// How far a foot reaches, m; a player's reaction, s.
    pub reach: f32,
    pub react: f32,
    /// The paces agents pass at, m/s.
    pub kick: [f32; 2],
    /// The world's pace while you have the ball (1 = real time); world seconds before a pass reaches
    /// you that time begins to soften.
    pub hold_pace: f32,
    pub soften: f32,
    /// How long an agent keeps the ball, s (a range).
    pub hold: [f32; 2],
    /// Your teammates' leaning toward giving it to you.
    pub to_you: f32,
    /// Your field of view and a glance's, half-angles, radians.
    pub view: f32,
    pub glance: f32,
    pub seed: u64,
}

impl Config {
    /// A rondo-like box: you (team 0, index 0) at `you`, your teammates, the pressers (team 1); the
    /// ball starts with the first teammate.
    pub fn rondo(you: V2, mates: &[V2], pressers: &[V2], half: f32) -> Config {
        let mut players = vec![Spec::new(0, you)];
        players.extend(mates.iter().map(|&h| Spec::new(0, h)));
        players.extend(pressers.iter().map(|&h| Spec::new(1, h)));
        Config {
            players,
            human: Some(0),
            half: v2(half, half),
            swap: false,
            goals: false,
            follow: 0.25,
            drift: 1.0,
            run_speed: 5.5,
            friction: 2.2,
            control: 15.0,
            reach: 1.0,
            react: 0.18,
            kick: [8.0, 17.0],
            hold_pace: 0.05,
            soften: 0.7,
            hold: [0.45, 1.1],
            to_you: 3.0,
            view: 1.05,
            glance: 1.0,
            seed: 7,
        }
    }

    /// Eleven a side on a 105 × 68 m pitch, both teams agents (4-3-3 against 4-4-2), nobody human.
    pub fn eleven() -> Config {
        let a = [(-48.0, 0.0), (-33.0, -20.0), (-36.0, -7.0), (-36.0, 7.0), (-33.0, 20.0), (-22.0, 0.0), (-12.0, -12.0), (-12.0, 12.0), (2.0, -22.0), (6.0, 0.0), (2.0, 22.0)];
        let b = [(48.0, 0.0), (34.0, -20.0), (36.0, -7.0), (36.0, 7.0), (34.0, 20.0), (18.0, -21.0), (20.0, -7.0), (20.0, 7.0), (18.0, 21.0), (6.0, -6.0), (6.0, 6.0)];
        let mut players: Vec<Spec> = a.iter().map(|&(x, y)| Spec::new(0, v2(x, y))).collect();
        players.extend(b.iter().map(|&(x, y)| Spec::new(1, v2(x, y))));
        Config {
            players,
            human: None,
            half: v2(52.5, 34.0),
            swap: true,
            goals: true,
            follow: 0.45,
            drift: 0.0,
            run_speed: 6.5,
            kick: [9.0, 24.0],
            hold: [0.5, 1.4],
            to_you: 1.0,
            seed: 3,
            ..Config::default()
        }
    }
}

impl Default for Config {
    /// A positional box around the pivot: two centre-backs behind, two full-backs wide, two
    /// interiors ahead; three pressers.
    fn default() -> Config {
        Config::rondo(
            v2(0.0, 0.0),
            &[v2(-7.0, -9.0), v2(7.0, -9.0), v2(-14.0, -1.0), v2(14.0, -1.0), v2(-6.5, 9.0), v2(6.5, 9.0)],
            &[v2(-3.0, 5.0), v2(3.5, 5.5), v2(0.0, -4.0)],
            19.0,
        )
    }
}

/// How the takes tell players apart in a rondo: you, your side, the other side.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Kind {
    You,
    Mate,
    Presser,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Player {
    pub kind: Kind,
    pub team: u8,
    pub traits: Traits,
    pub pos: V2,
    pub vel: V2,
    /// The direction faced, radians.
    pub facing: f32,
    /// Home slot (relative to its team's shape centre).
    pub home: V2,
    /// Where it is going.
    pub target: V2,
    /// Where you last saw it, and how long ago (world seconds).
    pub seen: V2,
    pub seen_age: f32,
    /// Seconds it cannot act (after a poke).
    pub stun: f32,
    /// A presser's guess of where the pass will go (a unit direction from the ball) and how committed
    /// it is to it (0..1): draw it leaning. For an agent with the ball: where it means to pass.
    pub lean: V2,
    pub commit: f32,
    /// A run in behind under way: where to, and world seconds left.
    pub run_to: V2,
    pub running: f32,
    phase: f32,
    retarget: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Ball {
    Held { by: usize, t: f32, hold: f32 },
    /// Rolling: from whom (`usize::MAX`: nobody, a loose ball), meant for whom, passed first time,
    /// world seconds since.
    Rolling { vel: V2, from: usize, to: Option<usize>, first_time: bool, t: f32 },
    /// Out of play; it restarts with `to` after a moment.
    Dead { to: usize, t: f32 },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Event {
    /// A pass: who to whom (meant), from where, how open it was (truly), direction, the distance it
    /// would roll, its pace (m/s).
    Pass { from: usize, to: usize, at: V2, open: f32, angle: f32, length: f32, speed: f32, first_time: bool, yours: bool },
    /// Taken under control. `grace` 0..1: how well the weight met the player (an easy pace to a
    /// player in stride is 1).
    Received { by: usize, first_time: bool, grace: f32 },
    Poked { by: usize, at: V2 },
    Recovered { by: usize },
    /// The ball changed hands (`swap`): the team that has it now.
    Turnover { team: u8, by: usize },
    Out { at: V2 },
    Chance { team: u8, at: V2 },
    Glance { angle: f32 },
    /// Your pass was waiting for the ball and went the moment it came.
    Queued,
}

/// What a pass would come to.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Fate {
    /// A teammate takes it.
    Received,
    /// The other side gets there first.
    Cut,
    /// It runs out of play.
    Out,
}

#[derive(Clone, Copy, Debug)]
pub struct Outcome {
    pub fate: Fate,
    /// Who takes it, where, after how long, at what pace.
    pub by: Option<usize>,
    pub at: V2,
    pub t: f32,
    pub speed: f32,
    /// The taker's lead over the quickest of the other side to that point, s (negative: cut).
    pub margin: f32,
}

/// A pass you could make: to whom, how open it looks (from what you remember) and truly is, and the
/// pace that would weight it best.
#[derive(Clone, Copy, Debug)]
pub struct Lane {
    pub to: usize,
    pub a: V2,
    pub b: V2,
    pub seen: Openness,
    pub truly: Openness,
    pub pace: f32,
}

#[derive(Clone, Debug)]
pub struct World {
    pub cfg: Config,
    pub players: Vec<Player>,
    pub ball: Ball,
    pub ball_pos: V2,
    /// The team with the ball (or last to have it).
    pub attack: u8,
    /// The world's pace now (1 = real time).
    pub pace: f32,
    /// World seconds and real seconds since the start.
    pub t: f64,
    pub real_t: f64,
    /// The rondo shape's centre and turn.
    pub center: V2,
    pub turn: f32,
    /// Passes in a row without a loss; yours.
    pub chain: u32,
    pub your_touches: u32,
    /// A glance under way: its direction and real seconds left.
    pub glance: Option<(f32, f32)>,
    /// World seconds a look costs while you hold the ball (the world moves while you turn your head).
    pub look_cost: f32,
    /// A pass you asked for before the ball arrived (direction, pace m/s), played first time.
    pub queued: Option<(V2, f32)>,
    /// A pass being aimed (direction, pace m/s): the takes draw `preview` of it.
    pub aim: Option<(V2, f32)>,
    events: Vec<Event>,
    rng: Rng,
}

/// Positions at a moment, for the rules that look at everyone.
struct Snapshot {
    pos: Vec<V2>,
    team: Vec<u8>,
    stun: Vec<bool>,
}

impl Snapshot {
    fn others(&self, team: u8) -> Vec<Option<V2>> {
        (0..self.pos.len()).filter(|&i| self.team[i] != team).map(|i| (!self.stun[i]).then_some(self.pos[i])).collect()
    }
}

impl World {
    pub fn new(cfg: Config) -> World {
        let specs = cfg.players.clone();
        let holder = (0..specs.len()).find(|&i| specs[i].team == 0 && Some(i) != cfg.human).or(Some(0));
        let ball = holder.map(|i| specs[i].pos).unwrap_or(V2::ZERO);
        World::from_snapshot(cfg, specs, ball, holder)
    }

    /// A world from any moment: these players where they are, the ball at `ball` (with `holder` if
    /// someone has it); then the agents play on. `cfg.players` is replaced by `players`.
    pub fn from_snapshot(mut cfg: Config, players: Vec<Spec>, ball: V2, holder: Option<usize>) -> World {
        cfg.players = players.clone();
        let mut rng = Rng::new(cfg.seed);
        let ps: Vec<Player> = players
            .iter()
            .enumerate()
            .map(|(i, s)| Player {
                kind: if Some(i) == cfg.human { Kind::You } else if s.team == 0 { Kind::Mate } else { Kind::Presser },
                team: s.team,
                traits: s.traits,
                pos: s.pos,
                vel: V2::ZERO,
                facing: if s.team == 0 { PI / 2.0 } else { -PI / 2.0 },
                home: s.home,
                target: s.pos,
                seen: s.pos,
                seen_age: 0.0,
                stun: 0.0,
                lean: V2::ZERO,
                commit: 0.0,
                run_to: s.pos,
                running: 0.0,
                phase: rng.f() * 10.0,
                retarget: rng.f() * 0.3,
            })
            .collect();
        let attack = holder.map(|h| ps[h].team).unwrap_or(0);
        let ball_state = match holder {
            Some(by) => Ball::Held { by, t: 0.0, hold: 0.8 },
            None => Ball::Rolling { vel: V2::ZERO, from: usize::MAX, to: None, first_time: false, t: 1.0 },
        };
        let mut w = World {
            cfg,
            players: ps,
            ball: ball_state,
            ball_pos: ball,
            attack,
            pace: 1.0,
            t: 0.0,
            real_t: 0.0,
            center: V2::ZERO,
            turn: 0.0,
            chain: 0,
            your_touches: 0,
            glance: None,
            look_cost: 0.0,
            queued: None,
            aim: None,
            events: Vec::new(),
            rng,
        };
        w.face_ball(1.0);
        w
    }

    fn human(&self) -> usize {
        self.cfg.human.unwrap_or(usize::MAX)
    }

    pub fn holder(&self) -> Option<usize> {
        match self.ball {
            Ball::Held { by, .. } => Some(by),
            _ => None,
        }
    }

    /// You have the ball.
    pub fn yours(&self) -> bool {
        self.cfg.human.is_some() && self.holder() == self.cfg.human
    }

    /// The ball is rolling to you: about how many world seconds until it arrives.
    pub fn incoming(&self) -> Option<f32> {
        let h = self.cfg.human?;
        match self.ball {
            Ball::Rolling { to: Some(to), vel, .. } if to == h => Some(self.players[h].pos.dist(self.ball_pos) / vel.len().max(2.0)),
            _ => None,
        }
    }

    pub fn speed_of(&self, i: usize) -> f32 {
        let p = &self.players[i];
        self.cfg.run_speed * p.traits.speed * if p.team != self.attack { 0.9 + 0.2 * p.traits.press } else { 1.0 }
    }

    /// The events since the last call.
    pub fn drain(&mut self) -> Vec<Event> {
        std::mem::take(&mut self.events)
    }

    /// Where the ball is after `t` seconds rolling from `p` at `v`, and its velocity then.
    pub fn roll(&self, p: V2, v: V2, t: f32) -> (V2, V2) {
        let s = v.len();
        if s < 1e-4 {
            return (p, V2::ZERO);
        }
        let t = t.min(s / self.cfg.friction);
        let d = s * t - 0.5 * self.cfg.friction * t * t;
        let dir = v * (1.0 / s);
        (p + dir * d, dir * (s - self.cfg.friction * t))
    }

    /// The seconds player `i` needs to put a foot on `at` (from `from`), counting its reaction and, for
    /// a presser leaning the other way, the time to turn.
    fn reach_time(&self, i: usize, from: V2, at: V2, reach: f32) -> f32 {
        let p = &self.players[i];
        let mut react = self.cfg.react;
        if p.commit > 0.0 {
            let to = (at - self.ball_pos).norm();
            react += p.commit * 0.5 * (1.0 - p.lean.dot(to)) * 0.5;
        }
        (from.dist(at) - reach).max(0.0) / self.speed_of(i) + react
    }

    /// What a pass from `from` at velocity `vel` by team `team` comes to (with players where you
    /// remember them, when `seen`). `passer` cannot take its own pass.
    pub fn predict(&self, from: V2, vel: V2, team: u8, passer: usize, seen: bool) -> Outcome {
        let n = self.players.len();
        let dt = 1.0 / 30.0;
        let pos = |i: usize| if seen && i != self.human() { self.players[i].seen } else { self.players[i].pos };
        for k in 1..=150 {
            let t = k as f32 * dt;
            let (p, v) = self.roll(from, vel, t);
            let speed = v.len();
            if p.x.abs() > self.cfg.half.x || p.y.abs() > self.cfg.half.y {
                return Outcome { fate: Fate::Out, by: None, at: p, t, speed, margin: -1.0 };
            }
            let mut ours: Option<(usize, f32)> = None;
            let mut theirs: Option<(usize, f32)> = None;
            for i in 0..n {
                if i == passer || self.players[i].stun > t {
                    continue;
                }
                let mine = self.players[i].team == team;
                let rt = self.reach_time(i, pos(i), p, if mine { 0.8 } else { self.cfg.reach * 0.85 });
                if rt <= t {
                    if mine && speed <= self.cfg.control {
                        if ours.is_none_or(|o| rt < o.1) {
                            ours = Some((i, rt));
                        }
                    } else if !mine && theirs.is_none_or(|o| rt < o.1) {
                        theirs = Some((i, rt));
                    }
                }
            }
            if let Some((i, rt)) = theirs {
                return Outcome { fate: Fate::Cut, by: Some(i), at: p, t, speed, margin: rt - t - 0.05 };
            }
            if let Some((i, _)) = ours {
                let mut lead = f32::INFINITY;
                for j in 0..n {
                    if self.players[j].team != team && self.players[j].stun <= t {
                        lead = lead.min(self.reach_time(j, pos(j), p, self.cfg.reach * 0.85) - t);
                    }
                }
                return Outcome { fate: Fate::Received, by: Some(i), at: p, t, speed, margin: lead.min(3.0) };
            }
            if speed < 0.05 {
                break;
            }
        }
        // it stops: whoever gets there first
        let (p, _) = self.roll(from, vel, 20.0);
        let first = (0..n).filter(|&i| i != passer).map(|i| (i, self.reach_time(i, pos(i), p, 0.8))).min_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
        match first {
            Some((i, t)) if self.players[i].team == team => Outcome { fate: Fate::Received, by: Some(i), at: p, t, speed: 0.0, margin: 0.0 },
            Some((i, t)) => Outcome { fate: Fate::Cut, by: Some(i), at: p, t, speed: 0.0, margin: -0.5 },
            None => Outcome { fate: Fate::Out, by: None, at: p, t: 0.0, speed: 0.0, margin: -1.0 },
        }
    }

    /// The best pass from `from` to teammate `j`: aimed where it will be, at the pace that gives it
    /// the most lead and an easy touch. (velocity, outcome)
    pub fn weigh(&self, from: V2, j: usize, team: u8, passer: usize, seen: bool) -> (V2, Outcome) {
        let target = self.players[j].pos + self.players[j].vel * 0.5;
        let dir = (target - from).norm();
        let floor = (2.0 * self.cfg.friction * from.dist(target)).sqrt() + 0.5;
        let mut best: Option<(f32, V2, Outcome)> = None;
        for k in 0..7 {
            let s = (self.cfg.kick[0] + (self.cfg.kick[1] - self.cfg.kick[0]) * k as f32 / 6.0).max(floor);
            let v = dir * s;
            let o = self.predict(from, v, team, passer, seen);
            let score = match (o.fate, o.by) {
                (Fate::Received, Some(b)) if b == j => 2.0 + o.margin.min(1.0) - (o.speed - self.cfg.control * 0.45).abs() * 0.04,
                (Fate::Received, _) => 0.5 + o.margin.min(1.0) * 0.5,
                _ => o.margin,
            };
            if best.is_none_or(|b| score > b.0) {
                best = Some((score, v, o));
            }
        }
        let (_, v, o) = best.unwrap();
        (v, o)
    }

    /// What your pass at velocity `vel` would come to, from what you remember.
    pub fn preview(&self, vel: V2) -> Option<Outcome> {
        let h = self.cfg.human?;
        let from = if self.yours() { self.ball_pos } else { self.players[h].pos };
        Some(self.predict(from, vel, self.players[h].team, h, true))
    }

    /// The passes open to you now (from the ball, or from you when it is coming).
    pub fn lanes(&self) -> Vec<Lane> {
        let Some(h) = self.cfg.human else { return Vec::new() };
        let team = self.players[h].team;
        let a = if self.yours() { self.ball_pos } else { self.players[h].pos };
        let open = |o: &Outcome, j: usize| {
            let m = if o.fate == Fate::Received && o.by == Some(j) { o.margin } else { -0.5 };
            Openness { open: smoothstep(-0.05, 0.6, m), margin: m, threat: None }
        };
        (0..self.players.len())
            .filter(|&j| j != h && self.players[j].team == team)
            .map(|j| {
                let (v, seen) = self.weigh(a, j, team, h, true);
                let truly = self.predict(a, v, team, h, false);
                Lane { to: j, a, b: self.players[j].pos, seen: open(&seen, j), truly: open(&truly, j), pace: v.len() }
            })
            .collect()
    }

    /// Glances toward `angle` (radians, world): you see that side for a moment. With the ball at your
    /// feet a fresh look costs world time, and the pressers read it.
    pub fn glance(&mut self, angle: f32) {
        let fresh = !self.glance.is_some_and(|(a, _)| angle_diff(a, angle).abs() < 0.35);
        self.glance = Some((angle, 0.4));
        if fresh {
            if self.yours() {
                self.look_cost = 0.3;
            }
            self.events.push(Event::Glance { angle });
        }
    }

    /// A pass of yours in world direction `dir` at `pace` m/s. With the ball coming it waits and goes
    /// first time. Without the ball: nothing.
    pub fn kick(&mut self, dir: V2, pace: f32) -> bool {
        let Some(h) = self.cfg.human else { return false };
        let pace = pace.clamp(4.0, 28.0);
        if self.incoming().is_some() {
            self.queued = Some((dir.norm(), pace));
            return true;
        }
        if !self.yours() {
            return false;
        }
        let dir = self.assist(dir, pace);
        self.pass_vel(h, dir * pace, false);
        true
    }

    /// A gentle hand on the aim: within a few degrees of where a teammate will meet it, it goes there.
    fn assist(&self, dir: V2, pace: f32) -> V2 {
        let h = self.human();
        let from = self.ball_pos;
        let want = dir.angle();
        let mut best = (0.12f32, dir.norm());
        for (j, p) in self.players.iter().enumerate() {
            if j == h || p.team != self.players[h].team {
                continue;
            }
            let at = p.pos + p.vel * (from.dist(p.pos) / pace.max(4.0));
            let d = angle_diff((at - from).angle(), want).abs();
            if d < best.0 {
                best = (d, (at - from).norm());
            }
        }
        best.1
    }

    /// A tap at a world point: with the ball (or it coming), a pass to the teammate nearest the point,
    /// weighted for you; without, a glance toward it.
    pub fn tap(&mut self, at: V2) -> bool {
        let Some(h) = self.cfg.human else { return false };
        let you = self.players[h].pos;
        if self.yours() || self.incoming().is_some() {
            let team = self.players[h].team;
            let best = (0..self.players.len()).filter(|&j| j != h && self.players[j].team == team).min_by(|&a, &b| self.players[a].pos.dist(at).partial_cmp(&self.players[b].pos.dist(at)).unwrap());
            if let Some(j) = best {
                if self.players[j].pos.dist(at) < 6.0 {
                    let from = if self.yours() { self.ball_pos } else { you };
                    let (v, _) = self.weigh(from, j, team, h, true);
                    if self.incoming().is_some() {
                        self.queued = Some((v.norm(), v.len()));
                    } else {
                        self.pass_vel(h, v, false);
                    }
                    return true;
                }
            }
            return false;
        }
        self.glance((at - you).angle());
        true
    }

    /// A pass toward the teammate who best answers `dir`, weighted for you (strength 1: as weighed).
    pub fn flick(&mut self, dir: V2, strength: f32) -> bool {
        let Some(h) = self.cfg.human else { return false };
        if !self.yours() {
            return self.kick(dir, 13.0 * strength);
        }
        let want = dir.angle();
        let pick = self.lanes().into_iter().filter(|l| angle_diff((l.b - l.a).angle(), want).abs() < 0.96).max_by(|a, b| {
            let s = |l: &Lane| angle_diff((l.b - l.a).angle(), want).cos() * (0.55 + 0.45 * l.seen.open);
            s(a).partial_cmp(&s(b)).unwrap()
        });
        let Some(l) = pick else { return false };
        let (v, _) = self.weigh(self.ball_pos, l.to, self.players[h].team, h, true);
        self.pass_vel(h, v * strength.clamp(0.7, 1.4), false);
        true
    }

    fn pass_vel(&mut self, from: usize, vel: V2, first_time: bool) {
        let a = self.ball_pos;
        let team = self.players[from].team;
        let o = self.predict(a, vel, team, from, false);
        // who it is meant for: the teammate it would reach, else the one nearest its line
        let end = self.roll(a, vel, 20.0).0;
        let to = match (o.fate, o.by) {
            (Fate::Received, Some(j)) => j,
            _ => (0..self.players.len())
                .filter(|&j| j != from && self.players[j].team == team)
                .min_by(|&x, &y| {
                    let d = |q: usize| closest_on_segment(self.players[q].pos, a, end).0.dist(self.players[q].pos);
                    d(x).partial_cmp(&d(y)).unwrap()
                })
                .unwrap_or(from),
        };
        self.ball = Ball::Rolling { vel, from, to: Some(to), first_time, t: 0.0 };
        if Some(from) == self.cfg.human {
            self.your_touches += 1;
        }
        let open = if o.fate == Fate::Received { smoothstep(-0.05, 0.6, o.margin) } else { 0.0 };
        let length = vel.len() * vel.len() / (2.0 * self.cfg.friction);
        self.events.push(Event::Pass { from, to, at: a, open, angle: vel.angle(), length, speed: vel.len(), first_time, yours: Some(from) == self.cfg.human });
    }

    fn face_ball(&mut self, k: f32) {
        for i in 0..self.players.len() {
            let to = self.ball_pos - self.players[i].pos;
            if to.len() > 0.3 {
                let f = self.players[i].facing;
                self.players[i].facing = f + angle_diff(to.angle(), f) * k;
            }
        }
    }

    /// Time moves by `dt` real seconds.
    pub fn step(&mut self, dt: f32) {
        let dt = dt.clamp(0.0, 0.1);
        self.real_t += dt as f64;
        let target = if self.cfg.human.is_none() || (self.yours() && self.look_cost > 0.0) {
            1.0
        } else if self.yours() {
            self.cfg.hold_pace
        } else if let Some(rem) = self.incoming() {
            // softening, not stopping: the ball still arrives in about a real second
            0.3 + 0.7 * smoothstep(0.0, self.cfg.soften, rem)
        } else {
            1.0
        };
        let k = 1.0 - (-dt * if target < self.pace { 10.0 } else if self.look_cost > 0.0 { 14.0 } else { 3.0 }).exp();
        self.pace += (target - self.pace) * k;
        let wdt = dt * self.pace;
        self.t += wdt as f64;
        self.look_cost = if self.yours() { (self.look_cost - wdt).max(0.0) } else { 0.0 };

        if let Some((a, left)) = self.glance {
            let left = left - dt;
            self.glance = (left > 0.0).then_some((a, left));
        }
        if let Some(h) = self.cfg.human {
            let face = match self.glance {
                Some((a, _)) => a,
                None if self.yours() => self.players[h].facing,
                None => (self.ball_pos - self.players[h].pos).angle(),
            };
            let f = self.players[h].facing;
            self.players[h].facing = f + angle_diff(face, f) * (1.0 - (-dt * 14.0).exp());
        }

        self.see(wdt);
        self.shape(wdt);
        self.read(wdt);
        self.think(wdt);
        self.move_players(wdt);
        self.ball_step(wdt);
        let k = 1.0 - (-wdt * 6.0).exp();
        for i in 0..self.players.len() {
            if Some(i) == self.cfg.human {
                continue;
            }
            let to = self.ball_pos - self.players[i].pos;
            // an agent with the ball faces where it means to pass (the pressers read that)
            let want = if self.holder() == Some(i) && self.players[i].lean.len() > 0.5 {
                Some(self.players[i].lean.angle())
            } else if to.len() > 0.3 {
                Some(to.angle())
            } else {
                None
            };
            if let Some(w) = want {
                let f = self.players[i].facing;
                self.players[i].facing = f + angle_diff(w, f) * k;
            }
        }
    }

    /// What you see: your view cone, a glance's cone, anyone within a few metres.
    fn see(&mut self, wdt: f32) {
        let Some(h) = self.cfg.human else {
            for p in self.players.iter_mut() {
                p.seen = p.pos;
            }
            return;
        };
        let you = self.players[h].pos;
        let facing = self.players[h].facing;
        let glance = self.glance.map(|g| g.0);
        let (view, gl) = (self.cfg.view, self.cfg.glance);
        for (i, p) in self.players.iter_mut().enumerate() {
            if i == h {
                continue;
            }
            let to = p.pos - you;
            let a = to.angle();
            if to.len() < 3.0 || angle_diff(a, facing).abs() < view || glance.is_some_and(|g| angle_diff(a, g).abs() < gl) {
                p.seen = p.pos;
                p.seen_age = 0.0;
            } else {
                p.seen_age += wdt;
            }
        }
    }

    /// The rondo shape breathes: its centre leans toward the ball and turns slowly back and forth.
    fn shape(&mut self, wdt: f32) {
        let t = self.t as f32;
        let d = self.cfg.drift;
        let want = self.ball_pos * self.cfg.follow + v2((t * 0.05).sin() * 2.0, (t * 0.037).cos() * 1.5) * d;
        self.center = self.center.lerp(want, 1.0 - (-wdt * 0.6).exp());
        self.turn = ((t * 0.021).sin() * 0.22 + (t * 0.0083).sin() * 0.3) * d;
    }

    /// Where player `i`'s home is now. In a rondo: its slot in the breathing shape. On a pitch: its
    /// slot shifted with the ball, the side without it squeezed toward the ball.
    fn home(&self, i: usize) -> V2 {
        let p = &self.players[i];
        if self.cfg.swap {
            let squeeze = if p.team == self.attack { 1.0 } else { 0.72 };
            let shift = v2(self.ball_pos.x * self.cfg.follow, self.ball_pos.y * self.cfg.follow * 0.6);
            let at = p.home * squeeze + shift;
            v2(at.x.clamp(-self.cfg.half.x + 1.0, self.cfg.half.x - 1.0), at.y.clamp(-self.cfg.half.y + 1.0, self.cfg.half.y - 1.0))
        } else {
            let (s, c) = self.turn.sin_cos();
            self.center + v2(p.home.x * c - p.home.y * s, p.home.x * s + p.home.y * c)
        }
    }

    /// The pressers read the passer's body: where it looks (a glance, else its facing) is where they
    /// guess the pass will go; held long enough, the guess becomes a commitment.
    fn read(&mut self, wdt: f32) {
        let Some(by) = self.holder() else {
            for p in self.players.iter_mut() {
                p.commit = (p.commit - wdt * 1.5).max(0.0);
            }
            return;
        };
        let look = if Some(by) == self.cfg.human { self.glance.map(|g| g.0).unwrap_or(self.players[by].facing) } else { self.players[by].facing };
        let dir = V2::from_angle(look);
        let team = self.players[by].team;
        for p in self.players.iter_mut() {
            if p.team == team || p.stun > 0.0 {
                continue;
            }
            let r = p.traits.read;
            if p.lean.dot(dir) < 0.7 {
                // a new guess: the old commitment has to come down first
                p.commit = (p.commit - wdt * 2.5).max(0.0);
                if p.commit < 0.15 {
                    p.lean = p.lean.lerp(dir, (wdt * 16.0 * r).min(1.0)).norm();
                }
            } else {
                p.lean = p.lean.lerp(dir, (wdt * 3.0).min(1.0)).norm();
                p.commit = (p.commit + wdt * 3.0 * r).min(r);
            }
        }
    }

    fn snapshot(&self) -> Snapshot {
        Snapshot { pos: self.players.iter().map(|p| p.pos).collect(), team: self.players.iter().map(|p| p.team).collect(), stun: self.players.iter().map(|p| p.stun > 0.0).collect() }
    }

    pub fn lane_params(&self) -> crate::lane::LaneParams {
        crate::lane::LaneParams { ball_speed: (self.cfg.kick[0] + self.cfg.kick[1]) * 0.5, presser_speed: self.cfg.run_speed, reach: self.cfg.reach, react: self.cfg.react }
    }

    /// Where the nearest player of `team` meets the rolling ball: (player, point).
    fn meets(&self, team: u8, vel: V2) -> Option<(usize, V2)> {
        let mut best: Option<(usize, f32, V2)> = None;
        let fresh = match self.ball {
            Ball::Rolling { from, t, .. } if t < 0.4 => from,
            _ => usize::MAX,
        };
        for i in 0..self.players.len() {
            if self.players[i].team != team || self.players[i].stun > 0.0 || i == fresh {
                continue;
            }
            for k in 0..=30 {
                let tt = k as f32 * 0.1;
                let (p, _) = self.roll(self.ball_pos, vel, tt);
                let rt = self.reach_time(i, self.players[i].pos, p, 0.6);
                if rt <= tt + 0.05 || k == 30 {
                    let when = rt.max(tt);
                    if best.is_none_or(|b| when < b.1) {
                        best = Some((i, when, p));
                    }
                    break;
                }
            }
        }
        best.map(|b| (b.0, b.2))
    }

    /// Choosing where to go.
    fn think(&mut self, wdt: f32) {
        let n = self.players.len();
        let (rolling, ball_vel, receiver) = match self.ball {
            Ball::Rolling { vel, to, .. } => (true, vel, to),
            _ => (false, V2::ZERO, None),
        };
        let chase = if rolling { [self.meets(0, ball_vel), self.meets(1, ball_vel)] } else { [None, None] };
        let ball_from = if rolling { self.roll(self.ball_pos, ball_vel, 1.0).0 } else { self.ball_pos };
        let attack = self.attack;
        let snap = self.snapshot();
        let lp = self.lane_params();
        let others = snap.others(attack);

        // the side with the ball: offer an angle, run in behind
        for i in 0..n {
            if self.players[i].team != attack {
                continue;
            }
            self.players[i].retarget -= wdt;
            self.players[i].running = (self.players[i].running - wdt).max(0.0);
            if let Some((c, at)) = chase[attack as usize] {
                if c == i {
                    self.players[i].target = at;
                    continue;
                }
            }
            if rolling && Some(i) == receiver {
                continue;
            }
            if self.holder() == Some(i) {
                self.players[i].target = self.players[i].pos;
                continue;
            }
            if self.players[i].running > 0.0 {
                self.players[i].target = self.players[i].run_to;
                continue;
            }
            if self.players[i].retarget > 0.0 {
                continue;
            }
            let pivot = Some(i) == self.cfg.human;
            let tr = self.players[i].traits;
            self.players[i].retarget = if pivot { 0.15 } else { 0.25 + self.rng.f() * 0.2 };
            let home = self.home(i);
            // a run in behind, now and then: past the pressers, away from the ball
            if !pivot && self.holder().is_some() && self.rng.f() < tr.run * 0.2 {
                let out = (self.players[i].pos - ball_from).norm();
                let fwd = if self.cfg.swap { v2(if attack == 0 { 1.0 } else { -1.0 }, 0.0) } else { out };
                let to = self.players[i].pos + (fwd * 0.7 + out * 0.3).norm() * (6.0 + 5.0 * self.rng.f());
                let to = v2(to.x.clamp(-self.cfg.half.x + 1.0, self.cfg.half.x - 1.0), to.y.clamp(-self.cfg.half.y + 1.0, self.cfg.half.y - 1.0));
                self.players[i].run_to = to;
                self.players[i].running = 1.6 + self.rng.f();
                self.players[i].target = to;
                continue;
            }
            let mut best = (f32::NEG_INFINITY, home);
            let (k_n, r) = if pivot { (13, 4.5) } else { (9, 1.2 + 3.0 * tr.offer) };
            for k in 0..k_n {
                let c = if k == 0 { home } else { home + V2::from_angle(k as f32 * PI * 2.0 / (k_n - 1) as f32 + self.rng.s() * 0.3) * (r * (0.5 + 0.5 * self.rng.f()) + 0.6) };
                let open = crate::lane::openness(ball_from, c, &others, &lp).open;
                let mut s = open * (0.5 + tr.offer) - c.dist(home) * 0.07;
                if let Some(h) = self.cfg.human {
                    if h != i && self.players[h].team == attack {
                        s += 0.25 * crate::lane::openness(c, self.players[h].pos, &others, &lp).open;
                    }
                }
                for (j, q) in snap.pos.iter().enumerate() {
                    if j != i && snap.team[j] == attack {
                        s -= smoothstep(4.0, 0.0, q.dist(c)) * 0.6;
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

        // the side without it: one to the ball, the others into the most open lanes (or home)
        let defend: Vec<usize> = (0..n).filter(|&i| self.players[i].team != attack).collect();
        if defend.is_empty() {
            return;
        }
        let ball = self.ball_pos;
        let dteam = 1 - attack as usize;
        let to_ball = match chase[dteam] {
            Some((c, _)) => Some(c),
            None => defend.iter().copied().filter(|&i| self.players[i].stun <= 0.0).min_by(|&a, &b| self.players[a].pos.dist(ball).partial_cmp(&self.players[b].pos.dist(ball)).unwrap()),
        };
        let mut lanes: Vec<(f32, V2, V2)> = Vec::new();
        for j in 0..n {
            if self.players[j].team != attack || Some(j) == self.holder() {
                continue;
            }
            let o = crate::lane::openness(ball_from, self.players[j].pos, &others, &lp).open;
            lanes.push((o, ball_from, self.players[j].pos));
        }
        let mut taken = vec![false; lanes.len()];
        for &i in &defend {
            let p = self.players[i].clone();
            if p.stun > 0.0 {
                self.players[i].target = p.pos + (p.pos - ball).norm() * 2.0;
                continue;
            }
            if Some(i) == to_ball {
                let t = match chase[dteam] {
                    Some((c, at)) if c == i => at,
                    _ => ball,
                };
                let held = match self.ball {
                    Ball::Held { t: h, .. } => h,
                    _ => 0.0,
                };
                // stand off the holder, closer the longer it holds, shading toward its guess
                let off = (p.pos - t).norm() * ((1.3 - 0.5 * p.traits.press) - held * 0.5).max(0.2);
                self.players[i].target = t + off + p.lean * (p.commit * 1.2 * (1.0 - held).max(0.0));
                continue;
            }
            if p.retarget > 0.0 {
                self.players[i].retarget -= wdt;
                // a committed presser drifts toward the lane it guessed
                if p.commit > 0.2 && !rolling {
                    let guess = ball + p.lean * 5.0;
                    let t = self.players[i].target.lerp(guess, (wdt * p.commit * 1.5).min(1.0));
                    self.players[i].target = t;
                }
                continue;
            }
            self.players[i].retarget = 0.6 + self.rng.f() * 0.5;
            let mut best: Option<(f32, usize, V2)> = None;
            for (k, (o, a, b)) in lanes.iter().enumerate() {
                if taken[k] {
                    continue;
                }
                let (c, u) = closest_on_segment(p.pos, *a, *b);
                if c.dist(p.pos) > 18.0 {
                    continue;
                }
                let u = u.clamp(0.3, 0.75);
                let c = a.lerp(*b, u).lerp(c, 0.3);
                let guessed = p.commit * smoothstep(0.5, 1.0, (*b - *a).norm().dot(p.lean));
                let s = *o * (0.5 + 0.4 * p.traits.press) + guessed - p.pos.dist(c) * 0.08 + self.rng.f() * 0.3;
                if best.is_none_or(|bb| s > bb.0) {
                    best = Some((s, k, c));
                }
            }
            if let Some((_, k, c)) = best {
                taken[k] = true;
                self.players[i].target = c + v2(self.rng.s(), self.rng.s()) * 0.5;
            } else {
                self.players[i].target = self.home(i);
            }
        }
    }

    fn move_players(&mut self, wdt: f32) {
        if wdt <= 0.0 {
            return;
        }
        let n = self.players.len();
        let (hx, hy) = (self.cfg.half.x, self.cfg.half.y);
        for i in 0..n {
            let (pos, vel, target) = (self.players[i].pos, self.players[i].vel, self.players[i].target);
            let mut max = self.speed_of(i);
            if Some(i) == self.cfg.human {
                max *= 0.95;
            }
            if self.holder() == Some(i) {
                max *= 0.25;
            }
            let to = target - pos;
            let want = to.norm() * (max * smoothstep(0.0, 2.5, to.len()));
            let mut acc = (want - vel) * 3.5;
            for j in 0..n {
                if i == j {
                    continue;
                }
                let away = pos - self.players[j].pos;
                let l = away.len();
                // keep apart (opponents may come closer: that is pressing)
                let room = if self.players[j].team == self.players[i].team { 1.6 } else { 0.7 };
                if l < room && l > 1e-4 {
                    acc += away.norm() * (room - l) * 6.0;
                }
            }
            let ph = self.players[i].phase + self.t as f32 * 0.7;
            acc += v2(ph.sin(), (ph * 1.3).cos()) * 0.35;
            let p = &mut self.players[i];
            p.vel = (p.vel + acc * wdt).clamp_len(max * 1.1);
            p.pos += p.vel * wdt;
            p.pos.x = p.pos.x.clamp(-hx, hx);
            p.pos.y = p.pos.y.clamp(-hy, hy);
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
                let team = self.players[by].team;
                // a ball held too long (world time) can be taken off the foot
                if t > 0.6 {
                    let near = (0..self.players.len()).find(|&q| self.players[q].team != team && self.players[q].stun <= 0.0 && self.players[q].pos.dist(self.ball_pos) < self.cfg.reach * 0.6);
                    if let Some(i) = near {
                        let out = (self.ball_pos - self.players[i].pos).norm().perp() * (self.cfg.control * 0.35);
                        self.won_by(i, out);
                        return;
                    }
                }
                if self.cfg.goals {
                    let goal = v2(if team == 0 { self.cfg.half.x } else { -self.cfg.half.x }, 0.0);
                    if self.ball_pos.dist(goal) < 14.0 {
                        self.events.push(Event::Chance { team, at: self.ball_pos });
                        let keeper = (0..self.players.len()).filter(|&q| self.players[q].team != team).min_by(|&a, &b| self.players[a].pos.dist(goal).partial_cmp(&self.players[b].pos.dist(goal)).unwrap());
                        if let Some(k) = keeper {
                            self.ball = Ball::Dead { to: k, t: 0.0 };
                            return;
                        }
                    }
                }
                if Some(by) != self.cfg.human && t > hold {
                    self.agent_passes(by, t);
                }
            }
            Ball::Rolling { vel, from, to, first_time, t } => {
                let (p, v) = self.roll(self.ball_pos, vel, wdt);
                self.ball_pos = p;
                let t = t + wdt;
                self.ball = Ball::Rolling { vel: v, from, to, first_time, t };
                let speed = v.len();
                let passer_team = self.players.get(from).map(|q| q.team).unwrap_or(self.attack);
                if p.x.abs() > self.cfg.half.x || p.y.abs() > self.cfg.half.y {
                    self.events.push(Event::Out { at: p });
                    self.chain = 0;
                    self.queued = None;
                    let team = if self.cfg.swap { 1 - passer_team } else { 0 };
                    let at = v2(p.x.clamp(-self.cfg.half.x, self.cfg.half.x), p.y.clamp(-self.cfg.half.y, self.cfg.half.y));
                    let to = (0..self.players.len()).filter(|&q| self.players[q].team == team && Some(q) != self.cfg.human).min_by(|&a, &b| self.players[a].pos.dist(at).partial_cmp(&self.players[b].pos.dist(at)).unwrap()).unwrap_or(0);
                    self.ball_pos = at;
                    self.ball = Ball::Dead { to, t: 0.0 };
                    return;
                }
                // the other side's feet first: a ball within reach is poked or won
                let loose = from == usize::MAX;
                for i in 0..self.players.len() {
                    let q = &self.players[i];
                    if (q.team == passer_team && !loose) || q.stun > 0.0 || q.pos.dist(p) > self.cfg.reach * 0.85 {
                        continue;
                    }
                    if loose && (q.team == self.attack || !self.cfg.swap) {
                        continue;
                    }
                    let dir = v.norm();
                    let side = if (q.pos - p).cross(dir) > 0.0 { -1.0 } else { 1.0 };
                    let out = (dir * 0.5 + dir.perp() * side * (0.6 + self.rng.f() * 0.5)).norm();
                    self.won_by(i, out * (speed * 0.45 + 2.0));
                    return;
                }
                // then whoever of the passer's side can control it
                let got = (0..self.players.len()).find(|&i| {
                    let q = &self.players[i];
                    let r = if Some(i) == to { 0.95 } else { 0.75 };
                    (q.team == passer_team || (loose && q.team == self.attack)) && q.stun <= 0.0 && !(i == from && t < 0.4) && q.pos.dist(p) < r && speed <= self.cfg.control
                });
                if let Some(by) = got {
                    let stride = (self.players[by].vel.len() / self.speed_of(by).max(0.1)).min(1.0);
                    let ease = 1.0 - ((speed - self.cfg.control * 0.45) / (self.cfg.control * 0.55)).abs().min(1.0);
                    let grace = (ease * (0.55 + 0.45 * stride) * if Some(by) == to { 1.0 } else { 0.6 }).clamp(0.0, 1.0);
                    self.ball = Ball::Held { by, t: 0.0, hold: self.hold_time(by) };
                    self.attack = self.players[by].team;
                    self.players[by].running = 0.0;
                    if loose || from == by {
                        self.events.push(Event::Recovered { by });
                    } else {
                        self.chain += 1;
                        self.events.push(Event::Received { by, first_time, grace });
                    }
                    if Some(by) == self.cfg.human {
                        if let Some((dir, pace)) = self.queued.take() {
                            self.events.push(Event::Queued);
                            let dir = self.assist(dir, pace);
                            self.pass_vel(by, dir * pace, true);
                        }
                    }
                    return;
                }
                // a dead ball nobody claimed: the nearest takes it (either side, with `swap`)
                if speed < 0.3 {
                    let near = (0..self.players.len()).filter(|&i| self.players[i].stun <= 0.0 && (self.cfg.swap || self.players[i].team == self.attack)).min_by(|&a, &b| self.players[a].pos.dist(p).partial_cmp(&self.players[b].pos.dist(p)).unwrap());
                    if let Some(i) = near {
                        if self.players[i].pos.dist(p) < 0.8 {
                            if self.players[i].team != self.attack {
                                self.attack = self.players[i].team;
                                self.events.push(Event::Turnover { team: self.attack, by: i });
                            } else {
                                self.events.push(Event::Recovered { by: i });
                            }
                            self.ball = Ball::Held { by: i, t: 0.0, hold: self.hold_time(i) };
                        }
                    }
                }
            }
            Ball::Dead { to, t } => {
                let t = t + wdt;
                self.players[to].target = self.ball_pos;
                if t > 0.8 || self.players[to].pos.dist(self.ball_pos) < 1.0 {
                    self.players[to].pos = self.ball_pos;
                    self.attack = self.players[to].team;
                    self.ball = Ball::Held { by: to, t: 0.0, hold: self.hold_time(to) + 0.3 };
                    self.events.push(Event::Recovered { by: to });
                } else {
                    self.ball = Ball::Dead { to, t };
                }
            }
        }
    }

    /// A player of the other side gets a foot to it: in a rondo it is poked loose (the poker stunned a
    /// moment); with `swap` a slowish ball is won outright.
    fn won_by(&mut self, i: usize, loose: V2) {
        self.chain = 0;
        self.queued = None;
        self.events.push(Event::Poked { by: i, at: self.ball_pos });
        let slow = match self.ball {
            Ball::Rolling { vel, .. } => vel.len() < self.cfg.control * 0.7,
            _ => true,
        };
        if self.cfg.swap && slow {
            self.attack = self.players[i].team;
            self.ball = Ball::Held { by: i, t: 0.0, hold: self.hold_time(i) };
            self.events.push(Event::Turnover { team: self.attack, by: i });
        } else {
            self.ball = Ball::Rolling { vel: loose, from: usize::MAX, to: None, first_time: false, t: 0.0 };
            self.players[i].stun = 1.4;
        }
        for p in self.players.iter_mut() {
            p.commit = 0.0;
        }
    }

    fn hold_time(&mut self, i: usize) -> f32 {
        let s = self.players[i].traits.scan;
        self.rng.range(self.cfg.hold[0], self.cfg.hold[1]) * (1.3 - 0.6 * s)
    }

    /// An agent with the ball weighs a pass to each teammate (a runner where it is going) and plays
    /// the best, taking more risk the bolder it is; in a game it leans toward you.
    fn agent_passes(&mut self, by: usize, held: f32) {
        let team = self.players[by].team;
        let tr = self.players[by].traits;
        let from = self.ball_pos;
        let fwd = if self.cfg.swap { v2(if team == 0 { 1.0 } else { -1.0 }, 0.0) } else { V2::ZERO };
        let mut best: Option<(f32, V2, usize, f32)> = None;
        for j in 0..self.players.len() {
            if j == by || self.players[j].team != team || self.players[j].pos.dist(from) < 3.0 {
                continue;
            }
            let (v, o) = self.weigh(from, j, team, by, false);
            let ok = o.fate == Fate::Received;
            let safe = if ok { smoothstep(-0.05, 0.6, o.margin) } else { 0.0 };
            let gain = (o.at - from).dot(fwd) * 0.03 + if self.players[j].running > 0.0 { 0.4 } else { 0.0 };
            let mut s = if ok { safe * (1.2 - tr.risk) + gain * (0.3 + tr.risk) + 0.3 } else { -0.5 };
            if Some(j) == self.cfg.human {
                s = s * self.cfg.to_you + if safe > 0.3 { 0.5 } else { 0.0 };
            }
            s += self.rng.f() * (0.35 - 0.25 * tr.scan);
            if best.is_none_or(|b| s > b.0) {
                best = Some((s, v, j, safe));
            }
        }
        let Some((s, v, j, safe)) = best else { return };
        // it faces the pass it is choosing (and the pressers read that)
        self.players[by].lean = (self.players[j].pos - from).norm();
        if safe < 0.2 && held < 2.5 && s < 0.6 {
            return; // shield it a moment longer
        }
        if let Some(h) = self.cfg.human {
            if self.players[h].team == team && h != j && held < self.cfg.hold[1] + 0.8 && self.rng.f() < 0.9 {
                let (_, oy) = self.weigh(from, h, team, by, false);
                if oy.fate == Fate::Received && oy.margin > -0.2 && oy.margin < 0.15 {
                    return; // wait a beat for the pivot to come free
                }
            }
        }
        self.pass_vel(by, v, false);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(w: &mut World, secs: f32) -> Vec<Event> {
        let mut ev = Vec::new();
        for _ in 0..(secs * 60.0) as usize {
            w.step(1.0 / 60.0);
            ev.extend(w.drain());
        }
        ev
    }

    fn until_yours(w: &mut World) {
        while !w.yours() {
            w.step(1.0 / 60.0);
            assert!(w.real_t < 90.0, "the ball never came to the pivot");
        }
    }

    #[test]
    fn the_ball_comes_to_you_and_time_holds() {
        let mut w = World::new(Config::default());
        until_yours(&mut w);
        run(&mut w, 1.0);
        assert!(w.pace < 0.1, "time holds while you decide: pace {}", w.pace);
        let before = w.t;
        run(&mut w, 10.0);
        assert!(w.t - before < 1.0, "ten real seconds are under one world second while you hold: {}", w.t - before);
    }

    #[test]
    fn a_pass_and_time_returns() {
        let mut w = World::new(Config::default());
        until_yours(&mut w);
        let l = w.lanes().into_iter().max_by(|a, b| a.seen.open.partial_cmp(&b.seen.open).unwrap()).unwrap();
        assert!(w.kick(l.b - l.a, l.pace));
        assert!(!w.yours());
        let mut top: f32 = 0.0;
        for _ in 0..60 {
            w.step(1.0 / 60.0);
            top = top.max(w.pace);
        }
        assert!(top > 0.5, "pace back up: {top}");
    }

    #[test]
    fn weight_matters() {
        // one direction, three paces: the weighed one arrives; far too firm is not controlled by the
        // same player; far too soft is slower or cut
        let mut w = World::new(Config::default());
        until_yours(&mut w);
        let l = w.lanes().into_iter().max_by(|a, b| a.truly.open.partial_cmp(&b.truly.open).unwrap()).unwrap();
        let dir = (l.b - l.a).norm();
        let right = w.predict(w.ball_pos, dir * l.pace, 0, 0, false);
        let firm = w.predict(w.ball_pos, dir * 30.0, 0, 0, false);
        let soft = w.predict(w.ball_pos, dir * 3.0, 0, 0, false);
        assert_eq!(right.fate, Fate::Received, "{right:?}");
        assert!(firm.fate != Fate::Received || firm.by != right.by, "{firm:?}");
        assert!(soft.fate != Fate::Received || soft.t > right.t, "{soft:?}");
    }

    #[test]
    fn a_look_with_the_ball_costs_world_time_and_pressers_buy_it() {
        let mut w = World::new(Config::default());
        until_yours(&mut w);
        run(&mut w, 1.0);
        let before = w.t;
        w.glance(0.0);
        for _ in 0..20 {
            w.step(1.0 / 60.0);
            w.glance(0.0);
        }
        assert!(w.t - before > 0.2, "a glance spent {} world s", w.t - before);
        let dir = V2::from_angle(0.0);
        let leaning = w.players.iter().filter(|p| p.team == 1 && p.commit > 0.05 && p.lean.dot(dir) > 0.7).count();
        assert!(leaning >= 1, "someone bought the look: {:?}", w.players.iter().filter(|p| p.team == 1).map(|p| (p.lean, p.commit)).collect::<Vec<_>>());
    }

    #[test]
    fn held_forever_it_is_taken() {
        let mut w = World::new(Config::default());
        until_yours(&mut w);
        let start = w.real_t;
        let mut lost = false;
        for _ in 0..(60 * 120) {
            w.step(1.0 / 60.0);
            if !w.yours() {
                lost = true;
                break;
            }
        }
        assert!(lost, "two real minutes of holding and nobody came for it");
        assert!(w.real_t - start > 6.0, "but not in a hurry: {}", w.real_t - start);
    }

    #[test]
    fn the_rondo_goes_on_without_you() {
        let mut w = World::new(Config { human: None, ..Config::default() });
        let ev = run(&mut w, 120.0);
        let passes = ev.iter().filter(|e| matches!(e, Event::Pass { .. })).count();
        let pokes = ev.iter().filter(|e| matches!(e, Event::Poked { .. })).count();
        assert!(passes > 40, "passes {passes}");
        assert!(pokes >= 1, "pokes {pokes}");
        for p in &w.players {
            assert!(p.pos.x.is_finite() && p.pos.x.abs() <= w.cfg.half.x);
        }
    }

    #[test]
    fn eleven_a_side_plays_itself() {
        let mut w = World::new(Config::eleven());
        let ev = run(&mut w, 180.0);
        let passes = ev.iter().filter(|e| matches!(e, Event::Pass { .. })).count();
        let turnovers = ev.iter().filter(|e| matches!(e, Event::Turnover { .. })).count();
        assert!(passes > 40, "passes {passes}");
        assert!(turnovers >= 1, "turnovers {turnovers}");
    }

    #[test]
    fn what_you_have_not_seen_you_remember() {
        let mut w = World::new(Config::default());
        run(&mut w, 8.0);
        let (i, _) = w.players.iter().enumerate().filter(|(_, p)| p.team == 1).max_by(|a, b| a.1.seen_age.partial_cmp(&b.1.seen_age).unwrap()).unwrap();
        assert!(w.players[i].seen_age > 0.0);
        let a = (w.players[i].pos - w.players[0].pos).angle();
        w.glance(a);
        w.step(1.0 / 60.0);
        assert_eq!(w.players[i].seen_age, 0.0);
    }

    #[test]
    fn a_snapshot_plays_on() {
        let specs = vec![Spec::new(0, v2(0.0, 0.0)), Spec::new(0, v2(10.0, 0.0)), Spec::new(1, v2(5.0, 5.0))];
        let mut w = World::from_snapshot(Config { human: None, swap: true, ..Config::default() }, specs, v2(0.4, 0.0), Some(0));
        let ev = run(&mut w, 10.0);
        assert!(ev.iter().any(|e| matches!(e, Event::Pass { .. })));
    }
}
