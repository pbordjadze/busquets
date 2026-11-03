//! The headless check every toy has: a plan of fingers played on Linux (Mesa's software Vulkan) or a
//! Mac, its moments written as PNG, a film of frames, and the toy's sound as WAV. A toy's
//! `src/bin/check.rs` is one line: `fn main() { toy_kit::check::main::<my_toy::MyToy>("my-toy") }`.
//!
//! ```text
//! cargo run -p <toy> --bin <toy>-check --release -- play <plan.json> <out-dir> [--size WxH] [--ppp n]
//! ```
//!
//! A plan (JSON; times in seconds from the start, places in points from the top left):
//!
//! ```json
//! { "size": [1179, 2556], "ppp": 3, "night": false, "reduce_motion": false, "seconds": 8, "fps": 30,
//!   "events": [
//!     { "t": 1.0, "tap": [200, 400] },
//!     { "t": 2.0, "drag": [[100, 500], [300, 450]], "for": 0.6 },
//!     { "t": 3.0, "pinch": [[150, 400], [250, 400]], "to": 2.0, "for": 0.5 },
//!     { "t": 4.0, "hold": [200, 300], "for": 1.5 },
//!     { "t": 5.0, "down": [10, 10], "id": 7 }, { "t": 5.2, "move": [40, 10], "id": 7 }, { "t": 5.3, "up": [40, 10], "id": 7 },
//!     { "t": 6.0, "shake": true }
//!   ],
//!   "shots": [0.5, { "t": 3.5, "name": "pinched" }],
//!   "film": { "from": 0, "to": 8, "every": 0.1 },
//!   "audio": true }
//! ```
//!
//! `shots` are written as `<out>/<name or t>.png`; `film` as `<out>/film/0000.png…` (tools/toys/film.sh
//! makes an MP4 and a contact sheet of them); `audio` as `<out>/audio.wav` (48 kHz stereo, the whole
//! plan). Each shot prints a line: its time and what the frame cost on the CPU.

use crate::{block_on, Ctx, DirAssets, Gpu, Host, Offscreen, Phase, PointerEvent, Toy};
use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Deserialize, Default)]
#[serde(default)]
pub struct Plan {
    pub size: Option<[u32; 2]>,
    pub ppp: Option<f64>,
    pub night: bool,
    pub reduce_motion: bool,
    pub seconds: Option<f64>,
    pub fps: Option<f64>,
    pub events: Vec<Event>,
    pub shots: Vec<Shot>,
    pub film: Option<Film>,
    pub audio: bool,
    /// Text handed to the toy's `restore` before the first frame.
    pub restore: Option<String>,
}

#[derive(Deserialize, Default, Clone)]
#[serde(default)]
pub struct Event {
    pub t: f64,
    pub tap: Option<[f64; 2]>,
    pub drag: Option<[[f64; 2]; 2]>,
    pub pinch: Option<[[f64; 2]; 2]>,
    pub to: Option<f64>,
    pub hold: Option<[f64; 2]>,
    #[serde(rename = "for")]
    pub dur: Option<f64>,
    pub down: Option<[f64; 2]>,
    #[serde(rename = "move")]
    pub mv: Option<[f64; 2]>,
    pub up: Option<[f64; 2]>,
    pub id: Option<u64>,
    pub shake: bool,
}

#[derive(Deserialize, Clone)]
#[serde(untagged)]
pub enum Shot {
    At(f64),
    Named { t: f64, name: String },
}

#[derive(Deserialize, Clone)]
pub struct Film {
    #[serde(default)]
    pub from: f64,
    pub to: f64,
    pub every: f64,
}

/// A plan's fingers as pointer events, in time order.
pub fn pointers(events: &[Event]) -> Vec<(f64, Option<PointerEvent>)> {
    let mut out: Vec<(f64, Option<PointerEvent>)> = Vec::new();
    let mut next_id = 1000u64;
    let pe = |id, phase, p: [f64; 2], t| (t, Some(PointerEvent { id, phase, x: p[0], y: p[1], t }));
    for e in events {
        let t = e.t;
        if let Some(p) = e.tap {
            next_id += 1;
            out.push(pe(next_id, Phase::Down, p, t));
            out.push(pe(next_id, Phase::Up, p, t + 0.08));
        }
        if let Some(p) = e.hold {
            next_id += 1;
            out.push(pe(next_id, Phase::Down, p, t));
            out.push(pe(next_id, Phase::Up, p, t + e.dur.unwrap_or(1.0)));
        }
        if let Some([a, b]) = e.drag {
            next_id += 1;
            let d = e.dur.unwrap_or(0.5);
            let n = (d * 60.0).ceil().max(2.0) as usize;
            out.push(pe(next_id, Phase::Down, a, t));
            for i in 1..=n {
                let k = i as f64 / n as f64;
                out.push(pe(next_id, Phase::Move, [a[0] + (b[0] - a[0]) * k, a[1] + (b[1] - a[1]) * k], t + d * k));
            }
            out.push(pe(next_id, Phase::Up, b, t + d + 0.001));
        }
        if let Some([a, b]) = e.pinch {
            let (i1, i2) = (next_id + 1, next_id + 2);
            next_id += 2;
            let d = e.dur.unwrap_or(0.5);
            let s = e.to.unwrap_or(2.0);
            let c = [(a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0];
            let at = |p: [f64; 2], f: f64| [c[0] + (p[0] - c[0]) * f, c[1] + (p[1] - c[1]) * f];
            out.push(pe(i1, Phase::Down, a, t));
            out.push(pe(i2, Phase::Down, b, t));
            let n = (d * 60.0).ceil().max(2.0) as usize;
            for i in 1..=n {
                let f = 1.0 + (s - 1.0) * i as f64 / n as f64;
                let tt = t + d * i as f64 / n as f64;
                out.push(pe(i1, Phase::Move, at(a, f), tt));
                out.push(pe(i2, Phase::Move, at(b, f), tt));
            }
            out.push(pe(i1, Phase::Up, at(a, s), t + d + 0.001));
            out.push(pe(i2, Phase::Up, at(b, s), t + d + 0.001));
        }
        let id = e.id.unwrap_or(1);
        if let Some(p) = e.down {
            out.push(pe(id, Phase::Down, p, t));
        }
        if let Some(p) = e.mv {
            out.push(pe(id, Phase::Move, p, t));
        }
        if let Some(p) = e.up {
            out.push(pe(id, Phase::Up, p, t));
        }
        if e.shake {
            out.push((t, None));
        }
    }
    out.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    out
}

/// A headless GPU (Vulkan on Linux, Metal on a Mac).
pub fn gpu() -> anyhow::Result<Gpu> {
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor { backends: wgpu::Backends::VULKAN | wgpu::Backends::METAL, ..Default::default() });
    block_on(Gpu::new(&instance, None))
}

/// The toy's data directory: `TOY_ASSETS`, or `toys/build/<toy>/assets` (tools/toys/assets.py).
pub fn assets_dir(toy: &str) -> PathBuf {
    if let Ok(d) = std::env::var("TOY_ASSETS") {
        return d.into();
    }
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../build").join(toy).join("assets")
}

pub fn write_png(path: &Path, w: u32, h: u32, rgba: &[u8]) -> anyhow::Result<()> {
    if let Some(d) = path.parent() {
        std::fs::create_dir_all(d)?;
    }
    image::save_buffer(path, rgba, w, h, image::ExtendedColorType::Rgba8)?;
    Ok(())
}

pub fn write_wav(path: &Path, samples: &[f32], channels: u16, rate: u32) -> anyhow::Result<()> {
    let mut b = Vec::with_capacity(44 + samples.len() * 2);
    let data = (samples.len() * 2) as u32;
    b.extend_from_slice(b"RIFF");
    b.extend_from_slice(&(36 + data).to_le_bytes());
    b.extend_from_slice(b"WAVEfmt ");
    b.extend_from_slice(&16u32.to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes());
    b.extend_from_slice(&channels.to_le_bytes());
    b.extend_from_slice(&rate.to_le_bytes());
    b.extend_from_slice(&(rate * channels as u32 * 2).to_le_bytes());
    b.extend_from_slice(&(channels * 2).to_le_bytes());
    b.extend_from_slice(&16u16.to_le_bytes());
    b.extend_from_slice(b"data");
    b.extend_from_slice(&data.to_le_bytes());
    for s in samples {
        b.extend_from_slice(&((s.clamp(-1.0, 1.0) * 32767.0) as i16).to_le_bytes());
    }
    std::fs::write(path, b)?;
    Ok(())
}

/// Plays `plan` with toy `T` into `out`.
pub fn play<T: Toy>(name: &str, plan: &Plan, out: &Path, size: Option<[u32; 2]>, ppp: Option<f64>) -> anyhow::Result<()> {
    std::fs::create_dir_all(out)?;
    let [w, h] = size.or(plan.size).unwrap_or([1179, 2556]);
    let ppp = ppp.or(plan.ppp).unwrap_or(3.0);
    let gpu = gpu()?;
    let target = Offscreen::new(&gpu, w, h);
    let mut ctx = Ctx::new(gpu, Box::new(DirAssets(assets_dir(name))), w, h, ppp);
    ctx.night = plan.night;
    ctx.reduce_motion = plan.reduce_motion;
    let mut host = Host::<T>::new(ctx)?;
    host.toy.settings(&mut host.ctx);
    if let Some(s) = &plan.restore {
        host.toy.restore(&mut host.ctx, s);
    }
    let fps = plan.fps.unwrap_or(30.0);
    let mut shots: Vec<(f64, String)> = plan
        .shots
        .iter()
        .map(|s| match s {
            Shot::At(t) => (*t, format!("{t:06.2}")),
            Shot::Named { t, name } => (*t, name.clone()),
        })
        .collect();
    shots.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    let last_shot = shots.last().map(|s| s.0).unwrap_or(0.0);
    let film_end = plan.film.as_ref().map(|f| f.to).unwrap_or(0.0);
    let seconds = plan.seconds.unwrap_or(last_shot.max(film_end)).max(last_shot).max(film_end);
    let events = pointers(&plan.events);
    let mut ei = 0;
    let mut si = 0;
    let mut film_next = plan.film.as_ref().map(|f| f.from);
    let mut film_i = 0;
    let rate = 48_000.0;
    let mut sound: Vec<f32> = Vec::new();
    let frames = (seconds * fps).ceil() as usize;
    for f in 0..=frames {
        let t = f as f64 / fps;
        while ei < events.len() && events[ei].0 <= t {
            match events[ei].1 {
                Some(e) => host.pointer(e),
                None => host.toy.shaken(&mut host.ctx),
            }
            ei += 1;
        }
        let start = std::time::Instant::now();
        host.update(t);
        let cost = start.elapsed().as_secs_f64() * 1000.0;
        let _ = host.take_haptic();
        if plan.audio && f > 0 {
            let n = ((t * rate) as usize).saturating_sub(sound.len() / 2);
            let mut buf = vec![0.0f32; n * 2];
            Host::<T>::audio(host.synth.as_ref(), &mut buf, 2, rate);
            sound.extend_from_slice(&buf);
        }
        let shot_now = si < shots.len() && shots[si].0 <= t + 1e-9;
        let film_now = matches!(film_next, Some(n) if n <= t + 1e-9 && t <= film_end + 1e-9);
        if shot_now || film_now {
            host.draw(&target.view);
            let px = target.read(&host.ctx.gpu);
            while si < shots.len() && shots[si].0 <= t + 1e-9 {
                let p = out.join(format!("{}.png", shots[si].1));
                write_png(&p, w, h, &px)?;
                println!("shot {:>7.2}s {} (update {cost:.2} ms)", t, p.display());
                si += 1;
            }
            if film_now {
                write_png(&out.join("film").join(format!("{film_i:04}.png")), w, h, &px)?;
                film_i += 1;
                let every = plan.film.as_ref().unwrap().every;
                film_next = film_next.map(|n| n + every);
            }
        }
    }
    if plan.audio {
        let p = out.join("audio.wav");
        write_wav(&p, &sound, 2, rate as u32)?;
        let peak = sound.iter().fold(0.0f32, |m, s| m.max(s.abs()));
        println!("audio {} ({:.1} s, peak {peak:.2})", p.display(), sound.len() as f64 / 2.0 / rate);
    }
    if film_i > 0 {
        println!("film {} frames in {}", film_i, out.join("film").display());
    }
    Ok(())
}

/// The command line of a toy's `check` binary.
pub fn main<T: Toy>(name: &str) {
    let args: Vec<String> = std::env::args().collect();
    let flag = |k: &str| args.iter().position(|a| a == k).and_then(|i| args.get(i + 1)).cloned();
    let size = flag("--size").and_then(|s| {
        let (a, b) = s.split_once('x')?;
        Some([a.parse().ok()?, b.parse().ok()?])
    });
    let ppp = flag("--ppp").and_then(|s| s.parse().ok());
    let pos: Vec<&String> = {
        let mut v = Vec::new();
        let mut skip = false;
        for a in args.iter().skip(1) {
            if skip {
                skip = false;
                continue;
            }
            if a.starts_with("--") {
                skip = true;
                continue;
            }
            v.push(a);
        }
        v
    };
    let r = match pos.as_slice() {
        [cmd, plan, out] if cmd.as_str() == "play" => std::fs::read_to_string(plan)
            .map_err(anyhow::Error::from)
            .and_then(|s| Ok(serde_json::from_str::<Plan>(&s)?))
            .and_then(|p| play::<T>(name, &p, Path::new(out), size, ppp)),
        _ => Err(anyhow::anyhow!("usage: check play <plan.json> <out-dir> [--size WxH] [--ppp n]")),
    };
    if let Err(e) = r {
        eprintln!("check: {e:#}");
        std::process::exit(1);
    }
}
