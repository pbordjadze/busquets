//! The passing-lane model: how open a pass from `a` to `b` is, given where the pressers are (or where
//! you last saw them). Each point along the line is reached by the ball at some time; each presser
//! could reach it at another (its distance less its reach, at its speed, after a reaction). The
//! lane's margin is the smallest lead the ball has over any presser at any point; open is that margin
//! eased into 0..1. One rule decides both what glows and what is intercepted, so what you see is
//! what happens (when you have looked).

use crate::v2::{smoothstep, V2};

#[derive(Clone, Copy, Debug)]
pub struct LaneParams {
    /// The ball's speed, m/s.
    pub ball_speed: f32,
    /// A presser's speed, m/s.
    pub presser_speed: f32,
    /// How far a presser's foot reaches, m.
    pub reach: f32,
    /// A presser's reaction, s.
    pub react: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Openness {
    /// 0 shut … 1 wide open.
    pub open: f32,
    /// The smallest lead of the ball over a presser, s (negative: a presser gets there first).
    pub margin: f32,
    /// The presser that closes it most, and where along the lane (0..1).
    pub threat: Option<(usize, f32)>,
}

/// How open the lane `a` → `b` is against `pressers` (positions; `None` for one that cannot act).
pub fn openness(a: V2, b: V2, pressers: &[Option<V2>], p: &LaneParams) -> Openness {
    let len = a.dist(b);
    let mut margin = f32::INFINITY;
    let mut threat = None;
    const N: usize = 12;
    for k in 1..=N {
        let u = k as f32 / N as f32;
        let at = a.lerp(b, u);
        let t_ball = len * u / p.ball_speed;
        // the receiver shields the last metre or so
        let reach = if u > 0.9 { p.reach * 0.7 } else { p.reach };
        for (i, q) in pressers.iter().enumerate() {
            let Some(q) = q else { continue };
            let t_p = (q.dist(at) - reach).max(0.0) / p.presser_speed + p.react;
            let m = t_p - t_ball;
            if m < margin {
                margin = m;
                threat = Some((i, u));
            }
        }
    }
    if !margin.is_finite() {
        return Openness { open: 1.0, margin: 9.0, threat: None };
    }
    Openness { open: smoothstep(-0.05, 0.45, margin), margin, threat }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v2::v2;

    const P: LaneParams = LaneParams { ball_speed: 12.0, presser_speed: 5.0, reach: 0.9, react: 0.15 };

    #[test]
    fn empty_is_open() {
        assert_eq!(openness(v2(0.0, 0.0), v2(10.0, 0.0), &[], &P).open, 1.0);
    }

    #[test]
    fn a_presser_on_the_line_shuts_it() {
        let o = openness(v2(0.0, 0.0), v2(10.0, 0.0), &[Some(v2(5.0, 0.2))], &P);
        assert!(o.open < 0.05, "{o:?}");
        assert_eq!(o.threat.map(|t| t.0), Some(0));
    }

    #[test]
    fn a_presser_far_off_leaves_it_open() {
        let o = openness(v2(0.0, 0.0), v2(10.0, 0.0), &[Some(v2(5.0, 8.0))], &P);
        assert!(o.open > 0.95, "{o:?}");
    }

    #[test]
    fn closer_means_shut_more() {
        let near = openness(v2(0.0, 0.0), v2(10.0, 0.0), &[Some(v2(5.0, 2.0))], &P).open;
        let far = openness(v2(0.0, 0.0), v2(10.0, 0.0), &[Some(v2(5.0, 3.5))], &P).open;
        assert!(near < far, "{near} {far}");
    }
}
