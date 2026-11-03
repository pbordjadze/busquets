//! From the world to the screen and back, and the fingers' grammar every take shares:
//!
//! - a finger that moves is a look: you glance where it points (and keep looking while it moves);
//!   with the ball, the pressers read that look, so a look one way and a pass the other is a feint;
//! - let it go fast and it is a pass that way, its pace the finger's speed (`pace_of`): weighting the
//!   pass is in the hand. While the finger moves with the ball, `World::aim` holds the pass it would
//!   make, for the take to draw;
//! - a tap on a teammate passes to them, weighted for you; a tap anywhere else looks there.

use crate::gesture::{Gesture, Gestures};
use crate::v2::{v2, V2};
use crate::world::World;

/// Pixels a metre, the world point at the screen's centre, the screen in pixels, and a turn
/// (radians: the world's forward is the screen's up when 0).
#[derive(Clone, Copy, Debug)]
pub struct View {
    pub center: V2,
    pub scale: f32,
    pub size: [f32; 2],
    pub turn: f32,
}

impl View {
    pub fn to_screen(&self, p: V2) -> [f32; 2] {
        let d = p - self.center;
        let (s, c) = self.turn.sin_cos();
        let r = v2(d.x * c - d.y * s, d.x * s + d.y * c);
        [self.size[0] * 0.5 + r.x * self.scale, self.size[1] * 0.5 - r.y * self.scale]
    }
    pub fn to_world(&self, px: [f32; 2]) -> V2 {
        let r = v2((px[0] - self.size[0] * 0.5) / self.scale, -(px[1] - self.size[1] * 0.5) / self.scale);
        let (s, c) = (-self.turn).sin_cos();
        self.center + v2(r.x * c - r.y * s, r.x * s + r.y * c)
    }
    /// A screen direction (y down) as a world direction.
    pub fn dir_to_world(&self, d: [f32; 2]) -> V2 {
        let r = v2(d[0], -d[1]);
        let (s, c) = (-self.turn).sin_cos();
        v2(r.x * c - r.y * s, r.x * s + r.y * c)
    }
}

/// Points a second a release must have to be a pass rather than a look.
pub const PASS_SPEED: f32 = 520.0;

/// A release's speed (points a second) as the ball's pace (m/s): continuous, from a gentle 6 m/s
/// at the threshold to a driven 26 m/s.
pub fn pace_of(finger: f32) -> f32 {
    (6.0 + (finger - PASS_SPEED).max(0.0) / 130.0).min(26.0)
}

/// What a finger did, for the take to answer (a sound, a haptic).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Touch {
    Looked,
    Passed,
    Nothing,
}

/// One pointer event (points; `phase` 0 down, 1 move, 2 up, 3 cancel) applied to the world. `ppp`
/// turns points into the view's pixels.
#[allow(clippy::too_many_arguments)]
pub fn drive(world: &mut World, g: &mut Gestures, view: &View, ppp: f32, id: u64, phase: u32, x: f32, y: f32, t: f64) -> Touch {
    let px = |p: [f32; 2]| [p[0] * ppp, p[1] * ppp];
    let gesture = match phase {
        0 => {
            g.down(id, x, y, t);
            None
        }
        1 => g.moved(id, x, y, t),
        2 => g.up(id, x, y, t),
        _ => {
            g.cancel(id);
            world.aim = None;
            None
        }
    };
    match gesture {
        Some(Gesture::Swipe { dir, .. }) => {
            let d = view.dir_to_world(dir);
            world.glance(d.angle());
            let fast = g.speed(id, t).unwrap_or(0.0);
            world.aim = (world.yours() && fast >= PASS_SPEED).then(|| (d, pace_of(fast)));
            Touch::Looked
        }
        Some(Gesture::Flick { dir, speed, .. }) => {
            world.aim = None;
            if speed < PASS_SPEED {
                return Touch::Looked;
            }
            if world.kick(view.dir_to_world(dir), pace_of(speed)) {
                Touch::Passed
            } else {
                Touch::Nothing
            }
        }
        Some(Gesture::Tap { at }) => {
            let on_ball = world.yours() || world.incoming().is_some();
            match (world.tap(view.to_world(px(at))), on_ball) {
                (true, true) => Touch::Passed,
                (true, false) => Touch::Looked,
                _ => Touch::Nothing,
            }
        }
        None => Touch::Nothing,
    }
}
