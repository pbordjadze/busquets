# Busquets

Sketches of one feeling: being the pivot. The ball comes to you, you look, you decide, you let it go,
at your own pace. A shared engine for a few takes on it, and the first three takes. The ask is
painting-company row 207; the names are working titles (the names are Paul's).

**The loop every take shares:** receive, scan, decide, release.

- **Receive.** The ball comes to you and time softens to your pace; with it at your feet the world all
  but holds. Never a clock.
- **Scan.** A finger that moves is your head turning: you see that side for a moment. What you have
  not looked at lately you only remember (where you last saw it). The lanes you are shown are reckoned
  from what you remember; the ball meets the real pressers.
- **Feint.** The pressers read your body: where you face and where you glance is where they guess the
  pass will go, and held a moment the guess becomes a commitment (they lean, and are slow to turn).
  Look one way, release the other. A look with the ball at your feet costs a little world time, which
  is what sells it.
- **Decide / release, weighted.** Let the finger go fast and the ball goes that way at the finger's
  pace (`pitch::pace_of`): the ball rolls and slows, so too soft is cut out, too firm runs past, and
  just right meets the runner in stride (a little bloom). While the finger moves with the ball the
  pass it would make is drawn. A flick while the ball is still coming goes first time; a tap on a
  teammate passes to them, weighted for you.
- **Without you.** Every player is an agent with tunable traits; the same world plays itself, from a
  rondo to eleven a side (`Config::eleven`), for watching and tinkering.

## The app

`takes/busquets` is the one app (working title Busquets, `com.pbordjadze.busquets`): its home is a ring of
seven glowing doors around you, a ball going round them; tap a door to enter a take, swipe in from
the left edge to come back (each take keeps going where you left it). Each take is also a crate on its
own (its `app` feature gives it its own C ABI); the one app turns that off and holds them all. CI
builds only this app's IPA and the SideStore source lists only it.

## The takes

| take | what it is | bundle id |
| --- | --- | --- |
| `takes/pivot` | The literal one: a floodlit pitch from above, the never-ending rondo, you the holding midfielder; lanes glow and close as the pressers shift; unseen pressers are rings where you last saw them. | `com.pbordjadze.busquets.pivot` |
| `takes/pulse` | The abstract one: teammates a drifting constellation, pressers dark fields, the ball a pulse of light; each pass a note (direction picks the degree, length the octave), so a good sequence is a melody. | `com.pbordjadze.busquets.pulse` |
| `takes/watch` | The engine playing itself, eleven a side or a rondo; your hand on the dials (traits, shape). | `com.pbordjadze.busquets.watch` |
| `takes/eyes` | Pivot over the shoulder, in 3D: scanning, feinting and weighting through your own eyes. | `com.pbordjadze.busquets.eyes` |
| `takes/studies` | Four visual directions (night cam, paper cut, living lines, wash), each a loop of the rondo playing itself; web only. | |
| `takes/garden` | The meditative one: a slower endless rondo on pale ground; every pass leaves its trail in ink and the pitch is slowly drawn (kept between visits; shake to wash it clean). | `com.pbordjadze.busquets.garden` |

Each is a crate: `src/lib.rs` the take (a `kit::Toy`), `src/bin/check.rs` its headless check,
`plans/` the touch plans CI plays, `toy.json` its app name and bundle id, `icon.png` (drawn by
`tools/icon.py`).

## The engine

- `crates/pitch`: the football, no GPU. `world.rs` two teams of agents (traits: scan, press, risk,
  offer, run, read, speed): the side with the ball keeps a breathing shape, offers angles and runs in
  behind; the other sends one to the ball and the rest into the most open lanes, reading the passer's
  body; a rolling ball with friction, taken by whoever reaches it first at a pace they can control
  (`predict` plays a pass forward on that rule; `weigh` finds the best pace; agents choose with it,
  takes draw it); your-pace time; what you have seen; possession swaps and chances on a full pitch;
  `World::from_snapshot` starts it from any moment (real-match data enters here). `lane.rs` the
  quick lane model the agents use to find space;
  `gesture.rs` and `view.rs` the fingers' grammar and the screen↔world mapping every take shares.
  `cargo run --release -p pitch --example timeline` prints a minute of play.
- the kit: toy-kit from terra-incognita, a pinned git dependency without its globe (NOTICE.md): the
  `Toy` trait, the iOS C ABI (`ios_shell!`), the headless check (plans to PNG, film, WAV, loudness), the
  GPU, `paint` (instanced soft discs, rings and strokes), `backdrop` (a take's full-screen shader),
  `layer` (a persistent texture), `sound` (struck notes and a bed), `web` (the WebGPU shell). Imported
  as `kit` (`package = "toy-kit"`). Behind this proxy, cargo needs `CARGO_NET_GIT_FETCH_WITH_CLI=true`.
- `crates/web`: every take in one wasm; `web/` the pages.
- `ios/`: the Swift shell every take shares, and `build.sh <take>` (an unsigned IPA).

## Build and check on Linux

    sudo apt-get install -y mesa-vulkan-drivers ffmpeg      # once: Vulkan in software, for wgpu
    cargo test --release -p pitch
    cargo run --release -p pivot --bin pivot-check -- play takes/pivot/plans/play.json build/pivot/play
    bash tools/film.sh build/pivot/play                      # an MP4 and a contact sheet of the film

A plan is fingers over time (taps, drags, holds, raw down/move/up, a shake), shots, a film and the
sound (toy-kit's `check.rs` has the format). Look at every render before pushing.

The web build: `rustup target add wasm32-unknown-unknown`, `cargo install wasm-bindgen-cli --version
0.2.130 --locked`, then `bash tools/web-build.sh` and serve `build/web` (`/pivot/`, `/pulse/`,
`/garden/`). It needs WebGPU (Safari 26, Chrome, Edge).

## CI and the device

`.github/workflows/ci.yml`: every push checks each take on Linux (plans played, renders and films as
artifacts) and builds the web. The IPAs only when asked (`[ipa]` in a push's first line, or a run by
hand): one macOS job builds all three in turn, because the five macOS runners are shared with every
app; check the queue first. On main they are released as `<take>-build-<n>` and this repository's own
SideStore source is regenerated:

    https://raw.githubusercontent.com/pbordjadze/busquets/sidestore/source.json

## Adding a take

Copy a take's directory, rename it in `Cargo.toml`, `toy.json` (a new bundle id) and the check binary,
add it to the workspace, `crates/web` (Cargo.toml and the `match`), `web/takes.json`, `tools/icon.py`,
`ci/sidestore_apps.json` and the two lists in `ci.yml`. The world's rules are `pitch::Config`; a take
reads `World` and paints it its own way.
