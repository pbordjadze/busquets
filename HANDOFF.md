# Handoff (Nov 3, 2026)

## Nov 4: three new takes (row 223)

Iron Filings (`takes/filings`), Lighthouse (`takes/lighthouse`) and The Hole (`takes/hole`) are the
app's only three doors. Retired from the app, in the same release (owner's decision): Pivot, Pulse,
Garden, Light, Eyes and Ghost; Watch is an internal sandbox with no door. Their crates, checks and web
pages stay; `crates/real` (Ghost's loader) feeds The Hole. A door is a `Door` in `DOORS`
(`takes/busquets/src/lib.rs`) plus its dependency there and its `info-*`/`rules-*` plans, whose door
taps are computed from the ring (radius 0.37 of the short side, door i at -90° + i·360°/n). Filings is its own small
world, not `pitch::World`. Lighthouse is `pitch::World` with `view = 0` and the beam a constant
glance. The Hole fetches Ghost's match (events and 360) and reads seven hand-picked events
(`MOMENTS`); his position is the frame's actor dot.


The ask is painting-company row 207. ROUNDS.md has the rounds, README.md the engine and the touch
grammar, NOTICE.md the sources.

## Nov 4: the info marks (row 213)

Every screen has a small ⓘ bottom right (toy-kit's `info`, kit pinned at b888c41). Home's demo: a
fingertip taps the Pivot door, then swipes in from the left edge back home. Each take's `Toy::demo`
plays its own gesture(s) with one line; the hub hands over the open take's (`Scene::demo`). The shell
shows the line in system text (VoiceOver reads it) and now writes `toy-log.txt` to the app's
Documents (Files › On My iPhone › Busquets). A "?" beside it (row 214, the kit's rules mark) shows each
screen's few rules and what is real and what is drawn. Ghost's scrub bar ends short of both marks.


- **The app.** `takes/busquets` is the one app, "Busquets" (a working title), `com.pbordjadze.busquets`,
  build 21 in this repository's SideStore source
  (`https://raw.githubusercontent.com/pbordjadze/busquets/sidestore/source.json`). Its home is a ring of
  seven doors: Pivot, Pulse, Rondo Garden, Watch, Eyes, Ghost, Light. Tap a door to enter, swipe in from
  the left edge to come back. CI builds this IPA only when a push to main has `[ipa]` in its first line,
  on one macOS runner, and then regenerates the source.
- **The engine.**
  - `crates/pitch`: two teams of agents with traits. The ball rolls, so weighting a pass is a skill
    (`predict` and `weigh`). Pressers read the passer's look and commit, so feints work.
  - Time softens to your pace and holds while you have the ball.
  - Scanning, with memory of what you haven't looked at.
  - Anything from a rondo to 11v11; `World::from_snapshot` starts the world from any moment.
  - 13 tests.
- **The kit.** toy-kit 0.2.0 from terra-incognita, a git dependency pinned to `85b00e8`, without the
  globe. Behind a proxy, cargo needs `CARGO_NET_GIT_FETCH_WITH_CLI=true`.
- **Real matches.** `crates/real`, Ghost and Light, from the real-matches session. StatsBomb open data
  is fetched at run time and never committed: on the web by the browser, on the device by the Swift
  shell's URLSession (`toy_fetch`), in the checks from a local cache (`tools/fetch-statsbomb.sh`).
- **Web.** Private claude.ai Artifacts (links in the session's report) for Pivot, Pulse, Garden, Watch,
  Eyes and the four looks. Ghost and Light have no web link: the Artifact CSP blocks their data fetch,
  and GitHub Pages is Paul's decision (one setting, plus a deploy job nobody has added).

## What the critics last said

- **The takes (round 1).** Pivot first: the decision reads in a still frame, and remembered pressers
  are the clearest picture of scanning. Then Pulse (the most beautiful, but its pressers didn't read),
  then Garden. The weak points it named were all fixed in round 1 and haven't been judged since.
- **The four looks (blind, round 2).** Wash first, then Night, Paper, Lines. Its advice: take Wash's
  medium forward (paper, ink strokes, a wash that remembers), with Night's pressure trails and Lines'
  facing ticks, and keep the wash faint so the first frame stays clean.

## What I'd do next

1. Play build 21 on a phone and fix what the device shows. Nothing has run on hardware yet.
2. Paul picks a take and a look; rebuild that take in the Wash medium.
3. Tune feints and weighting by hand: the commitment rates in `World::read`, `pace_of`, `control`.
4. Move Ghost's "what if" onto `World::from_snapshot` (the real-matches session's next step).

## Open issues

- **Untested on a device:** the hub's take switching, the device fetch for Ghost and Light, sound
  routing through the hub, and memory with seven takes open.
- **A held ball gets taken** after roughly a dozen real seconds of standing still. Remove that if Paul
  wants truly endless holding: the `t > 0.6` check in `World::ball_step`.
- **Two passes-as-light pieces:** Light in the app, and a web page on `claude/passes-as-light`. Paul
  picks.
- **The old per-take apps** (pivot, pulse, …) stay installed on Paul's phone if he installed them, but
  get no more updates. Their release tags are still on GitHub.
- **Ghost and Light credit StatsBomb in small text**, as their licence requires. Their logo isn't
  drawn yet.
