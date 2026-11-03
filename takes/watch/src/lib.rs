//! Watch (a working title): the engine playing itself, eleven a side on a floodlit pitch stood on its
//! end (the bottom side attacks up), and your hand on the dials. No menus and no words: the dials are
//! in the fingers and the change shows in the drawing.
//!
//! - **Press.** Drag up or down on a side's half: that side presses harder or sits off. Its players'
//!   auras swell when it defends, and its pressers lean and commit toward the pass they guess.
//! - **Risk.** Drag sideways on a side's half: bolder passes (and more runs in behind). With the ball,
//!   the lanes it would dare to play glow from the ball: a bold side sees many, a careful one few.
//! - **Shape.** Pinch on a side's half: it compacts or stretches (along the pinch: fingers apart up the
//!   screen stretch it long, across the screen wide). Each player is threaded to its two nearest
//!   teammates, so a compact side is a dense web and a stretched one a thin net.
//! - **Two fingers tapped, or a shake:** from eleven a side to the rondo and back (the dials stay).
//!
//! What it shows: possession as the light (the grass leans to the side with the ball), the ball's
//! path drawn in the passer's colour and fading, a ring tightening round the ball when the other side
//! traps it, a flare at the box when an attack gets there. Faint lines at the edges (a vertical one
//! beside each half for press, a level one past each goal line for risk) wake when touched.

use kit::backdrop::{Backdrop, BackdropParams};
use kit::paint::Painter;
use kit::sound::{Bells, Note};
use kit::{Ctx, Phase, PointerEvent, Synth, Toy};
use pitch::{openness, v2, Ball, Config, Event, View, World, V2};
use std::collections::HashMap;
use std::f32::consts::FRAC_PI_2;

const TEAM: [[f32; 3]; 2] = [[0.80, 0.90, 1.0], [1.0, 0.52, 0.40]];

const PITCH: &str = r#"
fn outline(p: vec2<f32>, c: vec2<f32>, b: vec2<f32>) -> f32 {
  let d = abs(p - c) - b;
  return abs(length(max(d, vec2<f32>(0.0))) + min(max(d.x, d.y), 0.0));
}
fn backdrop(px: vec2<f32>, wp: vec2<f32>) -> vec4<f32> {
  // the pitch stands on its end: world x runs up the screen
  let w = vec2<f32>(-(px.y - u.size.y * 0.5), -(px.x - u.size.x * 0.5)) / u.scale;
  let poss = u.p0.x;
  let ball = u.p0.yz;
  let eleven = u.p0.w;
  let half = u.p1.xy;
  let fade = u.p1.z;
  let aa = 1.2 / u.scale;
  let band = mix(4.0, 2.0 * half.x / 12.0, eleven);
  let stripe = step(0.5, fract(w.x / band + 0.5));
  let n = noise2(w * 1.7) * 0.5 + noise2(w * 6.0) * 0.5;
  var g = mix(vec3<f32>(0.066, 0.17, 0.105), vec3<f32>(0.078, 0.198, 0.122), stripe) * (0.92 + 0.12 * n);
  // possession is the light: the grass leans to the side that has it
  let tint = mix(vec3<f32>(0.80, 0.90, 1.0), vec3<f32>(1.0, 0.52, 0.40), poss);
  g = g + tint * 0.05;
  // a pool of floodlight round the ball
  let db = length(w - ball) / (half.y * 0.9);
  g = g * (0.86 + 0.3 * exp(-db * db * 2.0));
  // the markings
  var d = outline(w, vec2<f32>(0.0), half);
  if (eleven > 0.5) {
    d = min(d, abs(w.x));
    d = min(d, abs(length(w) - 9.15));
    d = min(d, max(length(w) - 0.25, 0.0));
    for (var k = 0; k < 2; k = k + 1) {
      let sx = select(-1.0, 1.0, k == 1);
      d = min(d, outline(w, vec2<f32>(sx * (half.x - 8.25), 0.0), vec2<f32>(8.25, 20.16)));
      d = min(d, outline(w, vec2<f32>(sx * (half.x - 2.75), 0.0), vec2<f32>(2.75, 9.16)));
      let spot = vec2<f32>(sx * (half.x - 11.0), 0.0);
      d = min(d, max(length(w - spot) - 0.2, 0.0));
      if (abs(w.x) < half.x - 16.5) { d = min(d, abs(length(w - spot) - 9.15)); }
      if (abs(w.x) > half.x) { d = min(d, outline(w, vec2<f32>(sx * (half.x + 1.0), 0.0), vec2<f32>(1.0, 3.66))); }
    }
  } else {
    d = min(d, max(length(w) - 0.2, 0.0));
  }
  let line = 1.0 - smoothstep(0.06, 0.06 + aa, d);
  g = mix(g, vec3<f32>(0.86, 0.93, 0.88), line * 0.3);
  // the grass runs a little past the lines, then night
  let out = max(abs(w.x) - half.x, abs(w.y) - half.y) - 2.5;
  let on = 1.0 - smoothstep(-1.5, 1.0, out);
  let night = vec3<f32>(0.02, 0.032, 0.034) + tint * 0.012;
  var c = mix(night, g, on);
  let v = length((px / u.size) - vec2<f32>(0.5));
  c = c * (1.0 - 0.35 * v * v);
  return vec4<f32>(c * fade, 1.0);
}
"#;

/// One side's dials: how hard it presses, how much it risks, its shape's stretch (depth, width).
#[derive(Clone, Copy)]
struct Dials {
    press: f32,
    risk: f32,
    shape: V2,
}

impl Default for Dials {
    fn default() -> Dials {
        Dials { press: 0.6, risk: 0.4, shape: v2(1.0, 1.0) }
    }
}

/// A finger on the glass (pixels): where it began, where it is, the side it touches, the axis it
/// settled on (0 not yet, 1 up/down: press, 2 sideways: risk, 3 spent on a pinch), the dials then.
struct Finger {
    start: [f32; 2],
    at: [f32; 2],
    team: usize,
    axis: u8,
    base: Dials,
}

struct Pinch {
    ids: (u64, u64),
    team: usize,
    d0: [f32; 2],
    base: V2,
    t0: f64,
    moved: f32,
}

pub struct Watch {
    world: World,
    eleven: bool,
    /// Each player's home as the shape was drawn, the keepers, each side's centre.
    base: Vec<V2>,
    keeper: Vec<bool>,
    pivot: [V2; 2],
    dials: [Dials; 2],
    /// Seconds since each side's dials moved.
    touched: [f32; 2],
    fingers: HashMap<u64, Finger>,
    pinch: Option<Pinch>,
    /// The ball's path: point, the passer's side, age.
    trail: Vec<(V2, usize, f32)>,
    bursts: Vec<(V2, usize, f32, f32)>,
    poss: f32,
    fade: f32,
    time: f32,
    seed: u64,
    painter: Painter,
    pitch: Backdrop,
    bells: Bells,
}

impl Watch {
    fn reset(&mut self) {
        let cfg = if self.eleven { Config { seed: self.seed, ..Config::eleven() } } else { Config { human: None, seed: self.seed, ..Config::default() } };
        self.world = World::new(cfg);
        (self.base, self.keeper, self.pivot) = shape_of(&self.world, self.eleven);
        self.trail.clear();
        self.bursts.clear();
        self.fade = 0.0;
        self.poss = self.world.attack as f32;
        self.apply();
    }

    /// The dials into the world: every player's traits and home.
    fn apply(&mut self) {
        for (i, p) in self.world.players.iter_mut().enumerate() {
            let t = p.team as usize;
            let d = self.dials[t];
            p.traits.press = d.press;
            p.traits.risk = d.risk;
            p.traits.run = 0.1 + 0.45 * d.risk;
            let b = self.base[i];
            p.home = if self.keeper[i] {
                b
            } else {
                let o = b - self.pivot[t];
                self.pivot[t] + v2(o.x * d.shape.x, o.y * d.shape.y)
            };
        }
    }

    fn view(&self, ctx: &Ctx) -> View {
        let (w, h) = (ctx.width as f32, ctx.height as f32);
        let half = self.world.cfg.half;
        let m = if self.eleven { v2(7.0, 12.0) } else { v2(8.0, 8.0) };
        let scale = (w / (2.0 * half.y + m.x)).min(h / (2.0 * half.x + m.y));
        View { center: V2::ZERO, scale, size: [w, h], turn: FRAC_PI_2 }
    }

    fn toggle(&mut self) {
        self.eleven = !self.eleven;
        self.seed += 1;
        self.reset();
    }
}

fn shape_of(w: &World, eleven: bool) -> (Vec<V2>, Vec<bool>, [V2; 2]) {
    let base: Vec<V2> = w.players.iter().map(|p| p.home).collect();
    let keeper: Vec<bool> = base.iter().map(|b| eleven && b.x.abs() > 45.0).collect();
    let mut pivot = [V2::ZERO; 2];
    if eleven {
        for (t, pv) in pivot.iter_mut().enumerate() {
            let outfield: Vec<V2> = (0..base.len()).filter(|&i| w.players[i].team as usize == t && !keeper[i]).map(|i| base[i]).collect();
            let sum = outfield.iter().fold(V2::ZERO, |a, &b| a + b);
            *pv = sum * (1.0 / outfield.len().max(1) as f32);
        }
    }
    (base, keeper, pivot)
}

fn sub(a: [f32; 2], b: [f32; 2]) -> [f32; 2] {
    [a[0] - b[0], a[1] - b[1]]
}

fn len(a: [f32; 2]) -> f32 {
    (a[0] * a[0] + a[1] * a[1]).sqrt()
}

impl Toy for Watch {
    fn new(ctx: &mut Ctx) -> anyhow::Result<Self> {
        let world = World::new(Config::eleven());
        let bells = Bells::new();
        bells.bed(0.12, 49.0);
        let mut w = Watch {
            world,
            eleven: true,
            base: Vec::new(),
            keeper: Vec::new(),
            pivot: [V2::ZERO; 2],
            dials: [Dials::default(); 2],
            touched: [9.0; 2],
            fingers: HashMap::new(),
            pinch: None,
            trail: Vec::new(),
            bursts: Vec::new(),
            poss: 0.0,
            fade: 0.0,
            time: 0.0,
            seed: 3,
            painter: Painter::new(&ctx.gpu),
            pitch: Backdrop::new(&ctx.gpu, "pitch", PITCH),
            bells,
        };
        w.reset();
        Ok(w)
    }

    fn pointer(&mut self, ctx: &mut Ctx, e: PointerEvent) {
        let ppp = ctx.ppp as f32;
        let p = [e.x as f32 * ppp, e.y as f32 * ppp];
        let (w, h) = (ctx.width as f32, ctx.height as f32);
        let side = |y: f32| if y > h * 0.5 { 0 } else { 1 };
        match e.phase {
            Phase::Down => {
                let team = side(p[1]);
                self.fingers.insert(e.id, Finger { start: p, at: p, team, axis: 0, base: self.dials[team] });
                if self.fingers.len() == 2 && self.pinch.is_none() {
                    let ids: Vec<u64> = self.fingers.keys().copied().collect();
                    let (a, b) = (self.fingers[&ids[0]].at, self.fingers[&ids[1]].at);
                    // two fingers: whatever the first was dragging goes back, and they pinch
                    for f in self.fingers.values_mut() {
                        self.dials[f.team].press = f.base.press;
                        self.dials[f.team].risk = f.base.risk;
                        f.axis = 3;
                    }
                    let team = side((a[1] + b[1]) * 0.5);
                    self.pinch = Some(Pinch { ids: (ids[0], ids[1]), team, d0: sub(b, a), base: self.dials[team].shape, t0: e.t, moved: 0.0 });
                }
            }
            Phase::Move => {
                match self.fingers.get_mut(&e.id) {
                    Some(f) => f.at = p,
                    None => return,
                }
                if let Some(pi) = &mut self.pinch {
                    if let (Some(a), Some(b)) = (self.fingers.get(&pi.ids.0), self.fingers.get(&pi.ids.1)) {
                        let d1 = sub(b.at, a.at);
                        let l0 = len(pi.d0).max(1.0);
                        let k = (len(d1) / l0).clamp(0.4, 2.5);
                        pi.moved = pi.moved.max((k - 1.0).abs());
                        // fingers apart up the screen: depth (world x); across it: width
                        let wx = (pi.d0[1] / l0).powi(2);
                        let s = v2(pi.base.x * k.powf(wx), pi.base.y * k.powf(1.0 - wx));
                        self.dials[pi.team].shape = v2(s.x.clamp(0.5, 1.4), s.y.clamp(0.5, 1.4));
                        self.touched[pi.team] = 0.0;
                    }
                    return;
                }
                let Some(f) = self.fingers.get_mut(&e.id) else { return };
                let d = sub(f.at, f.start);
                if f.axis == 0 && len(d) > 10.0 * ppp {
                    f.axis = if d[1].abs() > d[0].abs() { 1 } else { 2 };
                }
                let t = f.team;
                match f.axis {
                    1 => self.dials[t].press = (f.base.press - d[1] / (h * 0.35)).clamp(0.0, 1.0),
                    2 => self.dials[t].risk = (f.base.risk + d[0] / (w * 0.7)).clamp(0.0, 1.0),
                    _ => return,
                }
                self.touched[t] = 0.0;
            }
            Phase::Up | Phase::Cancel => {
                self.fingers.remove(&e.id);
                if let Some(pi) = &self.pinch {
                    if pi.ids.0 == e.id || pi.ids.1 == e.id {
                        let tap = pi.moved < 0.08 && e.t - pi.t0 < 0.35 && e.phase == Phase::Up;
                        self.pinch = None;
                        if tap {
                            self.toggle();
                            ctx.haptic();
                        }
                    }
                }
            }
        }
    }

    fn update(&mut self, _ctx: &mut Ctx, _t: f64, dt: f64) {
        let dt = dt as f32;
        self.time += dt;
        self.apply();
        self.world.step(dt);
        let half = self.world.cfg.half;
        let pan = |q: V2| (-q.y / half.y).clamp(-1.0, 1.0);
        for e in self.world.drain() {
            match e {
                Event::Pass { at, from, .. } => {
                    let t = self.world.players[from].team;
                    self.bells.strike(Note { freq: if t == 0 { 147.0 } else { 131.0 }, amp: 0.07, decay: 0.12, pan: pan(at), bright: 2.0 });
                }
                Event::Turnover { by, .. } => {
                    let q = self.world.players[by].pos;
                    self.bursts.push((q, self.world.players[by].team as usize, 0.0, 0.7));
                    self.bells.strike(Note { freq: 98.0, amp: 0.16, decay: 0.6, pan: pan(q), bright: 1.0 });
                }
                Event::Poked { at, by } => {
                    self.bursts.push((at, self.world.players[by].team as usize, 0.0, 0.5));
                }
                Event::Chance { team, at } => {
                    self.bursts.push((at, team as usize, 0.0, 2.0));
                    self.bells.strike(Note { freq: 294.0, amp: 0.12, decay: 1.4, pan: pan(at), bright: 0.6 });
                    self.bells.strike(Note { freq: 440.0, amp: 0.08, decay: 1.8, pan: pan(at), bright: 0.4 });
                }
                _ => {}
            }
        }
        if let Ball::Rolling { from, .. } = self.world.ball {
            let t = self.world.players.get(from).map(|p| p.team).unwrap_or(self.world.attack) as usize;
            self.trail.push((self.world.ball_pos, t, 0.0));
        }
        for q in &mut self.trail {
            q.2 += dt;
        }
        self.trail.retain(|q| q.2 < 4.0);
        for b in &mut self.bursts {
            b.2 += dt;
        }
        self.bursts.retain(|b| b.2 < 1.6);
        self.poss += (self.world.attack as f32 - self.poss) * (1.0 - (-dt * 1.2).exp());
        for t in &mut self.touched {
            *t += dt;
        }
        self.fade = (self.fade + dt * 1.2).min(1.0);
    }

    fn draw(&mut self, ctx: &mut Ctx, target: &wgpu::TextureView) {
        let view = self.view(ctx);
        let s = view.scale;
        let (wd, ht) = (ctx.width as f32, ctx.height as f32);
        let size = [ctx.width, ctx.height];
        let w = &self.world;
        let half = w.cfg.half;
        let fade = self.fade * self.fade * (3.0 - 2.0 * self.fade);
        let mut enc = ctx.gpu.device.create_command_encoder(&Default::default());
        self.pitch.render(
            &ctx.gpu,
            &mut enc,
            target,
            size,
            &BackdropParams { time: self.time, scale: s, center: [0.0, 0.0], p: [self.poss, w.ball_pos.x, w.ball_pos.y, if self.eleven { 1.0 } else { 0.0 }, half.x, half.y, fade, 0.0] },
        );
        let p = &mut self.painter;
        let sc = |q: V2| view.to_screen(q);
        let col = |t: usize, a: f32| [TEAM[t][0], TEAM[t][1], TEAM[t][2], a * fade];
        let attack = w.attack as usize;
        let defend = 1 - attack;
        let pr = if self.eleven { 0.85 } else { 0.7 };

        // the ball's path, in the passer's colour, fading
        for k in 1..self.trail.len() {
            let (a, ta, _) = self.trail[k - 1];
            let (b, tb, age) = self.trail[k];
            if ta != tb || a.dist(b) > 3.0 {
                continue;
            }
            let f = 1.0 - age / 4.0;
            p.glow(sc(a), sc(b), 0.09 * s, 0.25 * s, col(tb, 0.45 * f * f));
        }
        // each side's web: every player threaded to its two nearest teammates
        let reach = if self.eleven { 24.0 } else { 14.0 };
        for (i, pi) in w.players.iter().enumerate() {
            let mut near: Vec<(f32, usize)> = w.players.iter().enumerate().filter(|(j, q)| *j != i && q.team == pi.team).map(|(j, q)| (q.pos.dist(pi.pos), j)).filter(|x| x.0 < reach).collect();
            near.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
            for &(d, j) in near.iter().take(2) {
                let a = 0.26 * (1.0 - d / reach);
                p.line(sc(pi.pos), sc(w.players[j].pos), 0.05 * s, 0.05 * s, col(pi.team as usize, a));
            }
        }
        // the lanes the side with the ball would dare: more the bolder it is
        if let Some(by) = w.holder() {
            let others: Vec<Option<V2>> = w.players.iter().filter(|q| q.team as usize != attack).map(|q| (q.stun <= 0.0).then_some(q.pos)).collect();
            let lp = w.lane_params();
            let th = 0.9 - 0.7 * self.dials[attack].risk;
            for (j, q) in w.players.iter().enumerate() {
                if j == by || q.team as usize != attack {
                    continue;
                }
                let o = openness(w.ball_pos, q.pos, &others, &lp).open;
                if o > th {
                    let k = ((o - th) / (1.0 - th).max(0.05)).min(1.0);
                    p.glow(sc(w.ball_pos), sc(q.pos), 0.05 * s, 0.3 * s, col(attack, 0.22 * k));
                }
            }
        }
        // the side without the ball: auras as wide as it presses, bodies leaning to their guess
        let press = self.dials[defend].press;
        for q in w.players.iter().filter(|q| q.team as usize == defend) {
            p.glow(sc(q.pos), sc(q.pos), (0.8 + 2.4 * press) * s * 0.5, (0.8 + 2.4 * press) * s, col(defend, 0.06 + 0.14 * press));
            if q.commit > 0.05 {
                let tip = q.pos + q.lean * (0.6 + 1.6 * q.commit);
                p.line(sc(q.pos), sc(tip), 0.42 * pr * s, 0.12 * s, col(defend, 0.35 + 0.4 * q.commit));
            }
        }
        // shadows
        for q in &w.players {
            p.disc(sc(q.pos + v2(-0.2, -0.25)), pr * s, 0.35 * s, [0.0, 0.02, 0.01, 0.4 * fade]);
        }
        p.render(&ctx.gpu, &mut enc, target, size, None);

        // a trap: the other side closing round the ball
        let mut ds: Vec<f32> = w.players.iter().filter(|q| q.team as usize == defend).map(|q| q.pos.dist(w.ball_pos)).collect();
        ds.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let close = ds.iter().filter(|&&d| d < 7.0).count();
        if close >= 2 {
            let r = ds.iter().take(3).sum::<f32>() / 3.0;
            p.glow_ring(sc(w.ball_pos), r * s, 0.08 * s, 0.25 * s, col(defend, (0.12 * (close - 1) as f32).min(0.4)));
        }
        // players
        for q in &w.players {
            let t = q.team as usize;
            let c = sc(q.pos);
            let k = if t == attack { 1.0 } else { 0.88 };
            p.disc(c, pr * s, 0.08 * s, [TEAM[t][0] * k, TEAM[t][1] * k, TEAM[t][2] * k, fade]);
            let tip = q.pos + V2::from_angle(q.facing) * (pr * 0.62);
            p.disc(sc(tip), pr * 0.24 * s, 0.06 * s, [TEAM[t][0] * 0.45, TEAM[t][1] * 0.45, TEAM[t][2] * 0.5, fade]);
            if q.stun > 0.0 {
                p.ring(c, pr * 1.4 * s, 0.04 * s, 0.06 * s, col(t, 0.3));
            }
        }
        // the ball
        let b = w.ball_pos;
        p.disc(sc(b + v2(-0.1, -0.12)), 0.36 * s, 0.12 * s, [0.0, 0.0, 0.0, 0.4 * fade]);
        p.disc(sc(b), 0.34 * s, 0.05 * s, [1.0, 1.0, 0.97, fade]);
        p.glow(sc(b), sc(b), 0.3 * s, 1.0 * s, [1.0, 1.0, 0.9, 0.18 * fade]);
        for &(at, t, age, size) in &self.bursts {
            let k = (1.0 - age / 1.6).max(0.0);
            p.glow_ring(sc(at), (0.6 + age * 3.0 * size) * s, 0.08 * s, 0.25 * s, col(t, 0.5 * k));
            if size > 1.5 {
                p.glow(sc(at), sc(at), 2.0 * s, 6.0 * s, col(t, 0.2 * k));
            }
        }
        // the dials at the edges: faint, waking when touched
        let side_x = wd - (wd - 2.0 * half.y * s) * 0.25;
        for t in 0..2 {
            let wake = (-self.touched[t] * 0.8).exp();
            let a = 0.06 + 0.5 * wake;
            let d = self.dials[t];
            // press: beside the side's half, from the halfway line toward its own goal
            let (y0, y1) = if t == 0 { (ht * 0.5 + 6.0 * s, ht * 0.5 + (half.x - 4.0) * s) } else { (ht * 0.5 - 6.0 * s, ht * 0.5 - (half.x - 4.0) * s) };
            p.line([side_x, y0], [side_x, y1], 0.6, 1.0, [1.0, 1.0, 1.0, a * 0.25 * fade]);
            let yp = y1 + (y0 - y1) * d.press;
            p.glow([side_x, y1], [side_x, yp], 1.6, 2.5, col(t, a));
            // risk: past the goal line, a level line lit from the left
            let ym = if t == 0 { (ht * 0.5 + half.x * s + ht) * 0.5 } else { (ht * 0.5 - half.x * s) * 0.5 };
            let (x0, x1) = (wd * 0.3, wd * 0.7);
            p.line([x0, ym], [x1, ym], 0.6, 1.0, [1.0, 1.0, 1.0, a * 0.25 * fade]);
            p.glow([x0, ym], [x0 + (x1 - x0) * d.risk, ym], 1.6, 2.5, col(t, a));
        }
        p.render(&ctx.gpu, &mut enc, target, size, None);
        ctx.gpu.queue.submit([enc.finish()]);
    }

    fn synth(&mut self) -> Option<Box<dyn Synth>> {
        Some(self.bells.synth())
    }

    fn shaken(&mut self, ctx: &mut Ctx) {
        self.toggle();
        ctx.haptic();
    }
}

kit::ios_shell!(Watch);
