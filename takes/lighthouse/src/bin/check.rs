//! The headless check: `cargo run --release -p lighthouse --bin lighthouse-check -- play <plan.json> <out-dir>` (kit/src/check.rs).
fn main() {
    kit::check::main::<lighthouse::Lighthouse>("lighthouse")
}
