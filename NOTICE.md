# Notice

The kit is now a git dependency: toy-kit 0.2.0 from
[pbordjadze/terra-incognita](https://github.com/pbordjadze/terra-incognita) (`toys/crates/toy-kit`), pinned
to commit `b888c41` (Cargo.toml; its info mark), without its globe (`default-features = false`; `web` in crates/web).
It took in the modules this repository added (paint, backdrop, layer, sound, web, `check::main_with`), so
nothing is vendored any more. `tools/sim_launch.sh` and the shell's info caption come from its
`tools/toys/sim_launch.sh` and `toys/ios/Sources/main.swift`. What follows is the history of the copy it replaced.

`crates/kit` was vendored from [pbordjadze/terra-incognita](https://github.com/pbordjadze/terra-incognita)
`toys/crates/toy-kit` at commit `6bf0613` (Nov 3, 2026), itself copied from Adrift's engine
(`adrift/crates/adrift-core`): the `Toy` trait, `Host`, `Ctx`, `Synth`, the `ios_shell!` C ABI, the
headless check (`check.rs`) and the GPU setup (`gpu.rs`) are theirs, trimmed of the globe and the real-world
data. `ios/` (the Swift shell, `Info.plist`, `build.sh`) and `tools/film.sh` come from the same place
(`toys/ios`, `tools/toys`); `ci/sidestore_source.py` and `ci/sidestore_publish.sh` from its `ci/`; the web
shell (`crates/kit/src/web.rs`) is patterned on `adrift/crates/adrift-web`. Same author, MIT.

Vendored rather than taken as a git dependency: toy-kit carries the globe and its NASA data paths, lives
in a large repository whose every CI build would clone it, and is changed round by round for four other
toys; its own README says it was copied from Adrift "so neither breaks the other". The same reasoning
holds here. New here: `paint`, `backdrop`, `layer`, `sound`, `web`.

## Real matches: StatsBomb open data

`crates/real`, `takes/ghost` and `takes/light` read [StatsBomb's open data](https://github.com/statsbomb/open-data)
(events, lineups, 360 freeze-frames). Data provided by StatsBomb. Their
[Public Data User Agreement](https://github.com/statsbomb/open-data/blob/master/LICENSE.pdf) does not allow the data to
be distributed or reproduced to third parties (1.2.1) or exploited commercially, along with any analysis derived from
it (1.2.2), and asks that any published analysis carry the StatsBomb logo (1.4). So none of it is in this repository:
the takes fetch it at run time from StatsBomb's own repository, and the headless checks read a local cache
(`tools/fetch-statsbomb.sh`, into the git-ignored `data/`). The apps are for private use; anything beyond that
(publishing, a store) is Paul's decision and needs StatsBomb's terms read again (their logo, their consent).
