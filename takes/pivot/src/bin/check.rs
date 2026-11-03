//! The headless check: `cargo run --release -p pivot --bin pivot-check -- play <plan.json> <out-dir>` (kit/src/check.rs).
fn main() {
    kit::check::main::<pivot::Pivot>("pivot")
}
