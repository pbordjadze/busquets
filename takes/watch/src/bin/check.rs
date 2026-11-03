//! The headless check: `cargo run --release -p watch --bin watch-check -- play <plan.json> <out-dir>`.
fn main() {
    kit::check::main::<watch::Watch>("watch")
}
