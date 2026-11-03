//! Rondo Garden (a working title): the meditative take on the shared engine (pitch). An endless,
//! slower rondo on pale ground, seen whole from above; every pass leaves its trail in ink, so the
//! pitch is slowly drawn by the play: the rondo's geometry, its favourite lines, where it was poked.
//! Yours are the dark strokes; the others' a wash. The drawing is kept between visits.
//!
//! The drawing lives in a persistent layer (kit::layer) in screen pixels; its strokes are kept in
//! world metres too, so a new screen size redraws it, and a visit's end saves it.

use kit::backdrop::{Backdrop, BackdropParams};
use kit::layer::Layer;
use kit::paint::Painter;
use kit::sound::{pentatonic, Bells, Note};
use kit::{Ctx, PointerEvent, Synth, Toy};
use pitch::v2::Rng;
use pitch::{drive, v2, Config, Event, Gestures, Kind, Touch, View, World, V2};
use serde::{Deserialize, Serialize};

const PAPER: &str = r#"
fn backdrop(px: vec2<f32>, w: vec2<f32>) -> vec4<f32> {
  let hold = u.p0.x;
  let fib = noise2(px * vec2<f32>(0.9, 0.12)) * 0.5 + noise2(px * 0.35) * 0.5;
  var c = vec3<f32>(0.918, 0.893, 0.835) * (0.975 + 0.04 * fib);
  c = c - vec3<f32>(0.03, 0.035, 0.04) * smoothstep(0.4, 0.9, noise2(w * 0.08 + 2.0));
  let v = length(px / u.size - vec2<f32>(0.5)) * 1.35;
  c = c * (1.0 - 0.18 * v * v) * (1.0 - 0.06 * hold);
  return vec4<f32>(c, 1.0);
}
"#;

/// Inks, slowly turning through the visit.
const INKS: [[f32; 3]; 5] = [[0.13, 0.18, 0.32], [0.45, 0.24, 0.14], [0.22, 0.36, 0.3], [0.55, 0.42, 0.16], [0.32, 0.2, 0.36]];

#[derive(Clone, Copy, Serialize, Deserialize)]
struct Stroke {
    a: V2,
    b: V2,
    yours: bool,
    ink: u8,
    bend: f32,
    /// 0 a pass, 1 a poke's splash
    kind: u8,
}

#[derive(Serialize, Deserialize, Default)]
struct Saved {
    strokes: Vec<Stroke>,
    passes: u32,
}

pub struct Garden {
    world: World,
    gestures: Gestures,
    painter: Painter,
    paper: Backdrop,
    layer: Option<Layer>,
    strokes: Vec<Stroke>,
    pending: Vec<Stroke>,
    /// The pass in flight: its stroke, waiting for where it lands.
    flying: Option<Stroke>,
    redraw: bool,
    bells: Bells,
    rng: Rng,
    passes: u32,
    time: f32,
}

fn config() -> Config {
    Config {
        run_speed: 3.6,
        kick: [6.0, 12.0],
        friction: 1.6,
        hold: [0.9, 1.9],
        hold_pace: 0.03,
        soften: 0.9,
        drift: 1.4,
        seed: 23,
        ..Config::rondo(v2(0.0, 0.0), &[v2(-8.0, -8.0), v2(8.0, -8.0), v2(-12.0, 3.0), v2(12.0, 3.0), v2(0.0, 12.0)], &[v2(-2.5, 4.0), v2(3.0, -3.5)], 16.0)
    }
}

const MAX_STROKES: usize = 2400;

impl Garden {
    fn view(&self, ctx: &Ctx) -> View {
        let (w, h) = (ctx.width as f32, ctx.height as f32);
        View { center: V2::ZERO, scale: w.min(h) / 33.0, size: [w, h], turn: 0.0 }
    }
    fn hold(&self) -> f32 {
        (1.0 - (self.world.pace - self.world.cfg.hold_pace) / (1.0 - self.world.cfg.hold_pace)).clamp(0.0, 1.0)
    }
    fn ink_now(&self) -> u8 {
        ((self.passes / 24) % INKS.len() as u32) as u8
    }
}

/// Paints one stroke into the painter: a slightly bowed line of a few overlapping hairs, ink pooled at
/// its ends (a pass), or a scatter of drops (a poke).
fn paint_stroke(p: &mut Painter, view: &View, st: &Stroke) {
    let s = view.scale;
    let c = INKS[st.ink as usize % INKS.len()];
    if st.kind == 1 {
        let mut r = Rng::new((st.a.x.to_bits() as u64) << 32 | st.a.y.to_bits() as u64);
        for _ in 0..7 {
            let q = st.a + v2(r.s(), r.s()) * 0.9;
            p.disc(view.to_screen(q), (0.04 + 0.08 * r.f()) * s, 0.02 * s, [0.5, 0.2, 0.15, 0.35]);
        }
        return;
    }
    let (alpha, width) = if st.yours { (0.55, 0.06) } else { (0.14, 0.035) };
    let d = st.b - st.a;
    let n = d.perp().norm();
    // one line, many short overlapping dabs along a bow, the pen thinning as it travels
    let steps = ((d.len() * s / (width * s * 0.8).max(1.0)) as usize).clamp(8, 600);
    for k in 0..=steps {
        let u = k as f32 / steps as f32;
        let bow = (u * std::f32::consts::PI).sin() * st.bend;
        let q = st.a + d * u + n * bow;
        let w = width * (1.0 - 0.45 * u);
        p.disc(view.to_screen(q), w * s, 0.02 * s, [c[0], c[1], c[2], alpha * 0.18]);
    }
    let pool = if st.yours { 0.12 } else { 0.07 };
    p.disc(view.to_screen(st.a), pool * s, 0.05 * s, [c[0], c[1], c[2], alpha * 0.7]);
    p.disc(view.to_screen(st.b), pool * 0.7 * s, 0.05 * s, [c[0], c[1], c[2], alpha * 0.5]);
}

impl Toy for Garden {
    fn new(ctx: &mut Ctx) -> anyhow::Result<Self> {
        let world = World::new(config());
        let bells = Bells::new();
        bells.bed(0.18, 49.0);
        Ok(Garden {
            world,
            gestures: Gestures::new(),
            painter: Painter::new(&ctx.gpu),
            paper: Backdrop::new(&ctx.gpu, "paper", PAPER),
            layer: None,
            strokes: Vec::new(),
            pending: Vec::new(),
            flying: None,
            redraw: true,
            bells,
            rng: Rng::new(5),
            passes: 0,
            time: 0.0,
        })
    }

    fn resize(&mut self, _ctx: &mut Ctx) {
        self.layer = None;
        self.redraw = true;
    }

    fn pointer(&mut self, ctx: &mut Ctx, e: PointerEvent) {
        let view = self.view(ctx);
        if drive(&mut self.world, &mut self.gestures, &view, ctx.ppp as f32, e.id, e.phase as u32, e.x as f32, e.y as f32, e.t) == Touch::Passed {
            ctx.haptic();
        }
    }

    fn update(&mut self, ctx: &mut Ctx, _t: f64, dt: f64) {
        let dt = dt as f32;
        self.time += dt;
        self.world.step(dt);
        for e in self.world.drain() {
            match e {
                Event::Pass { from, at: a, angle, length, yours, .. } => {
                    let b = a + V2::from_angle(angle) * length;
                    self.flying = Some(Stroke { a, b, yours, ink: self.ink_now(), bend: self.rng.s() * 0.12 * length.sqrt(), kind: 0 });
                    self.passes += 1;
                    let deg = ((angle / std::f32::consts::TAU + 1.0) * 5.0).round() as i32 % 5;
                    let pan = (self.world.players[from].pos.x / 16.0).clamp(-1.0, 1.0);
                    self.bells.strike(Note { freq: pentatonic(if yours { 330.0 } else { 165.0 }, deg), amp: if yours { 0.2 } else { 0.08 }, decay: if yours { 1.6 } else { 0.7 }, pan, bright: 0.4 });
                }
                Event::Poked { at, .. } => {
                    if let Some(mut st) = self.flying.take() {
                        st.b = at;
                        self.pending.push(st);
                    }
                    self.pending.push(Stroke { a: at, b: at, yours: false, ink: 0, bend: 0.0, kind: 1 });
                    self.bells.strike(Note { freq: 98.0, amp: 0.15, decay: 0.6, pan: 0.0, bright: 1.5 });
                }
                Event::Received { by, .. } => {
                    if let Some(mut st) = self.flying.take() {
                        st.b = self.world.players[by].pos;
                        self.pending.push(st);
                        self.bells.strike(Note { freq: 130.0, amp: 0.12, decay: 0.12, pan: 0.0, bright: 3.0 });
                    }
                    if by == 0 {
                        ctx.haptic();
                    }
                }
                _ => {}
            }
        }
    }

    fn draw(&mut self, ctx: &mut Ctx, target: &wgpu::TextureView) {
        let view = self.view(ctx);
        let s = view.scale;
        let size = [ctx.width, ctx.height];
        let hold = self.hold();
        let mut enc = ctx.gpu.device.create_command_encoder(&Default::default());
        if self.layer.is_none() {
            self.layer = Some(Layer::new(&ctx.gpu, size));
            self.redraw = true;
        }
        let layer = self.layer.as_ref().unwrap();
        // the drawing: everything again after a resize, else only the new strokes
        if self.redraw {
            layer.clear(&mut enc);
            for chunk in self.strokes.chunks(400) {
                for st in chunk {
                    paint_stroke(&mut self.painter, &view, st);
                }
                self.painter.render(&ctx.gpu, &mut enc, &layer.view, size, None);
            }
            self.redraw = false;
        }
        if !self.pending.is_empty() {
            for st in &self.pending {
                paint_stroke(&mut self.painter, &view, st);
            }
            self.painter.render(&ctx.gpu, &mut enc, &layer.view, size, None);
            self.strokes.append(&mut self.pending);
            if self.strokes.len() > MAX_STROKES {
                self.strokes.drain(..self.strokes.len() - MAX_STROKES);
            }
        }
        self.paper.render(&ctx.gpu, &mut enc, target, size, &BackdropParams { time: self.time, scale: s, center: view.center.arr(), p: [hold, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0] });
        layer.composite(&mut enc, target);

        let w = &self.world;
        let p = &mut self.painter;
        let sc = |q: V2| view.to_screen(q);
        // the passes you could make: faint pencil, only while it is yours
        if w.yours() {
            for l in w.lanes() {
                let o = l.seen.open;
                if o > 0.1 {
                    p.line(sc(l.a), sc(l.b), 0.02 * s, 0.03 * s, [0.2, 0.2, 0.22, 0.25 * o * hold]);
                }
            }
        }
        for (i, pl) in w.players.iter().enumerate() {
            let c = sc(pl.pos);
            match pl.kind {
                Kind::Mate => {
                    p.disc(sc(pl.pos + v2(0.1, -0.12)), 0.48 * s, 0.15 * s, [0.3, 0.25, 0.2, 0.12]);
                    p.disc(c, 0.42 * s, 0.04 * s, [0.62, 0.68, 0.72, 0.95]);
                }
                Kind::Presser => {
                    let known = (1.0 - pl.seen_age / 4.0).clamp(0.25, 1.0);
                    p.ring(sc(pl.seen), 0.45 * s, 0.07 * s, 0.03 * s, [0.62, 0.33, 0.22, 0.85 * known]);
                }
                Kind::You => {
                    let breathe = 0.5 + 0.5 * (self.time * 1.2).sin();
                    if hold > 0.05 {
                        p.ring(c, (0.9 + 0.15 * breathe) * s, 0.02 * s, 0.03 * s, [0.15, 0.15, 0.18, 0.4 * hold]);
                    }
                    p.disc(sc(pl.pos + v2(0.1, -0.12)), 0.55 * s, 0.15 * s, [0.3, 0.25, 0.2, 0.15]);
                    p.disc(c, 0.5 * s, 0.04 * s, [0.16, 0.17, 0.2, 1.0]);
                    let tip = pl.pos + V2::from_angle(pl.facing) * 0.32;
                    p.disc(sc(tip), 0.11 * s, 0.03 * s, [0.92, 0.89, 0.83, 1.0]);
                    let _ = i;
                }
            }
        }
        p.disc(sc(w.ball_pos), 0.2 * s, 0.03 * s, [0.1, 0.1, 0.1, 1.0]);
        p.render(&ctx.gpu, &mut enc, target, size, None);
        ctx.gpu.queue.submit([enc.finish()]);
    }

    fn synth(&mut self) -> Option<Box<dyn Synth>> {
        Some(self.bells.synth())
    }

    fn save(&self) -> Option<String> {
        serde_json::to_string(&Saved { strokes: self.strokes.clone(), passes: self.passes }).ok()
    }

    fn restore(&mut self, _ctx: &mut Ctx, saved: &str) {
        if let Ok(s) = serde_json::from_str::<Saved>(saved) {
            self.strokes = s.strokes;
            self.passes = s.passes;
            self.redraw = true;
        }
    }

    /// A shake washes the ground clean.
    fn shaken(&mut self, _ctx: &mut Ctx) {
        self.strokes.clear();
        self.pending.clear();
        self.redraw = true;
    }
}

#[cfg(feature = "app")]
kit::ios_shell!(Garden);
