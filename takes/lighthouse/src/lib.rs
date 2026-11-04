//! Lighthouse (a working title): the scan is the game.
//!
//! The rondo (pitch's) in the dark. You are a lighthouse: your only light is a wide beam turning
//! round you on its own. Whatever it lights you see as it is; once it has passed, what it lit stays
//! where you saw it and fades, like memory. Hold a thumb down and time slows and the beam swings to
//! your thumb; lift it and the ball goes to the brightest teammate you remember (marked while you
//! hold). The pressers move in the dark, and read your beam: sometimes there is one where you were
//! sure it was empty, and the light flashes on him. Nothing ends: a cut ball comes back round.

use kit::backdrop::{Backdrop, BackdropParams};
use kit::info::{Demo, Rules, Stroke};
use kit::paint::Painter;
use kit::sound::{pentatonic, Bells, Note};
use kit::{Ctx, Phase, PointerEvent, Synth, Toy};
use pitch::v2::angle_diff;
use pitch::{v2, Ball, Config, Event, Kind, View, World, V2};

/// The beam's half-width, radians; its reach, m; how long memory lasts, world s; how fast it turns
/// on its own, rad/s.
const BEAM: f32 = 0.6;
const REACH: f32 = 34.0;
const FADE: f32 = 5.0;
const TURN: f32 = 1.05;

const DARK: &str = r#"
fn backdrop(px: vec2<f32>, w: vec2<f32>) -> vec4<f32> {
  // p0 = (you px x, you px y, beam angle (y up), half-width); p1 = (reach px, hold, time, 0)
  let d = px - u.p0.xy;
  let r = length(d);
  let a = atan2(-d.y, d.x);
  let off = abs(atan2(sin(a - u.p0.z), cos(a - u.p0.z)));
  let cone = 1.0 - smoothstep(u.p0.w * 0.75, u.p0.w * 1.05, off);
  let reach = 1.0 - smoothstep(u.p1.x * 0.55, u.p1.x, r);
  let lit = cone * reach * (0.55 + 0.45 * exp(-r / (u.p1.x * 0.35)));
  // the grass, only where the light falls: mown stripes, a little grain
  let g = w / u.scale;
  let stripe = select(0.0, 1.0, fract((px.y - u.p0.y) / (u.scale * 5.0) + 0.5) > 0.5);
  var grass = vec3<f32>(0.10, 0.20, 0.13) * (0.9 + 0.12 * stripe) * (0.85 + 0.2 * noise2(px * 0.08));
  var c = vec3<f32>(0.006, 0.01, 0.016) + vec3<f32>(0.006, 0.008, 0.014) * noise2(px * 0.004 + u.p1.z * 0.02);
  c = mix(c, grass * 1.6, lit);
  // a faint glow at your feet: the lamp
  c = c + vec3<f32>(0.18, 0.15, 0.09) * exp(-r / (u.scale * 2.2)) ;
  // time slowed: the dark goes a little blue
  c = c + vec3<f32>(0.0, 0.006, 0.02) * u.p1.y;
  // a ball cut by someone you didn't see: a red flush from the edges
  let edge = length(px / u.size - vec2<f32>(0.5));
  c = c + vec3<f32>(0.14, 0.02, 0.01) * u.p1.w * smoothstep(0.35, 0.8, edge);
  return vec4<f32>(c, 1.0);
}
"#;

fn config() -> Config {
    let mut c = Config::rondo(
        v2(0.0, 0.0),
        &[v2(-8.0, -9.0), v2(8.0, -9.0), v2(-14.0, 1.0), v2(14.0, 1.0), v2(-7.0, 10.0), v2(7.0, 10.0)],
        &[v2(-3.0, 5.0), v2(3.5, 5.5), v2(0.5, -4.5)],
        19.0,
    );
    // you see only what the beam lights (and whatever is within a stride of you)
    c.view = 0.0;
    c.glance = BEAM;
    c.seed = 23;
    c
}

pub struct Lighthouse {
    world: World,
    painter: Painter,
    dark: Backdrop,
    bells: Bells,
    beam: f32,
    thumb: Option<(u64, [f32; 2])>,
    slow: f32,
    /// A ball just cut: the dark flushes red for a moment.
    lost: f32,
    ages: Vec<f32>,
    flashes: Vec<(V2, f32, [f32; 3])>,
    trail: Vec<V2>,
    time: f32,
    cam: V2,
}

impl Lighthouse {
    fn view(&self, ctx: &Ctx) -> View {
        let (w, h) = (ctx.width as f32, ctx.height as f32);
        View { center: self.cam, scale: w.min(h) / 40.0, size: [w, h], turn: 0.0 }
    }
    /// How bright your memory of player `i` is (1 while lit).
    fn memory(&self, i: usize) -> f32 {
        (-self.world.players[i].seen_age / FADE).exp()
    }
    /// The teammate your release would go to: the brightest you remember, the open lanes a little brighter.
    fn choice(&self) -> Option<usize> {
        let lanes = self.world.lanes();
        lanes
            .iter()
            .map(|l| (l.to, self.memory(l.to) * (0.35 + 0.65 * l.seen.open)))
            .filter(|x| x.1 > 0.04)
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            .map(|x| x.0)
    }
    fn release(&mut self, ctx: &mut Ctx) {
        if !(self.world.yours() || self.world.incoming().is_some()) {
            return;
        }
        let ok = match self.choice() {
            Some(j) => {
                let at = self.world.players[j].pos;
                self.world.tap(at)
            }
            None => self.world.flick(V2::from_angle(self.beam), 1.0),
        };
        if ok {
            ctx.haptic();
        }
    }
}

impl Toy for Lighthouse {
    fn new(ctx: &mut Ctx) -> anyhow::Result<Self> {
        let world = World::new(config());
        let bells = Bells::new();
        bells.bed(0.22, 46.25);
        let n = world.players.len();
        Ok(Lighthouse {
            world,
            painter: Painter::new(&ctx.gpu),
            dark: Backdrop::new(&ctx.gpu, "dark", DARK),
            bells,
            beam: std::f32::consts::FRAC_PI_2,
            thumb: None,
            slow: 0.0,
            lost: 0.0,
            ages: vec![9.0; n],
            flashes: Vec::new(),
            trail: Vec::new(),
            time: 0.0,
            cam: V2::ZERO,
        })
    }

    fn pointer(&mut self, ctx: &mut Ctx, e: PointerEvent) {
        let k = ctx.ppp as f32;
        let at = [e.x as f32 * k, e.y as f32 * k];
        match e.phase {
            Phase::Down => {
                if self.thumb.is_none() {
                    self.thumb = Some((e.id, at));
                }
            }
            Phase::Move => {
                if let Some((id, _)) = self.thumb {
                    if id == e.id {
                        self.thumb = Some((id, at));
                    }
                }
            }
            Phase::Up => {
                if self.thumb.is_some_and(|t| t.0 == e.id) {
                    self.thumb = None;
                    self.release(ctx);
                }
            }
            Phase::Cancel => {
                if self.thumb.is_some_and(|t| t.0 == e.id) {
                    self.thumb = None;
                }
            }
        }
    }

    fn update(&mut self, ctx: &mut Ctx, _t: f64, dt: f64) {
        let dt = dt as f32;
        self.time += dt;
        let view = self.view(ctx);
        let you = self.world.players[0].pos;
        // the beam: turning on its own, or swung to your thumb
        match self.thumb {
            Some((_, px)) => {
                let s = view.to_screen(you);
                let (dx, dy) = (px[0] - s[0], px[1] - s[1]);
                if dx.hypot(dy) > 12.0 {
                    let want = (-dy).atan2(dx);
                    self.beam += angle_diff(want, self.beam) * (1.0 - (-dt * 14.0).exp());
                }
            }
            None => self.beam += TURN * dt,
        }
        self.beam = self.beam.rem_euclid(std::f32::consts::TAU);
        self.world.glance(self.beam);
        // a thumb down slows time
        let want = if self.thumb.is_some() { 1.0 } else { 0.0 };
        self.slow += (want - self.slow) * (1.0 - (-dt * 8.0).exp());
        self.world.step(dt * (1.0 - 0.75 * self.slow));

        // newly lit: a ping (teammates high and bright, pressers low)
        for i in 1..self.world.players.len() {
            let age = self.world.players[i].seen_age;
            if age == 0.0 && self.ages[i] > 1.2 {
                let p = self.world.players[i].pos;
                let pan = ((p.x - you.x) / 16.0).clamp(-1.0, 1.0);
                if self.world.players[i].kind == Kind::Presser {
                    self.bells.strike(Note { freq: 110.0, amp: 0.07, decay: 0.5, pan, bright: 1.6 });
                } else {
                    self.bells.strike(Note { freq: pentatonic(660.0, i as i32), amp: 0.045, decay: 0.9, pan, bright: 0.25 });
                }
            }
            self.ages[i] = age;
        }
        for e in self.world.drain() {
            match e {
                Event::Pass { from, angle, length, yours, .. } => {
                    let deg = ((angle / std::f32::consts::TAU + 1.0) * 5.0).round() as i32 % 5;
                    let oct = if length > 16.0 { -1 } else { 0 };
                    let pan = (self.world.players[from].pos.x / 18.0).clamp(-1.0, 1.0);
                    self.bells.strike(Note { freq: pentatonic(220.0, deg + 5 * oct + if yours { 5 } else { 0 }), amp: if yours { 0.22 } else { 0.08 }, decay: if yours { 1.8 } else { 0.9 }, pan, bright: 0.8 });
                }
                Event::Received { by, grace, .. } => {
                    if by == 0 {
                        ctx.haptic();
                    }
                    let p = self.world.players[by].pos;
                    self.flashes.push((p, 0.0, [1.0, 0.85, 0.55]));
                    if grace > 0.7 {
                        self.bells.strike(Note { freq: pentatonic(440.0, (self.world.chain % 5) as i32), amp: 0.08 * grace, decay: 2.0, pan: 0.0, bright: 0.3 });
                    }
                }
                Event::Poked { by, at } => {
                    self.lost = 1.0;
                    // the one you didn't see: the light finds him
                    let p = &mut self.world.players[by];
                    p.seen = p.pos;
                    p.seen_age = 0.0;
                    self.flashes.push((p.pos, 0.0, [1.0, 0.45, 0.35]));
                    self.flashes.push((at, 0.0, [1.0, 0.45, 0.35]));
                    self.bells.strike(Note { freq: 61.7, amp: 0.25, decay: 1.2, pan: 0.0, bright: 2.5 });
                    self.bells.strike(Note { freq: 92.5, amp: 0.12, decay: 1.0, pan: 0.0, bright: 1.0 });
                }
                _ => {}
            }
        }
        let want = you * 0.7 + self.world.center * 0.3;
        self.cam = self.cam.lerp(want, 1.0 - (-dt * 0.8).exp());
        if matches!(self.world.ball, Ball::Rolling { .. }) {
            self.trail.push(self.world.ball_pos);
            if self.trail.len() > 20 {
                self.trail.remove(0);
            }
        } else if !self.trail.is_empty() {
            self.trail.remove(0);
        }
        self.lost = (self.lost - dt * 0.9).max(0.0);
        for f in &mut self.flashes {
            f.1 += dt;
        }
        self.flashes.retain(|f| f.1 < 1.4);
    }

    fn draw(&mut self, ctx: &mut Ctx, target: &wgpu::TextureView) {
        let view = self.view(ctx);
        let s = view.scale;
        let size = [ctx.width, ctx.height];
        let w = &self.world;
        let you = w.players[0].pos;
        let ys = view.to_screen(you);
        let pick = if w.yours() || w.incoming().is_some() { self.choice() } else { None };
        let mem: Vec<f32> = (0..w.players.len()).map(|i| self.memory(i)).collect();
        let mut enc = ctx.gpu.device.create_command_encoder(&Default::default());
        self.dark.render(&ctx.gpu, &mut enc, target, size, &BackdropParams { time: self.time, scale: s, center: [0.0, 0.0], p: [ys[0], ys[1], self.beam, BEAM, REACH * s, self.slow, self.time, self.lost] });
        let p = &mut self.painter;
        let sc = |q: V2| view.to_screen(q);
        let lit = |i: usize| w.players[i].seen_age == 0.0;

        // the beam's two edges, faint
        for side in [-1.0f32, 1.0] {
            let a = self.beam + side * BEAM;
            p.glow(ys, sc(you + V2::from_angle(a) * REACH * 0.7), 0.02 * s, 0.25 * s, [1.0, 0.92, 0.7, 0.06]);
        }
        // the release, while you hold: a thread to whom it will go
        if let Some(j) = pick {
            let q = sc(w.players[j].seen);
            let k = 0.25 + 0.6 * self.slow;
            p.glow(sc(w.ball_pos), q, 0.04 * s, 0.3 * s, [1.0, 0.9, 0.6, 0.25 * k]);
        }
        for (i, pl) in w.players.iter().enumerate() {
            if i == 0 {
                continue;
            }
            let m = mem[i];
            let now = lit(i);
            // where you saw it (the truth while it is lit)
            let at = sc(if now { pl.pos } else { pl.seen });
            if pl.kind == Kind::Presser {
                // pressers you are not lighting go back into the dark, quicker than your teammates
                let a = if now { 1.0 } else { 0.6 * (-pl.seen_age / (FADE * 0.4)).exp() };
                if a < 0.02 {
                    continue;
                }
                p.glow(at, at, 0.9 * s, 1.4 * s, [0.9, 0.3, 0.25, 0.22 * a]);
                p.disc(at, 0.75 * s, (0.1 + 0.6 * (1.0 - a)) * s, [0.06, 0.02, 0.03, 0.9 * a]);
                p.glow_ring(at, 0.8 * s, 0.07 * s, (0.1 + 0.5 * (1.0 - a)) * s, [1.0, 0.42, 0.32, 0.85 * a]);
            } else {
                if now {
                    p.glow(at, at, 0.6 * s, 2.6 * s, [1.0, 0.85, 0.55, 0.3]);
                    p.glow(at, at, 0.55 * s, 0.1 * s, [1.0, 0.97, 0.88, 1.0]);
                } else {
                    // remembered: a soft, cooling ghost of where you saw them
                    let a = 0.12 + 0.5 * m;
                    p.glow(at, at, 0.45 * s, (0.3 + 0.6 * (1.0 - m)) * s, [0.85, 0.8, 0.7, a]);
                    p.glow_ring(at, 0.75 * s, 0.04 * s, 0.15 * s, [1.0, 0.9, 0.7, 0.4 * a]);
                }
                if Some(i) == pick {
                    let b = 0.5 + 0.5 * (self.time * 5.0).sin();
                    p.glow_ring(at, (1.4 + 0.25 * b) * s, 0.16 * s, 0.3 * s, [1.0, 0.92, 0.6, 0.6 + 0.4 * self.slow]);
                    p.glow_ring(at, (2.2 + 0.4 * b) * s, 0.05 * s, 0.4 * s, [1.0, 0.92, 0.6, 0.25 + 0.3 * self.slow]);
                }
            }
        }
        // you: the lamp
        p.glow(ys, ys, 0.7 * s, 2.0 * s, [1.0, 0.9, 0.6, 0.35]);
        p.glow(ys, ys, 0.6 * s, 0.15 * s, [1.0, 0.98, 0.92, 1.0]);
        if self.slow > 0.02 {
            p.glow_ring(ys, 2.4 * s, 0.05 * s, 0.4 * s, [0.6, 0.75, 1.0, 0.35 * self.slow]);
        }
        // the ball: always a small light of its own
        for (k, q) in self.trail.iter().enumerate() {
            let f = k as f32 / self.trail.len() as f32;
            p.glow(sc(*q), sc(*q), 0.1 * s * f, 0.4 * s * f, [1.0, 1.0, 0.95, 0.3 * f]);
        }
        let b = sc(w.ball_pos);
        p.glow(b, b, 0.45 * s, 2.2 * s, [1.0, 1.0, 0.95, 0.45]);
        p.glow(b, b, 0.42 * s, 0.08 * s, [1.0, 1.0, 1.0, 1.0]);
        p.glow_ring(b, 0.75 * s, 0.05 * s, 0.1 * s, [1.0, 1.0, 1.0, 0.6]);
        for (at, t, c) in &self.flashes {
            let k = 1.0 - t / 1.4;
            p.glow_ring(sc(*at), (0.8 + t * 3.0) * s, 0.12 * s, 0.4 * s, [c[0], c[1], c[2], 0.8 * k]);
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

    /// The info mark: a thumb held beside you sweeps the beam round, then lifts.
    fn demo(&self, ctx: &Ctx) -> Option<Demo> {
        let k = ctx.ppp;
        let y = self.view(ctx).to_screen(self.world.players[0].pos);
        let c = [y[0] as f64 / k, y[1] as f64 / k];
        let from = [c[0] + 110.0, c[1] + 60.0];
        let to = [c[0] - 60.0, c[1] - 120.0];
        Some(Demo::new("Hold to slow time and aim the beam. Lift to pass.", vec![Stroke::drag(0.4, 2.4, from, to)]))
    }

    fn rules(&self) -> Option<Rules> {
        Some(Rules::new(
            &[
                "It is dark: you see only what your beam lights, and what it lit fades like memory.",
                "Hold to slow time and steer the beam; lift and the ball goes to the brightest teammate you remember.",
                "Pressers vanish into the dark once the beam has passed, and they read your beam.",
            ],
            "Drawn: all of it, Pivot's simulated rondo at night.",
        ))
    }
}

#[cfg(feature = "app")]
kit::ios_shell!(Lighthouse);
