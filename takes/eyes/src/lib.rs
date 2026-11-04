//! Eyes (a working title): the pivot over your own shoulder. The same rondo as Pivot, seen from just
//! behind and above you, looking where you look:
//!
//! - a perspective camera on your facing; what is behind you does not exist until you turn;
//! - a finger moving sideways is your head turning (a glance); the pressers you have not looked at
//!   lately are only ghosts where you last saw them, and the ones you see lean the way they read you;
//! - flick up and the ball goes along your view (bent by the flick's own angle), its pace the
//!   flick's speed; while the finger moves fast with the ball its path is drawn on the grass;
//! - a tap on a teammate passes to them;
//! - while you hold the ball the light drains and closes in.

use kit::backdrop::{Backdrop, BackdropParams};
use kit::paint::{pm, Painter, Shape};
use kit::sound::{Bells, Note};
use kit::{Ctx, PointerEvent, Synth, Toy};
use pitch::v2::angle_diff;
use pitch::{pace_of, v2, Ball, Config, Event, Fate, Gesture, Gestures, Kind, World, V2};

/// Points a second a release must have to be a pass rather than a look.
const PASS_SPEED: f32 = pitch::view::PASS_SPEED;

const GROUND: &str = r#"
fn backdrop(px: vec2<f32>, w: vec2<f32>) -> vec4<f32> {
  let eye = vec3<f32>(u.center, u.p0.x);
  let a = u.p0.y; let pt = u.p0.z; let cy = u.p0.w; let f = u.scale;
  let hold = u.p1.x; let half = u.p1.y;
  let fw = vec3<f32>(cos(a), sin(a), 0.0);
  let rt = vec3<f32>(sin(a), -cos(a), 0.0);
  let cf = fw * cos(pt) - vec3<f32>(0.0, 0.0, sin(pt));
  let up = fw * sin(pt) + vec3<f32>(0.0, 0.0, cos(pt));
  let d = normalize(cf + rt * ((px.x - u.size.x * 0.5) / f) - up * ((px.y - cy) / f));
  // the ground, cast for every pixel (derivatives first, in uniform flow)
  let t = eye.z / max(-d.z, 1e-4);
  let g = eye.xy + d.xy * t;
  let fwm = fwidth(g);
  let blur = max(fwm.x, fwm.y);
  let haze = vec3<f32>(0.42, 0.50, 0.50);
  // mown stripes, fading to their average where they would shimmer
  let sv = g.y / 5.0;
  let sw = fwidth(sv);
  let stripe = mix(smoothstep(0.25 - sw, 0.25 + sw, abs(fract(sv) - 0.5)), 0.5, smoothstep(0.15, 0.45, sw));
  let n = noise2(g * 2.7) * 0.5 + noise2(g * 9.0) * 0.5;
  var grass = mix(vec3<f32>(0.10, 0.27, 0.15), vec3<f32>(0.13, 0.33, 0.18), stripe);
  grass = grass * (0.9 + 0.16 * mix(n, 0.5, smoothstep(0.05, 0.3, blur)));
  // the floodlit box and its chalk
  let r = length(g) / 30.0;
  grass = grass * (1.12 - 0.5 * min(r * r, 1.0));
  let box = abs(max(abs(g.x), abs(g.y)) - half);
  let lw = 0.06;
  let chalk = (1.0 - smoothstep(lw, lw + blur * 1.5 + 0.01, box)) * (1.0 - smoothstep(0.3, 1.2, blur));
  grass = mix(grass, vec3<f32>(0.86, 0.9, 0.86), chalk * 0.85);
  let circle = abs(length(g) - 9.15);
  let c2 = (1.0 - smoothstep(lw, lw + blur * 1.5 + 0.01, circle)) * (1.0 - smoothstep(0.3, 1.2, blur));
  grass = mix(grass, vec3<f32>(0.8, 0.86, 0.82), c2 * 0.35);
  let fog = 1.0 - exp(-t / 75.0);
  grass = mix(grass, haze * 0.8, fog);
  // the sky: dusk, a dark ring of stands, four floodlights
  let el = d.z;
  let az = atan2(d.y, d.x);
  var sky = mix(vec3<f32>(0.46, 0.50, 0.50), vec3<f32>(0.20, 0.26, 0.38), smoothstep(0.0, 0.25, el));
  sky = mix(sky, vec3<f32>(0.05, 0.07, 0.15), smoothstep(0.2, 0.9, el));
  let stand = 0.07 + 0.025 * noise2(vec2<f32>(az * 9.0, 0.0)) + 0.012 * step(0.5, fract(az * 40.0));
  let roof = 1.0 - smoothstep(stand - 0.004, stand + 0.004, el);
  let crowd = noise2(vec2<f32>(az * 300.0, el * 300.0));
  sky = mix(sky, vec3<f32>(0.07, 0.08, 0.11) + vec3<f32>(0.10, 0.08, 0.06) * crowd * smoothstep(0.0, 0.05, el), roof);
  for (var i = 0; i < 4; i = i + 1) {
    let la = 0.785398 + f32(i) * 1.570796;
    let da = atan2(sin(az - la), cos(az - la));
    let q = vec2<f32>(da, el - 0.16);
    let mast = exp(-da * da / 0.00002) * step(el, 0.16) * 0.25;
    sky = mix(sky, vec3<f32>(0.06, 0.07, 0.1), mast);
    sky = sky + vec3<f32>(1.0, 0.95, 0.82) * (exp(-dot(q, q) / 0.00012) * 1.5 + exp(-dot(q, q) / 0.006) * 0.25);
  }
  var col = select(sky, grass, d.z < 0.0);
  // a soft line of haze along the horizon
  col = mix(col, haze, exp(-abs(el) * 60.0) * 0.35);
  // time held: the colour drains and the light closes in
  let lum = dot(col, vec3<f32>(0.3, 0.55, 0.15));
  col = mix(col, vec3<f32>(lum) * vec3<f32>(0.85, 0.95, 1.05), hold * 0.5);
  let v = length((px - vec2<f32>(u.size.x * 0.5, cy)) / u.size.y) * 1.6;
  col = col * (1.0 - hold * 0.55 * v * v) * (1.0 - 0.18 * v * v);
  return vec4<f32>(col, 1.0);
}
"#;

/// The camera: just behind and above your right shoulder, looking along `fw`, pitched down.
#[derive(Clone, Copy)]
struct Cam {
    eye: [f32; 3],
    fw: V2,
    rt: V2,
    angle: f32,
    pitch: f32,
    f: f32,
    cx: f32,
    cy: f32,
}

const BACK: f32 = 2.8;
const SIDE: f32 = 1.0;
const HEIGHT: f32 = 2.7;
const PITCH: f32 = 0.30;

impl Cam {
    fn new(you: V2, angle: f32, w: f32, h: f32) -> Cam {
        let fw = V2::from_angle(angle);
        let rt = v2(fw.y, -fw.x);
        let e = you - fw * BACK + rt * SIDE;
        Cam { eye: [e.x, e.y, HEIGHT], fw, rt, angle, pitch: PITCH, f: w * 0.5 / 0.5f32.tan(), cx: w * 0.5, cy: h * 0.56 }
    }
    /// Camera space: right, up, ahead (metres).
    fn space(&self, p: V2, z: f32) -> [f32; 3] {
        let d = p - v2(self.eye[0], self.eye[1]);
        let dz = z - self.eye[2];
        let along = d.dot(self.fw);
        let (s, c) = self.pitch.sin_cos();
        [d.dot(self.rt), along * s + dz * c, along * c - dz * s]
    }
    /// The screen point (pixels) and depth of a world point `z` metres up; None behind you.
    fn project(&self, p: V2, z: f32) -> Option<([f32; 2], f32)> {
        let [x, y, zc] = self.space(p, z);
        (zc > 0.3).then(|| ([self.cx + self.f * x / zc, self.cy - self.f * y / zc], zc))
    }
    fn px(&self, p: V2, z: f32) -> Option<[f32; 2]> {
        self.project(p, z).map(|q| q.0)
    }
    /// Where a pixel's ray meets the grass.
    fn ground(&self, px: [f32; 2]) -> Option<V2> {
        let (s, c) = self.pitch.sin_cos();
        let sx = (px[0] - self.cx) / self.f;
        let sy = (px[1] - self.cy) / self.f;
        let dz = -s - c * sy;
        if dz > -1e-3 {
            return None;
        }
        let dxy = self.fw * (c - s * sy) + self.rt * sx;
        let t = -self.eye[2] / dz;
        Some(v2(self.eye[0], self.eye[1]) + dxy * t)
    }
}

/// A finger on the glass: where it came down, the head's direction then, whether it is a look.
struct Finger {
    id: u64,
    at: [f32; 2],
    base: f32,
    looking: bool,
}

pub struct Eyes {
    world: World,
    gestures: Gestures,
    painter: Painter,
    ground: Backdrop,
    bells: Bells,
    finger: Option<Finger>,
    cam_pos: V2,
    cam_angle: f32,
    /// Where good weights met their man, and pokes: where, how long ago, how well.
    graces: Vec<(V2, f32, f32)>,
    bursts: Vec<(V2, f32)>,
    trail: Vec<V2>,
    time: f32,
}

impl Eyes {
    fn cam(&self, ctx: &Ctx) -> Cam {
        Cam::new(self.cam_pos, self.cam_angle, ctx.width as f32, ctx.height as f32)
    }

    fn hold(&self) -> f32 {
        (1.0 - (self.world.pace - self.world.cfg.hold_pace) / (1.0 - self.world.cfg.hold_pace)).clamp(0.0, 1.0)
    }

    /// The pass a finger moving upward at `dir` (screen, unit) would make: world direction.
    fn along_view(&self, dir: [f32; 2]) -> Option<V2> {
        if dir[1] > -0.45 {
            return None;
        }
        let theta = dir[0].atan2(-dir[1]);
        Some(V2::from_angle(self.world.players[0].facing - theta))
    }

    /// The teammate under a pixel, if any.
    fn mate_at(&self, cam: &Cam, px: [f32; 2]) -> Option<usize> {
        let mut best: Option<(f32, usize)> = None;
        for (j, p) in self.world.players.iter().enumerate() {
            if p.kind != Kind::Mate {
                continue;
            }
            let (Some((a, za)), Some(b)) = (cam.project(p.pos, 0.0), cam.px(p.pos, 1.7)) else { continue };
            let k = cam.f / za;
            let d = seg_dist(px, a, b) - 0.4 * k;
            if d < 45.0 && best.is_none_or(|(bd, _)| d < bd) {
                best = Some((d, j));
            }
        }
        best.map(|b| b.1)
    }
}

fn seg_dist(p: [f32; 2], a: [f32; 2], b: [f32; 2]) -> f32 {
    let (ab, ap) = ([b[0] - a[0], b[1] - a[1]], [p[0] - a[0], p[1] - a[1]]);
    let h = ((ap[0] * ab[0] + ap[1] * ab[1]) / (ab[0] * ab[0] + ab[1] * ab[1]).max(1e-6)).clamp(0.0, 1.0);
    let (dx, dy) = (ap[0] - ab[0] * h, ap[1] - ab[1] * h);
    (dx * dx + dy * dy).sqrt()
}

fn mix3(a: [f32; 3], b: [f32; 3], k: f32) -> [f32; 3] {
    [a[0] + (b[0] - a[0]) * k, a[1] + (b[1] - a[1]) * k, a[2] + (b[2] - a[2]) * k]
}

/// A circle on the grass of radius `r` as the screen sees it: a flat stadium (ends, half-height).
fn ground_ellipse(cam: &Cam, at: V2, r: f32) -> Option<([f32; 2], [f32; 2], f32)> {
    let (l, rr, n, fa) = (cam.px(at - cam.rt * r, 0.0)?, cam.px(at + cam.rt * r, 0.0)?, cam.px(at - cam.fw * r, 0.0)?, cam.px(at + cam.fw * r, 0.0)?);
    let hh = ((n[1] - fa[1]).abs() * 0.5).max(0.5);
    let y = (n[1] + fa[1]) * 0.5;
    let half = ((rr[0] - l[0]) * 0.5 - hh).max(0.0);
    let cx = (l[0] + rr[0]) * 0.5;
    Some(([cx - half, y], [cx + half, y], hh))
}

/// A ring of light on the grass.
fn ground_ring(p: &mut Painter, cam: &Cam, at: V2, r: f32, width: f32, color: [f32; 4]) {
    if let (Some((a, b, hh)), Some((_, z))) = (ground_ellipse(cam, at, r), cam.project(at, 0.0)) {
        let w = width * cam.f / z;
        p.shapes_add.push(Shape { a, b, r: hh, ring: w.max(1.0), soft: w * 0.8 + 1.0, _pad: 0.0, color: pm(color) });
    }
}

impl Toy for Eyes {
    /// The rules mark: the few rules that make the scene, and what is real and what is drawn.
    fn rules(&self) -> Option<kit::info::Rules> {
        Some(kit::info::Rules::new(
            &[
                "You see only where you face; behind you, players are where you last looked.",
                "The pressers you can see lean the way they read you.",
                "While you hold the ball, the light closes in.",
            ],
            "Drawn: all of it, Pivot's simulated rondo over your shoulder.",
        ))
    }

    /// The info mark: a look over the shoulder, then a flick up the view.
    fn demo(&self, ctx: &Ctx) -> Option<kit::info::Demo> {
        use kit::info::{Demo, Stroke};
        let [w, h] = ctx.points();
        let (x, y) = (w * 0.5, h * 0.72);
        Some(Demo::new(
            "Behind you, a presser is only where you last saw him.",
            vec![Stroke::drag(0.4, 1.1, [x + 70.0, y], [x - 70.0, y]), Stroke::drag(2.2, 0.12, [x, y], [x, y - 120.0])],
        ))
    }

    fn new(ctx: &mut Ctx) -> anyhow::Result<Self> {
        let world = World::new(Config::default());
        let bells = Bells::new();
        bells.bed(0.22, 55.0);
        Ok(Eyes {
            cam_pos: world.players[0].pos,
            cam_angle: world.players[0].facing,
            world,
            gestures: Gestures::new(),
            painter: Painter::new(&ctx.gpu),
            ground: Backdrop::new(&ctx.gpu, "eyes ground", GROUND),
            bells,
            finger: None,
            graces: Vec::new(),
            bursts: Vec::new(),
            trail: Vec::new(),
            time: 0.0,
        })
    }

    fn pointer(&mut self, ctx: &mut Ctx, e: PointerEvent) {
        let (x, y, t) = (e.x as f32, e.y as f32, e.t);
        let ppp = ctx.ppp as f32;
        let gesture = match e.phase as u32 {
            0 => {
                self.gestures.down(e.id, x, y, t);
                self.finger = Some(Finger { id: e.id, at: [x, y], base: self.world.players[0].facing, looking: false });
                None
            }
            1 => self.gestures.moved(e.id, x, y, t),
            2 => self.gestures.up(e.id, x, y, t),
            _ => {
                self.gestures.cancel(e.id);
                self.world.aim = None;
                None
            }
        };
        let width_pts = ctx.width as f32 / ppp;
        match gesture {
            Some(Gesture::Swipe { dir, .. }) => {
                let Some(f) = self.finger.as_mut().filter(|f| f.id == e.id) else { return };
                let (dx, dy) = (x - f.at[0], y - f.at[1]);
                // sideways is the head turning: a full width of glass is half a turn
                if f.looking || (dx.abs() > 18.0 && dx.abs() > 0.6 * dy.abs()) {
                    f.looking = true;
                    let base = f.base;
                    self.world.glance(base - dx / width_pts * std::f32::consts::PI);
                }
                let fast = self.gestures.speed(e.id, t).unwrap_or(0.0);
                let aim = self.along_view(dir).filter(|_| (self.world.yours() || self.world.incoming().is_some()) && fast >= PASS_SPEED);
                self.world.aim = aim.map(|d| (d, pace_of(fast)));
            }
            Some(Gesture::Flick { dir, speed, .. }) => {
                self.world.aim = None;
                self.finger = None;
                if speed >= PASS_SPEED {
                    if let Some(d) = self.along_view(dir) {
                        if self.world.kick(d, pace_of(speed)) {
                            ctx.haptic();
                        }
                    }
                }
            }
            Some(Gesture::Tap { at }) => {
                self.finger = None;
                let cam = self.cam(ctx);
                let px = [at[0] * ppp, at[1] * ppp];
                let on_ball = self.world.yours() || self.world.incoming().is_some();
                if let Some(j) = self.mate_at(&cam, px) {
                    let to = self.world.players[j].pos;
                    if self.world.tap(to) && on_ball {
                        ctx.haptic();
                    }
                } else if !on_ball {
                    // a look where you tapped (on the grass, or up into the stands)
                    let a = match cam.ground(px) {
                        Some(g) => (g - self.world.players[0].pos).angle(),
                        None => self.world.players[0].facing - ((px[0] - cam.cx) / cam.f).atan(),
                    };
                    self.world.glance(a);
                }
            }
            None => {
                if e.phase as u32 >= 2 {
                    self.finger = None;
                    self.world.aim = None;
                }
            }
        }
    }

    fn update(&mut self, _ctx: &mut Ctx, _t: f64, dt: f64) {
        let dt = dt as f32;
        self.time += dt;
        self.world.step(dt);
        for e in self.world.drain() {
            match e {
                Event::Received { by, first_time, grace } => {
                    let you = by == 0;
                    if you || grace > 0.8 {
                        self.graces.push((self.world.players[by].pos, 0.0, grace.max(0.3)));
                    }
                    let pan = (self.world.players[by].pos - self.cam_pos).dot(V2::from_angle(self.cam_angle).perp() * -1.0) / 15.0;
                    let pan = pan.clamp(-1.0, 1.0);
                    self.bells.strike(Note { freq: 92.0, amp: if you { 0.5 } else { 0.25 }, decay: 0.07, pan, bright: 4.0 });
                    self.bells.strike(Note { freq: if you { 196.0 } else { 147.0 }, amp: if you { 0.3 } else { 0.15 }, decay: 0.35, pan, bright: if first_time { 2.0 } else { 0.8 } });
                }
                Event::Pass { yours, open, .. } => {
                    self.bells.strike(Note { freq: 110.0, amp: if yours { 0.55 } else { 0.22 }, decay: 0.06, pan: 0.0, bright: 5.0 });
                    let f = if yours { 262.0 * (1.0 + open * 0.5) } else { 175.0 };
                    self.bells.strike(Note { freq: f, amp: if yours { 0.3 } else { 0.12 }, decay: 0.25, pan: 0.0, bright: 1.5 });
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
        // the camera rides your shoulder and turns with your head, a touch behind it
        let you = &self.world.players[0];
        self.cam_pos = self.cam_pos.lerp(you.pos, 1.0 - (-dt * 6.0).exp());
        self.cam_angle += angle_diff(you.facing, self.cam_angle) * (1.0 - (-dt * 9.0).exp());
        if matches!(self.world.ball, Ball::Rolling { .. }) {
            self.trail.push(self.world.ball_pos);
        } else if !self.trail.is_empty() {
            self.trail.remove(0);
        }
        if self.trail.len() > 14 {
            self.trail.remove(0);
        }
        for g in &mut self.graces {
            g.1 += dt;
        }
        self.graces.retain(|g| g.1 < 1.4);
        for b in &mut self.bursts {
            b.1 += dt;
        }
        self.bursts.retain(|b| b.1 < 1.0);
    }

    fn draw(&mut self, ctx: &mut Ctx, target: &wgpu::TextureView) {
        let cam = self.cam(ctx);
        let (wpx, hpx) = (ctx.width as f32, ctx.height as f32);
        let size = [ctx.width, ctx.height];
        let hold = self.hold();
        let mut enc = ctx.gpu.device.create_command_encoder(&Default::default());
        self.ground.render(
            &ctx.gpu,
            &mut enc,
            target,
            size,
            &BackdropParams {
                time: self.time,
                scale: cam.f,
                center: [cam.eye[0], cam.eye[1]],
                p: [cam.eye[2], cam.angle, cam.pitch, cam.cy, hold, self.world.cfg.half.x, 0.0, 0.0],
            },
        );
        let w = &self.world;
        let p = &mut self.painter;
        let k_at = |z: f32| cam.f / z;
        let haze = [0.42, 0.50, 0.50];

        // on the grass: shadows, the pass in your hand, rings
        let light = v2(0.35, -0.25);
        let shadow = |p: &mut Painter, at: V2, r: f32, a: f32| {
            if let Some((l, rr, hh)) = ground_ellipse(&cam, at + light, r) {
                p.line(l, rr, hh * 0.8, hh * 0.4 + 1.0, [0.0, 0.04, 0.02, a]);
            }
        };
        for pl in &w.players {
            let (at, a) = if pl.kind == Kind::Presser { (pl.seen, (1.0 - pl.seen_age / 2.0).clamp(0.0, 1.0) * 0.45) } else { (pl.pos, 0.45) };
            shadow(p, at, 0.34, a);
        }
        shadow(p, w.ball_pos, 0.13, 0.5);
        if let Some((dir, pace)) = w.aim {
            if let Some(o) = w.preview(dir * pace) {
                let from = w.ball_pos;
                let good = o.fate == Fate::Received;
                let c = if good { [1.0, 0.93, 0.7] } else { [1.0, 0.45, 0.38] };
                let mut prev = cam.project(from, 0.03);
                for i in 1..=40 {
                    let q = w.roll(from, dir * pace, o.t * i as f32 / 40.0).0;
                    let qq = cam.project(q, 0.03);
                    if let (Some((a, za)), Some((b, _))) = (prev, qq) {
                        let k = k_at(za);
                        let fade = 1.0 - 0.5 * i as f32 / 40.0;
                        p.glow(a, b, 0.05 * k, 0.12 * k + 1.0, [c[0], c[1], c[2], 0.6 * fade]);
                        p.glow(a, b, 0.2 * k, 0.4 * k, [c[0], c[1], c[2], 0.08 * fade]);
                    }
                    prev = qq;
                }
                ground_ring(p, &cam, o.at, 0.7, 0.05, [c[0], c[1], c[2], 0.8]);
            }
        }
        for (at, t, g) in &self.graces {
            let k = 1.0 - t / 1.4;
            ground_ring(p, &cam, *at, 0.6 + t * 1.8, 0.05, [1.0, 0.9, 0.65, 0.5 * k * g]);
            if let Some((a, b, hh)) = ground_ellipse(&cam, *at, 1.2) {
                p.glow(a, b, hh * 0.6, hh * 0.6, [1.0, 0.92, 0.7, 0.18 * k * g]);
            }
        }
        for (at, t) in &self.bursts {
            ground_ring(p, &cam, *at, 0.4 + t * 3.0, 0.08, [1.0, 0.55, 0.45, 0.7 * (1.0 - t)]);
        }
        for (i, q) in self.trail.iter().enumerate() {
            let k = i as f32 / self.trail.len() as f32;
            if let Some((s, z)) = cam.project(*q, 0.11) {
                p.glow(s, s, 0.08 * k_at(z) * k, 0.1 * k_at(z), [1.0, 1.0, 0.95, 0.25 * k]);
            }
        }
        p.render(&ctx.gpu, &mut enc, target, size, None);

        // the figures, far to near
        enum Fig {
            Body(usize),
            Ball,
        }
        let mut figs: Vec<(f32, Fig)> = Vec::new();
        for (i, pl) in w.players.iter().enumerate() {
            let at = if pl.kind == Kind::Presser { pl.seen } else { pl.pos };
            if let Some((_, z)) = cam.project(at, 0.9) {
                figs.push((z, Fig::Body(i)));
            }
        }
        if let Some((_, z)) = cam.project(w.ball_pos, 0.11) {
            figs.push((z - 0.3, Fig::Ball));
        }
        figs.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
        for (z, fig) in &figs {
            let fog = 1.0 - (-z / 75.0).exp();
            match fig {
                Fig::Ball => {
                    if let Some((s, z)) = cam.project(w.ball_pos, 0.11) {
                        let k = k_at(z);
                        p.disc(s, 0.11 * k, 0.02 * k + 0.8, [0.97, 0.97, 0.94, 1.0]);
                        p.disc([s[0] + 0.03 * k, s[1] + 0.04 * k], 0.06 * k, 0.05 * k, [0.7, 0.72, 0.75, 0.5]);
                        p.glow(s, s, 0.12 * k, 0.5 * k, [1.0, 1.0, 0.9, 0.12 + 0.25 * hold]);
                    }
                }
                Fig::Body(i) => {
                    let pl = &w.players[*i];
                    let (at, alpha, ghost) = match pl.kind {
                        Kind::Presser => {
                            let fresh = (1.0 - pl.seen_age / 0.5).clamp(0.0, 1.0);
                            let memory = (1.0 - pl.seen_age / 4.0).clamp(0.2, 1.0);
                            (pl.seen, if fresh > 0.0 { fresh.max(0.6) } else { memory * 0.6 }, fresh <= 0.0)
                        }
                        _ => (pl.pos, 1.0, false),
                    };
                    let (shirt, shorts, skin) = match pl.kind {
                        Kind::You => ([0.98, 0.94, 0.84], [0.16, 0.2, 0.32], [0.55, 0.4, 0.3]),
                        Kind::Mate => ([0.84, 0.9, 0.97], [0.16, 0.2, 0.32], [0.5, 0.38, 0.3]),
                        Kind::Presser => ([0.93, 0.42, 0.34], [0.12, 0.1, 0.12], [0.45, 0.32, 0.26]),
                    };
                    let fogged = |c: [f32; 3]| mix3(c, haze, fog * 0.8);
                    // the lean: a presser's body goes where it reads you
                    let lean = if pl.kind == Kind::Presser && !ghost { pl.lean * (pl.commit * 0.35) } else { pl.vel * 0.03 };
                    let (Some(foot_l), Some(foot_r), Some(hip), Some(chest), Some(sh), Some(head)) = (
                        cam.px(at - cam.rt * 0.12, 0.08),
                        cam.px(at + cam.rt * 0.12, 0.08),
                        cam.px(at + lean * 0.3, 0.85),
                        cam.px(at + lean * 0.6, 1.05),
                        cam.px(at + lean * 0.85, 1.38),
                        cam.project(at + lean, 1.66),
                    ) else {
                        continue;
                    };
                    let k = k_at(head.1);
                    if ghost {
                        // only remembered: an outline where you last saw it
                        let c = [0.95, 0.55, 0.48, alpha];
                        let ring = |a: [f32; 2], b: [f32; 2], r: f32| Shape { a, b, r, ring: 0.035 * k + 1.0, soft: 0.03 * k + 1.0, _pad: 0.0, color: pm(c) };
                        p.shapes_over.push(ring(chest, sh, 0.25 * k));
                        p.shapes_over.push(ring(head.0, head.0, 0.12 * k));
                        p.line(foot_l, hip, 0.03 * k, 0.03 * k, [c[0], c[1], c[2], alpha * 0.5]);
                        p.line(foot_r, hip, 0.03 * k, 0.03 * k, [c[0], c[1], c[2], alpha * 0.5]);
                        continue;
                    }
                    let sock = fogged([0.08, 0.08, 0.1]);
                    let soft = 0.03 * k + 0.8;
                    // a body right in front of the lens thins away rather than block the view
                    let a = alpha * (0.25 + 0.75 * ((head.1 - 1.6) / 1.4).clamp(0.0, 1.0));
                    let o = |c: [f32; 3]| [c[0], c[1], c[2], a];
                    p.line(foot_l, hip, 0.075 * k, soft, o(sock));
                    p.line(foot_r, hip, 0.075 * k, soft, o(sock));
                    p.line(hip, hip, 0.2 * k, soft, o(fogged(shorts)));
                    let (sl, sr) = ([sh[0] - 0.17 * k, sh[1] + 0.03 * k], [sh[0] + 0.17 * k, sh[1] + 0.03 * k]);
                    let swing = (self.time * 7.0 + *i as f32).sin() * pl.vel.len().min(4.0) * 0.012 * k;
                    p.line(sl, [sl[0] - 0.05 * k + swing, sl[1] + 0.55 * k], 0.055 * k, soft, o(fogged(skin)));
                    p.line(sr, [sr[0] + 0.05 * k - swing, sr[1] + 0.55 * k], 0.055 * k, soft, o(fogged(skin)));
                    p.line(chest, sh, 0.19 * k, soft, o(fogged(shirt)));
                    p.line(sl, sr, 0.065 * k, soft, o(fogged(shirt)));
                    // light from the upper left: a lit flank
                    let lit = [chest[0] - 0.06 * k, chest[1] - 0.02 * k];
                    let lit2 = [sh[0] - 0.08 * k, sh[1]];
                    p.line(lit, lit2, 0.08 * k, 0.1 * k, o(fogged(mix3(shirt, [1.0, 1.0, 1.0], 0.4))));
                    p.disc(head.0, 0.115 * k, soft, o(fogged(skin)));
                    p.disc([head.0[0] - 0.03 * k, head.0[1] - 0.04 * k], 0.07 * k, 0.06 * k, o(fogged(mix3(skin, [1.0, 0.9, 0.8], 0.3))));
                    if pl.kind == Kind::Presser && pl.stun > 0.0 {
                        p.glow_ring(head.0, 0.22 * k, 0.02 * k, 0.04 * k, [1.0, 0.7, 0.6, 0.5]);
                    }
                }
            }
        }
        // time held: the light narrows around what you are looking at
        if hold > 0.01 {
            let c = [wpx * 0.5, cam.cy];
            let r = hpx * (0.62 - 0.12 * hold);
            p.ring(c, r + hpx * 0.4, hpx * 0.8, hpx * 0.25, [0.0, 0.01, 0.03, 0.55 * hold]);
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

#[cfg(feature = "app")]
kit::ios_shell!(Eyes);
