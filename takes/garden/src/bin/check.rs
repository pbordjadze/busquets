//! The headless check: `cargo run --release -p garden --bin garden-check -- play <plan.json> <out-dir>` (kit/src/check.rs).
fn main() {
    kit::check::main::<garden::Garden>("garden")
}
