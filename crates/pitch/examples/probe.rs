use pitch::{openness, Config, Event, Kind, World};
fn main() {
    let mut w = World::new(Config::default());
    let mut held = 0.0;
    let mut hist = [0; 5];
    for _ in 0..(60 * 60) {
        w.step(1.0 / 60.0);
        if w.yours() { held += 1.0/60.0; if held > 1.0 { let b = w.lanes().into_iter().max_by(|a, b| a.seen.open.partial_cmp(&b.seen.open).unwrap()).unwrap(); w.flick(b.b - b.a, 1.0); held = 0.0; } }
        for e in w.drain() {
            if let Event::Pass { yours: false, to, .. } = e {
                let lp = w.lane_params();
                let pr: Vec<_> = w.players.iter().filter(|p| p.kind == Kind::Presser).map(|p| Some(p.pos)).collect();
                let o = openness(w.ball_pos, w.players[0].pos, &pr, &lp).open;
                hist[((o * 4.99) as usize).min(4)] += 1;
                if to == 0 { print!("Y "); } else { print!("{:.2} ", o); }
            }
        }
    }
    println!("\nopen-to-you histogram at others' passes (0-.2,.2-.4,...): {hist:?}");
}
