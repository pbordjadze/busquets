# Notice

`crates/kit` is vendored from [pbordjadze/terra-incognita](https://github.com/pbordjadze/terra-incognita)
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
