//! The headless check: `cargo run --release -p pulse --bin pulse-check -- play <plan.json> <out-dir>` (kit/src/check.rs).
fn main() {
    kit::check::main::<pulse::Pulse>("pulse")
}
