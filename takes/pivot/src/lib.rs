//! Pivot (a working title): the never-ending rondo, seen from above, you the holding midfielder in
//! the middle of it. The literal take on the shared engine (pitch):
//!
//! - the grass, mown in stripes, under floodlight; your teammates pale, the pressers coral;
//! - when the ball comes to you the light and the world slow to your pace and hold;
//! - the passes you could make glow from the ball, brighter the more open, and close as pressers
//!   shift; a presser you have not looked at lately is only a ring where you last saw it;
//! - a finger moving is your head turning (a glance); let it go fast and the ball goes that way.

use kit::backdrop::{Backdrop, BackdropParams};
use kit::paint::Painter;
use kit::sound::{Bells, Note};
use kit::{Ctx, PointerEvent, Synth, Toy};
use pitch::{drive, Ball, Config, Event, Gestures, Kind, Touch, View, World, V2};

const GRASS: &str = r#"
fn backdrop(px: vec2<f32>, w: vec2<f32>) -> vec4<f32> {
  let hold = u.p0.x;          // 0 running … 1 held
  let stripe = step(0.5, fract(w.y / 5.0));
  let n = noise2(w * 3.0) * 0.5 + noise2(w * 11.0) * 0.5;
  var g = mix(vec3<f32>(0.10, 0.27, 0.15), vec3<f32>(0.12, 0.31, 0.17), stripe);
  g = g * (0.92 + 0.12 * n);
  // the floodlight: brighter toward the middle of the box, a cool fall-off beyond the square
  let r = length(w - u.p0.zw) / 26.0;
  g = g * (1.05 - 0.45 * r * r);
  let outside = smoothstep(u.p1.x, u.p1.x + 2.5, max(abs(w.x), abs(w.y)));
  g = mix(g, g * vec3<f32>(0.55, 0.62, 0.7), outside);
  // time held: the colour drains a little and the light closes in
  let lum = dot(g, vec3<f32>(0.3, 0.55, 0.15));
  g = mix(g, vec3<f32>(lum) * vec3<f32>(0.85, 0.95, 1.0), hold * 0.45);
  let v = length((px / u.size) - vec2<f32>(0.5)) * 1.3;
  g = g * (1.0 - hold * 0.35 * v * v);
  return vec4<f32>(g, 1.0);
}
"#;

pub struct Pivot {
    world: World,
    gestures: Gestures,
    painter: Painter,
    grass: Backdrop,
    bells: Bells,
    cam: V2,
    trail: Vec<V2>,
    bursts: Vec<(V2, f32)>,
    time: f32,
}

impl Pivot {
    fn view(&self, ctx: &Ctx) -> View {
        let (w, h) = (ctx.width as f32, ctx.height as f32);
        View { center: self.cam, scale: w.min(h) / 30.0, size: [w, h], turn: 0.0 }
    }

    fn hold(&self) -> f32 {
        1.0 - (self.world.pace - self.world.cfg.hold_pace) / (1.0 - self.world.cfg.hold_pace)
    }
}

impl Toy for Pivot {
    fn new(ctx: &mut Ctx) -> anyhow::Result<Self> {
        let world = World::new(Config::default());
        let bells = Bells::new();
        bells.bed(0.25, 55.0);
        Ok(Pivot {
            cam: world.players[0].pos,
            world,
            gestures: Gestures::new(),
            painter: Painter::new(&ctx.gpu),
            grass: Backdrop::new(&ctx.gpu, "grass", GRASS),
            bells,
            trail: Vec::new(),
            bursts: Vec::new(),
            time: 0.0,
        })
    }

    fn pointer(&mut self, ctx: &mut Ctx, e: PointerEvent) {
        let view = self.view(ctx);
        let phase = e.phase as u32;
        let touch = drive(&mut self.world, &mut self.gestures, &view, ctx.ppp as f32, e.id, phase, e.x as f32, e.y as f32, e.t);
        if touch == Touch::Passed {
            ctx.haptic();
        }
    }

    fn update(&mut self, ctx: &mut Ctx, _t: f64, dt: f64) {
        let dt = dt as f32;
        self.time += dt;
        self.world.step(dt);
        for e in self.world.drain() {
            match e {
                Event::Received { by, first_time } => {
                    let you = by == 0;
                    if you {
                        ctx.haptic();
                    }
                    let pan = (self.world.players[by].pos.x / 20.0).clamp(-1.0, 1.0);
                    self.bells.strike(Note { freq: if you { 196.0 } else { 147.0 }, amp: if you { 0.32 } else { 0.16 }, decay: 0.35, pan, bright: if first_time { 2.0 } else { 0.8 } });
                }
                Event::Pass { yours, open, from, .. } => {
                    let pan = (self.world.players[from].pos.x / 20.0).clamp(-1.0, 1.0);
                    let f = if yours { 262.0 * (1.0 + open * 0.5) } else { 175.0 };
                    self.bells.strike(Note { freq: f, amp: if yours { 0.3 } else { 0.14 }, decay: 0.25, pan, bright: 1.5 });
                }
                Event::Poked { at, .. } => {
                    self.bursts.push((at, 0.0));
                    self.bells.strike(Note { freq: 82.0, amp: 0.35, decay: 0.5, pan: 0.0, bright: 3.0 });
                }
                Event::Glance { .. } => {
                    self.bells.strike(Note { freq: 880.0, amp: 0.02, decay: 0.12, pan: 0.0, bright: 0.0 });
                }
                _ => {}
            }
        }
        // the camera follows you and leans toward the ball
        let want = self.world.players[0].pos * 0.7 + self.world.ball_pos * 0.3;
        self.cam = self.cam.lerp(want, 1.0 - (-dt * 1.5).exp());
        if matches!(self.world.ball, Ball::Flight { .. } | Ball::Loose { .. }) {
            self.trail.push(self.world.ball_pos);
        } else if !self.trail.is_empty() {
            self.trail.remove(0);
        }
        if self.trail.len() > 18 {
            self.trail.remove(0);
        }
        for b in &mut self.bursts {
            b.1 += dt;
        }
        self.bursts.retain(|b| b.1 < 1.0);
        let _ = ctx;
    }

    fn draw(&mut self, ctx: &mut Ctx, target: &wgpu::TextureView) {
        let view = self.view(ctx);
        let s = view.scale;
        let size = [ctx.width, ctx.height];
        let hold = self.hold().clamp(0.0, 1.0);
        let w = &self.world;
        let mut enc = ctx.gpu.device.create_command_encoder(&Default::default());
        let you = w.players[0].pos;
        self.grass.render(
            &ctx.gpu,
            &mut enc,
            target,
            size,
            &BackdropParams { time: self.time, scale: s, center: view.center.arr(), p: [hold, 0.0, you.x, you.y, w.cfg.area + 1.0, 0.0, 0.0, 0.0] },
        );
        let p = &mut self.painter;
        let sc = |q: V2| view.to_screen(q);

        // the lanes you could pass along, from what you remember
        if w.yours() || w.incoming().is_some() {
            let k = if w.yours() { 1.0 } else { 0.35 + 0.65 * (1.0 - w.incoming().unwrap_or(1.0) / 1.2).clamp(0.0, 1.0) };
            for l in w.lanes() {
                let o = l.seen.open;
                if o < 0.08 {
                    continue;
                }
                let a = sc(l.a);
                let b = sc(l.b);
                p.glow(a, b, 0.06 * s * o, 0.35 * s, [1.0, 0.85, 0.45, 0.22 * o * k]);
                p.glow(a, b, 0.02 * s, 0.05 * s, [1.0, 0.93, 0.7, 0.55 * o * o * k]);
            }
        }
        // shadows
        for pl in &w.players {
            let at = if pl.kind == Kind::Presser { pl.seen } else { pl.pos };
            let a = if pl.kind == Kind::Presser { (1.0 - pl.seen_age / 3.0).clamp(0.0, 1.0) } else { 1.0 };
            p.disc(sc(at + pitch::v2(0.18, -0.22)), 0.75 * s, 0.3 * s, [0.0, 0.05, 0.02, 0.35 * a]);
        }
        p.render(&ctx.gpu, &mut enc, target, size, None);

        // your gaze: a soft wedge of light where you are looking
        let f = w.players[0].facing;
        let glancing = w.glance.is_some();
        for i in -3..=3 {
            let a = f + i as f32 * 0.17;
            let len = if glancing { 9.0 } else { 5.0 };
            let tip = you + V2::from_angle(a) * len;
            p.glow(sc(you), sc(tip), 0.3 * s, 1.2 * s, [0.75, 0.9, 1.0, if glancing { 0.05 } else { 0.022 }]);
        }
        // the ball's trail
        for (i, q) in self.trail.iter().enumerate() {
            let k = i as f32 / self.trail.len() as f32;
            p.glow(sc(*q), sc(*q), 0.12 * s * k, 0.12 * s, [1.0, 1.0, 0.95, 0.12 * k]);
        }
        // players
        for (i, pl) in w.players.iter().enumerate() {
            match pl.kind {
                Kind::Presser => {
                    let age = pl.seen_age;
                    let fresh = (1.0 - age / 0.5).clamp(0.0, 1.0);
                    let memory = (1.0 - age / 4.0).clamp(0.25, 1.0);
                    let c = sc(pl.seen);
                    if fresh > 0.0 {
                        p.disc(c, 0.72 * s, 0.06 * s, [0.93, 0.42, 0.36, fresh]);
                    }
                    p.ring(c, 0.72 * s, 0.09 * s, 0.05 * s, [0.95, 0.5, 0.45, memory * (1.0 - fresh)]);
                    if pl.stun > 0.0 && fresh > 0.0 {
                        p.ring(c, 1.0 * s, 0.03 * s, 0.05 * s, [1.0, 0.7, 0.6, 0.4 * fresh]);
                    }
                }
                Kind::Mate => {
                    let c = sc(pl.pos);
                    p.disc(c, 0.72 * s, 0.06 * s, [0.78, 0.88, 0.98, 1.0]);
                    let tip = pl.pos + V2::from_angle(pl.facing) * 0.48;
                    p.disc(sc(tip), 0.17 * s, 0.05 * s, [0.45, 0.6, 0.78, 1.0]);
                }
                Kind::You => {
                    let c = sc(pl.pos);
                    let breathe = 0.5 + 0.5 * (self.time * 1.6).sin();
                    if hold > 0.05 {
                        p.glow_ring(c, (1.1 + 0.25 * breathe) * s, 0.04 * s, 0.25 * s, [1.0, 0.95, 0.8, 0.35 * hold]);
                    }
                    p.disc(c, 0.8 * s, 0.06 * s, [0.99, 0.96, 0.88, 1.0]);
                    let tip = pl.pos + V2::from_angle(pl.facing) * 0.52;
                    p.disc(sc(tip), 0.18 * s, 0.05 * s, [0.55, 0.45, 0.3, 1.0]);
                    let _ = i;
                }
            }
        }
        // the ball
        let b = w.ball_pos;
        p.disc(sc(b + pitch::v2(0.08, -0.1)), 0.3 * s, 0.1 * s, [0.0, 0.0, 0.0, 0.35]);
        p.disc(sc(b), 0.28 * s, 0.04 * s, [1.0, 1.0, 0.98, 1.0]);
        p.glow(sc(b), sc(b), 0.2 * s, 0.6 * s, [1.0, 1.0, 0.9, 0.12 + 0.2 * hold]);
        for (at, t) in &self.bursts {
            p.glow_ring(sc(*at), (0.4 + t * 2.5) * s, 0.06 * s, 0.15 * s, [1.0, 0.6, 0.5, 0.5 * (1.0 - t)]);
        }
        p.render(&ctx.gpu, &mut enc, target, size, None);
        ctx.gpu.queue.submit([enc.finish()]);
    }

    fn synth(&mut self) -> Option<Box<dyn Synth>> {
        Some(self.bells.synth())
    }

    fn shaken(&mut self, _ctx: &mut Ctx) {
        let seed = self.world.cfg.seed + 1;
        self.world = World::new(Config { seed, ..Config::default() });
    }
}

kit::ios_shell!(Pivot);
