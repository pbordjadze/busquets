//! Ghost (a working title): a real match on the engine. StatsBomb's open data (fetched at run time
//! from StatsBomb's own repository; nothing is kept here) is rebuilt into every player's path
//! (crates/real), and played back on a night pitch, from above or through any player's eyes,
//! Busquets' first. Every figure is honest about itself: solid where the data names them there, solid
//! but unnamed-sure where a 360 freeze-frame shows someone there, and a fading ghost where we only
//! infer it (the shape, the nearest real sightings). When a freeze-frame is taken the camera's real
//! dots flash on the grass: the truth underneath.
//!
//! Fingers: drag along the bottom to scrub time. From above, tap a player to see through their eyes;
//! tap the grass to pause or play. Through someone's eyes, swipe sideways for the next teammate, tap
//! to rise above. WHAT IF (top right) changes one thing (the default: Spain's line ten metres
//! higher) and lets the agents play on from this moment; the real match goes on beside it as faint
//! rings, a thread from each real player to your version of them. Tap it again for the next change,
//! and once more to go back to what happened.

use kit::backdrop::{Backdrop, BackdropParams};
use kit::paint::Painter;
use kit::{Ctx, Phase, PointerEvent, Toy};
use pitch::{v2, V2};
use real::fetch::{Got, Loader};
use real::play::{Sim, Traits, Tweak};
use real::{Game, Quality, Tracks, BUSQUETS};
use std::collections::HashMap;

/// Croatia 3-5 Spain, Euro 2020 round of 16 (Copenhagen, 28 June 2021): Busquets for 108 minutes,
/// with 360 freeze-frames.
pub const MATCH: u32 = 3794686;

const FIELD: &str = r#"
fn sdbox(p: vec2<f32>, b: vec2<f32>) -> f32 {
  let q = abs(p) - b;
  return length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0);
}
fn markings(w: vec2<f32>) -> f32 {
  let a = abs(w);
  var d = abs(sdbox(w, vec2<f32>(52.5, 34.0)));
  d = min(d, length(vec2<f32>(w.x, max(a.y - 34.0, 0.0))));
  d = min(d, abs(length(w) - 9.15));
  d = min(d, max(length(w) - 0.25, 0.0));
  d = min(d, abs(sdbox(vec2<f32>(a.x - 44.25, w.y), vec2<f32>(8.25, 20.16))));
  d = min(d, abs(sdbox(vec2<f32>(a.x - 49.75, w.y), vec2<f32>(2.75, 9.16))));
  d = min(d, max(length(vec2<f32>(a.x - 41.5, w.y)) - 0.2, 0.0));
  if (a.x < 36.0) { d = min(d, abs(length(vec2<f32>(a.x - 41.5, w.y)) - 9.15)); }
  let c = a - vec2<f32>(52.5, 34.0);
  if (c.x < 0.0 && c.y < 0.0) { d = min(d, abs(length(c) - 1.0)); }
  return d;
}
fn grass(w: vec2<f32>, fw: f32, dist: f32) -> vec3<f32> {
  let a = abs(w);
  let inside = 1.0 - smoothstep(0.0, 1.5, max(a.x - 54.5, a.y - 36.5));
  let stripe = select(0.0, 1.0, fract(w.x / 10.5) > 0.5);
  var c = mix(vec3<f32>(0.018, 0.035, 0.03), vec3<f32>(0.04, 0.12, 0.07) * (0.93 + 0.1 * stripe), inside);
  c = c * (0.9 + 0.12 * noise2(w * 0.7));
  // the floodlights' pools
  let pool = exp(-dot(w, w) / 2600.0);
  c = c * (0.8 + 0.4 * pool);
  let hw = 0.065;
  let d = markings(w);
  let cov = clamp((hw + fw - d) / (2.0 * fw), 0.0, 1.0) * min(1.0, 2.0 * hw / max(fw, 1e-4));
  c = mix(c, vec3<f32>(0.78, 0.84, 0.8), cov * 0.75 * inside);
  return c;
}
fn backdrop(px: vec2<f32>, world: vec2<f32>) -> vec4<f32> {
  let mode = u.p0.x;
  if (mode < 0.5) {
    // from above: p1 = (basis xx, xy, yx, yy) mapping screen offset (px) to world
    let o = (px - u.size * 0.5) / u.scale;
    let w = u.center + vec2<f32>(u.p1.x * o.x + u.p1.y * o.y, u.p1.z * o.x + u.p1.w * o.y);
    let fw = 0.8 / u.scale;
    var c = grass(w, fw, 0.0);
    let v = length(px / u.size - vec2<f32>(0.5)) * 1.2;
    c = c * (1.0 - 0.35 * v * v);
    return vec4<f32>(c, 1.0);
  }
  // through someone's eyes: p0 = (1, eye x, eye y, eye z), p1 = (yaw, tilt, focal px, 0)
  let yaw = u.p1.x; let tilt = u.p1.y; let focal = u.p1.z;
  let f2 = vec2<f32>(cos(yaw), sin(yaw));
  let fwd = vec3<f32>(f2 * cos(tilt), sin(tilt));
  let up = vec3<f32>(-f2 * sin(tilt), cos(tilt));
  let right = vec3<f32>(f2.y, -f2.x, 0.0);
  let dx = (px.x - u.size.x * 0.5) / focal;
  let dy = -(px.y - u.size.y * 0.5) / focal;
  let ray = fwd + right * dx + up * dy;
  let eye = u.p0.yzw;
  let horizon = ray.z / length(ray);
  // the ground hit, and its footprint, before any branch on the pixel (fwidth wants uniform flow)
  let t = -eye.z / min(ray.z, -0.002);
  let w = eye.xy + ray.xy * t;
  let fw = length(fwidth(w)) * 0.7;
  if (ray.z > -0.002) {
    // the night: deep blue, a haze of floodlight at the horizon
    let h = clamp(horizon, 0.0, 1.0);
    var c = mix(vec3<f32>(0.06, 0.08, 0.12), vec3<f32>(0.008, 0.012, 0.03), pow(h, 0.45));
    // the stands: a dark band just above the horizon, speckled
    let az = atan2(ray.y, ray.x);
    let stand = (1.0 - smoothstep(0.05, 0.11, h)) * smoothstep(-0.01, 0.01, h);
    let crowd = noise2(vec2<f32>(az * 220.0, h * 900.0));
    c = mix(c, vec3<f32>(0.03, 0.035, 0.05) + crowd * vec3<f32>(0.05, 0.045, 0.04), stand);
    // floodlights: four soft suns at the corners
    for (var k = 0; k < 4; k = k + 1) {
      let corner = vec2<f32>(select(-62.0, 62.0, (k & 1) == 1), select(-46.0, 46.0, (k & 2) == 2));
      let dir = normalize(vec3<f32>(corner - eye.xy, 38.0 - eye.z));
      let g = max(dot(normalize(ray), dir), 0.0);
      c = c + vec3<f32>(1.0, 0.95, 0.85) * (pow(g, 6000.0) * 1.2 + pow(g, 300.0) * 0.12 + pow(g, 20.0) * 0.03);
    }
    return vec4<f32>(c, 1.0);
  }
  var c = grass(w, fw, t);
  // fog into the dark with distance
  let fog = 1.0 - exp(-t / 140.0);
  c = mix(c, vec3<f32>(0.035, 0.045, 0.06), fog * 0.7);
  return vec4<f32>(c, 1.0);
}
"#;

const WARM: [f32; 3] = [0.98, 0.83, 0.55];
const COOL: [f32; 3] = [0.55, 0.68, 0.92];
const EYE: f32 = 1.72;
const TILT: f32 = -0.07;

struct Loaded {
    game: Game,
    tracks: Tracks,
    busquets: Option<usize>,
}

enum State {
    Loading,
    Ready(Box<Loaded>),
    Failed(String),
}

/// The changes WHAT IF offers, in turn.
const TWEAKS: [(&str, Tweak); 2] = [
    ("SPAIN'S LINE +10 M", Tweak { line: [10.0, 0.0], press: [1.0, 1.0] }),
    ("CROATIA PRESS HIGH", Tweak { line: [0.0, 8.0], press: [1.0, 1.6] }),
];

struct Finger {
    start: [f64; 2],
    last: [f64; 2],
    t0: f64,
    scrub: bool,
}

pub struct Ghost {
    loader: Loader,
    state: State,
    painter: Painter,
    field: Backdrop,
    /// Match time, s.
    t: f32,
    playing: bool,
    pov: Option<usize>,
    cam_yaw: f32,
    cam_set: bool,
    fingers: HashMap<u64, Finger>,
    what_if: usize,
    sim: Option<Sim>,
    clock: f32,
    traits_for: fn(&Game, usize) -> Traits,
}

fn generic(_: &Game, _: usize) -> Traits {
    Traits::default()
}

/// The camera through someone's eyes.
#[derive(Clone, Copy)]
struct Cam {
    eye: V2,
    yaw: f32,
    focal: f32,
    w: f32,
    h: f32,
}

impl Cam {
    fn basis(&self) -> ([f32; 3], [f32; 3], [f32; 3]) {
        let (c, s) = (self.yaw.cos(), self.yaw.sin());
        let (ct, st) = (TILT.cos(), TILT.sin());
        ([c * ct, s * ct, st], [-c * st, -s * st, ct], [s, -c, 0.0])
    }
    /// A world point (x, y, z) to the screen, and its depth; None behind the eye.
    fn project(&self, p: V2, z: f32) -> Option<([f32; 2], f32)> {
        let (f, u, r) = self.basis();
        let d = [p.x - self.eye.x, p.y - self.eye.y, z - EYE];
        let zc = d[0] * f[0] + d[1] * f[1] + d[2] * f[2];
        if zc < 0.4 {
            return None;
        }
        let xr = d[0] * r[0] + d[1] * r[1];
        let yu = d[0] * u[0] + d[1] * u[1] + d[2] * u[2];
        Some(([self.w * 0.5 + self.focal * xr / zc, self.h * 0.5 - self.focal * yu / zc], zc))
    }
}

/// From above: where a world point lands on the screen.
#[derive(Clone, Copy)]
struct TopView {
    w: f32,
    cy: f32,
    s: f32,
    portrait: bool,
}

impl TopView {
    fn screen(&self, d: V2) -> [f32; 2] {
        if self.portrait {
            [self.w * 0.5 - d.y * self.s, self.cy - d.x * self.s]
        } else {
            [self.w * 0.5 + d.x * self.s, self.cy - d.y * self.s]
        }
    }
}

fn rgba(c: [f32; 3], a: f32) -> [f32; 4] {
    [c[0], c[1], c[2], a]
}

/// Writes `text` in dots, `cell` px a dot step, from `at` (top left), or centred on it.
fn write(p: &mut Painter, text: &str, at: [f32; 2], cell: f32, color: [f32; 4], centre: bool) -> f32 {
    let (dots, w) = real::dots::layout(text);
    let x0 = if centre { at[0] - w * cell * 0.5 } else { at[0] };
    for d in dots {
        p.disc([x0 + d[0] * cell, at[1] + d[1] * cell], cell * 0.42, cell * 0.25, color);
    }
    w * cell
}

impl Ghost {
    fn points(&self, ctx: &Ctx) -> (f32, f32, f32) {
        (ctx.width as f32, ctx.height as f32, ctx.ppp as f32)
    }

    /// From above: the pitch's long side along the screen's long side; returns (centre, scale px/m,
    /// portrait).
    fn top(&self, ctx: &Ctx) -> (V2, f32, bool) {
        let (w, h, ppp) = self.points(ctx);
        let portrait = h > w;
        let (long, short) = if portrait { (h - 150.0 * ppp, w - 24.0 * ppp) } else { (w - 48.0 * ppp, h - 160.0 * ppp) };
        let s = (long / 110.0).min(short / 72.0);
        (V2::ZERO, s, portrait)
    }

    fn top_view(&self, ctx: &Ctx) -> TopView {
        let (w, h, ppp) = self.points(ctx);
        let (_, s, portrait) = self.top(ctx);
        TopView { w, cy: h * 0.5 - 18.0 * ppp, s, portrait }
    }

    fn to_screen(&self, ctx: &Ctx, q: V2) -> [f32; 2] {
        self.top_view(ctx).screen(q)
    }

    fn bar(&self, ctx: &Ctx) -> (f32, f32, f32) {
        let (w, h, ppp) = self.points(ctx);
        (28.0 * ppp, w - 28.0 * ppp, h - 46.0 * ppp)
    }

    fn cam(&self, ctx: &Ctx, eye: V2) -> Cam {
        let (w, h, _) = self.points(ctx);
        let focal = (w * 0.5 / 27f32.to_radians().tan()).min(h * 0.5 / 30f32.to_radians().tan());
        Cam { eye, yaw: self.cam_yaw, focal, w, h }
    }

    fn start_sim(&mut self) {
        let State::Ready(l) = &self.state else { return };
        let tw = TWEAKS[(self.what_if - 1) % TWEAKS.len()].1;
        let tf = self.traits_for;
        let g = &l.game;
        self.sim = Some(Sim::from_moment(g, &l.tracks, self.t, tw, &|p| tf(g, p), 11));
    }

    /// Where a person stands now: in your version when one is playing, else the real one.
    fn sim_pos(&self, person: usize) -> Option<V2> {
        let s = self.sim.as_ref()?;
        s.agents.iter().find(|a| a.person == person).map(|a| a.pos)
    }

    fn pov_order(&self, l: &Loaded, side: u8) -> Vec<usize> {
        let mut v: Vec<(usize, f32)> = l.game.people.iter().enumerate().filter(|(_, p)| p.side == side && p.playing(self.t)).map(|(i, p)| (i, real::track::slot(p.position_at(self.t)).y)).collect();
        v.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        v.into_iter().map(|x| x.0).collect()
    }

    fn tap(&mut self, ctx: &Ctx, x: f32, y: f32) {
        let (w, _h, ppp) = self.points(ctx);
        // WHAT IF
        if x > w - 150.0 * ppp && y < 90.0 * ppp {
            self.what_if = (self.what_if + 1) % (TWEAKS.len() + 1);
            if self.what_if == 0 {
                self.sim = None;
            } else {
                self.start_sim();
                self.playing = true;
            }
            return;
        }
        if self.pov.is_some() {
            self.pov = None;
            return;
        }
        let State::Ready(l) = &self.state else { return };
        let mut best = (None, 30.0 * ppp);
        for (i, p) in l.game.people.iter().enumerate() {
            if !p.playing(self.t) {
                continue;
            }
            let at = self.sim_pos(i).unwrap_or_else(|| l.tracks.pos_at(i, self.t));
            let s = self.to_screen(ctx, at);
            let d = ((s[0] - x).powi(2) + (s[1] - y).powi(2)).sqrt();
            if d < best.1 {
                best = (Some(i), d);
            }
        }
        if let Some(i) = best.0 {
            self.pov = Some(i);
            self.cam_set = false;
        } else {
            self.playing = !self.playing;
        }
    }
}

impl Toy for Ghost {
    fn new(ctx: &mut Ctx) -> anyhow::Result<Ghost> {
        let loader = Loader::new();
        loader.ask(&format!("events/{MATCH}.json"));
        loader.ask(&format!("three-sixty/{MATCH}.json"));
        Ok(Ghost {
            loader,
            state: State::Loading,
            painter: Painter::new(&ctx.gpu),
            field: Backdrop::new(&ctx.gpu, "ghost-field", FIELD),
            t: 0.0,
            playing: true,
            pov: None,
            cam_yaw: 0.0,
            cam_set: false,
            fingers: HashMap::new(),
            what_if: 0,
            sim: None,
            clock: 0.0,
            traits_for: generic,
        })
    }

    fn pointer(&mut self, ctx: &mut Ctx, e: PointerEvent) {
        let ppp = ctx.ppp as f32;
        let (x, y) = (e.x as f32 * ppp, e.y as f32 * ppp);
        let (x0, x1, by) = self.bar(ctx);
        let end = match &self.state {
            State::Ready(l) => l.game.end,
            _ => 1.0,
        };
        match e.phase {
            Phase::Down => {
                let scrub = y > by - 30.0 * ppp;
                self.fingers.insert(e.id, Finger { start: [x as f64, y as f64], last: [x as f64, y as f64], t0: e.t, scrub });
                if scrub {
                    self.t = ((x - x0) / (x1 - x0)).clamp(0.0, 1.0) * end;
                    self.sim = None;
                }
            }
            Phase::Move => {
                if let Some(f) = self.fingers.get_mut(&e.id) {
                    f.last = [x as f64, y as f64];
                    if f.scrub {
                        self.t = ((x - x0) / (x1 - x0)).clamp(0.0, 1.0) * end;
                        self.sim = None;
                        self.what_if = 0;
                    }
                }
            }
            Phase::Up | Phase::Cancel => {
                let Some(f) = self.fingers.remove(&e.id) else { return };
                if f.scrub || e.phase == Phase::Cancel {
                    return;
                }
                let (dx, dy) = (x - f.start[0] as f32, y - f.start[1] as f32);
                let quick = e.t - f.t0 < 0.45;
                if dx.abs() < 12.0 * ppp && dy.abs() < 12.0 * ppp && quick {
                    self.tap(ctx, x, y);
                } else if dx.abs() > 50.0 * ppp && dx.abs() > dy.abs() * 1.5 {
                    // through someone's eyes: the next teammate along
                    if let (Some(p), State::Ready(l)) = (self.pov, &self.state) {
                        let order = self.pov_order(l, l.game.people[p].side);
                        if let Some(k) = order.iter().position(|&q| q == p) {
                            let n = order.len();
                            let next = if dx < 0.0 { order[(k + 1) % n] } else { order[(k + n - 1) % n] };
                            self.pov = Some(next);
                            self.cam_set = false;
                        }
                    }
                }
            }
        }
    }

    fn update(&mut self, ctx: &mut Ctx, _t: f64, dt: f64) {
        let dt = dt as f32;
        self.clock += dt;
        if let State::Loading = self.state {
            let ev = self.loader.get(&format!("events/{MATCH}.json"));
            let fr = self.loader.get(&format!("three-sixty/{MATCH}.json"));
            match (ev, fr) {
                (Got::Failed(e), _) => self.state = State::Failed(e),
                (Got::Bytes(ev), Got::Bytes(fr)) => {
                    self.state = match Game::read(&ev, Some(&fr), BUSQUETS) {
                        Ok(game) => {
                            let tracks = Tracks::build(&game);
                            let busquets = game.find(BUSQUETS);
                            self.pov = busquets;
                            self.cam_set = false;
                            // a few seconds before his first touch
                            if let Some(b) = busquets {
                                if let Some(e) = game.events.iter().find(|e| e.who == Some(b) && e.kind.on_ball()) {
                                    self.t = (e.t - 6.0).max(0.0);
                                }
                            }
                            State::Ready(Box::new(Loaded { game, tracks, busquets }))
                        }
                        Err(e) => State::Failed(format!("{e:#}")),
                    };
                    self.loader.forget(&format!("events/{MATCH}.json"));
                    self.loader.forget(&format!("three-sixty/{MATCH}.json"));
                }
                (Got::Bytes(_), Got::Failed(_)) => {
                    // no 360 for this match: events only
                    if let Got::Bytes(ev) = self.loader.get(&format!("events/{MATCH}.json")) {
                        self.state = match Game::read(&ev, None, BUSQUETS) {
                            Ok(game) => {
                                let tracks = Tracks::build(&game);
                                let busquets = game.find(BUSQUETS);
                                self.pov = busquets;
                                State::Ready(Box::new(Loaded { game, tracks, busquets }))
                            }
                            Err(e) => State::Failed(format!("{e:#}")),
                        };
                    }
                }
                _ => {}
            }
            return;
        }
        let State::Ready(l) = &self.state else { return };
        if self.playing {
            self.t = (self.t + dt).min(l.game.end);
            if let Some(s) = &mut self.sim {
                // the agents keep the same clock as the ghost
                let steps = (dt / 0.02).ceil().max(1.0) as usize;
                for _ in 0..steps {
                    s.step(dt / steps as f32);
                }
            }
        }
        // the camera turns toward the ball, a little late, as a head does
        if let Some(p) = self.pov {
            let (me, ball) = match &self.sim {
                Some(s) => (self.sim_pos(p).unwrap_or(V2::ZERO), s.ball_pos),
                None => (l.tracks.pos_at(p, self.t), l.tracks.ball_at(self.t)),
            };
            let to = ball - me;
            let want = if to.len() > 1.5 { to.angle() } else { l.tracks.heading(p, self.t, ball).angle() };
            if !self.cam_set {
                self.cam_yaw = want;
                self.cam_set = true;
            }
            let k = 1.0 - (-dt / 0.35).exp();
            self.cam_yaw += pitch::v2::angle_diff(want, self.cam_yaw) * k;
        }
        let _ = ctx;
    }

    fn draw(&mut self, ctx: &mut Ctx, target: &wgpu::TextureView) {
        let (w, h, ppp) = self.points(ctx);
        let size = [ctx.width, ctx.height];
        let mut enc = ctx.gpu.device.create_command_encoder(&Default::default());
        let (_, top_s, portrait) = self.top(ctx);
        let basis = if portrait { [0.0, -1.0, -1.0, 0.0] } else { [1.0, 0.0, 0.0, -1.0] };

        let loaded = match &self.state {
            State::Ready(l) => Some(l),
            _ => None,
        };
        let pov_eye = self.pov.and_then(|p| loaded.map(|l| self.sim_pos(p).unwrap_or_else(|| l.tracks.pos_at(p, self.t))));
        let cam = pov_eye.map(|e| self.cam(ctx, e));
        // the field
        let params = match cam {
            Some(c) => BackdropParams { time: self.clock, scale: 1.0, center: [0.0, 0.0], p: [1.0, c.eye.x, c.eye.y, EYE, c.yaw, TILT, c.focal, 0.0] },
            None => {
                // centre the pitch where to_screen puts it
                let off = self.to_screen(ctx, V2::ZERO);
                let o = [(w * 0.5 - off[0]) / top_s, (h * 0.5 - off[1]) / top_s];
                let centre = [basis[0] * o[0] + basis[1] * o[1], basis[2] * o[0] + basis[3] * o[1]];
                BackdropParams { time: self.clock, scale: top_s, center: centre, p: [0.0, 0.0, 0.0, 0.0, basis[0], basis[1], basis[2], basis[3]] }
            }
        };
        self.field.render(&ctx.gpu, &mut enc, target, size, &params);

        let Some(l) = loaded else {
            let p = &mut self.painter;
            let msg = match &self.state {
                State::Failed(e) => format!("NO DATA: {}", e.chars().take(40).collect::<String>()),
                _ => "LOADING A REAL MATCH".to_string(),
            };
            let pulse = 0.5 + 0.5 * (self.clock * 3.0).sin();
            p.glow([w * 0.5, h * 0.45], [w * 0.5, h * 0.45], 8.0 * ppp, 30.0 * ppp, [1.0, 0.95, 0.85, 0.15 + 0.25 * pulse]);
            p.disc([w * 0.5, h * 0.45], 5.0 * ppp, 1.0 * ppp, [1.0, 1.0, 0.97, 1.0]);
            write(p, &msg, [w * 0.5, h * 0.52], 2.2 * ppp, [0.85, 0.88, 0.9, 0.8], true);
            write(p, "STATSBOMB OPEN DATA", [w * 0.5, h * 0.52 + 26.0 * ppp], 1.6 * ppp, [0.6, 0.65, 0.7, 0.6], true);
            p.render(&ctx.gpu, &mut enc, target, size, None);
            ctx.gpu.queue.submit([enc.finish()]);
            return;
        };
        let t = self.t;
        let g = &l.game;
        let tr = &l.tracks;
        let sim = self.sim.as_ref();
        let ball = sim.map(|s| s.ball_pos).unwrap_or_else(|| tr.ball_at(t));
        let ball_z = if sim.is_some() { 0.0 } else { tr.ball_z_at(t) };
        // the freeze-frame nearest now (shown for 0.6 s after it is taken)
        let frame = {
            let i = g.event_at(t);
            (i.saturating_sub(3)..(i + 1).min(g.events.len())).rev().find_map(|k| g.events[k].frame.map(|f| (f, t - g.events[k].t))).filter(|(_, age)| *age >= 0.0 && *age < 0.6)
        };
        let tv = self.top_view(ctx);

        struct Fig {
            at: V2,
            side: u8,
            solid: f32,
            busquets: bool,
            pov: bool,
            real_at: Option<V2>,
        }
        let mut figs: Vec<Fig> = vec![];
        for (i, pe) in g.people.iter().enumerate() {
            if !pe.playing(t) {
                continue;
            }
            let real_at = tr.pos_at(i, t);
            let q = tr.quality_at(i, t);
            let c = tr.conf_at(i, t);
            // solid at a real sighting, fading toward a ghost as it gets older
            let solid = match q {
                Quality::Observed | Quality::Seen => 1.0,
                _ => (0.2 + 0.85 * c * c).min(1.0),
            };
            let (at, solid, ghost) = match sim.and_then(|_| self.sim_pos(i)) {
                Some(s) => (s, 1.0, Some(real_at)),
                None => (real_at, solid, None),
            };
            figs.push(Fig { at, side: pe.side, solid, busquets: Some(i) == l.busquets, pov: Some(i) == self.pov, real_at: ghost });
        }

        let (x0, x1, by) = self.bar(ctx);
        let (playing, pov, what_if, clock) = (self.playing, self.pov, self.what_if, self.clock);
        let p = &mut self.painter;
        match cam {
            Some(cam) => {
                // through someone's eyes: far to near
                let mut order: Vec<(f32, usize)> = figs.iter().enumerate().filter(|(_, f)| !f.pov).filter_map(|(k, f)| cam.project(f.at, 0.0).map(|(_, z)| (z, k))).collect();
                order.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
                // the goals
                for sx in [-52.5f32, 52.5] {
                    let posts = [v2(sx, -3.66), v2(sx, 3.66)];
                    let mut top = vec![];
                    for post in posts {
                        if let (Some((a, z)), Some((b, _))) = (cam.project(post, 0.0), cam.project(post, 2.44)) {
                            p.line(a, b, (0.07 * cam.focal / z).max(0.6), 0.6, [0.9, 0.92, 0.95, 0.85]);
                            top.push((b, z));
                        }
                    }
                    if top.len() == 2 {
                        p.line(top[0].0, top[1].0, (0.07 * cam.focal / top[0].1).max(0.6), 0.6, [0.9, 0.92, 0.95, 0.85]);
                    }
                }
                // the camera's real dots, flashing on the grass
                if let (Some((fi, age)), None) = (frame, sim) {
                    let a = (1.0 - age / 0.6).max(0.0);
                    for d in &g.frames[fi].dots {
                        if let Some((s, z)) = cam.project(d.at, 0.0) {
                            let col = if d.side == 0 { WARM } else { COOL };
                            let r = (0.35 * cam.focal / z).clamp(1.5 * ppp, 14.0 * ppp);
                            p.glow([s[0] - r * 2.0, s[1]], [s[0] + r * 2.0, s[1]], r * 0.3, r, rgba(col, 0.4 * a));
                        }
                    }
                }
                // ghosts of the real match, when you are playing your version
                for f in figs.iter().filter(|f| f.real_at.is_some()) {
                    let ra = f.real_at.unwrap();
                    if let (Some((a, z)), Some((b, _))) = (cam.project(ra, 0.3), cam.project(ra, 1.6)) {
                        let col = if f.side == 0 { WARM } else { COOL };
                        p.line(a, b, 0.24 * cam.focal / z, 0.2 * cam.focal / z, rgba(col, 0.18));
                    }
                }
                for (z, k) in order {
                    let f = &figs[k];
                    let col = if f.side == 0 { WARM } else { COOL };
                    let s = cam.focal / z;
                    let a = f.solid;
                    if let (Some((foot, _)), Some((hip, _)), Some((head, _))) = (cam.project(f.at, 0.02), cam.project(f.at, 0.9), cam.project(f.at, 1.68)) {
                        p.disc(foot, 0.45 * s, 0.35 * s, [0.0, 0.0, 0.0, 0.35 * a]);
                        // legs, body, head
                        p.line([foot[0] - 0.09 * s, foot[1]], [hip[0] - 0.08 * s, hip[1]], 0.08 * s, 0.03 * s, rgba([0.12, 0.13, 0.16], a));
                        p.line([foot[0] + 0.09 * s, foot[1]], [hip[0] + 0.08 * s, hip[1]], 0.08 * s, 0.03 * s, rgba([0.12, 0.13, 0.16], a));
                        let chest = cam.project(f.at, 1.38).map(|x| x.0).unwrap_or(head);
                        p.line(hip, chest, 0.21 * s, 0.05 * s, rgba(col, a));
                        p.disc(head, 0.12 * s, 0.03 * s, rgba([0.86, 0.72, 0.6], a));
                        if f.busquets {
                            p.glow(chest, chest, 0.4 * s, 1.2 * s, [1.0, 0.95, 0.8, 0.12 * a]);
                        }
                    }
                }
                // the ball
                if let Some((s, z)) = cam.project(ball, ball_z + 0.11) {
                    if let Some((sh, _)) = cam.project(ball, 0.0) {
                        p.disc(sh, 0.16 * cam.focal / z, 0.1 * cam.focal / z, [0.0, 0.0, 0.0, 0.4]);
                    }
                    let r = (0.11 * cam.focal / z).max(1.4 * ppp);
                    p.glow(s, s, r, r * 3.0, [1.0, 1.0, 0.9, 0.25]);
                    p.disc(s, r, r * 0.25, [1.0, 1.0, 0.97, 1.0]);
                }
            }
            None => {
                let s = top_s;
                // the camera's view, faint, and its real dots
                if let (Some((fi, age)), None) = (frame, sim) {
                    let a = (1.0 - age / 0.6).max(0.0);
                    let fr = &g.frames[fi];
                    for k in 0..fr.visible.len() {
                        let (pa, pb) = (fr.visible[k], fr.visible[(k + 1) % fr.visible.len()]);
                        let (pa, pb) = (real::track::clamp_pitch(pa), real::track::clamp_pitch(pb));
                        p.line(tv.screen(pa), tv.screen(pb), 0.08 * s, 0.1 * s, [0.8, 0.85, 1.0, 0.08 * a]);
                    }
                    for d in &fr.dots {
                        let c = tv.screen(d.at);
                        let col = if d.side == 0 { WARM } else { COOL };
                        p.glow(c, c, 0.5 * s, 1.6 * s, rgba(col, 0.3 * a));
                    }
                }
                for f in &figs {
                    let c = tv.screen(f.at);
                    let col = if f.side == 0 { WARM } else { COOL };
                    if let Some(ra) = f.real_at {
                        let rc = tv.screen(ra);
                        p.line(rc, c, 0.06 * s, 0.1 * s, rgba(col, 0.18));
                        p.ring(rc, 0.85 * s, 0.1 * s, 0.1 * s, rgba(col, 0.35));
                    }
                    p.disc([c[0] + 0.2 * s, c[1] + 0.25 * s], 1.0 * s, 0.5 * s, [0.0, 0.0, 0.0, 0.3 * f.solid]);
                    // a filled disc as much as we know they were there, a ring for the rest
                    p.disc(c, 0.95 * s, 0.14 * s, rgba(col, f.solid * f.solid));
                    p.ring(c, 0.9 * s, 0.14 * s, 0.12 * s, rgba(col, 0.35 + 0.6 * f.solid));
                    if f.busquets {
                        let br = 0.5 + 0.5 * (clock * 2.0).sin();
                        p.glow_ring(c, (1.6 + 0.2 * br) * s, 0.08 * s, 0.4 * s, [1.0, 0.95, 0.8, 0.45]);
                    }
                    if f.pov {
                        p.ring(c, 1.5 * s, 0.12 * s, 0.1 * s, [1.0, 1.0, 1.0, 0.8]);
                    }
                }
                let b = tv.screen(ball);
                let lift = ball_z * 0.35 * s;
                p.disc([b[0] + 0.15 * s + lift * 0.6, b[1] + 0.2 * s + lift], 0.4 * s, 0.25 * s, [0.0, 0.0, 0.0, 0.4]);
                p.glow(b, b, 0.4 * s, 1.5 * s, [1.0, 1.0, 0.9, 0.3]);
                p.disc(b, (0.42 + ball_z * 0.03) * s, 0.08 * s, [1.0, 1.0, 0.97, 1.0]);
            }
        }

        // the words: the clock and score, whose eyes, the credit
        let (mm, ss) = g.clock(t);
        let sc = g.score(t);
        let cell = 2.0 * ppp;
        let top_y = 26.0 * ppp;
        let ink = [0.92, 0.94, 0.96, 0.9];
        let dim = [0.7, 0.75, 0.8, 0.55];
        let xw = write(p, &format!("{mm:02}:{ss:02}"), [20.0 * ppp, top_y], cell, ink, false);
        let score = format!("{} {}-{} {}", short_team(&g.teams[1]), sc[1], sc[0], short_team(&g.teams[0]));
        write(p, &score, [20.0 * ppp + xw + 14.0 * ppp, top_y + 2.0 * ppp], cell * 0.75, dim, false);
        let who = match pov {
            Some(pv) => format!("{} SEES", g.people[pv].short),
            None => "FROM ABOVE".to_string(),
        };
        write(p, &who, [20.0 * ppp, top_y + 22.0 * ppp], cell * 0.8, if pov == l.busquets && pov.is_some() { [1.0, 0.92, 0.75, 0.95] } else { dim }, false);
        if !playing {
            write(p, "PAUSED", [20.0 * ppp, top_y + 40.0 * ppp], cell * 0.7, dim, false);
        }
        // WHAT IF
        let label = if what_if == 0 { "WHAT IF".to_string() } else { TWEAKS[(what_if - 1) % TWEAKS.len()].0.to_string() };
        let bx = w - 20.0 * ppp;
        let top_y = top_y + 22.0 * ppp;
        let (dots, lw) = real::dots::layout(&label);
        let c2 = cell * 0.7;
        let wx0 = bx - lw * c2;
        let on = what_if > 0;
        p.line([wx0 - 10.0 * ppp, top_y + 3.0 * c2], [bx + 6.0 * ppp, top_y + 3.0 * c2], 9.0 * ppp, 1.0 * ppp, if on { [1.0, 0.9, 0.7, 0.22] } else { [1.0, 1.0, 1.0, 0.08] });
        for d in dots {
            p.disc([wx0 + d[0] * c2, top_y + d[1] * c2], c2 * 0.42, c2 * 0.25, if on { [1.0, 0.93, 0.78, 1.0] } else { ink });
        }
        if sim.is_some() {
            write(p, "RINGS: WHAT HAPPENED", [bx - 21.0 * 6.0 * cell * 0.55, top_y + 22.0 * ppp], cell * 0.55, dim, false);
        }
        // the time bar
        let u = t / g.end;
        p.line([x0, by], [x1, by], 1.2 * ppp, 0.6 * ppp, [1.0, 1.0, 1.0, 0.16]);
        p.line([x0, by], [x0 + (x1 - x0) * u, by], 1.4 * ppp, 0.6 * ppp, [1.0, 0.95, 0.85, 0.7]);
        for &(a, _) in &g.periods[1..] {
            let x = x0 + (x1 - x0) * a / g.end;
            p.disc([x, by], 2.0 * ppp, 0.5 * ppp, [0.0, 0.0, 0.0, 0.8]);
        }
        for &(gt, side) in &g.goals {
            let x = x0 + (x1 - x0) * gt / g.end;
            p.disc([x, by - 8.0 * ppp], 2.4 * ppp, 0.8 * ppp, rgba(if side == 0 { WARM } else { COOL }, 0.95));
        }
        p.glow([x0 + (x1 - x0) * u, by], [x0 + (x1 - x0) * u, by], 4.0 * ppp, 8.0 * ppp, [1.0, 0.95, 0.85, 0.5]);
        p.disc([x0 + (x1 - x0) * u, by], 4.5 * ppp, 0.8 * ppp, [1.0, 0.97, 0.9, 1.0]);
        write(p, "DATA: STATSBOMB OPEN DATA. FADED = INFERRED", [w * 0.5, by + 14.0 * ppp], 1.15 * ppp, [0.65, 0.7, 0.75, 0.55], true);
        p.render(&ctx.gpu, &mut enc, target, size, None);
        ctx.gpu.queue.submit([enc.finish()]);
    }
}

fn short_team(name: &str) -> String {
    name.split_whitespace().next().unwrap_or(name).to_uppercase()
}

kit::ios_shell!(Ghost);
