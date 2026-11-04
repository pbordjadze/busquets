//! The headless check: `cargo run --release -p hole --bin hole-check -- play <plan.json> <out-dir>` (kit/src/check.rs).
fn main() {
    kit::check::main::<hole::Hole>("hole")
}
