//! `cargo run --release -p real --example playon -- [match id]`: the play-on agents from many real
//! moments, Busquets as himself and as a generic central midfielder, 45 s each: how often the ball
//! reaches him and leaves him, how fast, how many of the side's passes arrive.
use real::play::{Sim, SimBall, SimEvent, Traits, Tweak};
use real::{Game, Tracks, BUSQUETS};
fn main() -> anyhow::Result<()> {
    let id: u32 = std::env::args().nth(1).unwrap_or("3794686".into()).parse()?;
    let dir = std::env::var("BUSQUETS_DATA").unwrap_or_else(|_| "data/statsbomb".into());
    let g = Game::read(&std::fs::read(format!("{dir}/events/{id}.json"))?, std::fs::read(format!("{dir}/three-sixty/{id}.json")).ok().as_deref(), BUSQUETS)?;
    let tr = Tracks::build(&g);
    let b = g.find(BUSQUETS).unwrap();
    for (name, himself) in [("Busquets as himself", true), ("a generic 5 in his place", false)] {
        let (mut passes, mut his, mut his_ok, mut ok, mut holds, mut secs) = (0, 0, 0, 0, vec![], 0.0);
        let mut k = 0;
        let mut t0 = 120.0;
        while t0 < g.people[b].off.min(g.end) - 60.0 {
            let traits = |p: usize| if p == b && himself { Traits::busquets() } else { Traits::default() };
            let mut s = Sim::from_moment(&g, &tr, t0, Tweak { press: [1.0, 1.0], ..Default::default() }, &traits, 100 + k);
            let me = s.agents.iter().position(|a| a.person == b).unwrap();
            let mut held_since: Option<f32> = None;
            for _ in 0..(45.0 / 0.02) as usize {
                s.step(0.02);
                match s.ball {
                    SimBall::Held { by, .. } if by == me => {
                        if held_since.is_none() {
                            held_since = Some(s.t)
                        }
                    }
                    _ => {
                        if let Some(h) = held_since.take() {
                            holds.push(s.t - h);
                        }
                    }
                }
            }
            for e in &s.events {
                if let SimEvent::Pass { from, to, .. } = e {
                    if s.agents[*from].side == 0 {
                        passes += 1;
                    }
                    let _ = to;
                    if *from == me {
                        his += 1;
                    }
                }
            }
            // a pass arrived if the next event is not a cut
            for w in s.events.windows(2) {
                if let (SimEvent::Pass { from, .. }, nxt) = (&w[0], &w[1]) {
                    let arrived = !matches!(nxt, SimEvent::Cut { .. });
                    if s.agents[*from].side == 0 && arrived {
                        ok += 1;
                    }
                    if *from == me && arrived {
                        his_ok += 1;
                    }
                }
            }
            secs += 45.0;
            k += 1;
            t0 += 240.0;
        }
        holds.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let med = holds.get(holds.len() / 2).copied().unwrap_or(0.0);
        println!("{name}: {k} play-ons of 45 s; his passes per 90 {:.0} (arrived {:.0}%), median time on the ball {:.2} s; Spain's passes arrived {:.0}%", his as f32 * 5400.0 / secs, 100.0 * his_ok as f32 / his.max(1) as f32, med, 100.0 * ok as f32 / passes.max(1) as f32);
    }
    Ok(())
}
