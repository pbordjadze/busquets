//! Busquets (a working title): the one app, every take inside it.
//!
//! The way in is itself a little rondo: seven glowing doors keep a ring around you, each one a take
//! (its colour and a mark of its own), and a ball goes round between them. Tap a door and you are in.
//! A swipe in from the screen's left edge brings you back; each take keeps going where you left it.
//! No words, no chrome.
//!
//! The takes are built without their own app shell (their `app` feature off) and driven here: the
//! one app's pointer, frame, sound, shake and saved state go to whichever take is open.

use kit::backdrop::{Backdrop, BackdropParams};
use kit::paint::Painter;
use kit::sound::{Bells, Note};
use kit::{Ctx, Phase, PointerEvent, SharedSynth, Synth, Toy};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// A take, behind a pointer (Toy is not object-safe because of `new`).
trait Scene {
    fn resize(&mut self, ctx: &mut Ctx);
    fn pointer(&mut self, ctx: &mut Ctx, e: PointerEvent);
    fn update(&mut self, ctx: &mut Ctx, t: f64, dt: f64);
    fn draw(&mut self, ctx: &mut Ctx, target: &wgpu::TextureView);
    fn shaken(&mut self, ctx: &mut Ctx);
    fn settings(&mut self, ctx: &mut Ctx);
    fn save(&self) -> Option<String>;
}

/// A take as a scene.
struct Take<T>(T);

impl<T: Toy> Scene for Take<T> {
    fn resize(&mut self, ctx: &mut Ctx) {
        Toy::resize(&mut self.0, ctx)
    }
    fn pointer(&mut self, ctx: &mut Ctx, e: PointerEvent) {
        Toy::pointer(&mut self.0, ctx, e)
    }
    fn update(&mut self, ctx: &mut Ctx, t: f64, dt: f64) {
        Toy::update(&mut self.0, ctx, t, dt)
    }
    fn draw(&mut self, ctx: &mut Ctx, target: &wgpu::TextureView) {
        Toy::draw(&mut self.0, ctx, target)
    }
    fn shaken(&mut self, ctx: &mut Ctx) {
        Toy::shaken(&mut self.0, ctx)
    }
    fn settings(&mut self, ctx: &mut Ctx) {
        Toy::settings(&mut self.0, ctx)
    }
    fn save(&self) -> Option<String> {
        Toy::save(&self.0)
    }
}

type Made = (Box<dyn Scene>, Option<SharedSynth>);

fn make<T: Toy>(ctx: &mut Ctx, saved: Option<&String>) -> anyhow::Result<Made> {
    let mut t = T::new(ctx)?;
    if let Some(s) = saved {
        t.restore(ctx, s);
    }
    let synth = t.synth().map(|s| Arc::new(Mutex::new(s)));
    Ok((Box::new(Take(t)), synth))
}

/// The doors: a name (for the saved state), a colour, and how to make the take.
struct Door {
    key: &'static str,
    color: [f32; 3],
    make: fn(&mut Ctx, Option<&String>) -> anyhow::Result<Made>,
}

const DOORS: [Door; 7] = [
    Door { key: "pivot", color: [0.36, 0.78, 0.48], make: make::<pivot::Pivot> },
    Door { key: "pulse", color: [1.0, 0.8, 0.45], make: make::<pulse::Pulse> },
    Door { key: "garden", color: [0.9, 0.86, 0.76], make: make::<garden::Garden> },
    Door { key: "watch", color: [0.6, 0.85, 1.0], make: make::<watch::Watch> },
    Door { key: "eyes", color: [0.55, 0.62, 1.0], make: make::<eyes::Eyes> },
    Door { key: "ghost", color: [0.92, 0.94, 1.0], make: make::<ghost::Ghost> },
    Door { key: "light", color: [1.0, 0.55, 0.3], make: make::<light::Light> },
];

const NIGHT: &str = r#"
fn backdrop(px: vec2<f32>, w: vec2<f32>) -> vec4<f32> {
  let c = px / u.size - vec2<f32>(0.5);
  let r = length(c * vec2<f32>(u.size.x / u.size.y, 1.0));
  var g = mix(vec3<f32>(0.035, 0.07, 0.05), vec3<f32>(0.01, 0.015, 0.02), smoothstep(0.0, 0.7, r));
  g = g + vec3<f32>(0.01, 0.02, 0.012) * noise2(px * 0.01 + u.time * 0.02);
  return vec4<f32>(g, 1.0);
}
"#;

/// The sound out: whatever the open take plays (the home's own bells when none is open).
struct Mix {
    slot: Arc<Mutex<Option<SharedSynth>>>,
    home: Box<dyn Synth>,
}

impl Synth for Mix {
    fn render(&mut self, out: &mut [f32], channels: usize, rate: f64) {
        let take = self.slot.try_lock().ok().and_then(|s| s.clone());
        match take {
            Some(s) => match s.try_lock() {
                Ok(mut s) => s.render(out, channels, rate),
                Err(_) => out.fill(0.0),
            },
            None => self.home.render(out, channels, rate),
        }
    }
}

pub struct Busquets {
    scenes: Vec<Option<Made>>,
    open: Option<usize>,
    saved: HashMap<String, String>,
    slot: Arc<Mutex<Option<SharedSynth>>>,
    bells: Bells,
    painter: Painter,
    night: Backdrop,
    time: f32,
    /// The home's ball: from door a to door b, how far.
    ball: (usize, usize, f32),
    /// A finger that began at the left edge (going home), and where it is.
    edge: Option<(u64, f64)>,
    fade: f32,
}

impl Busquets {
    fn door_at(&self, ctx: &Ctx, i: usize) -> [f32; 2] {
        let (w, h) = (ctx.width as f32, ctx.height as f32);
        let r = w.min(h) * 0.34;
        let a = -std::f32::consts::FRAC_PI_2 + i as f32 * std::f32::consts::TAU / DOORS.len() as f32 + (self.time * 0.03).sin() * 0.05;
        [w * 0.5 + a.cos() * r, h * 0.5 + a.sin() * r]
    }

    fn enter(&mut self, ctx: &mut Ctx, i: usize) {
        if self.scenes[i].is_none() {
            match (DOORS[i].make)(ctx, self.saved.get(DOORS[i].key)) {
                Ok(m) => self.scenes[i] = Some(m),
                Err(e) => {
                    log::error!("busquets: {}: {e:#}", DOORS[i].key);
                    return;
                }
            }
        }
        if let Some((scene, synth)) = &mut self.scenes[i] {
            scene.resize(ctx);
            scene.settings(ctx);
            if let Ok(mut s) = self.slot.lock() {
                *s = synth.clone();
            }
        }
        self.open = Some(i);
        self.fade = 0.0;
        ctx.haptic();
    }

    fn home(&mut self) {
        if let Some(i) = self.open.take() {
            if let Some((scene, _)) = &self.scenes[i] {
                if let Some(s) = scene.save() {
                    self.saved.insert(DOORS[i].key.to_string(), s);
                }
            }
        }
        if let Ok(mut s) = self.slot.lock() {
            *s = None;
        }
        self.fade = 0.0;
    }
}

impl Toy for Busquets {
    fn new(ctx: &mut Ctx) -> anyhow::Result<Self> {
        let bells = Bells::new();
        bells.bed(0.15, 49.0);
        Ok(Busquets {
            scenes: (0..DOORS.len()).map(|_| None).collect(),
            open: None,
            saved: HashMap::new(),
            slot: Arc::new(Mutex::new(None)),
            bells,
            painter: Painter::new(&ctx.gpu),
            night: Backdrop::new(&ctx.gpu, "home", NIGHT),
            time: 0.0,
            ball: (0, 2, 0.0),
            edge: None,
            fade: 1.0,
        })
    }

    fn resize(&mut self, ctx: &mut Ctx) {
        if let Some(i) = self.open {
            if let Some((s, _)) = &mut self.scenes[i] {
                s.resize(ctx);
            }
        }
    }

    fn pointer(&mut self, ctx: &mut Ctx, e: PointerEvent) {
        // a finger from the left edge, swept right, goes home (the take never sees it)
        if self.open.is_some() {
            match e.phase {
                Phase::Down if e.x < 18.0 => {
                    self.edge = Some((e.id, e.x));
                    return;
                }
                Phase::Move | Phase::Up | Phase::Cancel if self.edge.is_some_and(|d| d.0 == e.id) => {
                    let from = self.edge.unwrap().1;
                    if e.x - from > 70.0 {
                        self.edge = None;
                        self.home();
                        ctx.haptic();
                    } else if e.phase != Phase::Move {
                        self.edge = None;
                    }
                    return;
                }
                _ => {}
            }
            if let Some(i) = self.open {
                if let Some((s, _)) = &mut self.scenes[i] {
                    s.pointer(ctx, e);
                }
            }
            return;
        }
        // at home: a tap on a door opens it
        if e.phase == Phase::Up {
            let p = [e.x as f32 * ctx.ppp as f32, e.y as f32 * ctx.ppp as f32];
            let reach = ctx.width.min(ctx.height) as f32 * 0.09;
            let hit = (0..DOORS.len()).find(|&i| {
                let d = self.door_at(ctx, i);
                (d[0] - p[0]).hypot(d[1] - p[1]) < reach
            });
            if let Some(i) = hit {
                self.enter(ctx, i);
            }
        }
    }

    fn update(&mut self, ctx: &mut Ctx, t: f64, dt: f64) {
        self.time += dt as f32;
        self.fade = (self.fade + dt as f32 * 2.5).min(1.0);
        if let Some(i) = self.open {
            if let Some((s, _)) = &mut self.scenes[i] {
                s.update(ctx, t, dt);
            }
            return;
        }
        // the home's rondo: the ball goes door to door, a soft note at each
        self.ball.2 += dt as f32 * 0.9;
        if self.ball.2 >= 1.0 {
            let (_, b, _) = self.ball;
            let next = (b + 2 + (self.time as usize % 3)) % DOORS.len();
            self.ball = (b, next, 0.0);
            let f = kit::sound::pentatonic(220.0, b as i32);
            self.bells.strike(Note { freq: f, amp: 0.08, decay: 1.4, pan: 0.0, bright: 0.6 });
        }
    }

    fn draw(&mut self, ctx: &mut Ctx, target: &wgpu::TextureView) {
        if let Some(i) = self.open {
            if let Some((s, _)) = &mut self.scenes[i] {
                s.draw(ctx, target);
                if self.fade < 1.0 {
                    let mut enc = ctx.gpu.device.create_command_encoder(&Default::default());
                    let (w, h) = (ctx.width as f32, ctx.height as f32);
                    self.painter.disc([w * 0.5, h * 0.5], w.max(h), 1.0, [0.0, 0.0, 0.0, 1.0 - self.fade]);
                    self.painter.render(&ctx.gpu, &mut enc, target, [ctx.width, ctx.height], None);
                    ctx.gpu.queue.submit([enc.finish()]);
                }
                return;
            }
        }
        let size = [ctx.width, ctx.height];
        let (w, h) = (ctx.width as f32, ctx.height as f32);
        let s = w.min(h) / 100.0;
        let mut enc = ctx.gpu.device.create_command_encoder(&Default::default());
        self.night.render(&ctx.gpu, &mut enc, target, size, &BackdropParams { time: self.time, scale: 1.0, center: [0.0, 0.0], p: [0.0; 8] });
        let doors: Vec<[f32; 2]> = (0..DOORS.len()).map(|i| self.door_at(ctx, i)).collect();
        let p = &mut self.painter;
        let c = [w * 0.5, h * 0.5];
        // the ring's threads, and you in the middle
        for i in 0..doors.len() {
            let j = (i + 1) % doors.len();
            p.glow(doors[i], doors[j], 0.15 * s, 0.4 * s, [0.6, 0.75, 0.65, 0.12 * self.fade]);
            p.glow(c, doors[i], 0.1 * s, 0.3 * s, [0.6, 0.75, 0.65, 0.05 * self.fade]);
        }
        let breathe = 0.5 + 0.5 * (self.time * 1.3).sin();
        p.glow(c, c, 2.4 * s, 6.0 * s, [1.0, 0.97, 0.9, 0.25 * self.fade]);
        p.glow(c, c, 2.0 * s, 0.6 * s, [1.0, 0.98, 0.94, self.fade]);
        p.glow_ring(c, (4.5 + breathe) * s, 0.3 * s, 0.8 * s, [1.0, 0.95, 0.8, 0.2 * self.fade]);
        // the doors, each with its own mark
        for (i, d) in doors.iter().enumerate() {
            let col = DOORS[i].color;
            let k = 0.85 + 0.15 * (self.time * 0.9 + i as f32 * 1.3).sin();
            let a = |al: f32| [col[0], col[1], col[2], al * self.fade * k];
            let r = 7.5 * s;
            p.glow(*d, *d, r, 4.0 * s, a(0.18));
            p.glow_ring(*d, r, 0.35 * s, 0.6 * s, a(0.8));
            let dd = |x: f32, y: f32| [d[0] + x * s, d[1] + y * s];
            match DOORS[i].key {
                // the pivot: you among teammates
                "pivot" => {
                    for (x, y) in [(-3.5, -2.5), (3.5, -2.5), (0.0, 3.5)] {
                        p.glow(dd(x, y), dd(x, y), 0.9 * s, 0.4 * s, a(0.9));
                    }
                    p.glow(dd(0.0, 0.0), dd(0.0, 0.0), 1.3 * s, 0.4 * s, [1.0, 1.0, 1.0, self.fade]);
                }
                // a constellation
                "pulse" => {
                    let pts = [(-4.0, 2.0), (-1.0, -3.0), (2.5, -1.0), (4.0, 3.0)];
                    for w2 in pts.windows(2) {
                        p.glow(dd(w2[0].0, w2[0].1), dd(w2[1].0, w2[1].1), 0.15 * s, 0.3 * s, a(0.6));
                    }
                    for (x, y) in pts {
                        p.glow(dd(x, y), dd(x, y), 0.5 * s, 0.8 * s, a(1.0));
                    }
                }
                // an ink stroke
                "garden" => p.glow(dd(-4.0, 2.5), dd(4.0, -2.5), 0.5 * s, 0.3 * s, a(0.9)),
                // two teams
                "watch" => {
                    for k2 in 0..6 {
                        let x = -3.5 + (k2 % 3) as f32 * 3.5;
                        let y = if k2 < 3 { -2.5 } else { 2.5 };
                        let cc = if k2 < 3 { [1.0, 0.45, 0.4, self.fade] } else { a(1.0) };
                        p.glow(dd(x, y), dd(x, y), 0.6 * s, 0.3 * s, cc);
                    }
                }
                // a horizon and a figure
                "eyes" => {
                    p.glow(dd(-5.0, -0.5), dd(5.0, -0.5), 0.15 * s, 0.3 * s, a(0.7));
                    p.glow(dd(-1.0, 1.0), dd(-1.0, 3.5), 0.8 * s, 0.3 * s, a(0.9));
                    p.glow(dd(-1.0, 0.0), dd(-1.0, 0.0), 0.6 * s, 0.3 * s, a(0.9));
                }
                // a ghost and its real self
                "ghost" => {
                    p.glow_ring(dd(-1.5, 0.0), 2.2 * s, 0.25 * s, 0.4 * s, a(0.6));
                    p.glow(dd(1.5, 0.0), dd(1.5, 0.0), 1.2 * s, 0.4 * s, a(1.0));
                }
                // a pass as light, an ember at its end
                _ => {
                    p.glow(dd(-4.0, 3.0), dd(3.5, -3.0), 0.25 * s, 1.0 * s, a(0.8));
                    p.glow(dd(3.5, -3.0), dd(3.5, -3.0), 0.8 * s, 1.2 * s, [1.0, 0.8, 0.5, self.fade]);
                }
            }
        }
        // the ball going round
        let (a0, b0, u) = self.ball;
        let e = u * u * (3.0 - 2.0 * u);
        let q = [doors[a0][0] + (doors[b0][0] - doors[a0][0]) * e, doors[a0][1] + (doors[b0][1] - doors[a0][1]) * e];
        p.glow(q, q, 0.9 * s, 2.5 * s, [1.0, 1.0, 0.95, 0.35 * self.fade]);
        p.glow(q, q, 0.8 * s, 0.25 * s, [1.0, 1.0, 1.0, self.fade]);
        p.render(&ctx.gpu, &mut enc, target, size, None);
        ctx.gpu.queue.submit([enc.finish()]);
    }

    fn synth(&mut self) -> Option<Box<dyn Synth>> {
        Some(Box::new(Mix { slot: self.slot.clone(), home: self.bells.synth() }))
    }

    fn shaken(&mut self, ctx: &mut Ctx) {
        if let Some(i) = self.open {
            if let Some((s, _)) = &mut self.scenes[i] {
                s.shaken(ctx);
            }
        }
    }

    fn settings(&mut self, ctx: &mut Ctx) {
        if let Some(i) = self.open {
            if let Some((s, _)) = &mut self.scenes[i] {
                s.settings(ctx);
            }
        }
    }

    fn save(&self) -> Option<String> {
        let mut all = self.saved.clone();
        for (i, m) in self.scenes.iter().enumerate() {
            if let Some((s, _)) = m {
                if let Some(v) = s.save() {
                    all.insert(DOORS[i].key.to_string(), v);
                }
            }
        }
        serde_json::to_string(&all).ok()
    }

    fn restore(&mut self, _ctx: &mut Ctx, saved: &str) {
        if let Ok(m) = serde_json::from_str::<HashMap<String, String>>(saved) {
            self.saved = m;
        }
    }
}

kit::ios_shell!(Busquets);
