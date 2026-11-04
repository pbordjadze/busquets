# Rounds

One line a round (what changed, what the critic said, what is next).

1. Nov 3: the seam (kit vendored from toy-kit, pitch: rondo, your-pace time, scanning, lanes) and a
   rough sketch of each take (pivot, pulse, garden); web build, CI, the IPA job and the SideStore source.
   Critic (fresh Opus): Pivot 1st (the decision readable in a still frame; remembered pressers the
   clearest picture of scanning), Pulse 2nd (most beautiful, pressers unreadable), Garden 3rd (strokes
   stitched and ending in mid-air). Engine: a look with the ball was free, a held ball never in danger,
   the screen wasted, look and pass shared a gesture. Fixed the same round: a look with the ball costs
   0.3 world s, a ball held long is taken, pass flicks need 520 pt/s, Pivot and Pulse pushed in, Pulse's
   pressers hard-edged when just seen, Garden strokes run passer to receiver, Pivot's touches thock.
2. Nov 3: three asks relayed from Paul: feints, pass weighting, agents without a human. The engine
   became two teams of agents with traits, a rolling ball (predict/weigh, grace), pressers reading
   the passer's look and committing, rondo to 11v11, from_snapshot. New: Watch (dials), Eyes (over
   the shoulder, 3D), four visual directions (Wash, Night, Paper, Lines; blind critic: Wash first,
   then Night, Paper, Lines; take Wash's medium forward with Night's pressure trails and Lines'
   facing ticks). Merged the real-matches session's ghost and light (StatsBomb at run time).
3. Nov 3: "Busquets should just be an app" (relayed): takes/busquets holds every take behind a home of
   seven doors (edge swipe back); Ghost and Light fetch StatsBomb on the device through the Swift
   shell's URLSession (toy_fetch), still never bundled; CI builds and publishes only this one app.
   The kit became toy-kit 0.2.0 from terra-incognita, a pinned git dependency.
4. Nov 4 (painting-company row 213): "it needs some info buttons too—one for the main page, then one
   for each toy within it". toy-kit moved to b888c41 (its info mark): a small ⓘ bottom right; a tap
   plays a ghost fingertip doing the gesture on the live scene, with one line; a tap closes it. Home:
   a door tapped, then the edge swipe back. Each take: its own (`Toy::demo`; `pitch::demo_look_pass`
   for the top-down three). Proof: `takes/busquets/plans/info*.json`; the IPA job now opens the app
   in the iOS Simulator with Metal's validation on (`tools/sim_launch.sh`). One critic round on the
   shots: Watch's line named a drag it never showed, and Pivot's flick was too quick to see; both fixed.
5. Nov 4 (painting-company row 214): Paul, relayed: "I need the info and ? Buttons there for sure".
   toy-kit at cc0f56c (its rules mark): a "?" beside the ⓘ; a tap shows the few rules that make the
   scene and one line of what is real (StatsBomb open data, in Ghost and Light) and what is drawn
   (everything else: a simulation). The home's says what every door shares. Ghost's scrub bar now
   ends short of the two marks. Proof: `takes/busquets/plans/rules*.json`; `tools/shell_text.py`
   draws the shell's text onto the Linux shots.
6. Nov 4 (painting-company row 223): three new takes, the old seven kept for now. **Iron Filings** (you
   never touch the ball: drag yourself and six teammates snap into triangles round you, bent by the
   pressers; the ball goes round on its own, each pass its triangle's note; six clean in a row is the
   full circle), **Lighthouse** (the rondo in the dark, the scan a turning beam, memory fading; hold
   to slow time and aim, lift to pass to the brightest teammate you remember) and **The Hole** (seven
   real 360 freeze-frames of his passes with him taken out; drag to his pocket, warmer as you near
   it; lift to see him and his pass). Ten doors on the home, the new three at the top; each has its
   ⓘ and ?. Critic round 1 (fresh, newcomer): The Hole first (new, a loop in six seconds, but warmth
   invisible without sound), Lighthouse second (whose ball, who gets it?), Filings last (a rigid
   hexagon, pressers without effect, could leave the screen). Fixed: the hole's dot warms, grows and
   a ring tightens, a word at the top, a verdict on lift; the lighthouse marks who gets the ball and
   flushes red when a hidden presser cuts it; the filings' hexagon bends round pressers, cut edges
   show red, the full circle has pips and a chord, the ring stays on the plate.
7. Nov 4 (row 223, relayed): the retirement confirmed, and the owner's notes. The app now has only the
   three new doors; Pivot, Pulse, Garden, Light, Eyes and Ghost are out of it, Watch internal. Critic
   round 2 (fresh): The Hole first (but warmth too narrow to show), Lighthouse (pressers must truly
   vanish), Filings (no real decision). The notes moved each take: Filings became a false 9 (pressers
   are drawn to you, so you pull them off the edges the ball wants); Lighthouse's payoff is no longer a
   surprise presser but the lights coming on when your pass lands, every remembered ghost beside the
   real player, a chord as full as your picture was right (a cut is now quiet); The Hole's reveal shows
   the teammates he linked (open lines from his spot), the value of the unflashy part.
