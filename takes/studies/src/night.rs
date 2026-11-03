//! Night: a night tactical cam. The players are lights on a dark pitch; each presser casts a cone of
//! attention from itself, bending past the ball toward its guess (`lean`), narrowing and burning
//! brighter as it commits; the carrier's body is a faint beam the cones read.

use crate::common::{Sim, WARM, WPOS};
use kit::backdrop::Backdrop;
use kit::paint::Painter;
use kit::{Ctx, PointerEvent, Toy};
use pitch::{Kind, V2};

const BODY: &str = r#"
fn backdrop(px: vec2<f32>, _w: vec2<f32>) -> vec4<f32> {
  let w = wpos(px);
  let stripe = smoothstep(0.45, 0.55, abs(fract(w.y / 8.0) - 0.5) * 2.0);
  var c = mix(vec3<f32>(0.012, 0.035, 0.04), vec3<f32>(0.018, 0.048, 0.052), stripe);
  c = c + vec3<f32>(0.0, 0.018, 0.02) * noise2(w * 1.3) + vec3<f32>(0.01, 0.02, 0.02) * noise2(w * 0.2);
  let g = abs(fract(w / 4.0 + 0.5) - 0.5) * 4.0;
  c = c + vec3<f32>(0.01, 0.04, 0.045) * (1.0 - smoothstep(0.0, 0.05, min(g.x, g.y)));
  c = c + vec3<f32>(0.05, 0.12, 0.12) * (1.0 - smoothstep(0.04, 0.11, pitch_lines(w)));
  // the light of the play: a cool pool around the ball
  let b = vec2<f32>(u.p0.x, u.p0.y);
  c = c + vec3<f32>(0.02, 0.06, 0.07) * exp(-dot(w - b, w - b) / 60.0);
  let v = (px / u.size - vec2<f32>(0.5)) * vec2<f32>(1.3, 1.0);
  c = c * (1.0 - 0.9 * dot(v, v));
  c = c + (hash2(px + vec2<f32>(fract(u.time * 7.0) * 311.0, 0.0)) - 0.5) * 0.018;
  return vec4<f32>(max(c, vec3<f32>(0.0)), 1.0);
}
"#;

pub struct Night {
    sim: Sim,
    painter: Painter,
    bg: Backdrop,
}

impl Toy for Night {
    fn new(ctx: &mut Ctx) -> anyhow::Result<Self> {
        let mut sim = Sim::new(21);
        for _ in 0..(WARM * 30.0) as usize {
            sim.tick(1.0 / 30.0);
        }
        Ok(Night { sim, painter: Painter::new(&ctx.gpu), bg: Backdrop::new(&ctx.gpu, "night", &format!("{BODY}{WPOS}")) })
    }
    fn pointer(&mut self, ctx: &mut Ctx, e: PointerEvent) {
        self.sim.touch(ctx, e);
    }
    fn update(&mut self, _c: &mut Ctx, _t: f64, dt: f64) {
        self.sim.tick(dt as f32);
    }
    fn draw(&mut self, ctx: &mut Ctx, target: &wgpu::TextureView) {
        let view = self.sim.view(ctx);
        let s = view.scale;
        let size = [ctx.width, ctx.height];
        let w = &self.sim.world;
        let t = self.sim.time;
        let ball = w.ball_pos;
        let mut enc = ctx.gpu.device.create_command_encoder(&Default::default());
        self.bg.render(&ctx.gpu, &mut enc, target, size, &self.sim.params(&view, [ball.x, ball.y, 0.0, 0.0, 0.0, 0.0, 0.0]));
        let p = &mut self.painter;
        let sc = |q: V2| view.to_screen(q);

        // the cones: from each presser, bending past the ball toward its guess
        for (i, pl) in w.players.iter().enumerate().filter(|(_, pl)| pl.kind == Kind::Presser) {
            let c = pl.commit.clamp(0.0, 1.0);
            let to_ball = ball - pl.pos;
            let reach = 5.0 + 9.0 * c;
            let guess = ball + pl.lean * reach;
            let side = pl.lean.perp();
            let spread = 2.0 + 6.0 * (1.0 - c);
            let ctrl = pl.pos + to_ball * 0.7;
            let flick = 0.85 + 0.15 * (t * 9.0 + i as f32 * 2.1).sin() * (t * 3.7).sin();
            for k in -4..=4 {
                let kf = k as f32 / 4.0;
                let end = guess + side * (kf * spread);
                let edge = 1.0 - kf.abs() * 0.6;
                let mut prev = pl.pos;
                for j in 1..=12 {
                    let u = j as f32 / 12.0;
                    let q = pl.pos * ((1.0 - u) * (1.0 - u)) + ctrl * (2.0 * u * (1.0 - u)) + end * (u * u);
                    let a = (0.04 + 0.08 * c) * edge * (1.0 - u * 0.7) * flick;
                    p.glow(sc(prev), sc(q), (0.1 + 0.45 * u) * s, (0.3 + 1.0 * u) * s, [1.0, 0.45, 0.16, a]);
                    prev = q;
                }
            }
            // where it thinks the ball will go: a smouldering mark
            p.glow_ring(sc(guess), (0.5 + 1.0 * (1.0 - c)) * s, 0.05 * s, 0.25 * s, [1.0, 0.5, 0.2, 0.15 + 0.4 * c]);
        }
        // the carrier's body: a narrow white beam where it faces (what the pressers read)
        if let Some(h) = w.holder() {
            let pl = &w.players[h];
            let f = V2::from_angle(pl.facing);
            for k in -2..=2 {
                let d = V2::from_angle(pl.facing + k as f32 * 0.07);
                p.glow(sc(pl.pos + f * 0.5), sc(pl.pos + d * 7.0), 0.15 * s, 0.6 * s, [0.6, 0.85, 1.0, 0.05]);
            }
        }
        // the ball's wake
        let n = self.sim.trail.len().max(1) as f32;
        for (k, q) in self.sim.trail.windows(2).enumerate() {
            let f = k as f32 / n;
            p.glow(sc(q[0]), sc(q[1]), 0.06 * s, 0.3 * s, [0.75, 0.95, 1.0, 0.5 * f]);
        }
        // the lights
        for (i, pl) in w.players.iter().enumerate() {
            let at = sc(pl.pos);
            let tw = 0.9 + 0.1 * (t * 2.0 + i as f32 * 1.3).sin();
            if pl.kind == Kind::Presser {
                let c = pl.commit;
                p.glow(at, at, 0.3 * s, 3.2 * s, [1.0, 0.35, 0.1, (0.18 + 0.15 * c) * tw]);
                p.glow(at, at, 0.18 * s, 0.25 * s, [1.0, 0.75, 0.5, 0.95]);
            } else {
                let has = w.holder() == Some(i);
                p.glow(at, at, 0.3 * s, 3.0 * s, [0.5, 0.85, 1.0, (0.16 + if has { 0.2 } else { 0.0 }) * tw]);
                p.glow(at, at, 0.2 * s, 0.25 * s, [0.85, 0.97, 1.0, 1.0]);
                if has {
                    let b = (t * 2.2).sin() * 0.5 + 0.5;
                    p.glow_ring(at, (1.1 + 0.3 * b) * s, 0.04 * s, 0.12 * s, [0.6, 0.9, 1.0, 0.35]);
                }
            }
        }
        let b = sc(ball);
        p.glow(b, b, 0.15 * s, 1.4 * s, [1.0, 1.0, 0.95, 0.35]);
        p.glow(b, b, 0.14 * s, 0.12 * s, [1.0, 1.0, 1.0, 1.0]);
        p.render(&ctx.gpu, &mut enc, target, size, None);
        ctx.gpu.queue.submit([enc.finish()]);
    }
}
