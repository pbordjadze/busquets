//! `cargo run --release -p real --example holdout -- [match id]`: how far off the reconstruction is
//! where it has to infer. Every other 360 freeze-frame is hidden; the paths are rebuilt from the rest;
//! then at each hidden frame, each real dot is compared with the nearest reconstructed player of its
//! side (lenient: it does not check it is the same person). Also: the same with no 360 at all (events only).
use real::{Game, Tracks, BUSQUETS};
fn main() -> anyhow::Result<()> {
    let id: u32 = std::env::args().nth(1).unwrap_or("3794686".into()).parse()?;
    let dir = std::env::var("BUSQUETS_DATA").unwrap_or_else(|_| "data/statsbomb".into());
    let g = Game::read(&std::fs::read(format!("{dir}/events/{id}.json"))?, std::fs::read(format!("{dir}/three-sixty/{id}.json")).ok().as_deref(), BUSQUETS)?;
    for (label, keep) in [("half the frames kept", 2usize), ("no frames (events only)", 0)] {
        let mut g2 = g.clone();
        let mut kept = vec![];
        for (k, f) in g.frames.iter().enumerate() {
            g2.events[f.event].frame = None;
            if keep > 0 && k % keep == 0 {
                g2.events[f.event].frame = Some(kept.len());
                kept.push(f.clone());
            }
        }
        g2.frames = kept;
        let tr = Tracks::build(&g2);
        let mut dots = vec![];
        for (k, f) in g.frames.iter().enumerate() {
            if keep > 0 && k % keep == 0 {
                continue;
            }
            let e = &g.events[f.event];
            for d in &f.dots {
                if d.actor {
                    continue; // named on the event itself
                }
                let best = g.people.iter().enumerate().filter(|(_, p)| p.side == d.side && p.playing(e.t)).map(|(i, _)| tr.pos_at(i, e.t).dist(d.at)).fold(f32::MAX, f32::min);
                dots.push(best);
            }
        }
        let q = |v: &mut Vec<f32>, p: f32| {
            v.sort_by(|a, b| a.partial_cmp(b).unwrap());
            v.get((v.len() as f32 * p) as usize).copied().unwrap_or(0.0)
        };
        println!("{label}: hidden dots {}: nearest reconstructed player of that side median {:.1} m, 80% within {:.1} m (lenient: nearest of the side, not the same person)", dots.len(), q(&mut dots, 0.5), q(&mut dots, 0.8));
    }
    Ok(())
}
