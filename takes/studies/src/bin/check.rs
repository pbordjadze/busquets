//! `cargo run --release -p studies --bin studies-check -- <night|paper|lines|wash> play <plan.json> <out-dir>`
fn main() {
    let which = std::env::args().nth(1).unwrap_or_default();
    let rest: Vec<String> = std::env::args().skip(2).collect();
    match which.as_str() {
        "night" => kit::check::main_with::<studies::Night>("studies", &rest),
        "paper" => kit::check::main_with::<studies::Paper>("studies", &rest),
        "lines" => kit::check::main_with::<studies::Lines>("studies", &rest),
        "wash" => kit::check::main_with::<studies::Wash>("studies", &rest),
        _ => eprintln!("usage: studies-check <night|paper|lines|wash> play <plan.json> <out-dir>"),
    }
}
