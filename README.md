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
- **Decide / release.** Let the finger go fast and the ball goes that way, to the teammate who best
  answers the direction. Flick while the ball is still coming and it goes first time. A tap on a
  teammate passes to them.

## The takes

| take | what it is | bundle id |
| --- | --- | --- |
| `takes/pivot` | The literal one: a floodlit pitch from above, the never-ending rondo, you the holding midfielder; lanes glow and close as the pressers shift; unseen pressers are rings where you last saw them. | `com.pbordjadze.busquets.pivot` |
| `takes/pulse` | The abstract one: teammates a drifting constellation, pressers dark fields, the ball a pulse of light; each pass a note (direction picks the degree, length the octave), so a good sequence is a melody. | `com.pbordjadze.busquets.pulse` |
| `takes/garden` | The meditative one: a slower endless rondo on pale ground; every pass leaves its trail in ink and the pitch is slowly drawn (kept between visits; shake to wash it clean). | `com.pbordjadze.busquets.garden` |

Each is a crate: `src/lib.rs` the take (a `kit::Toy`), `src/bin/check.rs` its headless check,
`plans/` the touch plans CI plays, `toy.json` its app name and bundle id, `icon.png` (drawn by
`tools/icon.py`).

## The engine

- `crates/pitch`: the football, no GPU. `world.rs` the emergent rondo (teammates keep a breathing
  shape and each tries a few spots near home for the most open lane from the ball; pressers split
  into one to the ball and the rest into the most open lanes, late and a little at random; a ball
  within a presser's reach is poked loose and the nearest of your side collects it), your-pace time,
  what you have seen; `lane.rs` the passing-lane model (the ball's lead over the quickest presser at
  every point of the line, eased into 0..1; one rule for what glows and what is intercepted);
  `gesture.rs` and `view.rs` the fingers' grammar and the screen↔world mapping every take shares.
  `cargo run --release -p pitch --example timeline` prints a minute of play.
- `crates/kit`: vendored from terra-incognita's toy-kit (NOTICE.md; why vendored, not a git dependency,
  is there too): the `Toy` trait, the iOS C ABI (`ios_shell!`), the headless check, the GPU. New here:
  `paint` (instanced soft discs, rings and strokes, over and additive), `backdrop` (a take's
  full-screen shader), `layer` (a persistent texture: the garden's drawing), `sound` (struck notes and
  a low bed), `web` (the WebGPU shell).
- `crates/web`: every take in one wasm; `web/` the pages.
- `ios/`: the Swift shell every take shares, and `build.sh <take>` (an unsigned IPA).

## Build and check on Linux

    sudo apt-get install -y mesa-vulkan-drivers ffmpeg      # once: Vulkan in software, for wgpu
    cargo test --release -p pitch
    cargo run --release -p pivot --bin pivot-check -- play takes/pivot/plans/play.json build/pivot/play
    bash tools/film.sh build/pivot/play                      # an MP4 and a contact sheet of the film

A plan is fingers over time (taps, drags, holds, raw down/move/up, a shake), shots, a film and the
sound (`crates/kit/src/check.rs` has the format). Look at every render before pushing.

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
