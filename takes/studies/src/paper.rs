//! Paper: layered card under a low sun. The pitch is cut paper in strata (a deep ground, the pitch
//! card, raised mowing strips, cream line strips, the centre circle cut through to ochre), every edge
//! deckled and casting its shadow; the players are paper-cut figures throwing long shadows.

use crate::common::{Sim, WARM, WPOS};
use kit::backdrop::Backdrop;
use kit::paint::Painter;
use kit::{Ctx, PointerEvent, Toy};
use pitch::{Kind, V2};

const BODY: &str = r#"
fn deck(w: vec2<f32>) -> f32 { return (noise2(w * 2.1) - 0.5) * 0.16 + (noise2(w * 8.0) - 0.5) * 0.05; }
fn hgt(w: vec2<f32>) -> f32 {
  let e = deck(w);
  let q = abs(w) - vec2<f32>(11.5, 24.0);
  let rect = length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0) + e;
  if (rect > 0.0) { return 0.0; }
  var h = 1.0;
  if (fract((w.y + e * 2.0) / 6.0) < 0.5) { h = 2.0; }
  let r = length(w) + e;
  if (abs(rect + 0.7) < 0.1 || (abs(w.y + e * 0.5) < 0.09) || abs(r - 5.4) < 0.1 || r < 0.22) { h = 3.0; }
  if (r < 5.0 && r > 0.4) { h = 0.5; }
  return h;
}
fn backdrop(px: vec2<f32>, _w: vec2<f32>) -> vec4<f32> {
  let w = wpos(px);
  let h = hgt(w);
  var c = vec3<f32>(0.13, 0.22, 0.17);
  if (h > 0.4) { c = vec3<f32>(0.72, 0.58, 0.40); }
  if (h > 0.9) { c = vec3<f32>(0.30, 0.47, 0.32); }
  if (h > 1.9) { c = vec3<f32>(0.38, 0.56, 0.37); }
  if (h > 2.9) { c = vec3<f32>(0.94, 0.90, 0.80); }
  // the low sun: shadows fall down and to the right of every raised edge
  let sun = rot(vec2<f32>(0.86, -0.5));
  var sh = 0.0;
  for (var k = 1; k <= 4; k = k + 1) {
    let up = hgt(w - sun * (0.12 * f32(k)));
    sh = sh + clamp(up - h, 0.0, 1.0) * 0.25;
  }
  c = c * (1.0 - 0.45 * sh);
  // a lit lip on edges facing the sun
  c = c + vec3<f32>(0.10, 0.08, 0.05) * clamp(h - hgt(w + sun * 0.08), 0.0, 1.0);
  // fibre and grain
  c = c * (0.93 + 0.05 * noise2(px * 0.35) + 0.04 * noise2(vec2<f32>(px.x * 0.04, px.y * 0.5)));
  // warm light from the upper left, cool shade to the lower right
  let g = px / u.size;
  c = c * mix(vec3<f32>(1.08, 1.0, 0.88), vec3<f32>(0.8, 0.85, 0.95), clamp(g.x * 0.4 + g.y * 0.7, 0.0, 1.0));
  return vec4<f32>(c, 1.0);
}
"#;

const MATE: [f32; 3] = [0.86, 0.27, 0.16];
const PRESS: [f32; 3] = [0.95, 0.74, 0.25];

pub struct Paper {
    sim: Sim,
    painter: Painter,
    bg: Backdrop,
}

impl Toy for Paper {
    fn new(ctx: &mut Ctx) -> anyhow::Result<Self> {
        let mut sim = Sim::new(5);
        for _ in 0..(WARM * 30.0) as usize {
            sim.tick(1.0 / 30.0);
        }
        Ok(Paper { sim, painter: Painter::new(&ctx.gpu), bg: Backdrop::new(&ctx.gpu, "paper", &format!("{BODY}{WPOS}")) })
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
        let mut enc = ctx.gpu.device.create_command_encoder(&Default::default());
        self.bg.render(&ctx.gpu, &mut enc, target, size, &self.sim.params(&view, [0.0; 7]));
        let p = &mut self.painter;
        let sc = |q: V2| view.to_screen(q);
        let sun = view.dir_to_world([0.86, 0.5]);
        let shade = [0.05, 0.07, 0.06, 0.34];

        // the pass in flight: a cream ribbon lifted off the card
        if let Some(ps) = self.sim.passes.last() {
            let fade = (1.0 - (t - ps.born - 0.6).max(0.0) / 1.2).clamp(0.0, 1.0);
            if fade > 0.0 {
                p.line(sc(ps.a + sun * 0.25), sc(ps.b + sun * 0.25), 0.07 * s, 0.06 * s, [shade[0], shade[1], shade[2], 0.25 * fade]);
                p.line(sc(ps.a), sc(ps.b), 0.06 * s, 0.015 * s, [0.97, 0.94, 0.86, 0.9 * fade]);
            }
        }
        // long shadows first, all of them, then the figures over them
        let figs: Vec<(V2, f32, [f32; 3], f32)> = w
            .players
            .iter()
            .map(|pl| {
                let press = pl.kind == Kind::Presser;
                let tilt = if press { pl.commit.clamp(0.0, 1.0) } else { 0.0 };
                (pl.pos + pl.lean * (tilt * 0.35), pl.facing, if press { PRESS } else { MATE }, 1.0 + 0.35 * tilt)
            })
            .collect();
        for &(at, _, _, long) in &figs {
            let tip = at + sun * (4.6 * long);
            p.line(sc(at), sc(at + sun * (3.7 * long)), 0.4 * s, 0.07 * s, shade);
            p.disc(sc(tip), 0.45 * s, 0.07 * s, shade);
        }
        let ball = w.ball_pos;
        let hop = if matches!(w.ball, pitch::Ball::Rolling { .. }) { 0.5 + 0.5 * (t * 6.0).sin().abs() } else { 0.0 };
        p.line(sc(ball), sc(ball + sun * (0.7 + hop)), 0.17 * s, 0.05 * s, shade);
        for &(at, facing, c, _) in &figs {
            let side = V2::from_angle(facing).perp() * 0.55;
            let rim = [c[0] * 0.5 + 0.5, c[1] * 0.5 + 0.5, c[2] * 0.5 + 0.5, 1.0];
            // card thickness: a hard drop just under, then the cream core, then the colour
            p.line(sc(at - side + sun * 0.09), sc(at + side + sun * 0.09), 0.45 * s, 0.03 * s, [0.0, 0.0, 0.0, 0.35]);
            p.line(sc(at - side), sc(at + side), 0.45 * s, 0.012 * s, rim);
            p.line(sc(at - side), sc(at + side), 0.38 * s, 0.012 * s, [c[0], c[1], c[2], 1.0]);
            p.disc(sc(at + sun * 0.05), 0.36 * s, 0.03 * s, [0.0, 0.0, 0.0, 0.35]);
            p.disc(sc(at), 0.33 * s, 0.012 * s, [c[0] * 0.75, c[1] * 0.75, c[2] * 0.75, 1.0]);
        }
        let bp = ball - sun * (0.2 * hop);
        p.disc(sc(bp + sun * 0.06), 0.24 * s, 0.03 * s, [0.0, 0.0, 0.0, 0.3]);
        p.disc(sc(bp), 0.22 * s, 0.01 * s, [0.98, 0.96, 0.9, 1.0]);
        p.render(&ctx.gpu, &mut enc, target, size, None);
        ctx.gpu.queue.submit([enc.finish()]);
    }
}
