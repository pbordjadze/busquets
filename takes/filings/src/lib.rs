//! Iron Filings (a working title): you never touch the ball.
//!
//! You are a magnet on a dark iron plate. Wherever you drag yourself your six teammates snap into
//! the hexagon of triangles around you, like filings round a magnet (each one clicks as it lands).
//! The ball goes round the ring on its own, teammate to teammate, and every pass strikes the note of
//! the triangle it passed through (you, the passer, the receiver): going round is a scale, turning
//! back comes down it. The pressers drift onto the edges the ball wants; move, and the shape moves
//! off them. Nothing ends: a pinched or cut ball just finds the nearest teammate.
//!
//! The football is drawn, not simulated by `pitch::World`: a tiny world of its own, so the snap can
//! be as quick as a magnet's. No words, no score.

use kit::backdrop::{Backdrop, BackdropParams};
use kit::info::{Demo, Rules, Stroke};
use kit::paint::Painter;
use kit::sound::{pentatonic, Bells, Note};
use kit::{Ctx, Phase, PointerEvent, Synth, Toy};
use pitch::v2::{closest_on_segment, smoothstep, Rng};
use pitch::{v2, V2};
use std::f32::consts::{PI, TAU};

/// The ring's radius, m; the plate's width, m.
const R: f32 = 7.5;
const WIDE: f32 = 40.0;

const PLATE: &str = r#"
fn streaks(d: vec2<f32>, k: f32, t: f32) -> f32 {
  // filings along the field: short streaks, radial around a pole
  let r = length(d);
  let a = atan2(d.y, d.x);
  let n = noise2(vec2<f32>(a * k, r * 0.9 - t * 0.0));
  let m = noise2(vec2<f32>(a * k * 2.3 + 7.0, r * 2.1));
  return smoothstep(0.55, 0.85, n) * (0.6 + 0.4 * m);
}
fn backdrop(px: vec2<f32>, w: vec2<f32>) -> vec4<f32> {
  let me = u.p0.xy;
  let dm = w - me;
  let im = 1.0 / (dot(dm, dm) * 0.02 + 1.0);
  var c = vec3<f32>(0.035, 0.038, 0.045) + vec3<f32>(0.012, 0.012, 0.014) * noise2(w * 0.7);
  // the magnet's field: filings stand radially, brighter near you
  var s = streaks(dm, 48.0, u.time) * (0.25 + 0.75 * im);
  var wsum = 1.0;
  // the pressers: the other pole, filings curling away from them
  let ps = array<vec2<f32>, 3>(u.p0.zw, u.p1.xy, u.p1.zw);
  var dark = 0.0;
  for (var k = 0; k < 3; k = k + 1) {
    let dp = w - ps[k];
    let ip = 0.7 / (dot(dp, dp) * 0.05 + 1.0);
    let sp = streaks(dp, 30.0, u.time);
    s = mix(s, sp * 0.5, ip / (ip + im + 0.15));
    dark = max(dark, ip);
  }
  c = c + vec3<f32>(0.24, 0.255, 0.28) * s;
  c = c * (1.0 - 0.35 * dark);
  c = c + vec3<f32>(0.03, 0.02, 0.008) * im;
  let v = length(px / u.size - vec2<f32>(0.5)) * 1.3;
  c = c * (1.0 - 0.45 * v * v);
  return vec4<f32>(c, 1.0);
}
"#;

#[derive(Clone, Copy)]
struct Mate {
    pos: V2,
    vel: V2,
    snapped: bool,
    flash: f32,
}

#[derive(Clone, Copy)]
struct Presser {
    pos: V2,
    vel: V2,
    target: V2,
    retarget: f32,
}

#[derive(Clone, Copy)]
enum Ball {
    Held { by: usize, t: f32, hold: f32, pinch: f32 },
    Rolling { from: usize, to: usize },
}

pub struct Filings {
    painter: Painter,
    plate: Backdrop,
    bells: Bells,
    me: V2,
    want: V2,
    grab: Option<(u64, V2)>,
    mates: Vec<Mate>,
    pressers: Vec<Presser>,
    /// The ring's turn, radians; which slot each teammate holds.
    rot: f32,
    slot_of: [usize; 6],
    perms: Vec<[usize; 6]>,
    reassign: f32,
    ball: Ball,
    ball_pos: V2,
    trail: Vec<V2>,
    /// Going round: +1 or -1.
    dir: i32,
    lit: [f32; 6],
    was_lit: [bool; 6],
    ripples: Vec<(V2, f32, [f32; 3], f32)>,
    hum: f32,
    /// Clean passes in a row; six is the full circle.
    chain: u32,
    circle: f32,
    time: f32,
    rng: Rng,
    half: V2,
    scale: f32,
    size: [f32; 2],
}

fn perms() -> Vec<[usize; 6]> {
    let mut out = Vec::new();
    let mut a = [0, 1, 2, 3, 4, 5];
    fn go(k: usize, a: &mut [usize; 6], out: &mut Vec<[usize; 6]>) {
        if k == 6 {
            out.push(*a);
            return;
        }
        for i in k..6 {
            a.swap(k, i);
            go(k + 1, a, out);
            a.swap(k, i);
        }
    }
    go(0, &mut a, &mut out);
    out
}

impl Filings {
    /// Slot k: the hexagon round you, bent away from any presser near it (the other pole), so the
    /// triangles change shape with where you stand.
    fn slot(&self, k: usize) -> V2 {
        let mut q = self.me + V2::from_angle(self.rot + k as f32 * PI / 3.0) * R;
        for p in &self.pressers {
            let a = q - p.pos;
            let d = a.len();
            if d < 4.5 && d > 1e-3 {
                q += a.norm() * (4.5 - d) * 0.75;
            }
        }
        q
    }
    fn mate_at_slot(&self, k: usize) -> usize {
        (0..6).find(|&i| self.slot_of[i] == k % 6).unwrap_or(0)
    }
    fn sc(&self, p: V2) -> [f32; 2] {
        [self.size[0] * 0.5 + p.x * self.scale, self.size[1] * 0.5 + p.y * self.scale]
    }
    fn world(&self, ctx: &Ctx, x: f64, y: f64) -> V2 {
        let k = ctx.ppp as f32;
        v2((x as f32 * k - self.size[0] * 0.5) / self.scale, (y as f32 * k - self.size[1] * 0.5) / self.scale)
    }
    fn fit(&mut self, ctx: &Ctx) {
        self.size = [ctx.width as f32, ctx.height as f32];
        self.scale = self.size[0].min(self.size[1]) / WIDE;
        self.half = v2(self.size[0] * 0.5 / self.scale, self.size[1] * 0.5 / self.scale);
    }
    /// Is the edge between two teammates clear of pressers?
    fn clear(&self, a: usize, b: usize) -> f32 {
        let (pa, pb) = (self.mates[a].pos, self.mates[b].pos);
        let d = self.pressers.iter().map(|p| closest_on_segment(p.pos, pa, pb).0.dist(p.pos)).fold(f32::MAX, f32::min);
        smoothstep(1.0, 2.2, d)
    }
    /// The wedge between slots k and k+1: lit when both teammates are snapped and the edge is clear.
    fn wedge_open(&self, k: usize) -> f32 {
        let (a, b) = (self.mate_at_slot(k), self.mate_at_slot(k + 1));
        if !(self.mates[a].snapped && self.mates[b].snapped) {
            return 0.0;
        }
        self.clear(a, b)
    }
    fn note_of_wedge(&self, k: usize) -> f32 {
        // the wedge's direction on the plate picks the degree (moving the ring moves the melody)
        let a = (self.rot + (k as f32 + 0.5) * PI / 3.0).rem_euclid(TAU);
        let deg = (a / TAU * 6.0).floor() as i32;
        pentatonic(220.0, deg)
    }
    fn pass(&mut self, from: usize, to: usize) {
        self.ball = Ball::Rolling { from, to };
    }
}

impl Toy for Filings {
    fn new(ctx: &mut Ctx) -> anyhow::Result<Self> {
        let bells = Bells::new();
        bells.bed(0.12, 41.2);
        let mut rng = Rng::new(5);
        let mates = (0..6).map(|k| Mate { pos: V2::from_angle(k as f32 * 1.1 + 0.4) * (14.0 + rng.f() * 6.0), vel: V2::ZERO, snapped: false, flash: 0.0 }).collect();
        let pressers = [v2(-6.0, -12.0), v2(7.0, 10.0), v2(10.0, -4.0)].iter().map(|&p| Presser { pos: p, vel: V2::ZERO, target: p, retarget: 0.0 }).collect();
        let mut f = Filings {
            painter: Painter::new(&ctx.gpu),
            plate: Backdrop::new(&ctx.gpu, "plate", PLATE),
            bells,
            me: v2(0.0, 2.0),
            want: v2(0.0, 2.0),
            grab: None,
            mates,
            pressers,
            rot: 0.2,
            slot_of: [0, 1, 2, 3, 4, 5],
            perms: perms(),
            reassign: 0.0,
            ball: Ball::Held { by: 0, t: 0.0, hold: 1.2, pinch: 0.0 },
            ball_pos: V2::ZERO,
            trail: Vec::new(),
            dir: 1,
            lit: [0.0; 6],
            was_lit: [false; 6],
            ripples: Vec::new(),
            hum: 0.0,
            chain: 0,
            circle: 0.0,
            time: 0.0,
            rng,
            half: v2(18.0, 30.0),
            scale: 10.0,
            size: [1.0, 1.0],
        };
        f.fit(ctx);
        f.ball_pos = f.mates[0].pos;
        Ok(f)
    }

    fn resize(&mut self, ctx: &mut Ctx) {
        self.fit(ctx);
    }

    fn pointer(&mut self, ctx: &mut Ctx, e: PointerEvent) {
        let at = self.world(ctx, e.x, e.y);
        match e.phase {
            Phase::Down => {
                if self.grab.is_none() {
                    // drag yourself from wherever the finger lands, without a jump
                    self.grab = Some((e.id, self.me - at));
                    ctx.haptic();
                }
            }
            Phase::Move => {
                if let Some((id, off)) = self.grab {
                    if id == e.id {
                        self.want = at + off;
                    }
                }
            }
            Phase::Up | Phase::Cancel => {
                if self.grab.is_some_and(|g| g.0 == e.id) {
                    self.grab = None;
                }
            }
        }
    }

    fn update(&mut self, ctx: &mut Ctx, _t: f64, dt: f64) {
        let dt = (dt as f32).clamp(0.0, 0.05);
        self.time += dt;
        let (hx, hy) = (self.half.x - 1.5, self.half.y - 1.5);
        // the whole ring stays on the plate
        let m = R + 1.5;
        self.want = v2(self.want.x.clamp(-hx + m, hx - m), self.want.y.clamp(-hy + m, hy - m));
        self.me = self.me.lerp(self.want, 1.0 - (-dt * 16.0).exp());
        self.rot += dt * 0.04;

        // who holds which slot: the cheapest fit, kept unless another is clearly better
        self.reassign -= dt;
        if self.reassign <= 0.0 {
            self.reassign = 0.25;
            let slots: Vec<V2> = (0..6).map(|k| self.slot(k)).collect();
            let cost = |p: &[usize; 6], m: &[Mate]| (0..6).map(|i| m[i].pos.dist(slots[p[i]]).powi(2)).sum::<f32>();
            let now = cost(&self.slot_of, &self.mates);
            let best = self.perms.iter().min_by(|a, b| cost(a, &self.mates).partial_cmp(&cost(b, &self.mates)).unwrap()).copied().unwrap();
            if cost(&best, &self.mates) < now * 0.75 - 2.0 {
                self.slot_of = best;
            }
        }

        // the teammates: a quick pull to their slots, a click as each lands
        for i in 0..6 {
            let target = self.slot(self.slot_of[i]);
            let d = target - self.mates[i].pos;
            let want = d.norm() * (d.len() * 9.0).min(13.0);
            let m = &mut self.mates[i];
            m.vel = m.vel.lerp(want, 1.0 - (-dt * 22.0).exp());
            m.pos += m.vel * dt;
            m.flash = (m.flash - dt * 2.5).max(0.0);
            let dist = d.len();
            if !m.snapped && dist < 0.7 {
                m.snapped = true;
                m.flash = 1.0;
                let f = 1900.0 + 260.0 * (i as f32);
                let pan = (m.pos.x / self.half.x).clamp(-1.0, 1.0);
                self.bells.strike(Note { freq: f, amp: 0.06, decay: 0.09, pan, bright: 2.8 });
                self.bells.strike(Note { freq: f * 0.25, amp: 0.035, decay: 0.12, pan, bright: 1.5 });
            } else if m.snapped && dist > 2.4 {
                m.snapped = false;
            }
        }
        // the pressers keep apart from each other
        for i in 0..self.pressers.len() {
            for j in 0..self.pressers.len() {
                if i != j {
                    let a = self.pressers[i].pos - self.pressers[j].pos;
                    if a.len() < 3.5 && a.len() > 1e-4 {
                        self.pressers[i].pos += a.norm() * (3.5 - a.len()) * 0.5 * (dt * 10.0).min(1.0);
                    }
                }
            }
        }
        // keep the pressers out of the teammates and of you
        for p in &mut self.pressers {
            for m in self.mates.iter_mut() {
                let a = p.pos - m.pos;
                if a.len() < 1.3 && a.len() > 1e-4 {
                    let push = a.norm() * (1.3 - a.len());
                    p.pos += push * 0.7;
                }
            }
            // you draw them, like a false 9: pressers near you are pulled toward you and off the
            // edges the ball wants (but never onto you)
            let a = p.pos - self.me;
            let d = a.len();
            if d < 11.0 && d > 2.8 {
                p.pos -= a.norm() * (3.4 * (1.0 - d / 11.0).sqrt()) * dt;
            }
            if d < 2.5 && d > 1e-4 {
                p.pos += a.norm() * (2.5 - d);
            }
        }

        // the pressers: one to the ball, the others onto the edges it wants
        let holder = match self.ball {
            Ball::Held { by, .. } => Some(by),
            Ball::Rolling { to, .. } => Some(to),
        };
        let edges: Vec<(V2, V2)> = holder
            .map(|h| {
                let k = self.slot_of[h];
                [(k + 1) % 6, (k + 5) % 6].iter().map(|&n| (self.mates[h].pos, self.mates[self.mate_at_slot(n)].pos)).collect()
            })
            .unwrap_or_default();
        let ball = self.ball_pos;
        let chaser = (0..self.pressers.len()).min_by(|&a, &b| self.pressers[a].pos.dist(ball).partial_cmp(&self.pressers[b].pos.dist(ball)).unwrap()).unwrap();
        let mut e = 0;
        for i in 0..self.pressers.len() {
            let p = &mut self.pressers[i];
            p.retarget -= dt;
            if i == chaser {
                p.target = ball + (p.pos - ball).norm() * 1.1;
            } else if p.retarget <= 0.0 {
                p.retarget = 0.7 + self.rng.f() * 0.6;
                if let Some(&(a, b)) = edges.get(e) {
                    p.target = a.lerp(b, 0.55);
                    e += 1;
                }
            }
            let d = p.target - p.pos;
            let want = d.norm() * 3.9 * smoothstep(0.0, 2.0, d.len());
            p.vel = p.vel.lerp(want, 1.0 - (-dt * 4.0).exp());
            p.pos += p.vel * dt;
            p.pos = v2(p.pos.x.clamp(-hx, hx), p.pos.y.clamp(-hy, hy));
        }

        // the wedges: lit while their two teammates are snapped and the edge between them is clear
        let mut all = true;
        for k in 0..6 {
            let o = self.wedge_open(k);
            self.lit[k] += (o - self.lit[k]) * (1.0 - (-dt * 10.0).exp());
            let now = o > 0.6;
            if now && !self.was_lit[k] {
                let f = self.note_of_wedge(k) * 2.0;
                self.bells.strike(Note { freq: f, amp: 0.035, decay: 0.5, pan: 0.0, bright: 0.4 });
            }
            self.was_lit[k] = now;
            all &= now;
        }
        self.hum += ((if all { 1.0 } else { 0.0 }) - self.hum) * (1.0 - (-dt * 1.5).exp());
        self.bells.bed(0.12 + 0.18 * self.hum, 41.2);

        // the ball: it goes round on its own
        match self.ball {
            Ball::Held { by, t, hold, pinch } => {
                let m = self.mates[by];
                self.ball_pos = m.pos + (self.me - m.pos).norm() * 0.5;
                let t = t + dt;
                let near = self.pressers.iter().any(|p| p.pos.dist(self.ball_pos) < 1.5);
                let pinch = if near { pinch + dt } else { (pinch - dt).max(0.0) };
                self.ball = Ball::Held { by, t, hold, pinch };
                if pinch > 0.7 {
                    // pinched: it squirts to whoever is nearest (never lost)
                    let to = (0..6).filter(|&j| j != by).min_by(|&a, &b| self.mates[a].pos.dist(m.pos).partial_cmp(&self.mates[b].pos.dist(m.pos)).unwrap()).unwrap();
                    self.ripples.push((self.ball_pos, 0.0, [0.7, 0.3, 0.25], 0.7));
                    self.bells.strike(Note { freq: 73.4, amp: 0.2, decay: 0.4, pan: 0.0, bright: 2.0 });
                    self.chain = 0;
                    self.pass(by, to);
                } else if t > hold && m.snapped {
                    let k = self.slot_of[by];
                    let step = |d: i32| self.mate_at_slot(((k as i32 + d).rem_euclid(6)) as usize);
                    let (fwd, back) = (step(self.dir), step(-self.dir));
                    let open = |j: usize| self.mates[j].snapped && self.clear(by, j) > 0.6;
                    if open(fwd) {
                        self.pass(by, fwd);
                    } else if open(back) {
                        self.dir = -self.dir;
                        self.pass(by, back);
                    } else if t > hold + 1.2 {
                        // nothing round the ring: across it, round you
                        let far = step(2 * self.dir);
                        if open(far) {
                            self.pass(by, far);
                        }
                    }
                }
            }
            Ball::Rolling { from, to } => {
                let target = self.mates[to].pos;
                let d = target - self.ball_pos;
                let step = 17.0 * dt;
                // a presser on the line gets a toe to it: it finds the nearest teammate instead
                if let Some(p) = self.pressers.iter().find(|p| p.pos.dist(self.ball_pos) < 0.8) {
                    let at = self.ball_pos;
                    let near = (0..6).filter(|&j| j != to).min_by(|&a, &b| self.mates[a].pos.dist(at).partial_cmp(&self.mates[b].pos.dist(at)).unwrap()).unwrap();
                    self.ripples.push((p.pos, 0.0, [0.7, 0.3, 0.25], 0.6));
                    self.bells.strike(Note { freq: 82.4, amp: 0.16, decay: 0.3, pan: 0.0, bright: 2.2 });
                    self.chain = 0;
                    self.ball_pos += (self.ball_pos - p.pos).norm() * 0.4;
                    self.ball = Ball::Rolling { from: to, to: near };
                } else if d.len() <= step + 0.4 {
                    self.ball_pos = target;
                    self.ball = Ball::Held { by: to, t: 0.0, hold: 0.35 + self.rng.f() * 0.35, pinch: 0.0 };
                    // the triangle it went through strikes its note
                    let (a, b) = (self.slot_of[from], self.slot_of[to]);
                    let k = if (a + 1) % 6 == b { a } else if (b + 1) % 6 == a { b } else { a.min(b) };
                    let f = self.note_of_wedge(k);
                    let clean = self.lit[k];
                    let pan = (target.x / self.half.x).clamp(-1.0, 1.0);
                    self.bells.strike(Note { freq: f, amp: 0.14 + 0.12 * clean, decay: 1.2 + clean, pan, bright: 0.6 + 0.8 * clean });
                    if clean > 0.6 {
                        self.bells.strike(Note { freq: f * 1.5, amp: 0.05, decay: 1.6, pan: -pan, bright: 0.2 });
                    }
                    let c = if clean > 0.6 { [1.0, 0.8, 0.45] } else { [0.7, 0.75, 0.8] };
                    let mid = (self.me + self.mates[from].pos + target) * (1.0 / 3.0);
                    self.ripples.push((mid, 0.0, c, 0.5 + 0.5 * clean));
                    self.mates[to].flash = self.mates[to].flash.max(0.6);
                    ctx.haptic();
                    if clean > 0.6 {
                        self.chain += 1;
                        if self.chain.is_multiple_of(6) {
                            // the full circle: a chord blooms from you
                            self.circle = 1.0;
                            for (d, a) in [(0, 0.14), (2, 0.1), (4, 0.09), (5, 0.07)] {
                                self.bells.strike(Note { freq: pentatonic(220.0, d), amp: a, decay: 3.5, pan: 0.0, bright: 0.5 });
                            }
                            self.ripples.push((self.me, 0.0, [1.0, 0.85, 0.5], 1.0));
                        }
                    } else {
                        self.chain = 0;
                    }
                } else {
                    self.ball_pos += d.norm() * step;
                }
            }
        }
        if matches!(self.ball, Ball::Rolling { .. }) {
            self.trail.push(self.ball_pos);
            if self.trail.len() > 14 {
                self.trail.remove(0);
            }
        } else if !self.trail.is_empty() {
            self.trail.remove(0);
        }
        self.circle = (self.circle - dt * 0.6).max(0.0);
        for r in &mut self.ripples {
            r.1 += dt;
        }
        self.ripples.retain(|r| r.1 < 1.0);
    }

    fn draw(&mut self, ctx: &mut Ctx, target: &wgpu::TextureView) {
        let size = [ctx.width, ctx.height];
        let s = self.scale;
        let pr = |i: usize| self.pressers.get(i).map(|p| p.pos).unwrap_or(v2(999.0, 999.0));
        let mut enc = ctx.gpu.device.create_command_encoder(&Default::default());
        let params = BackdropParams { time: self.time, scale: s, center: [0.0, 0.0], p: [self.me.x, self.me.y, pr(0).x, pr(0).y, pr(1).x, pr(1).y, pr(2).x, pr(2).y] };
        self.plate.render(&ctx.gpu, &mut enc, target, size, &params);
        let me = self.me;
        let sc = |p: V2| [self.size[0] * 0.5 + p.x * s, self.size[1] * 0.5 + p.y * s];
        let p = &mut self.painter;

        // the pressers: dark lumps, a rust rim
        for q in &self.pressers {
            p.glow(sc(q.pos), sc(q.pos), 1.0 * s, 2.2 * s, [0.85, 0.22, 0.12, 0.28]);
            p.disc(sc(q.pos), 1.0 * s, 0.2 * s, [0.09, 0.03, 0.03, 1.0]);
            p.glow_ring(sc(q.pos), 1.05 * s, 0.1 * s, 0.25 * s, [0.95, 0.38, 0.25, 0.9]);
        }
        // the lit wedges: a warm cell between you and two teammates
        let at_slot: Vec<usize> = (0..6).map(|k| (0..6).find(|&i| self.slot_of[i] == k).unwrap_or(0)).collect();
        for k in 0..6 {
            let (a, b) = (self.mates[at_slot[k]].pos, self.mates[at_slot[(k + 1) % 6]].pos);
            let l = self.lit[k];
            let c = (me + a + b) * (1.0 / 3.0);
            p.glow(sc(c), sc(c), 2.2 * s, 2.6 * s, [1.0, 0.72, 0.35, 0.10 * l + 0.04 * l * self.hum]);
            // the rim edge: the pass the ball can take
            p.glow(sc(a), sc(b), (0.05 + 0.08 * l) * s, 0.35 * s, [1.0, 0.82, 0.55, 0.12 + 0.5 * l]);
            // an edge a presser sits on: cut, red, the ball won't go that way
            let both = self.mates[at_slot[k]].snapped && self.mates[at_slot[(k + 1) % 6]].snapped;
            if both && l < 0.4 {
                for j in 0..8 {
                    if j % 2 == 0 {
                        let (u0, u1) = (j as f32 / 8.0, (j + 1) as f32 / 8.0);
                        p.glow(sc(a.lerp(b, u0)), sc(a.lerp(b, u1)), 0.16 * s, 0.25 * s, [1.0, 0.3, 0.2, 0.95 * (1.0 - l / 0.4)]);
                    }
                }
            }
        }
        // spokes: you to each teammate, taut when snapped
        for m in &self.mates {
            let k = if m.snapped { 1.0 } else { 0.35 };
            p.glow(sc(me), sc(m.pos), 0.03 * s, 0.2 * s, [0.75, 0.8, 0.88, 0.16 * k]);
        }
        // the teammates: filings, standing along your field
        for m in &self.mates {
            let dir = (m.pos - me).norm();
            let len = if m.snapped { 0.75 } else { 0.55 + 0.3 * (m.vel.len() / 13.0) };
            let (a, b) = (m.pos - dir * len, m.pos + dir * len);
            p.glow(sc(a), sc(b), 0.32 * s, 0.6 * s, [0.75, 0.82, 0.92, 0.25 + 0.4 * m.flash]);
            p.glow(sc(a), sc(b), 0.2 * s, 0.08 * s, [0.88, 0.92, 0.98, 1.0]);
            if m.flash > 0.0 {
                p.glow_ring(sc(m.pos), (1.0 + 1.6 * (1.0 - m.flash)) * s, 0.05 * s, 0.2 * s, [1.0, 0.95, 0.85, 0.6 * m.flash]);
            }
        }
        // you: the magnet
        let held = self.grab.is_some();
        let breathe = 0.5 + 0.5 * (self.time * 1.6).sin();
        p.glow(sc(me), sc(me), 1.2 * s, 3.5 * s, [1.0, 0.8, 0.5, 0.22 + 0.12 * self.hum + if held { 0.1 } else { 0.0 }]);
        p.glow(sc(me), sc(me), 0.9 * s, 0.25 * s, [1.0, 0.95, 0.88, 1.0]);
        p.glow_ring(sc(me), (1.8 + 0.25 * breathe + if held { 0.5 } else { 0.0 }) * s, 0.05 * s, 0.3 * s, [1.0, 0.85, 0.6, 0.35]);
        // the ball
        for (k, q) in self.trail.iter().enumerate() {
            let f = k as f32 / self.trail.len() as f32;
            p.glow(sc(*q), sc(*q), 0.12 * s * f, 0.4 * s * f, [1.0, 0.95, 0.85, 0.3 * f]);
        }
        let b = sc(self.ball_pos);
        p.glow(b, b, 0.75 * s, 2.8 * s, [1.0, 0.97, 0.85, 0.5]);
        p.glow(b, b, 0.65 * s, 0.1 * s, [1.0, 1.0, 1.0, 1.0]);
        // the circle so far: a pip round you for each clean pass in a row
        let pips = (self.chain % 6) as usize;
        for k in 0..6 {
            let q = me + V2::from_angle(-PI / 2.0 + k as f32 * PI / 3.0) * 2.6;
            let on = k < pips || self.circle > 0.0;
            let c = if on { [1.0, 0.85, 0.5, 0.95] } else { [0.7, 0.72, 0.75, 0.25] };
            p.glow(sc(q), sc(q), 0.42 * s, 0.25 * s, c);
        }
        if self.circle > 0.0 {
            p.glow(sc(me), sc(me), R * s, R * 0.6 * s, [1.0, 0.8, 0.45, 0.12 * self.circle]);
        }
        for (at, t, c, a) in &self.ripples {
            p.glow_ring(sc(*at), (0.6 + t * 3.5) * s, 0.05 * s, 0.3 * s, [c[0], c[1], c[2], a * 0.45 * (1.0 - t)]);
        }
        p.render(&ctx.gpu, &mut enc, target, size, None);
        ctx.gpu.queue.submit([enc.finish()]);
    }

    fn synth(&mut self) -> Option<Box<dyn Synth>> {
        Some(self.bells.synth())
    }

    fn shaken(&mut self, _ctx: &mut Ctx) {
        // scatter the filings
        for m in &mut self.mates {
            m.vel += v2(self.rng.s(), self.rng.s()) * 25.0;
            m.snapped = false;
        }
    }

    /// The info mark: a fingertip drags you across the plate; the ring follows and clicks back.
    fn demo(&self, ctx: &Ctx) -> Option<Demo> {
        let k = ctx.ppp as f32;
        let a = self.sc(self.me);
        let from = [(a[0] / k) as f64, (a[1] / k) as f64];
        let [w, h] = ctx.points();
        let to = [if from[0] < w * 0.5 { w * 0.68 } else { w * 0.32 }, (from[1] + if from[1] < h * 0.5 { 150.0 } else { -150.0 }).clamp(140.0, h - 160.0)];
        Some(Demo::new("Drag yourself: teammates snap to you, pressers follow you.", vec![Stroke::drag(0.4, 1.8, from, to)]))
    }

    fn rules(&self) -> Option<Rules> {
        Some(Rules::new(
            &[
                "You never touch the ball: it goes round your teammates on its own.",
                "Wherever you go they snap into triangles round you, like filings to a magnet.",
                "You draw the pressers too, like a false 9: go near one and it follows you off the edge it cuts (red).",
                "Each pass strikes its triangle's note.",
                "Six clean passes in a row make the full circle.",
            ],
            "Drawn: all of it, a small simulation.",
        ))
    }
}

#[cfg(feature = "app")]
kit::ios_shell!(Filings);
