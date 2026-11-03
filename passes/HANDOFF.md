# Handoff: Busquets in light (web page)

**State:** stable and finished for this session. The page is published as a private claude.ai artifact
(https://claude.ai/artifact/QhRi5tnF3N5ddtkLWxAMxd, v5). It stays on this branch; nothing is merged to main
and no CI is running. The engine's own take on this idea (`takes/light`, now inside `takes/busquets` on main)
is the one in the app. This page is the web-only sibling.

**What is here:** `page.html` (the WebGL2 template), `tools/fetch_passes.py` (fetches his passes from
StatsBomb open data at build time), `tools/build_page.py` (embeds the passes and StatsBomb's logo into a built
page outside the tracked files). No data is committed; StatsBomb's agreement forbids redistribution
(LICENSE.pdf 1.2.1) and requires their logo on anything published (1.4). Keep the built page private.

**Critic, five rounds (fresh Opus, on films):** the opening is beautiful (8/10 at once). The finished career
(28,754 strands) still reads as warm fur, not his shape (5/10 worth staring at). Exposure and tone are done;
what is left is structural. Pick one: let old seasons fade so about two stay lit; end on a single season; or
gather strands by zone and type so the pivot pocket and short sideways recycling show.

**How to look:** build the page (README), then `window.__seek(t)` and `window.__step(dt)` in the console let a
headless browser jump to and step the film (playback seconds; all in at ~175 s).
