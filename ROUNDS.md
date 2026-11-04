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
