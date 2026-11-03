//! `cargo run --release -p real --example probe -- <match id>`: reads a match from the cache and
//! prints what the reconstruction knows (how much of each player's path is observed, seen, inferred).
use real::{Game, Tracks, BUSQUETS};
fn main() -> anyhow::Result<()> {
    let id: u32 = std::env::args().nth(1).unwrap_or("3794686".into()).parse()?;
    let dir = std::env::var("BUSQUETS_DATA").unwrap_or_else(|_| "data/statsbomb".into());
    let ev = std::fs::read(format!("{dir}/events/{id}.json"))?;
    let fr = std::fs::read(format!("{dir}/three-sixty/{id}.json")).ok();
    let t0 = std::time::Instant::now();
    let g = Game::read(&ev, fr.as_deref(), BUSQUETS)?;
    let tr = Tracks::build(&g);
    println!("{} v {}: {} events, {} frames, {:.0} min, read+built in {:?}; 360 dots matched {}/{}", g.teams[0], g.teams[1], g.events.len(), g.frames.len(), g.end / 60.0, t0.elapsed(), tr.matched.0, tr.matched.1);
    for (p, pe) in g.people.iter().enumerate() {
        let mut c = [0usize; 4];
        for &q in &tr.quality[p] {
            c[q as usize] += 1;
        }
        let on = (c[1] + c[2] + c[3]).max(1) as f32;
        let conf: f32 = tr.conf[p].iter().zip(&tr.quality[p]).filter(|(_, &q)| q > 0).map(|(&c, _)| c as f32 / 255.0).sum::<f32>() / on;
        println!("  {} {:>2} {:<28} pos {:>2} on {:>4.0}-{:<4.0}  observed {:>4.1}%  seen {:>4.1}%  inferred {:>4.1}%  mean conf {:.2}", pe.side, pe.jersey, pe.name, pe.position_at(0.0), pe.on / 60.0, pe.off.min(g.end) / 60.0, 100.0 * c[3] as f32 / on, 100.0 * c[2] as f32 / on, 100.0 * c[1] as f32 / on, conf);
    }
    if let Some(b) = g.find(BUSQUETS) {
        for k in 0..6 {
            let t = 600.0 + k as f32 * 2.0;
            println!("t {t}: ball {:?} busquets {:?} {:?}", tr.ball_at(t), tr.pos_at(b, t), tr.quality_at(b, t));
        }
    }
    Ok(())
}
