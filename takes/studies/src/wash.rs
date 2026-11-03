//! Wash: pressure as weather, in watercolour. Each presser bleeds pigment where it presses (indigo,
//! warming to crimson as it commits), each drop spreading and drying with a darker wet edge; the
//! open grass stays bare paper. Teammates are small ochre dabs; every pass is one stroke of sumi
//! ink, swelling where the ball was struck and drying grey as it ages.

use crate::common::{Sim, WARM, WPOS};
use kit::backdrop::Backdrop;
use kit::paint::Painter;
use kit::{Ctx, PointerEvent, Toy};
use pitch::{Kind, V2};

const BODY: &str = r#"
fn backdrop(px: vec2<f32>, _w: vec2<f32>) -> vec4<f32> {
  let w = wpos(px);
  var c = vec3<f32>(0.955, 0.94, 0.9);
  c = c * (0.965 + 0.035 * noise2(px * 0.3) + 0.02 * noise2(px * 1.1)) - vec3<f32>(0.03, 0.03, 0.02) * smoothstep(0.55, 0.9, noise2(w * 0.25));
  // a graphite under-drawing of the pitch
  let l = 1.0 - smoothstep(0.02, 0.07, pitch_lines(w) + (noise2(w * 3.0) - 0.5) * 0.04);
  c = c - vec3<f32>(0.22, 0.21, 0.2) * l * (0.4 + 0.6 * noise2(w * 1.7));
  let v = px / u.size - vec2<f32>(0.5);
  c = c * (1.0 - 0.25 * dot(v, v));
  return vec4<f32>(c, 1.0);
}
"#;

/// A drop of pigment: where, when, how big, its colour.
struct Drop {
    at: V2,
    born: f32,
    r: f32,
    c: [f32; 3],
    life: f32,
}

pub struct Wash {
    sim: Sim,
    painter: Painter,
    bg: Backdrop,
    drops: Vec<Drop>,
    next: f32,
}

fn mix(a: [f32; 3], b: [f32; 3], k: f32) -> [f32; 3] {
    [a[0] + (b[0] - a[0]) * k, a[1] + (b[1] - a[1]) * k, a[2] + (b[2] - a[2]) * k]
}

impl Wash {
    fn tick(&mut self, dt: f32) {
        self.sim.tick(dt);
        let t = self.sim.time;
        if t >= self.next {
            self.next = t + 0.22;
            for pl in &self.sim.world.players {
                let k = pl.commit.clamp(0.0, 1.0);
                if pl.kind == Kind::Presser {
                    let c = mix([0.16, 0.24, 0.55], [0.72, 0.1, 0.22], k);
                    self.drops.push(Drop { at: pl.pos + pl.lean * (0.6 + 1.6 * k), born: t, r: 1.0 + 1.4 * k + 0.6 * pl.vel.len().min(3.0) / 3.0, c, life: 6.0 });
                } else if (t * 7.3 + pl.home.x).sin() > 0.6 {
                    self.drops.push(Drop { at: pl.pos, born: t, r: 0.7, c: [0.86, 0.6, 0.2], life: 4.0 });
                }
            }
        }
        self.drops.retain(|d| t - d.born < d.life);
    }
}

impl Toy for Wash {
    fn new(ctx: &mut Ctx) -> anyhow::Result<Self> {
        let mut me = Wash { sim: Sim::new(13), painter: Painter::new(&ctx.gpu), bg: Backdrop::new(&ctx.gpu, "wash", &format!("{BODY}{WPOS}")), drops: Vec::new(), next: 0.0 };
        for _ in 0..(WARM * 30.0) as usize {
            me.tick(1.0 / 30.0);
        }
        Ok(me)
    }
    fn pointer(&mut self, ctx: &mut Ctx, e: PointerEvent) {
        self.sim.touch(ctx, e);
    }
    fn update(&mut self, _c: &mut Ctx, _t: f64, dt: f64) {
        self.tick(dt as f32);
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

        // the pigment: each drop a few overlapping blooms, pale inside, darker at the wet edge
        for d in &self.drops {
            let age = (t - d.born) / d.life;
            let fade = (1.0 - age) * (age * 8.0).min(1.0);
            let r = d.r * (1.0 + 0.9 * age.sqrt());
            for k in 0..5 {
                let a = d.born * 13.7 + k as f32 * 2.1;
                let o = V2::from_angle(a) * (r * 0.45 * (k as f32).min(1.0));
                let rk = r * (0.45 + 0.35 * ((a * 3.1).sin() * 0.5 + 0.5));
                let at = sc(d.at + o);
                p.disc(at, rk * s, 0.3 * s, [d.c[0], d.c[1], d.c[2], 0.04 * fade]);
                p.ring(at, rk * s, 0.06 * s, 0.05 * s, [d.c[0] * 0.8, d.c[1] * 0.8, d.c[2] * 0.85, 0.05 * fade]);
            }
        }
        // the passes: one stroke of ink each, swelling where struck, drying grey
        for ps in &self.sim.passes {
            let age = t - ps.born;
            if age > 12.0 {
                continue;
            }
            let dry = (age / 12.0).clamp(0.0, 1.0);
            let ink = [0.08 + 0.3 * dry, 0.08 + 0.3 * dry, 0.1 + 0.3 * dry, 0.85 * (1.0 - dry)];
            let len = ps.a.dist(ps.b);
            let n = ((len / 0.35) as usize).clamp(1, 80);
            for j in 0..n {
                let u0 = j as f32 / n as f32;
                let u1 = (j + 1) as f32 / n as f32;
                let taper = (1.0 - u0).powf(0.6) * (0.4 + 0.6 * (u0 * 12.0).min(1.0));
                let grain = 0.8 + 0.2 * (ps.born * 31.0 + u0 * 9.0).sin();
                p.line(sc(ps.a.lerp(ps.b, u0)), sc(ps.a.lerp(ps.b, u1)), (0.03 + 0.17 * taper) * s * grain, 0.04 * s, ink);
            }
        }
        // the teammates: ochre dabs ringed in sienna; the pressers: a dark knot of pigment
        for (i, pl) in w.players.iter().enumerate() {
            let at = sc(pl.pos);
            if pl.kind == Kind::Presser {
                let c = mix([0.12, 0.17, 0.42], [0.55, 0.06, 0.16], pl.commit.clamp(0.0, 1.0));
                p.disc(at, 0.42 * s, 0.12 * s, [c[0], c[1], c[2], 0.75]);
            } else {
                let b = 1.0 + 0.05 * (t * 1.5 + i as f32).sin();
                p.disc(at, 0.5 * s * b, 0.12 * s, [0.93, 0.68, 0.25, 0.55]);
                p.ring(at, 0.5 * s * b, 0.08 * s, 0.04 * s, [0.65, 0.33, 0.12, 0.6]);
            }
        }
        let b = sc(w.ball_pos);
        p.disc(b, 0.2 * s, 0.04 * s, [0.05, 0.05, 0.07, 0.95]);
        p.render(&ctx.gpu, &mut enc, target, size, None);
        ctx.gpu.queue.submit([enc.finish()]);
    }
}
