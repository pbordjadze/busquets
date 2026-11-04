//! The Hole (a working title): a real moment with one player taken out.
//!
//! A StatsBomb 360 freeze-frame (fetched at run time from StatsBomb's own repository; nothing is
//! kept here) of Busquets' team at the instant he played a pass, every visible player where the
//! camera saw them, except him. Where he stood the shape strains: the grass puckers and the nearest
//! teammates tremble. Drag a dot into the shape and it breathes out as you come to his pocket (the
//! threads to your teammates warm, a rising note); lift and he appears where he really was, how far
//! you were, and the pass he played from there. Tap for the next moment; drag again to try again.
//! Nothing is scored and nothing ends.
//!
//! The moments are picked by hand (Croatia 3-5 Spain, Euro 2020, the same match Ghost replays): his
//! completed passes in the middle of the pitch, with most of both teams in the camera's view.

use kit::backdrop::{Backdrop, BackdropParams};
use kit::info::{Demo, Rules, Stroke};
use kit::paint::Painter;
use kit::sound::{pentatonic, Bells, Note};
use kit::{Ctx, Phase, PointerEvent, Synth, Toy};
use pitch::v2::smoothstep;
use pitch::{v2, V2};
use real::fetch::{Got, Loader};
use real::sb;

/// The match (Ghost's): Croatia 3-5 Spain, Euro 2020 round of 16.
pub const MATCH: u32 = 3794686;

/// The moments: events of his (passes, completed) with a 360 frame, chosen by hand.
const MOMENTS: [&str; 7] = [
    "3512c880-a673-4915-ae72-191c0127a791",
    "b47467fb-3bb2-489a-9b34-bf86ca84faa9",
    "038053c5-03ce-47b7-a7a6-49741226fbeb",
    "631a0156-1dce-48f7-ac58-9da9acfa8907",
    "ebe77a31-4c38-4eeb-a4f8-c84a69dfe8c4",
    "c1fefa57-183a-4e3f-88d2-dcd9155f6830",
    "ef5dfee9-f5f9-46d7-9f6d-ca704fcc0f11",
];

/// How near (m) counts as his pocket: the breath is fully out within about this.
const NEAR: f32 = 7.0;
/// The pitch width shown, m.
const SHOWN: f32 = 44.0;

const PITCH: &str = r#"
fn sdbox(p: vec2<f32>, b: vec2<f32>) -> f32 {
  let q = abs(p) - b;
  return length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0);
}
fn markings(w: vec2<f32>) -> f32 {
  let a = abs(w);
  var d = abs(sdbox(w, vec2<f32>(52.5, 34.0)));
  d = min(d, length(vec2<f32>(w.x, max(a.y - 34.0, 0.0))));
  d = min(d, abs(length(w) - 9.15));
  d = min(d, abs(sdbox(vec2<f32>(a.x - 44.25, w.y), vec2<f32>(8.25, 20.16))));
  d = min(d, abs(sdbox(vec2<f32>(a.x - 49.75, w.y), vec2<f32>(2.75, 9.16))));
  if (a.x < 36.0) { d = min(d, abs(length(vec2<f32>(a.x - 41.5, w.y)) - 9.15)); }
  return d;
}
fn backdrop(px: vec2<f32>, unused: vec2<f32>) -> vec4<f32> {
  // p0 = (centre x, centre y (pitch metres), px a metre, strain 0..1); p1 = (hole x, hole y (metres), time, exhale)
  let s = u.p0.z;
  let o = (px - u.size * 0.5) / s;
  // the screen's up is the attack (+x), its right is +y
  var w = vec2<f32>(u.p0.x - o.y, u.p0.y + o.x);
  // the strain: the pitch puckers toward the empty place, breathing
  let h = u.p1.xy;
  let dh = w - h;
  let r2 = dot(dh, dh);
  let pull = u.p0.w * (0.55 + 0.45 * sin(u.p1.z * 2.1)) * exp(-r2 / 60.0);
  w = w - dh * pull * 0.35;
  let a = abs(w);
  let inside = 1.0 - smoothstep(0.0, 1.5, max(a.x - 54.5, a.y - 36.5));
  let stripe = select(0.0, 1.0, fract(w.x / 10.5) > 0.5);
  var c = mix(vec3<f32>(0.02, 0.03, 0.03), vec3<f32>(0.05, 0.11, 0.075) * (0.92 + 0.1 * stripe), inside);
  c = c * (0.9 + 0.12 * noise2(w * 0.8));
  let fw = 0.9 / s;
  let d = markings(w);
  let cov = clamp((0.07 + fw - d) / (2.0 * fw), 0.0, 1.0);
  c = mix(c, vec3<f32>(0.7, 0.76, 0.72), cov * 0.5 * inside);
  // the strain shades the grass a little; the breath out warms it
  c = c * (1.0 - 0.25 * pull);
  c = c + vec3<f32>(0.10, 0.07, 0.03) * u.p1.w * exp(-r2 / 30.0);
  let v = length(px / u.size - vec2<f32>(0.5)) * 1.25;
  c = c * (1.0 - 0.4 * v * v);
  return vec4<f32>(c, 1.0);
}
"#;

/// One moment, in metres (centre spot at 0; his team attacks +x; +y to its right).
struct Moment {
    minute: u32,
    mates: Vec<V2>,
    others: Vec<V2>,
    keepers: Vec<(V2, bool)>,
    him: V2,
    pass_to: Option<V2>,
    seen: Vec<V2>,
    /// The camera's centre for this moment (not the answer: the middle of what the camera saw).
    centre: V2,
}

fn metres(l: &[f32]) -> V2 {
    v2((l.first().copied().unwrap_or(60.0) - 60.0) * 105.0 / 120.0, (l.get(1).copied().unwrap_or(40.0) - 40.0) * 68.0 / 80.0)
}

fn read(events: &[u8], frames: &[u8]) -> anyhow::Result<Vec<Moment>> {
    let evs: Vec<sb::Event> = serde_json::from_slice(events)?;
    let frs: Vec<sb::Frame> = serde_json::from_slice(frames)?;
    let mut out = Vec::new();
    for id in MOMENTS {
        let (Some(e), Some(f)) = (evs.iter().find(|e| e.id == id), frs.iter().find(|f| f.event_uuid == id)) else { continue };
        let Some(actor) = f.freeze_frame.iter().find(|d| d.actor) else { continue };
        let him = metres(&actor.location);
        let mut m = Moment {
            minute: sb::seconds(&e.timestamp) as u32 / 60 + if e.period == 2 { 45 } else if e.period == 3 { 90 } else if e.period == 4 { 105 } else { 0 },
            mates: vec![],
            others: vec![],
            keepers: vec![],
            him,
            pass_to: e.pass.as_ref().and_then(|p| p.end_location.as_ref()).map(|l| metres(l)),
            seen: f.visible_area.chunks(2).filter(|c| c.len() == 2).map(metres).collect(),
            centre: V2::ZERO,
        };
        for d in f.freeze_frame.iter().filter(|d| !d.actor) {
            let p = metres(&d.location);
            if d.keeper {
                m.keepers.push((p, d.teammate));
            } else if d.teammate {
                m.mates.push(p);
            } else {
                m.others.push(p);
            }
        }
        // the camera: the middle of the visible area, pulled toward the players near it
        let area = if m.seen.is_empty() { him } else { m.seen.iter().fold(V2::ZERO, |a, b| a + *b) * (1.0 / m.seen.len() as f32) };
        let crowd: Vec<V2> = m.mates.iter().chain(m.others.iter()).copied().filter(|p| p.dist(area) < 26.0).collect();
        let mid = if crowd.is_empty() { area } else { crowd.iter().fold(V2::ZERO, |a, b| a + *b) * (1.0 / crowd.len() as f32) };
        m.centre = area.lerp(mid, 0.6);
        out.push(m);
    }
    if out.is_empty() {
        anyhow::bail!("none of the moments are in the data");
    }
    Ok(out)
}

enum State {
    Loading,
    Ready(Vec<Moment>),
    Failed(#[allow(dead_code)] String),
}

pub struct Hole {
    loader: Loader,
    state: State,
    painter: Painter,
    pitch: Backdrop,
    bells: Bells,
    at: usize,
    /// Your dot (pitch metres) and the finger holding it.
    dot: Option<V2>,
    finger: Option<(u64, [f64; 2], f64, bool)>,
    /// Lifted: how long ago (s).
    shown: Option<f32>,
    exhale: f32,
    rung: usize,
    fade: f32,
    time: f32,
    size: [f32; 2],
    ppp: f32,
}

/// Dot letters (real::dots), centred or from the left.
fn write(p: &mut Painter, text: &str, at: [f32; 2], cell: f32, color: [f32; 4], centre: bool) {
    let (dots, w) = real::dots::layout(text);
    let x0 = if centre { at[0] - w * cell * 0.5 } else { at[0] };
    for d in dots {
        p.disc([x0 + d[0] * cell, at[1] + d[1] * cell], cell * 0.42, cell * 0.25, color);
    }
}

impl Hole {
    fn scale(&self) -> f32 {
        self.size[0].min(self.size[1]) / SHOWN
    }
    fn centre(&self) -> V2 {
        match &self.state {
            State::Ready(ms) => ms[self.at].centre,
            _ => V2::ZERO,
        }
    }
    /// Pitch metres to the screen (px): the attack up, its right to the right.
    fn sc(&self, p: V2) -> [f32; 2] {
        let (c, s) = (self.centre(), self.scale());
        [self.size[0] * 0.5 + (p.y - c.y) * s, self.size[1] * 0.5 - (p.x - c.x) * s]
    }
    fn world(&self, px: [f32; 2]) -> V2 {
        let (c, s) = (self.centre(), self.scale());
        v2(c.x - (px[1] - self.size[1] * 0.5) / s, c.y + (px[0] - self.size[0] * 0.5) / s)
    }
    /// The dot sits a little above the fingertip, so the finger doesn't hide it.
    fn dot_at(&self, pts: [f64; 2]) -> V2 {
        let k = self.ppp;
        self.world([pts[0] as f32 * k, pts[1] as f32 * k - 56.0 * k])
    }
    fn next(&mut self) {
        if let State::Ready(ms) = &self.state {
            self.at = (self.at + 1) % ms.len();
        }
        self.dot = None;
        self.shown = None;
        self.exhale = 0.0;
        self.rung = 0;
        self.fade = 0.0;
    }
    fn retry(&mut self) {
        self.loader = Loader::new();
        self.loader.ask(&format!("events/{MATCH}.json"));
        self.loader.ask(&format!("three-sixty/{MATCH}.json"));
        self.state = State::Loading;
    }
}

impl Toy for Hole {
    fn new(ctx: &mut Ctx) -> anyhow::Result<Self> {
        let loader = Loader::new();
        loader.ask(&format!("events/{MATCH}.json"));
        loader.ask(&format!("three-sixty/{MATCH}.json"));
        let bells = Bells::new();
        bells.bed(0.1, 43.65);
        Ok(Hole {
            loader,
            state: State::Loading,
            painter: Painter::new(&ctx.gpu),
            pitch: Backdrop::new(&ctx.gpu, "hole", PITCH),
            bells,
            at: 0,
            dot: None,
            finger: None,
            shown: None,
            exhale: 0.0,
            rung: 0,
            fade: 0.0,
            time: 0.0,
            size: [ctx.width as f32, ctx.height as f32],
            ppp: ctx.ppp as f32,
        })
    }

    fn resize(&mut self, ctx: &mut Ctx) {
        self.size = [ctx.width as f32, ctx.height as f32];
        self.ppp = ctx.ppp as f32;
    }

    fn pointer(&mut self, ctx: &mut Ctx, e: PointerEvent) {
        if let State::Failed(_) = self.state {
            if e.phase == Phase::Up {
                self.retry();
            }
            return;
        }
        if !matches!(self.state, State::Ready(_)) {
            return;
        }
        let pts = [e.x, e.y];
        match e.phase {
            Phase::Down => {
                if self.finger.is_none() {
                    self.finger = Some((e.id, pts, e.t, false));
                    if self.shown.is_none() {
                        self.dot = Some(self.dot_at(pts));
                    }
                }
            }
            Phase::Move => {
                if let Some((id, start, t0, moved)) = self.finger {
                    if id != e.id {
                        return;
                    }
                    let far = (pts[0] - start[0]).hypot(pts[1] - start[1]) > 10.0;
                    if far && !moved && self.shown.is_some() {
                        // a drag after the answer: try this moment again
                        self.shown = None;
                        self.rung = 0;
                    }
                    self.finger = Some((id, start, t0, moved || far));
                    if self.shown.is_none() {
                        self.dot = Some(self.dot_at(pts));
                    }
                }
            }
            Phase::Up | Phase::Cancel => {
                let Some((id, _, _, moved)) = self.finger else { return };
                if id != e.id {
                    return;
                }
                self.finger = None;
                if e.phase == Phase::Cancel {
                    return;
                }
                if self.shown.is_some() {
                    if !moved {
                        self.next();
                    }
                } else if self.dot.is_some() {
                    self.shown = Some(0.0);
                    ctx.haptic();
                    self.bells.strike(Note { freq: pentatonic(220.0, 5), amp: 0.16, decay: 2.5, pan: 0.0, bright: 0.5 });
                    self.bells.strike(Note { freq: pentatonic(220.0, 7), amp: 0.08, decay: 2.5, pan: 0.0, bright: 0.3 });
                }
            }
        }
    }

    fn update(&mut self, ctx: &mut Ctx, _t: f64, dt: f64) {
        let dt = dt as f32;
        self.time += dt;
        self.fade = (self.fade + dt * 1.8).min(1.0);
        if let State::Loading = self.state {
            let ev = self.loader.get(&format!("events/{MATCH}.json"));
            let fr = self.loader.get(&format!("three-sixty/{MATCH}.json"));
            match (ev, fr) {
                (Got::Failed(e), _) | (_, Got::Failed(e)) => self.state = State::Failed(e),
                (Got::Bytes(ev), Got::Bytes(fr)) => {
                    self.state = match read(&ev, &fr) {
                        Ok(ms) => State::Ready(ms),
                        Err(e) => State::Failed(format!("{e:#}")),
                    };
                    self.loader.forget(&format!("events/{MATCH}.json"));
                    self.loader.forget(&format!("three-sixty/{MATCH}.json"));
                    self.fade = 0.0;
                }
                _ => {}
            }
            return;
        }
        let State::Ready(ms) = &self.state else { return };
        let m = &ms[self.at];
        if let Some(s) = &mut self.shown {
            *s += dt;
        }
        // the breath: how near your dot is to where he stood
        let want = match (self.dot, self.shown) {
            (Some(d), None) => (-(d.dist(m.him) / NEAR).powi(2)).exp(),
            // shown: the dot keeps the warmth it was let go with
            (Some(_), Some(_)) => self.exhale,
            _ => 0.0,
        };
        self.exhale += (want - self.exhale) * (1.0 - (-dt * 6.0).exp());
        // rising notes as you come nearer (warmer), quiet as you go away
        if self.shown.is_none() && self.dot.is_some() {
            let steps = [0.2, 0.4, 0.6, 0.8, 0.93];
            let now = steps.iter().filter(|&&s| self.exhale > s).count();
            if now > self.rung {
                let f = pentatonic(330.0, now as i32 + 1);
                self.bells.strike(Note { freq: f, amp: 0.05 + 0.03 * now as f32, decay: 1.4, pan: 0.0, bright: 0.3 });
                if now == steps.len() {
                    ctx.haptic();
                    self.bells.strike(Note { freq: f * 0.5, amp: 0.08, decay: 2.5, pan: 0.0, bright: 0.2 });
                }
            }
            if now + 1 < self.rung || now > self.rung {
                self.rung = now;
            }
        }
    }

    fn draw(&mut self, ctx: &mut Ctx, target: &wgpu::TextureView) {
        let size = [ctx.width, ctx.height];
        let s = self.scale();
        let k = self.ppp;
        let c = self.centre();
        let (hole, strain) = match &self.state {
            State::Ready(ms) => (ms[self.at].him, if self.shown.is_some() { 0.0 } else { 1.0 - self.exhale }),
            _ => (V2::ZERO, 0.0),
        };
        let mut enc = ctx.gpu.device.create_command_encoder(&Default::default());
        self.pitch.render(&ctx.gpu, &mut enc, target, size, &BackdropParams { time: self.time, scale: s, center: [0.0, 0.0], p: [c.x, c.y, s, strain * self.fade, hole.x, hole.y, self.time, self.exhale] });
        let ink = [0.86, 0.9, 0.86, 0.75];
        let dim = [0.86, 0.9, 0.86, 0.4];
        let (w, h) = (self.size[0], self.size[1]);
        let cell = 2.2 * k;
        let st = match &self.state {
            State::Loading => Some("LOADING THE MATCH"),
            State::Failed(_) => Some("NO CONNECTION. TAP TO TRY AGAIN"),
            _ => None,
        };
        if let Some(text) = st {
            let p = &mut self.painter;
            write(p, text, [w * 0.5, h * 0.48], cell, ink, true);
            write(p, "STATSBOMB OPEN DATA", [w * 0.5, h * 0.48 + 14.0 * cell], cell * 0.6, dim, true);
            p.render(&ctx.gpu, &mut enc, target, size, None);
            ctx.gpu.queue.submit([enc.finish()]);
            return;
        }
        let State::Ready(ms) = &self.state else { return };
        let m = &ms[self.at];
        let a = self.fade;
        let sc = |q: V2| [w * 0.5 + (q.y - c.y) * s, h * 0.5 - (q.x - c.x) * s];
        let p = &mut self.painter;
        let t = self.time;

        // what the camera saw: its edge, faint
        for i in 0..m.seen.len() {
            let (p0, p1) = (m.seen[i], m.seen[(i + 1) % m.seen.len()]);
            for j in 0..12 {
                if j % 2 == 0 {
                    let (u0, u1) = (j as f32 / 12.0, (j + 1) as f32 / 12.0);
                    p.glow(sc(p0.lerp(p1, u0)), sc(p0.lerp(p1, u1)), 0.03 * s, 0.15 * s, [0.75, 0.85, 1.0, 0.08 * a]);
                }
            }
        }
        // your dot's threads to the teammates near it: warm as it breathes out
        let ex = self.exhale;
        if let (Some(d), None) = (self.dot, self.shown) {
            for q in &m.mates {
                let l = q.dist(d);
                if l > 24.0 {
                    continue;
                }
                let near = 1.0 - l / 24.0;
                let col = [0.6 + 0.4 * ex, 0.65 + 0.2 * ex, 0.75 - 0.25 * ex, (0.08 + 0.3 * ex) * near * a];
                p.glow(sc(d), sc(*q), (0.03 + 0.04 * ex) * s, 0.25 * s, col);
            }
        }
        // the other side: cool, still
        for q in &m.others {
            p.glow(sc(*q), sc(*q), 0.7 * s, 1.2 * s, [0.35, 0.5, 0.95, 0.18 * a]);
            p.disc(sc(*q), 0.62 * s, 0.12 * s, [0.42, 0.56, 0.95, 0.95 * a]);
        }
        for (q, mate) in &m.keepers {
            let col = if *mate { [1.0, 0.82, 0.5, 0.6 * a] } else { [0.42, 0.56, 0.95, 0.6 * a] };
            p.glow_ring(sc(*q), 0.7 * s, 0.06 * s, 0.15 * s, col);
        }
        // his teammates: warm; the ones nearest the hole tremble with the strain
        let strain = if self.shown.is_some() { 0.0 } else { 1.0 - ex };
        for (i, q) in m.mates.iter().enumerate() {
            let near = smoothstep(18.0, 4.0, q.dist(m.him));
            let j = strain * near * 0.22;
            let tq = *q + v2((t * 23.0 + i as f32 * 1.7).sin(), (t * 19.0 + i as f32 * 2.9).cos()) * j;
            p.glow(sc(tq), sc(tq), 0.7 * s, (1.4 + 1.2 * ex) * s, [1.0, 0.8, 0.45, (0.18 + 0.2 * ex) * a]);
            p.disc(sc(tq), 0.62 * s, 0.12 * s, [1.0, 0.88, 0.62, a]);
        }
        // the hole itself, before you find it: a slow pulse of nothing, broad (it shows where, not exactly)
        if self.shown.is_none() {
            let off = v2((self.at as f32 * 2.3).sin(), (self.at as f32 * 1.7).cos()) * 2.2;
            let b = 0.5 + 0.5 * (t * 2.1).sin();
            p.glow_ring(sc(m.him + off), (5.0 + 1.5 * b) * s, 0.5 * s, 3.0 * s, [0.0, 0.0, 0.0, 0.22 * strain * a]);
        }
        // you
        if let Some(d) = self.dot {
            // cold blue far off, warm gold near his spot, and bigger the warmer it is
            let x = sc(d);
            let col = [0.92 + 0.08 * ex, 0.92 - 0.1 * ex, 0.92 - 0.55 * ex];
            p.glow(x, x, (0.8 + 0.8 * ex) * s, (1.5 + 4.0 * ex) * s, [col[0], col[1], col[2], 0.25 + 0.35 * ex]);
            p.disc(x, (0.7 + 0.35 * ex) * s, 0.12 * s, [col[0], col[1], col[2], 1.0]);
            if self.shown.is_none() {
                // a ring that tightens onto the dot as you warm
                let rr = 1.4 + 6.0 * (1.0 - ex);
                p.glow_ring(x, rr * s, 0.06 * s, 0.2 * s, [col[0], col[1], col[2], 0.25 + 0.5 * ex]);
            }
        }
        // the answer
        if let Some(u) = self.shown {
            let r = smoothstep(0.0, 0.6, u);
            let him = sc(m.him);
            if let Some(d) = self.dot {
                p.glow(sc(d), him, 0.05 * s, 0.2 * s, [1.0, 1.0, 1.0, 0.45 * r]);
                let gap = d.dist(m.him);
                let verdict = if gap < 2.0 {
                    "SPOT ON"
                } else if gap < 5.0 {
                    "CLOSE"
                } else if gap < 10.0 {
                    "NEAR"
                } else {
                    "FAR"
                };
                let label = if gap < 1.0 { "SPOT ON".to_string() } else { format!("{:.0} M {}", gap, verdict) };
                // beside your dot, on the side away from him
                let me = sc(d);
                let away = if me[0] <= him[0] { -1.0 } else { 1.0 };
                let lw = real::dots::layout(&label).1 * cell;
                let x = if away < 0.0 { me[0] - 16.0 * k - lw } else { me[0] + 16.0 * k };
                write(p, &label, [x, me[1] - 3.5 * cell], cell, [1.0, 1.0, 1.0, 0.85 * r], false);
            }
            p.glow(him, him, 0.9 * s, 3.0 * s, [1.0, 0.92, 0.7, 0.35 * r]);
            write(p, "HIM", [him[0], him[1] + 2.0 * s], cell * 0.8, [1.0, 0.97, 0.9, 0.9 * r], true);
            p.disc(him, 0.8 * s, 0.12 * s, [1.0, 0.97, 0.9, r]);
            p.glow_ring(him, (0.9 + 3.0 * (1.0 - r)) * s, 0.08 * s, 0.2 * s, [1.0, 0.95, 0.8, 0.8 * r]);
            // the pass he played from there, again and again
            if let Some(to) = m.pass_to {
                let v = ((u - 0.7) / 1.3).rem_euclid(1.6).min(1.0);
                if u > 0.7 {
                    let e = 1.0 - (1.0 - v) * (1.0 - v);
                    let b = m.him.lerp(to, e);
                    p.glow(him, sc(b), 0.06 * s, 0.25 * s, [1.0, 0.95, 0.8, 0.5]);
                    p.glow(sc(b), sc(b), 0.3 * s, 1.0 * s, [1.0, 1.0, 0.95, 0.6]);
                    p.disc(sc(b), 0.3 * s, 0.08 * s, [1.0, 1.0, 1.0, 1.0]);
                }
                // where his pass went: a ring at its end
                p.glow_ring(sc(to), 0.9 * s, 0.08 * s, 0.2 * s, [1.0, 0.95, 0.8, 0.6 * r]);
            }
        }
        // words: the minute and the ask (top), the credit (bottom left)
        let top = 64.0 * k;
        write(p, &format!("{}'  {}/{}", m.minute, self.at + 1, ms.len()), [w * 0.5, top], cell * 0.8, dim, true);
        let ask = match (self.shown, self.dot) {
            (Some(_), _) => "TAP FOR THE NEXT ONE",
            (None, None) => "WHERE WAS HE?",
            (None, Some(_)) => {
                if ex > 0.93 {
                    "HOT. LET GO"
                } else if ex > 0.6 {
                    "WARM"
                } else if ex > 0.25 {
                    "WARMER"
                } else {
                    "COLD"
                }
            }
        };
        write(p, ask, [w * 0.5, top + 14.0 * cell * 0.8], cell, ink, true);
        write(p, "STATSBOMB OPEN DATA", [18.0 * k, h - 30.0 * k], cell * 0.5, dim, false);
        // the key: his team gold, the other side blue
        let ky = h - 52.0 * k;
        p.disc([22.0 * k, ky + 1.6 * k], 4.0 * k, 0.8 * k, [1.0, 0.88, 0.62, 0.9]);
        write(p, "HIS TEAM", [30.0 * k, ky], cell * 0.6, dim, false);
        p.disc([112.0 * k, ky + 1.6 * k], 4.0 * k, 0.8 * k, [0.42, 0.56, 0.95, 0.9]);
        write(p, "THEM", [120.0 * k, ky], cell * 0.6, dim, false);
        p.render(&ctx.gpu, &mut enc, target, size, None);
        ctx.gpu.queue.submit([enc.finish()]);
    }

    fn synth(&mut self) -> Option<Box<dyn Synth>> {
        Some(self.bells.synth())
    }

    fn save(&self) -> Option<String> {
        Some(self.at.to_string())
    }

    fn restore(&mut self, _ctx: &mut Ctx, saved: &str) {
        self.at = saved.trim().parse().unwrap_or(0) % MOMENTS.len();
    }

    /// The info mark: a fingertip drags a dot in from the side toward his pocket, and lets go.
    fn demo(&self, _ctx: &Ctx) -> Option<Demo> {
        let k = self.ppp as f64;
        let (from, to) = match &self.state {
            State::Ready(ms) => {
                let him = self.sc(ms[self.at].him);
                // the finger sits under the dot
                let to = [him[0] as f64 / k, him[1] as f64 / k + 56.0];
                ([to[0] + if to[0] > 196.0 { -130.0 } else { 130.0 }, to[1] + 90.0], to)
            }
            _ => ([120.0, 600.0], [200.0, 450.0]),
        };
        Some(Demo::new("Drag to where he stood. It breathes out when you're close.", vec![Stroke::drag(0.4, 2.2, from, to)]))
    }

    fn rules(&self) -> Option<Rules> {
        Some(Rules::new(
            &[
                "A real moment: everyone the camera saw as he passed, except Busquets.",
                "Where he stood the shape strains; near his spot it breathes out.",
                "Lift to see where he was and the pass he played. Tap for the next one.",
            ],
            "Real: StatsBomb open data (a 360 freeze-frame), fetched as you open it. Drawn: the breathing.",
        ))
    }
}

#[cfg(feature = "app")]
kit::ios_shell!(Hole);
