//! `cargo run --release -p real --example busquets`: what makes Busquets Busquets, from every open
//! StatsBomb match he played with 360 frames (tools/fetch-statsbomb.sh --busquets): his profile next to
//! other central midfielders', and a pass-choice model fit to his La Liga 2020/21 passes, scored on
//! his Euro 2020 and World Cup 2022 passes (a different team around him), against baselines and a
//! model fit to the other midfielders. Prints a report; `--json <path>` writes the numbers.
use real::choice::{self, Moment, Weights, FEATURES};
use real::traits::Profile;
use real::{Game, BUSQUETS};

fn main() -> anyhow::Result<()> {
    let dir = std::env::var("BUSQUETS_DATA").unwrap_or_else(|_| "data/statsbomb".into());
    let json_out = std::env::args().skip_while(|a| a != "--json").nth(1);
    let mut games: Vec<(u32, bool, Game)> = vec![]; // id, held out, game
    for (cs, held) in [("11_90", false), ("55_43", true), ("43_106", true)] {
        let ms: Vec<serde_json::Value> = serde_json::from_slice(&std::fs::read(format!("{dir}/matches/{cs}.json"))?)?;
        for m in ms {
            let names = [m["home_team"]["home_team_name"].as_str().unwrap_or(""), m["away_team"]["away_team_name"].as_str().unwrap_or("")];
            if !names.iter().any(|n| *n == "Spain" || *n == "Barcelona") {
                continue;
            }
            let id = m["match_id"].as_u64().unwrap() as u32;
            let Ok(ev) = std::fs::read(format!("{dir}/events/{id}.json")) else { continue };
            let fr = std::fs::read(format!("{dir}/three-sixty/{id}.json")).ok();
            let g = Game::read(&ev, fr.as_deref(), BUSQUETS)?;
            games.push((id, held, g));
        }
    }
    let played: Vec<&(u32, bool, Game)> = games.iter().filter(|g| g.2.find(BUSQUETS).is_some()).collect();
    println!("{} matches read, Busquets in {} ({} La Liga 2020/21, {} Spain)", games.len(), played.len(), played.iter().filter(|g| !g.1).count(), played.iter().filter(|g| g.1).count());

    // profiles: him, and central midfielders (StatsBomb positions 9, 10, 11, 13, 14, 15) in the same matches
    let cm = |g: &Game, i: usize| matches!(g.people[i].position_at(0.0).max(g.people[i].positions.last().map(|p| p.1).unwrap_or(0)), 9 | 10 | 11 | 13 | 14 | 15);
    let mut me = Profile::default();
    let mut peers = Profile::default();
    for (_, _, g) in &games {
        let b = g.find(BUSQUETS);
        me.add(g, &|i| Some(i) == b);
        peers.add(g, &|i| Some(i) != b && cm(g, i));
    }
    println!("\n{:<40} {:>10} {:>12}", "", "Busquets", "other CMs");
    let (a, b) = (me.row(), peers.row());
    for (x, y) in a.iter().zip(&b) {
        println!("{:<40} {:>10.1} {:>12.1}", x.0, x.1, y.1);
    }
    println!("({:.0} min of Busquets; {:.0} min of other central midfielders)", me.minutes, peers.minutes);

    // pass-choice moments
    let mut b_train: Vec<Moment> = vec![];
    let mut b_test: Vec<Moment> = vec![];
    let mut p_train: Vec<Moment> = vec![];
    let mut p_test: Vec<Moment> = vec![];
    for (id, held, g) in &games {
        let b = g.find(BUSQUETS);
        let ms = choice::moments_where(g, *id, |i| Some(i) == b);
        let ps = choice::moments_where(g, *id, |i| Some(i) != b && cm(g, i));
        if *held {
            b_test.extend(ms);
            p_test.extend(ps);
        } else {
            b_train.extend(ms);
            p_train.extend(ps);
        }
    }
    println!("\npass-choice moments (a pass with a 360 frame, its target among the visible teammates): Busquets {} train / {} held out; other CMs {} / {}", b_train.len(), b_test.len(), p_train.len(), p_test.len());
    let wb = choice::fit(&b_train, 400);
    let wp = choice::fit(&p_train, 400);
    println!("\n{:<40} {:>10} {:>12}", "weights", "Busquets", "other CMs");
    for k in 0..choice::NF {
        println!("{:<40} {:>10.2} {:>12.2}", FEATURES[k], wb[k], wp[k]);
    }
    let model = |w: Weights| move |m: &Moment| m.opts.iter().map(|f| choice::score(&w, f)).collect::<Vec<f32>>();
    let nearest = |m: &Moment| m.opts.iter().map(|f| -f[2] * 3.0).collect::<Vec<f32>>();
    let open = |m: &Moment| m.opts.iter().map(|f| f[0] * 3.0).collect::<Vec<f32>>();
    let rows: Vec<(&str, choice::Score)> = vec![
        ("Busquets model on Busquets (held out)", choice::evaluate(&b_test, &model(wb))),
        ("other-CM model on Busquets (held out)", choice::evaluate(&b_test, &model(wp))),
        ("nearest teammate on Busquets", choice::evaluate(&b_test, &nearest)),
        ("most open lane on Busquets", choice::evaluate(&b_test, &open)),
        ("Busquets model on other CMs (held out)", choice::evaluate(&p_test, &model(wb))),
        ("other-CM model on other CMs (held out)", choice::evaluate(&p_test, &model(wp))),
        ("Busquets model on Busquets (train)", choice::evaluate(&b_train, &model(wb))),
    ];
    println!("\n{:<42} {:>6} {:>7} {:>7} {:>9} {:>9}", "", "n", "top-1", "top-3", "loglik", "chance-1");
    for (name, s) in &rows {
        println!("{:<42} {:>6} {:>6.1}% {:>6.1}% {:>9.3} {:>8.1}%", name, s.n, 100.0 * s.top1, 100.0 * s.top3, s.ll, 100.0 * s.chance_top1);
    }
    println!("(loglik: mean log-probability of the real choice; chance's is {:.3} on Busquets' held-out moments)", rows[0].1.ll_chance);
    // a bootstrap on the held-out gap between his model and the other-CM model
    let per = |w: Weights, m: &Moment| {
        let s: Vec<f32> = m.opts.iter().map(|f| choice::score(&w, f)).collect();
        let mx = s.iter().cloned().fold(f32::MIN, f32::max);
        let z: f32 = s.iter().map(|x| (x - mx).exp()).sum();
        (s[m.chosen] - mx).exp() / z
    };
    let diffs: Vec<f32> = b_test.iter().map(|m| per(wb, m).max(1e-6).ln() - per(wp, m).max(1e-6).ln()).collect();
    let mut rng = pitch::v2::Rng::new(3);
    let mut boots: Vec<f32> = (0..1000)
        .map(|_| {
            let mut s = 0.0;
            for _ in 0..diffs.len() {
                s += diffs[(rng.f() * diffs.len() as f32) as usize % diffs.len()];
            }
            s / diffs.len() as f32
        })
        .collect();
    boots.sort_by(|a, b| a.partial_cmp(b).unwrap());
    println!("his model minus the other-CM model on his held-out passes, loglik per pass: {:.3} (95% bootstrap {:.3} to {:.3})", diffs.iter().sum::<f32>() / diffs.len() as f32, boots[25], boots[975]);
    println!("\ntempo for the agent: median receipt-to-release {:.2} s, {:.0}% released within 0.7 s (other CMs: {:.2} s, {:.0}%)", me.median_release(), 100.0 * me.quick_share(), peers.median_release(), 100.0 * peers.quick_share());
    if let Some(p) = json_out {
        let j = serde_json::json!({
            "busquets": a.iter().map(|x| (x.0, x.1)).collect::<Vec<_>>(),
            "peers": b.iter().map(|x| (x.0, x.1)).collect::<Vec<_>>(),
            "weights_busquets": wb, "weights_peers": wp, "features": FEATURES,
            "scores": rows.iter().map(|(n, s)| serde_json::json!({"name": n, "n": s.n, "top1": s.top1, "top3": s.top3, "ll": s.ll, "chance_top1": s.chance_top1, "ll_chance": s.ll_chance})).collect::<Vec<_>>(),
            "release_median": [me.median_release(), peers.median_release()], "quick_share": [me.quick_share(), peers.quick_share()],
        });
        std::fs::write(p, serde_json::to_string_pretty(&j)?)?;
    }
    Ok(())
}
