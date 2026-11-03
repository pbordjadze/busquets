# Handoff: real matches (crates/real, takes/ghost, takes/light)

The ask is painting-company row 209 (real matches as a ghost; Busquets' passes as light; a first Busquets agent).

## Where it stands (main a3ea85c)

- **crates/real.** StatsBomb open data fetched at run time: on the web through the browser's fetch, on the device
  through the Swift shell (`fetch.rs`, `device` module), and in checks from `tools/fetch-statsbomb.sh` into the
  git-ignored `data/`. It never goes in the repo (NOTICE.md, SOURCES.md).
  - `game.rs`: a match in metres, the focus side attacking +x.
  - `track.rs`: every player's path at 5 Hz, each sample tagged Observed (named on an event), Seen (a 360 dot matched
    by position) or Inferred (the shape bent toward the nearest sightings).
  - `traits.rs`, `choice.rs`: Busquets measured, and a pass-choice model.
  - `play.rs`: an 11v11 play-on for the what-ifs. It is crude: 77% of Spain's passes arrive, against about 88% in reality.
- **takes/ghost.** Croatia 3-5 Spain (Euro 2020) through Busquets' eyes or from above, with scrubbing and three
  what-ifs. Faded means inferred; in a what-if, white marks the real match.
- **takes/light.** Every Busquets pass in 45 open matches, drawn as light. A second version exists on
  `claude/passes-as-light`; Paul picks.

## Measured, honestly

Run these from the repo root:

- `cargo run --release -p real --example busquets` (needs `tools/fetch-statsbomb.sh --busquets`, ~550 MB).
  - Busquets vs other central midfielders: he releases the ball in 0.81 s vs 1.04 s, makes 83.5 passes per 90 vs 51,
    plays 62% of passes sideways and 12% back (vs 16%), and receives 10 m from the middle vs 14.5 m.
  - The choice model is trained on La Liga 20/21 and tested on his Spain matches. It gets 45% top-1 (chance 14%,
    nearest teammate 28%). A model fit to other midfielders does as well on him (bootstrap -0.04 to +0.00). So with
    these features his choices aren't distinctive; his tempo and volume are.
- `cargo run --release -p real --example holdout`: with every other freeze-frame hidden, someone of the right side
  stands within 1.4 m (median) of where the camera saw a player; with events only, 6.1 m. This is lenient: it does
  not check identity.
- `cargo run --release -p real --example playon`: the agent in the play-on, Busquets as himself vs a generic 5.

## Next

1. Move the what-if onto the engine's `pitch::World::from_snapshot` (the conductor agreed) and retire `real::play`.
2. Make the first-person view better than the critic's 3/10: real camera framing, upright see-through ghosts, the
   stadium rim, a sense of who is closing in.
3. Give the Busquets agent richer inputs: scan timing, body orientation, and the receiver's next options from 360 frames.
4. Draw the StatsBomb logo, which the licence (1.4) requires on anything published; the credit is text only today.
5. Light at full build still reads dense; consider one season, or letting only repeated routes glow.
