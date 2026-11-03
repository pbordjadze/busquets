//! The headless check: `cargo run --release -p eyes --bin eyes-check -- play <plan.json> <out-dir>`.
fn main() {
    kit::check::main::<eyes::Eyes>("eyes")
}
