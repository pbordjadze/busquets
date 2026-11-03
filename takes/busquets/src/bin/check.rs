//! The headless check: `cargo run --release -p busquets --bin busquets-check -- play <plan.json> <out-dir>`.
fn main() {
    kit::check::main::<busquets::Busquets>("busquets")
}
