//! Lines: the pitch drawn in living chalk-light that remembers every pass. Each pass lays a line where
//! the ball went: gold while fresh, cooling to blue as it ages, braided thicker each time the same
//! route is used again, every strand shimmering. The players are open rings, the pressers red.

use crate::common::{Sim, CIRCLE, HALF, WARM, WPOS};
use kit::backdrop::Backdrop;
use kit::paint::Painter;
use kit::{Ctx, PointerEvent, Toy};
use pitch::{v2, Kind, V2};

const BODY: &str = r#"
fn backdrop(px: vec2<f32>, _w: vec2<f32>) -> vec4<f32> {
  let w = wpos(px);
  let n = noise2(w * 0.15) * 0.6 + noise2(w * 0.6) * 0.3 + noise2(px * 0.5) * 0.1;
  var c = mix(vec3<f32>(0.02, 0.035, 0.09), vec3<f32>(0.05, 0.07, 0.16), n);
  let v = (px / u.size - vec2<f32>(0.5)) * vec2<f32>(1.2, 1.0);
  c = c * (1.0 - 0.8 * dot(v, v));
  return vec4<f32>(c, 1.0);
}
"#;

pub struct Lines {
    sim: Sim,
    painter: Painter,
    bg: Backdrop,
    marks: Vec<(V2, V2)>,
}

/// A line of light in `strands` shimmering threads.
#[allow(clippy::too_many_arguments)]
fn strand(p: &mut Painter, a: [f32; 2], b: [f32; 2], strands: u32, width: f32, t: f32, seed: f32, c: [f32; 3], alpha: f32) {
    let d = [b[0] - a[0], b[1] - a[1]];
    let len = (d[0] * d[0] + d[1] * d[1]).sqrt().max(1e-3);
    let nrm = [-d[1] / len, d[0] / len];
    for k in 0..strands {
        let kf = k as f32 + seed;
        let off = (t * (0.6 + 0.13 * kf) + kf * 2.3).sin() * width * (1.0 + k as f32 * 0.6);
        let sh = 0.7 + 0.3 * (t * (1.7 + 0.21 * kf) + kf * 5.1).sin();
        let o = [nrm[0] * off, nrm[1] * off];
        p.glow([a[0] + o[0], a[1] + o[1]], [b[0] + o[0], b[1] + o[1]], width * 0.35, width * 1.2, [c[0], c[1], c[2], alpha * sh]);
    }
}

impl Toy for Lines {
    fn new(ctx: &mut Ctx) -> anyhow::Result<Self> {
        let mut sim = Sim::new(9);
        for _ in 0..(WARM * 2.0 * 30.0) as usize {
            sim.tick(1.0 / 30.0);
        }
        let [hx, hy] = HALF;
        let mut marks = vec![(v2(-hx, -hy), v2(hx, -hy)), (v2(hx, -hy), v2(hx, hy)), (v2(hx, hy), v2(-hx, hy)), (v2(-hx, hy), v2(-hx, -hy)), (v2(-hx, 0.0), v2(hx, 0.0))];
        for k in 0..48 {
            let a = |k: i32| V2::from_angle(k as f32 / 48.0 * std::f32::consts::TAU) * CIRCLE;
            marks.push((a(k), a(k + 1)));
        }
        Ok(Lines { sim, painter: Painter::new(&ctx.gpu), bg: Backdrop::new(&ctx.gpu, "lines", &format!("{BODY}{WPOS}")), marks })
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

        // the pitch, breathing faintly
        for (k, &(a, b)) in self.marks.iter().enumerate() {
            strand(p, sc(a), sc(b), 2, 0.05 * s, t, k as f32 * 0.37, [0.65, 0.78, 1.0], 0.22);
        }
        // every pass remembered: gold when fresh, cooling to blue, braided by use
        for ps in &self.sim.passes {
            let age = t - ps.born;
            let fresh = (-age / 4.0).exp();
            let c = [0.45 + 0.55 * fresh, 0.62 + 0.2 * fresh, 1.0 - 0.6 * fresh];
            let alpha = 0.2 + 0.6 * fresh;
            let uses = ps.uses.min(6);
            strand(p, sc(ps.a), sc(ps.b), 1 + uses, (0.07 + 0.04 * uses as f32) * s, t, ps.born, c, alpha);
            p.glow(sc(ps.a), sc(ps.a), 0.08 * s, 0.25 * s, [c[0], c[1], c[2], alpha]);
        }
        // the ball's wake
        let n = self.sim.trail.len().max(1) as f32;
        for (k, q) in self.sim.trail.windows(2).enumerate() {
            p.glow(sc(q[0]), sc(q[1]), 0.08 * s, 0.2 * s, [1.0, 0.9, 0.6, 0.7 * k as f32 / n]);
        }
        // the players: chalk rings and a stroke for the way they face
        for (i, pl) in w.players.iter().enumerate() {
            let at = pl.pos;
            let press = pl.kind == Kind::Presser;
            let c = if press { [1.0, 0.4, 0.35] } else { [0.9, 0.95, 1.0] };
            let wob = 1.0 + 0.04 * (t * 3.0 + i as f32).sin();
            p.glow_ring(sc(at), 0.75 * s * wob, 0.06 * s, 0.05 * s, [c[0], c[1], c[2], 0.8]);
            p.glow_ring(sc(at), 0.75 * s * wob, 0.3 * s, 0.4 * s, [c[0], c[1], c[2], 0.08]);
            let f = V2::from_angle(pl.facing);
            p.glow(sc(at + f * 0.8), sc(at + f * 1.5), 0.04 * s, 0.05 * s, [c[0], c[1], c[2], 0.6]);
            if press && pl.commit > 0.05 {
                strand(p, sc(at), sc(at + pl.lean * (1.0 + 4.0 * pl.commit)), 2, 0.04 * s, t, i as f32, c, 0.3 * pl.commit);
            }
        }
        let b = sc(w.ball_pos);
        p.glow(b, b, 0.15 * s, 1.0 * s, [1.0, 0.85, 0.5, 0.35]);
        p.glow(b, b, 0.16 * s, 0.1 * s, [1.0, 1.0, 0.95, 1.0]);
        p.render(&ctx.gpu, &mut enc, target, size, None);
        ctx.gpu.queue.submit([enc.finish()]);
    }
}
