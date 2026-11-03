//! A finger (from toy-kit's model.rs; the globe's geometry left out).

use serde::{Deserialize, Serialize};

/// A finger (or a mouse button) on the screen, in points from the top left.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct PointerEvent {
    pub id: u64,
    pub phase: Phase,
    pub x: f64,
    pub y: f64,
    /// Seconds, the shell's clock.
    pub t: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Phase {
    Down,
    Move,
    Up,
    Cancel,
}
