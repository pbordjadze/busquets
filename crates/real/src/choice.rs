//! Who a player passes to: a choice model fit to their real decisions.
//!
//! A moment is a pass with a 360 freeze-frame: where the passer stood, the teammates the camera saw
//! (the options), and the opponents. The option chosen is the visible teammate nearest where the pass
//! was aimed (within 6 m; moments where none is are left out). Each option is described by a few
//! features (how open its lane is by the engine's own lane model, ground gained, length, how marked
//! the receiver is, ...); a conditional logit (softmax over options of w . features) is fit by
//! gradient ascent. The same `score` decides for an engine agent (play.rs), so the agent on the pitch
//! is exactly the model being scored here.
use crate::game::{EvKind, Game};
use pitch::lane::{openness, LaneParams};
use pitch::V2;

pub const NF: usize = 8;
pub const FEATURES: [&str; NF] = ["open lane", "ground gained /10m", "length /10m", "length^2", "sideways /10m", "receiver free (m to nearest opp, /5)", "backward", "receiver pressed (<3m)"];

pub type Weights = [f32; NF];

#[derive(Clone, Debug)]
pub struct Moment {
    pub opts: Vec<[f32; NF]>,
    pub chosen: usize,
    /// Where things were (for drawing and for baselines).
    pub from: V2,
    pub mates: Vec<V2>,
    pub opps: Vec<V2>,
    pub match_id: u32,
    pub t: f32,
}

pub fn lane_params() -> LaneParams {
    LaneParams { ball_speed: 15.0, presser_speed: 6.0, reach: 1.2, react: 0.25 }
}

/// An option's features, from `from` to `to` against `opps`; `sg` is +1 when the passer's side attacks +x.
pub fn features(from: V2, to: V2, opps: &[V2], sg: f32) -> [f32; NF] {
    let o: Vec<Option<V2>> = opps.iter().map(|&q| Some(q)).collect();
    let open = openness(from, to, &o, &lane_params()).open;
    let d = to - from;
    let len = d.len();
    let gain = d.x * sg;
    let free = opps.iter().map(|q| q.dist(to)).fold(30.0f32, f32::min);
    [open, gain / 10.0, len / 10.0, (len / 10.0).powi(2), d.y.abs() / 10.0, free.min(15.0) / 5.0, if gain < -2.0 { 1.0 } else { 0.0 }, if free < 3.0 { 1.0 } else { 0.0 }]
}

pub fn score(w: &Weights, f: &[f32; NF]) -> f32 {
    w.iter().zip(f).map(|(a, b)| a * b).sum()
}

/// Every pass `player` made with a freeze-frame, as moments.
pub fn moments(g: &Game, player: u32, match_id: u32) -> Vec<Moment> {
    let Some(p) = g.find(player) else { return vec![] };
    moments_where(g, match_id, |who| who == p)
}

/// Every pass by anyone `pick` accepts (by people index).
pub fn moments_where(g: &Game, match_id: u32, pick: impl Fn(usize) -> bool) -> Vec<Moment> {
    let mut out = vec![];
    for e in &g.events {
        if e.kind != EvKind::Pass || e.restart {
            continue;
        }
        let (Some(who), Some(fi), Some(from), Some(end)) = (e.who, e.frame, e.at, e.end) else { continue };
        if !pick(who) {
            continue;
        }
        let f = &g.frames[fi];
        let side = e.side;
        let sg = if side == 0 { 1.0 } else { -1.0 };
        let mates: Vec<V2> = f.dots.iter().filter(|d| d.side == side && !d.actor && !d.keeper).map(|d| d.at).collect();
        let opps: Vec<V2> = f.dots.iter().filter(|d| d.side != side).map(|d| d.at).collect();
        if mates.len() < 2 {
            continue;
        }
        let (k, dist) = mates.iter().enumerate().map(|(i, m)| (i, m.dist(end))).min_by(|a, b| a.1.partial_cmp(&b.1).unwrap()).unwrap();
        if dist > 6.0 {
            continue;
        }
        let opts = mates.iter().map(|&m| features(from, m, &opps, sg)).collect();
        out.push(Moment { opts, chosen: k, from, mates, opps, match_id, t: e.t });
    }
    out
}

/// Fits weights by gradient ascent on the log-likelihood (with a little L2).
pub fn fit(ms: &[Moment], iters: usize) -> Weights {
    let mut w = [0.0f32; NF];
    let lr = 0.5;
    let l2 = 0.002;
    for _ in 0..iters {
        let mut grad = [0.0f32; NF];
        for m in ms {
            let s: Vec<f32> = m.opts.iter().map(|f| score(&w, f)).collect();
            let mx = s.iter().cloned().fold(f32::MIN, f32::max);
            let e: Vec<f32> = s.iter().map(|x| (x - mx).exp()).collect();
            let z: f32 = e.iter().sum();
            for (i, f) in m.opts.iter().enumerate() {
                let p = e[i] / z;
                let y = if i == m.chosen { 1.0 } else { 0.0 };
                for k in 0..NF {
                    grad[k] += (y - p) * f[k];
                }
            }
        }
        for k in 0..NF {
            w[k] += lr * (grad[k] / ms.len().max(1) as f32 - l2 * w[k]);
        }
    }
    w
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Score {
    pub n: usize,
    pub top1: f32,
    pub top3: f32,
    /// Mean log-likelihood of the real choice (higher is better), and chance's.
    pub ll: f32,
    pub ll_chance: f32,
    pub chance_top1: f32,
}

/// How well a ranking (higher = more likely) predicts the real choices.
pub fn evaluate(ms: &[Moment], rank: &dyn Fn(&Moment) -> Vec<f32>) -> Score {
    let mut s = Score { n: ms.len(), ..Default::default() };
    for m in ms {
        let r = rank(m);
        let mx = r.iter().cloned().fold(f32::MIN, f32::max);
        let e: Vec<f32> = r.iter().map(|x| (x - mx).exp()).collect();
        let z: f32 = e.iter().sum();
        let better = r.iter().filter(|&&x| x > r[m.chosen]).count();
        if better == 0 {
            s.top1 += 1.0;
        }
        if better < 3 {
            s.top3 += 1.0;
        }
        s.ll += (e[m.chosen] / z).max(1e-6).ln();
        s.ll_chance += (1.0 / m.opts.len() as f32).ln();
        s.chance_top1 += 1.0 / m.opts.len() as f32;
    }
    let n = ms.len().max(1) as f32;
    s.top1 /= n;
    s.top3 /= n;
    s.ll /= n;
    s.ll_chance /= n;
    s.chance_top1 /= n;
    s
}
