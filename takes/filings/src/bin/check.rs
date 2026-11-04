//! The headless check: `cargo run --release -p filings --bin filings-check -- play <plan.json> <out-dir>` (kit/src/check.rs).
fn main() {
    kit::check::main::<filings::Filings>("filings")
}
