//! real: real matches on the engine. StatsBomb's open data (events for every on-ball action, and 360
//! freeze-frames: where the visible players stood at each event) are fetched at run time from
//! StatsBomb's own repository, never kept here (their user agreement does not allow redistribution;
//! NOTICE.md). From them:
//!
//! - `game`: one match as a timeline in metres (105 x 68, centre spot at 0, the focus side attacking +x
//!   in both halves).
//! - `track`: every player's path, sampled five times a second, each sample honest about where it came
//!   from: `Observed` (the player is named on an event there), `Seen` (a 360 freeze-frame shows someone
//!   there and we matched them by position: the place is true, the name inferred), or `Inferred` (the
//!   shape and the nearest real sightings, nothing more).
//! - `traits`, `choice`: what makes a player that player (Busquets first), and a pass-choice model fit
//!   to his real decisions, scored on matches it never saw.
//! - `play`: eleven-a-side agents that play on from any real moment, for the what-ifs.
//! - `fetch`: the data's addresses and the loader (the browser's fetch on the web; a local cache dir
//!   for the headless check, filled by tools/fetch-statsbomb.sh).

pub mod choice;
pub mod dots;
pub mod fetch;
pub mod game;
pub mod play;
pub mod sb;
pub mod track;
pub mod traits;

pub use game::{Ev, EvKind, Game, Person};
pub use track::{Quality, Tracks};

/// The pitch, metres.
pub const LEN: f32 = 105.0;
pub const WID: f32 = 68.0;

/// Sergio Busquets' StatsBomb player id.
pub const BUSQUETS: u32 = 5203;
