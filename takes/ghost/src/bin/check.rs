//! The headless check: `BUSQUETS_DATA=<cache> cargo run --release -p ghost --bin ghost-check -- play <plan.json> <out-dir>` (kit/src/check.rs; the cache from tools/fetch-statsbomb.sh).
fn main() {
    kit::check::main::<ghost::Ghost>("ghost")
}
