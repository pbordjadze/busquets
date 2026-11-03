//! Every player's path through a match, five samples a second, each honest about its source.
//!
//! The ball: through every on-ball event's location, along each pass and carry for its duration.
//! A player: a prior (their slot in the side's shape, the shape sliding with the ball and squeezing
//! without it), bent toward every real sighting by a residual that fades a few seconds either side of
//! it. Sightings are `Observed` (named on an event: the actor's spot, a pass's receiver at its end, a
//! carrier at its end) or `Seen` (a 360 freeze-frame dot, matched to the nearest predicted player of
//! that side: the place is real, the name is our guess). Between sightings it is `Inferred`, and its
//! confidence falls with the distance in time to the nearest sighting.
use crate::game::{EvKind, Game};
use pitch::{v2, V2};

pub const DT: f32 = 0.2;
/// How long a sighting bends the path either side of it, s.
const TAU: f32 = 3.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Quality {
    Off = 0,
    Inferred = 1,
    Seen = 2,
    Observed = 3,
}

impl Quality {
    fn from_u8(q: u8) -> Quality {
        match q {
            1 => Quality::Inferred,
            2 => Quality::Seen,
            3 => Quality::Observed,
            _ => Quality::Off,
        }
    }
}

pub struct Tracks {
    pub n: usize,
    pub ball: Vec<V2>,
    /// The ball's height, m (a rough arc for lofted passes).
    pub ball_z: Vec<f32>,
    /// pos[person][sample]
    pub pos: Vec<Vec<V2>>,
    pub quality: Vec<Vec<u8>>,
    /// 0..255: how near in time the nearest real sighting is.
    pub conf: Vec<Vec<u8>>,
    /// How many 360 dots were matched to someone, of how many offered.
    pub matched: (usize, usize),
}

/// A slot in the side's shape (StatsBomb position id), metres, attacking +x, y to its left.
pub fn slot(position: u32) -> V2 {
    let (x, y) = match position {
        1 => (-47.0, 0.0),
        2 => (-18.0, -26.0),
        3 => (-26.0, -10.0),
        4 => (-28.0, 0.0),
        5 => (-26.0, 10.0),
        6 => (-18.0, 26.0),
        7 => (-10.0, -28.0),
        8 => (-10.0, 28.0),
        9 => (-12.0, -8.0),
        10 => (-14.0, 0.0),
        11 => (-12.0, 8.0),
        12 => (2.0, -24.0),
        13 => (-4.0, -11.0),
        14 => (-4.0, 0.0),
        15 => (-4.0, 11.0),
        16 => (2.0, 24.0),
        17 => (12.0, -24.0),
        18 => (6.0, -10.0),
        19 => (6.0, 0.0),
        20 => (6.0, 10.0),
        21 => (12.0, 24.0),
        22 => (16.0, -6.0),
        23 => (18.0, 0.0),
        24 => (16.0, 6.0),
        25 => (12.0, 0.0),
        _ => (-5.0, 0.0),
    };
    v2(x, y)
}

/// Where a slot stands given the ball (smoothed) and who has it. `side` 0 attacks +x, 1 attacks -x.
pub fn shape_at(position: u32, side: u8, ball: V2, in_poss: bool) -> V2 {
    let sg = if side == 0 { 1.0 } else { -1.0 };
    let b = ball * sg;
    let s = slot(position);
    let p = if position == 1 {
        v2(-47.5 + 0.06 * (b.x + 52.5), 0.12 * b.y)
    } else {
        let (comp, push, wid) = if in_poss { (0.95, 7.0, 1.05) } else { (0.72, -5.0, 0.78) };
        let shift = (0.55 * b.x + push).clamp(-20.0, 24.0);
        v2(s.x * comp + shift, s.y * wid + 0.28 * b.y)
    };
    clamp_pitch(p * sg)
}

pub fn clamp_pitch(p: V2) -> V2 {
    v2(p.x.clamp(-52.0, 52.0), p.y.clamp(-33.5, 33.5))
}

/// Paths, their qualities and confidences, person by person.
type Rendered = (Vec<Vec<V2>>, Vec<Vec<u8>>, Vec<Vec<u8>>);

#[derive(Clone, Copy)]
struct Key {
    t: f32,
    at: V2,
    q: u8,
}

fn lerp_track(keys: &[(f32, V2)], t: f32) -> V2 {
    if keys.is_empty() {
        return V2::ZERO;
    }
    let i = keys.partition_point(|k| k.0 <= t);
    if i == 0 {
        return keys[0].1;
    }
    if i == keys.len() {
        return keys[i - 1].1;
    }
    let (a, b) = (keys[i - 1], keys[i]);
    let u = if b.0 > a.0 { (t - a.0) / (b.0 - a.0) } else { 1.0 };
    a.1.lerp(b.1, u)
}

impl Tracks {
    pub fn build(g: &Game) -> Tracks {
        let n = (g.end / DT).ceil() as usize + 1;
        // the ball
        let mut bk: Vec<(f32, V2)> = vec![];
        let mut zk: Vec<(f32, f32, f32)> = vec![]; // lofted flights: from, to, peak
        for e in &g.events {
            if !e.kind.on_ball() {
                continue;
            }
            let Some(at) = e.at else { continue };
            bk.push((e.t, at));
            if let Some(end) = e.end {
                let dur = if e.dur > 0.05 { e.dur } else { at.dist(end) / 15.0 };
                bk.push((e.t + dur, end));
                if e.kind == EvKind::Pass && e.height > 0 {
                    zk.push((e.t, e.t + dur, if e.height == 2 { (at.dist(end) * 0.12).clamp(2.0, 14.0) } else { 1.2 }));
                }
            }
        }
        bk.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        let mut ball = vec![V2::ZERO; n];
        let mut ball_z = vec![0.0f32; n];
        for (i, b) in ball.iter_mut().enumerate() {
            *b = lerp_track(&bk, i as f32 * DT);
        }
        for &(a, b, h) in &zk {
            let (i0, i1) = ((a / DT).ceil() as usize, ((b / DT) as usize).min(n - 1));
            for (i, z) in ball_z.iter_mut().enumerate().take(i1 + 1).skip(i0) {
                let u = ((i as f32 * DT - a) / (b - a).max(0.01)).clamp(0.0, 1.0);
                *z = z.max(4.0 * h * u * (1.0 - u));
            }
        }
        // the ball as the shape feels it (smoothed both ways, about 2 s)
        let mut slow = ball.clone();
        let k = 1.0 - (-DT / 1.6f32).exp();
        for i in 1..n {
            slow[i] = slow[i - 1].lerp(ball[i], k);
        }
        for i in (0..n.saturating_sub(1)).rev() {
            slow[i] = slow[i + 1].lerp(slow[i], 1.0 - k * 0.5);
        }
        // who has it, sample by sample
        let mut poss = vec![0u8; n];
        {
            let mut j = 0;
            for (i, ps) in poss.iter_mut().enumerate() {
                let t = i as f32 * DT;
                while j + 1 < g.events.len() && g.events[j + 1].t <= t {
                    j += 1;
                }
                *ps = g.events.get(j).map(|e| e.poss).unwrap_or(0);
            }
        }
        let prior = |p: usize, i: usize| -> V2 {
            let pe = &g.people[p];
            let t = i as f32 * DT;
            shape_at(pe.position_at(t), pe.side, slow[i], poss[i] == pe.side)
        };
        // sightings named on events
        let np = g.people.len();
        let mut keys: Vec<Vec<Key>> = vec![vec![]; np];
        for e in &g.events {
            let Some(at) = e.at else { continue };
            if let Some(w) = e.who {
                if e.kind != EvKind::Other || e.kind == EvKind::Foul {
                    keys[w].push(Key { t: e.t, at, q: 3 });
                }
                if e.kind == EvKind::Carry {
                    if let Some(end) = e.end {
                        keys[w].push(Key { t: e.t + e.dur, at: end, q: 3 });
                    }
                }
            }
            if e.kind == EvKind::Pass && e.ok {
                if let (Some(r), Some(end)) = (e.to, e.end) {
                    keys[r].push(Key { t: e.t + e.dur.max(0.3), at: end, q: 3 });
                }
            }
        }
        let first = Self::render(g, n, &keys, &prior);
        // the 360 dots, matched to the nearest predicted player of their side
        let (mut matched, mut offered) = (0, 0);
        for f in &g.frames {
            let e = &g.events[f.event];
            let t = e.t;
            let i = ((t / DT).round() as usize).min(n - 1);
            for side in 0..2u8 {
                let mut cands: Vec<usize> = (0..np).filter(|&p| g.people[p].side == side && g.people[p].playing(t)).collect();
                let mut dots: Vec<V2> = vec![];
                for d in f.dots.iter().filter(|d| d.side == side) {
                    if d.actor {
                        continue; // the actor is already named on the event
                    }
                    if d.keeper {
                        if let Some(k) = cands.iter().position(|&p| g.people[p].position_at(t) == 1) {
                            keys[cands[k]].push(Key { t, at: d.at, q: 2 });
                            cands.remove(k);
                            matched += 1;
                        }
                        offered += 1;
                        continue;
                    }
                    dots.push(d.at);
                }
                if let Some(w) = e.who {
                    cands.retain(|&p| p != w);
                }
                offered += dots.len();
                let mut pairs: Vec<(f32, usize, usize)> = vec![];
                for (di, d) in dots.iter().enumerate() {
                    for (ci, &p) in cands.iter().enumerate() {
                        let dd = first.0[p][i].dist(*d);
                        if dd < 20.0 {
                            pairs.push((dd, di, ci));
                        }
                    }
                }
                pairs.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
                let (mut dused, mut cused) = (vec![false; dots.len()], vec![false; cands.len()]);
                for (_, di, ci) in pairs {
                    if dused[di] || cused[ci] {
                        continue;
                    }
                    dused[di] = true;
                    cused[ci] = true;
                    keys[cands[ci]].push(Key { t, at: dots[di], q: 2 });
                    matched += 1;
                }
            }
        }
        let (pos, quality, conf) = Self::render(g, n, &keys, &prior);
        Tracks { n, ball, ball_z, pos, quality, conf, matched: (matched, offered) }
    }

    fn render(g: &Game, n: usize, keys: &[Vec<Key>], prior: &dyn Fn(usize, usize) -> V2) -> Rendered {
        let np = g.people.len();
        let mut pos = vec![vec![V2::ZERO; n]; np];
        let mut quality = vec![vec![0u8; n]; np];
        let mut conf = vec![vec![0u8; n]; np];
        for p in 0..np {
            let mut ks = keys[p].clone();
            ks.sort_by(|a, b| a.t.partial_cmp(&b.t).unwrap());
            // residuals from the prior at each sighting
            let res: Vec<(f32, V2, u8)> = ks
                .iter()
                .map(|k| {
                    let i = ((k.t / DT).round() as usize).min(n - 1);
                    (k.t, k.at - prior(p, i), k.q)
                })
                .collect();
            let pe = &g.people[p];
            for i in 0..n {
                let t = i as f32 * DT;
                if !pe.playing(t) {
                    continue;
                }
                let j = res.partition_point(|r| r.0 <= t);
                let (mut r, mut c, mut q) = (V2::ZERO, 0.0f32, 1u8);
                let near = |r0: &(f32, V2, u8)| (t - r0.0).abs() < DT * 0.75;
                match (j.checked_sub(1).map(|k| res[k]), res.get(j).copied()) {
                    (Some(a), Some(b)) => {
                        let u = ((t - a.0) / (b.0 - a.0).max(1e-3)).clamp(0.0, 1.0);
                        let (da, db) = ((-(t - a.0) / TAU).exp(), (-(b.0 - t) / TAU).exp());
                        r = a.1 * ((1.0 - u) * da) + b.1 * (u * db);
                        c = ((1.0 - u) * da).max(u * db);
                        if near(&a) {
                            q = a.2;
                        } else if near(&b) {
                            q = b.2;
                        }
                    }
                    (Some(a), None) => {
                        let da = (-(t - a.0) / TAU).exp();
                        r = a.1 * da;
                        c = da;
                        if near(&a) {
                            q = a.2;
                        }
                    }
                    (None, Some(b)) => {
                        let db = (-(b.0 - t) / TAU).exp();
                        r = b.1 * db;
                        c = db;
                        if near(&b) {
                            q = b.2;
                        }
                    }
                    _ => {}
                }
                pos[p][i] = clamp_pitch(prior(p, i) + r);
                quality[p][i] = q;
                conf[p][i] = (c.clamp(0.0, 1.0) * 255.0) as u8;
            }
        }
        (pos, quality, conf)
    }

    fn at<T: Copy>(v: &[T], t: f32) -> (T, T, f32) {
        let f = (t / DT).max(0.0);
        let i = (f as usize).min(v.len() - 1);
        let j = (i + 1).min(v.len() - 1);
        (v[i], v[j], f - i as f32)
    }

    pub fn ball_at(&self, t: f32) -> V2 {
        let (a, b, u) = Self::at(&self.ball, t);
        a.lerp(b, u.clamp(0.0, 1.0))
    }
    pub fn ball_z_at(&self, t: f32) -> f32 {
        let (a, b, u) = Self::at(&self.ball_z, t);
        a + (b - a) * u.clamp(0.0, 1.0)
    }
    pub fn pos_at(&self, p: usize, t: f32) -> V2 {
        let (a, b, u) = Self::at(&self.pos[p], t);
        a.lerp(b, u.clamp(0.0, 1.0))
    }
    pub fn quality_at(&self, p: usize, t: f32) -> Quality {
        let i = ((t / DT).round().max(0.0) as usize).min(self.n - 1);
        Quality::from_u8(self.quality[p][i])
    }
    pub fn conf_at(&self, p: usize, t: f32) -> f32 {
        let (a, b, u) = Self::at(&self.conf[p], t);
        (a as f32 + (b as f32 - a as f32) * u.clamp(0.0, 1.0)) / 255.0
    }
    /// The way a player is going (unit), from their path; the ball's way when they stand.
    pub fn heading(&self, p: usize, t: f32, ball: V2) -> V2 {
        let d = self.pos_at(p, t + 0.4) - self.pos_at(p, t - 0.4);
        let to_ball = (ball - self.pos_at(p, t)).norm();
        if d.len() > 0.6 {
            (d.norm() * 0.5 + to_ball * 0.5).norm()
        } else {
            to_ball
        }
    }
}
