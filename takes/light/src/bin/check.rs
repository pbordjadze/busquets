//! The headless check: `BUSQUETS_DATA=<cache> cargo run --release -p light --bin light-check -- play <plan.json> <out-dir>` (kit/src/check.rs; the cache from tools/fetch-statsbomb.sh).
fn main() {
    kit::check::main::<light::Light>("light")
}
