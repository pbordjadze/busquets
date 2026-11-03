//! pitch: the football every take shares (README.md). A small emergent world: teammates keeping a
//! shape and offering angles, pressers closing lanes, a ball passed between them, all on simple rules
//! with a little randomness. You are the pivot: when the ball comes to you, time softens to your pace
//! and holds while you decide (never a clock); a glance (a swipe while you are without the ball) shows
//! you what is behind you, and what you have not looked at you only remember; a flick or a tap passes.
//! Nothing here draws: the takes read `World` and paint it their own way.

pub mod gesture;
pub mod lane;
pub mod v2;
pub mod view;
pub mod world;

pub use gesture::{Gesture, Gestures};
pub use lane::{openness, LaneParams, Openness};
pub use v2::{v2, V2};
pub use view::{drive, pace_of, Touch, View};
pub use world::{Ball, Config, Event, Fate, Kind, Lane, Outcome, Player, Spec, Traits, World};
