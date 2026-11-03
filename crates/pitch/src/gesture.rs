//! Fingers to football: a tap, a swipe begun (reported as soon as the finger has moved far enough, so
//! a glance is immediate), and a flick (the finger lifted after a swipe, with its direction and speed).
//! Screen points in, screen directions out; the take turns them into the world's.

use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Gesture {
    /// A short touch that barely moved, at a point.
    Tap { at: [f32; 2] },
    /// A finger has moved past the threshold (and every move after): where it started, its direction so far.
    Swipe { from: [f32; 2], dir: [f32; 2], id: u64 },
    /// The swipe ended: overall direction (unit), speed in points a second.
    Flick { from: [f32; 2], dir: [f32; 2], speed: f32, id: u64 },
}

#[derive(Default)]
pub struct Gestures {
    downs: HashMap<u64, Down>,
}

struct Down {
    at: [f32; 2],
    t: f64,
    swiped: bool,
    last: [f32; 2],
    last_t: f64,
    recent: [f32; 2],
    recent_t: f64,
}

/// Points a finger must travel to be a swipe rather than a tap.
pub const SWIPE: f32 = 14.0;

impl Gestures {
    pub fn new() -> Gestures {
        Gestures::default()
    }

    pub fn down(&mut self, id: u64, x: f32, y: f32, t: f64) {
        self.downs.insert(id, Down { at: [x, y], t, swiped: false, last: [x, y], last_t: t, recent: [x, y], recent_t: t });
    }

    pub fn moved(&mut self, id: u64, x: f32, y: f32, t: f64) -> Option<Gesture> {
        let d = self.downs.get_mut(&id)?;
        if t - d.recent_t > 0.08 {
            d.recent = d.last;
            d.recent_t = d.last_t;
        }
        d.last = [x, y];
        d.last_t = t;
        let (dx, dy) = (x - d.at[0], y - d.at[1]);
        let l = (dx * dx + dy * dy).sqrt();
        if d.swiped || l > SWIPE {
            d.swiped = true;
            return Some(Gesture::Swipe { from: d.at, dir: [dx / l.max(1e-3), dy / l.max(1e-3)], id });
        }
        None
    }

    pub fn up(&mut self, id: u64, x: f32, y: f32, t: f64) -> Option<Gesture> {
        let d = self.downs.remove(&id)?;
        let (dx, dy) = (x - d.at[0], y - d.at[1]);
        let l = (dx * dx + dy * dy).sqrt();
        if l <= SWIPE && t - d.t < 0.5 {
            return Some(Gesture::Tap { at: [x, y] });
        }
        if l <= SWIPE {
            return None;
        }
        // speed over the last moment of the finger, direction over the whole swipe
        let (rx, ry) = (x - d.recent[0], y - d.recent[1]);
        let rt = (t - d.recent_t).max(1.0 / 120.0) as f32;
        let speed = (rx * rx + ry * ry).sqrt() / rt;
        Some(Gesture::Flick { from: d.at, dir: [dx / l, dy / l], speed, id })
    }

    /// The finger's speed now (points a second, over the last moment), if it is down.
    pub fn speed(&self, id: u64, t: f64) -> Option<f32> {
        let d = self.downs.get(&id)?;
        let (rx, ry) = (d.last[0] - d.recent[0], d.last[1] - d.recent[1]);
        let rt = (d.last_t - d.recent_t).max(1.0 / 120.0) as f32;
        let _ = t;
        Some((rx * rx + ry * ry).sqrt() / rt)
    }

    pub fn cancel(&mut self, id: u64) {
        self.downs.remove(&id);
    }
}
