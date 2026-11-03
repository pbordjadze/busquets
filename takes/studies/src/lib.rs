//! Visual directions: four looks for the same engine, each the rondo playing itself (nobody human),
//! for a blind first round. Same world, same camera (`common`); only the painting differs.
//!
//! - `Night`: a night tactical cam. Players are lights; each presser casts a cone of attention that
//!   bends from the ball toward its guess (`lean`), narrowing and brightening as it commits.
//! - `Paper`: layered card under a low sun. The pitch is cut paper in strata; the players are
//!   paper-cut figures with long shadows; the pass a paper ribbon.
//! - `Lines`: the pitch drawn in living chalk-light; every pass leaves a line that shimmers, cools
//!   from gold to blue as it ages and thickens each time the same route is used again.
//! - `Wash`: pressure as weather in watercolour. Pressers bleed indigo-to-crimson blooms that spread
//!   and dry where they press (wet edges, granulation); passes are single strokes of sumi ink.
//!
//! A touch only turns the ball carrier's head (`World::glance`), so the pressers may read it.

mod common;
mod lines;
mod night;
mod paper;
mod wash;

pub use lines::Lines;
pub use night::Night;
pub use paper::Paper;
pub use wash::Wash;
