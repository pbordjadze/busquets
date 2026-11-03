//! A feint, printed: wait for the ball, look one way for a third of a second, then see who leans.
use pitch::{v2, Config, World, V2};
fn main() {
    let mut w = World::new(Config::default());
    while !w.yours() { w.step(1.0 / 60.0); }
    for _ in 0..60 { w.step(1.0 / 60.0); }
    for f in 0..30 {
        w.glance(0.0);
        w.step(1.0 / 60.0);
        if f % 5 == 0 {
            let d: Vec<String> = w.players.iter().filter(|p| p.team == 1).map(|p| format!("lean {:.2},{:.2} commit {:.2} stun {:.1}", p.lean.x, p.lean.y, p.commit, p.stun)).collect();
            println!("pace {:.2} look_cost {:.2} yours {} | {}", w.pace, w.look_cost, w.yours(), d.join(" | "));
        }
    }
    let _ = (v2(0.0, 0.0), V2::ZERO);
}
