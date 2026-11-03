//! Prints a minute of the world, untouched except a pass whenever you have held the ball a real
//! second (toward the most open lane you can see): who has the ball, the pace, pokes.
//!     cargo run --release -p pitch --example timeline [seed]
use pitch::{Config, Event, World};
fn main() {
    let seed = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(7);
    let mut w = World::new(Config { seed, ..Config::default() });
    let mut held = 0.0;
    let (mut yours, mut passes, mut pokes, mut mine) = (0, 0, 0, 0);
    for f in 0..(60 * 60) {
        w.step(1.0 / 60.0);
        if w.yours() {
            held += 1.0 / 60.0;
            if held > 1.0 {
                let best = w.lanes().into_iter().max_by(|a, b| a.seen.open.partial_cmp(&b.seen.open).unwrap()).unwrap();
                w.flick(best.b - best.a, 1.0);
                held = 0.0;
            }
        }
        for e in w.drain() {
            match e {
                Event::Received { by: 0, .. } => { yours += 1; println!("{:5.1}s  yours (pace {:.2})", f as f32 / 60.0, w.pace) }
                Event::Pass { yours: true, open, .. } => { mine += 1; println!("{:5.1}s  you pass, open {open:.2}", f as f32 / 60.0) }
                Event::Pass { .. } => passes += 1,
                Event::Poked { .. } => { pokes += 1; println!("{:5.1}s  poked", f as f32 / 60.0) }
                Event::Recovered { by } => println!("{:5.1}s  recovered by {by}", f as f32 / 60.0),
                _ => {}
            }
        }
    }
    println!("a minute: {yours} to you, {mine} yours, {passes} others' passes, {pokes} pokes");
}
