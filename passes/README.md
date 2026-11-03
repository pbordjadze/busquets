# Busquets in light

Every pass Sergio Busquets played in StatsBomb's open data (La Liga 2008/09 to 2020/21, the 2018 and 2022
World Cups, Euro 2020, the Champions League finals in the set, Inter Miami 2023: 407 matches, 28,754
passes), drawn as light on a dark pitch in the order he played them. One match at first, a pass at a time,
then the seasons gather in seconds; the whole of it holds for half a minute, old passes flying again one at a time, and it
begins again.

Each pass is a ball of light on a curling arc that leaves its trace: warm going forward, cool going back,
a dim crimson when it didn't find its man. Traces accumulate in a half-float buffer; the eye adjusts
(exposure falls as the light gathers), the last few seconds glow brighter, a bloom, a tone curve, a pitch
in faint chalk the light catches.

## Data and licence

The data is StatsBomb's ([open-data](https://github.com/statsbomb/open-data)). Its user agreement
(`LICENSE.pdf` there) lets analysis made from it be published, credited with the StatsBomb logo, and forbids
redistributing the data itself. So this repository holds no data: `tools/fetch_passes.py` fetches it at build
time and `tools/build_page.py` writes the page with the passes and StatsBomb's logo embedded, outside the
tracked files. The page shows the logo, small, bottom right.

    python3 passes/tools/fetch_passes.py build/passes          # ~1.5 GB fetched, streamed; ~5 min
    python3 passes/tools/build_page.py build/passes build/passes/busquets-in-light.html

`page.html` is the template (WebGL2, no libraries). The built page carries the passes, so it is shared
privately, not committed.
