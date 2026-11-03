//! Pulse (a working title): the abstract take on the shared engine (pitch). The same rondo, as light:
//!
//! - your teammates a drifting constellation of warm stars, threaded to their nearest neighbours so
//!   the team's shape is a figure in the sky;
//! - the pressers dark fields that swallow the light around them;
//! - the ball a pulse of light, a comet in flight, a ripple where it lands;
//! - each pass a note (its direction picks the degree of a pentatonic scale, its length the octave),
//!   so a good sequence is a melody; yours ring brighter;
//! - when the pulse comes to you the sky slows and holds; the threads you could send it along glow.

use kit::backdrop::{Backdrop, BackdropParams};
use kit::paint::Painter;
use kit::sound::{pentatonic, Bells, Note};
use kit::{Ctx, PointerEvent, Synth, Toy};
use pitch::{drive, v2, Ball, Config, Event, Gestures, Kind, Touch, View, World, V2};

const SKY: &str = r#"
fn backdrop(px: vec2<f32>, w: vec2<f32>) -> vec4<f32> {
  let hold = u.p0.x;
  let t = u.p0.y;
  let q = w * 0.06 + vec2<f32>(t * 0.01, -t * 0.007);
  let n = noise2(q) * 0.6 + noise2(q * 2.7 + 3.1) * 0.3 + noise2(q * 7.0) * 0.1;
  var c = mix(vec3<f32>(0.025, 0.028, 0.065), vec3<f32>(0.12, 0.075, 0.2), n * n);
  c = c + vec3<f32>(0.05, 0.06, 0.11) * smoothstep(0.45, 0.9, noise2(w * 0.11 - 7.0));
  // far stars, still
  let g = floor(px / 3.0);
  let s = hash2(g);
  c = c + vec3<f32>(0.55, 0.6, 0.75) * step(0.9985, s) * (0.4 + 0.6 * hash2(g + 1.7));
  let v = length(px / u.size - vec2<f32>(0.5)) * 1.4;
  c = c * (1.0 - 0.5 * v * v) * (1.0 - 0.25 * hold);
  return vec4<f32>(c, 1.0);
}
"#;

/// Each teammate's hue.
const HUES: [[f32; 3]; 7] = [[1.0, 0.78, 0.45], [0.75, 0.86, 1.0], [1.0, 0.62, 0.42], [0.9, 0.95, 0.75], [1.0, 0.85, 0.65], [0.7, 0.8, 1.0], [1.0, 0.72, 0.6]];

pub struct Pulse {
    world: World,
    gestures: Gestures,
    painter: Painter,
    sky: Backdrop,
    bells: Bells,
    cam: V2,
    trail: Vec<V2>,
    ripples: Vec<(V2, f32, [f32; 3], f32)>,
    sparks: Vec<(usize, f32)>,
    time: f32,
}

fn config() -> Config {
    Config {
        seed: 11,
        ..Config::rondo(v2(0.0, 0.0), &[v2(-6.0, -10.0), v2(6.0, -10.0), v2(-13.0, 0.0), v2(13.0, 0.0), v2(-7.0, 9.0), v2(7.0, 9.0), v2(0.0, 15.0)], &[v2(-3.0, 5.0), v2(3.5, 5.5), v2(0.0, -5.0)], 19.0)
    }
}

impl Pulse {
    fn view(&self, ctx: &Ctx) -> View {
        let (w, h) = (ctx.width as f32, ctx.height as f32);
        View { center: self.cam, scale: w.min(h) / 24.0, size: [w, h], turn: 0.0 }
    }
    fn hold(&self) -> f32 {
        (1.0 - (self.world.pace - self.world.cfg.hold_pace) / (1.0 - self.world.cfg.hold_pace)).clamp(0.0, 1.0)
    }
    fn hue(&self, i: usize) -> [f32; 3] {
        hue(i)
    }
}

fn hue(i: usize) -> [f32; 3] {
    if i == 0 {
        [0.92, 0.97, 1.0]
    } else {
        HUES[(i - 1) % HUES.len()]
    }
}

impl Toy for Pulse {
    fn new(ctx: &mut Ctx) -> anyhow::Result<Self> {
        let world = World::new(config());
        let bells = Bells::new();
        bells.bed(0.35, 55.0);
        Ok(Pulse {
            cam: world.players[0].pos,
            world,
            gestures: Gestures::new(),
            painter: Painter::new(&ctx.gpu),
            sky: Backdrop::new(&ctx.gpu, "sky", SKY),
            bells,
            trail: Vec::new(),
            ripples: Vec::new(),
            sparks: Vec::new(),
            time: 0.0,
        })
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
                Event::Pass { from, angle, length, yours, open, first_time, .. } => {
                    // the direction picks the degree, the length the octave: a sequence is a melody
                    let deg = ((angle / std::f32::consts::TAU + 1.0) * 5.0).round() as i32 % 5;
                    let oct = if length > 16.0 { -1 } else if length < 8.0 { 1 } else { 0 };
                    let f = pentatonic(220.0, deg + 5 * oct + if yours { 5 } else { 0 });
                    let pan = (self.world.players[from].pos.x / 18.0).clamp(-1.0, 1.0);
                    self.bells.strike(Note { freq: f, amp: if yours { 0.3 } else { 0.13 }, decay: if yours { 2.2 } else { 1.1 }, pan, bright: if first_time { 2.2 } else { 1.0 } * (0.6 + 0.4 * open) });
                    if yours {
                        self.bells.strike(Note { freq: f * 0.5, amp: 0.12, decay: 3.0, pan, bright: 0.2 });
                    }
                }
                Event::Received { by, grace, .. } => {
                    if grace > 0.75 {
                        let f = pentatonic(440.0, (self.world.chain % 5) as i32);
                        self.bells.strike(Note { freq: f, amp: 0.1 * grace, decay: 2.5, pan: 0.0, bright: 0.3 });
                    }
                    let c = self.hue(by);
                    let at = self.world.players[by].pos;
                    self.ripples.push((at, 0.0, c, if by == 0 { 1.0 } else { 0.6 }));
                    self.sparks.push((by, 0.0));
                    if by == 0 {
                        ctx.haptic();
                    }
                }
                Event::Recovered { by } => {
                    let at = self.world.players[by].pos;
                    self.ripples.push((at, 0.0, self.hue(by), 0.5));
                }
                Event::Poked { at, .. } => {
                    self.ripples.push((at, 0.0, [0.55, 0.35, 0.9], 0.8));
                    self.bells.strike(Note { freq: 58.0, amp: 0.3, decay: 1.5, pan: 0.0, bright: 2.5 });
                    self.bells.strike(Note { freq: 61.7, amp: 0.2, decay: 1.5, pan: 0.0, bright: 2.5 });
                }
                Event::Glance { angle } => {
                    let deg = ((angle / std::f32::consts::TAU + 1.0) * 10.0).round() as i32 % 10;
                    self.bells.strike(Note { freq: pentatonic(880.0, deg), amp: 0.025, decay: 0.6, pan: angle.cos() * 0.7, bright: 0.0 });
                }
                _ => {}
            }
        }
        let want = self.world.players[0].pos * 0.6 + self.world.center * 0.4;
        self.cam = self.cam.lerp(want, 1.0 - (-dt * 0.8).exp());
        if matches!(self.world.ball, Ball::Rolling { .. }) {
            self.trail.push(self.world.ball_pos);
            if self.trail.len() > 28 {
                self.trail.remove(0);
            }
        } else if !self.trail.is_empty() {
            self.trail.remove(0);
        }
        let pace = self.world.pace;
        for r in &mut self.ripples {
            r.1 += dt * (0.6 + 0.9 * pace);
        }
        self.ripples.retain(|r| r.1 < 1.2);
        for s in &mut self.sparks {
            s.1 += dt;
        }
        self.sparks.retain(|s| s.1 < 1.0);
    }

    fn draw(&mut self, ctx: &mut Ctx, target: &wgpu::TextureView) {
        let view = self.view(ctx);
        let s = view.scale;
        let size = [ctx.width, ctx.height];
        let hold = self.hold();
        let w = &self.world;
        let wt = w.t as f32;
        let mut enc = ctx.gpu.device.create_command_encoder(&Default::default());
        self.sky.render(&ctx.gpu, &mut enc, target, size, &BackdropParams { time: self.time, scale: s, center: view.center.arr(), p: [hold, wt, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0] });
        let p = &mut self.painter;
        let sc = |q: V2| view.to_screen(q);

        // the pressers: dark fields, laid first so the light is swallowed around them
        for pl in w.players.iter().filter(|pl| pl.kind == Kind::Presser) {
            // just seen: a hard-edged hole; remembered: it blurs as the memory ages
            let fresh = (1.0 - pl.seen_age / 0.6).clamp(0.0, 1.0);
            let known = (1.0 - pl.seen_age / 5.0).clamp(0.3, 1.0);
            let breathe = 1.0 + 0.08 * (wt * 1.3 + pl.home.x).sin();
            let c = sc(pl.seen + pl.lean * (pl.commit * 1.4));
            p.disc(c, 1.8 * s * breathe, (0.15 + 3.0 * (1.0 - fresh)) * s, [0.0, 0.0, 0.01, 0.92 * known]);
            p.disc(c, 1.0 * s, 1.5 * s, [0.0, 0.0, 0.0, 0.6 * known]);
        }
        p.render(&ctx.gpu, &mut enc, target, size, None);

        // the constellation: each star threaded to its two nearest
        let stars: Vec<(usize, V2)> = w.players.iter().enumerate().filter(|(_, pl)| pl.kind != Kind::Presser).map(|(i, pl)| (i, pl.pos)).collect();
        for &(i, a) in &stars {
            let mut near: Vec<(f32, V2)> = stars.iter().filter(|(j, _)| *j != i).map(|(_, b)| (a.dist(*b), *b)).collect();
            near.sort_by(|x, y| x.0.partial_cmp(&y.0).unwrap());
            for (d, b) in near.iter().take(2) {
                let k = (1.0 - d / 22.0).clamp(0.0, 1.0);
                p.glow(sc(a), sc(*b), 0.015 * s, 0.03 * s, [0.6, 0.65, 0.9, 0.18 * k]);
            }
        }
        // the threads you could send the pulse along
        if w.yours() || w.incoming().is_some() {
            let k = if w.yours() { 1.0 } else { 0.4 };
            for l in w.lanes() {
                let o = l.seen.open;
                if o < 0.08 {
                    continue;
                }
                let c = hue(l.to);
                let sh = 0.85 + 0.15 * (self.time * 3.0 + l.to as f32).sin();
                p.glow(sc(l.a), sc(l.b), 0.04 * s * o, 0.25 * s, [c[0], c[1], c[2], 0.35 * o * k * sh]);
            }
        }
        // the glance: a faint arc of light sweeping where you look
        if let Some((a, left)) = w.glance {
            let you = w.players[0].pos;
            for i in -6..=6 {
                let aa = a + i as f32 * 0.14;
                let r = 4.0 + (0.4 - left) * 18.0;
                p.glow(sc(you + V2::from_angle(aa) * r), sc(you + V2::from_angle(aa + 0.14) * r), 0.03 * s, 0.25 * s, [0.7, 0.8, 1.0, 0.12 * (left / 0.4)]);
            }
        }
        // stars
        for (i, pl) in w.players.iter().enumerate() {
            if pl.kind == Kind::Presser {
                // a dim violet rim where you last saw it
                let fresh = (1.0 - pl.seen_age / 0.6).clamp(0.0, 1.0);
                let known = (1.0 - pl.seen_age / 5.0).clamp(0.2, 1.0);
                let breathe = 1.0 + 0.08 * (wt * 1.3 + pl.home.x).sin();
                p.glow_ring(sc(pl.seen), 1.85 * s * breathe, 0.04 * s, (0.06 + 0.4 * (1.0 - fresh)) * s, [0.55, 0.4, 0.95, 0.15 + 0.35 * fresh * known]);
                continue;
            }
            let c = hue(i);
            let tw = 0.85 + 0.15 * (wt * 2.3 + i as f32 * 1.7).sin();
            let spark = self.sparks.iter().filter(|s| s.0 == i).map(|s| 1.0 - s.1).fold(0.0, f32::max);
            let big = if i == 0 { 1.8 } else { 1.0 };
            if i == 0 {
                p.glow_ring(sc(pl.pos), 0.75 * s, 0.03 * s, 0.08 * s, [0.8, 0.9, 1.0, 0.5]);
            }
            let at = sc(pl.pos);
            p.glow(at, at, 0.15 * s * big, 1.6 * s * big * (1.0 + spark), [c[0], c[1], c[2], (0.22 + 0.3 * spark) * tw]);
            p.glow(at, at, 0.12 * s * big, 0.25 * s, [c[0], c[1], c[2], 0.9 * tw]);
            if i == 0 && hold > 0.05 {
                let b = 0.5 + 0.5 * (self.time * 1.4).sin();
                p.glow_ring(at, (1.6 + 0.3 * b) * s, 0.03 * s, 0.2 * s, [0.9, 0.95, 1.0, 0.3 * hold]);
            }
        }
        // the pulse
        for (k, q) in self.trail.iter().enumerate() {
            let f = k as f32 / self.trail.len() as f32;
            p.glow(sc(*q), sc(*q), 0.08 * s * f, 0.4 * s * f, [1.0, 0.95, 0.85, 0.25 * f]);
        }
        let b = sc(w.ball_pos);
        let beat = 0.5 + 0.5 * (self.time * if w.yours() { 2.0 } else { 6.0 }).sin();
        p.glow(b, b, 0.12 * s, 2.0 * s, [1.0, 0.92, 0.75, 0.25 + 0.15 * beat]);
        p.glow(b, b, 0.14 * s, 0.2 * s, [1.0, 1.0, 1.0, 1.0]);
        for (at, t, c, a) in &self.ripples {
            p.glow_ring(sc(*at), (0.5 + t * 2.5) * s, 0.04 * s, 0.25 * s, [c[0], c[1], c[2], a * 0.3 * (1.0 - t / 1.2)]);
        }
        p.render(&ctx.gpu, &mut enc, target, size, None);
        ctx.gpu.queue.submit([enc.finish()]);
    }

    fn synth(&mut self) -> Option<Box<dyn Synth>> {
        Some(self.bells.synth())
    }

    fn shaken(&mut self, _ctx: &mut Ctx) {
        let seed = self.world.cfg.seed + 1;
        self.world = World::new(Config { seed, ..config() });
    }
}

kit::ios_shell!(Pulse);
