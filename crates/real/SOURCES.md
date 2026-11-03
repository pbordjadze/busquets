# Sources

- **StatsBomb open data** (https://github.com/statsbomb/open-data): events, lineups and 360 freeze-frames. Data provided
  by StatsBomb, under their [Public Data User Agreement](https://github.com/statsbomb/open-data/blob/master/LICENSE.pdf):
  no distribution or reproduction to third parties (1.2.1), no commercial use of the data or analysis derived from it
  (1.2.2), and published analysis carries the StatsBomb logo (1.4). None of it is kept here: the takes fetch it at run
  time from StatsBomb's repository; checks read a local cache from `tools/fetch-statsbomb.sh`.
- The few numbers derived from it and written into code (`play::BUSQUETS_W`, `play::CM_W`, `Traits::busquets()`) are
  analysis (a fitted model's weights, medians), for private, non-commercial use, credited here.
- Checked but not used: Metrica Sports sample data (25 fps tracking, 3 anonymised games, "acknowledge the source"),
  SkillCorner open data (10 fps broadcast tracking, MIT, A-League 2024/25). Neither has Busquets.
