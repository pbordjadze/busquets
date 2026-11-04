//! Light (a working title): every pass Sergio Busquets made in StatsBomb's open data, drawn as light
//! on a dark pitch, building over time. Each pass flies as a small comet from where he played it to
//! where it went and leaves its trace; the traces stay and pile up, so the lanes he lived in burn
//! white-gold. Forward passes are warm, sideways pale, backward cool; the ones that did not arrive
//! end in a faint ember. Matches arrive one after another (fetched at run time from StatsBomb's own
//! repository; nothing is kept here), the first slowly enough to follow each pass, then faster.
//!
//! A finger held down hurries it; a tap hides or shows the words.

use kit::backdrop::{Backdrop, BackdropParams};
use kit::layer::Layer;
use kit::paint::Painter;
use kit::{Ctx, Phase, PointerEvent, Toy};
use pitch::v2::Rng;
use pitch::V2;
use real::fetch::{Got, Loader};
use real::{EvKind, Game, BUSQUETS};

/// Every open match of his with 360 data (StatsBomb open data), Croatia 3-5 Spain first.
const MATCHES: [(u32, &str); 45] = [
    (3794686, "CROATIA 3-5 SPAIN, EURO 2020"),
    (3788750, "SPAIN 0-0 SWEDEN, EURO 2020"),
    (3788762, "SPAIN 1-1 POLAND, EURO 2020"),
    (3788775, "SLOVAKIA 0-5 SPAIN, EURO 2020"),
    (3795108, "SWITZERLAND 1-1 SPAIN, EURO 2020"),
    (3795220, "ITALY 1-1 SPAIN, EURO 2020"),
    (3857291, "SPAIN 7-0 COSTA RICA, WORLD CUP 2022"),
    (3857263, "SPAIN 1-1 GERMANY, WORLD CUP 2022"),
    (3857255, "JAPAN 2-1 SPAIN, WORLD CUP 2022"),
    (3869220, "MOROCCO 0-0 SPAIN, WORLD CUP 2022"),
    (3773593, "BARCELONA 4-0 VILLARREAL, LA LIGA 20/21"),
    (3773466, "CELTA VIGO 0-3 BARCELONA, LA LIGA 20/21"),
    (3773672, "BARCELONA 1-1 SEVILLA, LA LIGA 20/21"),
    (3773587, "GETAFE 1-0 BARCELONA, LA LIGA 20/21"),
    (3773585, "BARCELONA 1-3 REAL MADRID, LA LIGA 20/21"),
    (3773386, "DEPORTIVO ALAVÉS 1-1 BARCELONA, LA LIGA 20/21"),
    (3773477, "BARCELONA 5-2 REAL BETIS, LA LIGA 20/21"),
    (3773656, "ATLÉTICO MADRID 1-0 BARCELONA, LA LIGA 20/21"),
    (3773547, "BARCELONA 4-0 OSASUNA, LA LIGA 20/21"),
    (3773428, "CÁDIZ 2-1 BARCELONA, LA LIGA 20/21"),
    (3773660, "BARCELONA 1-0 LEVANTE UD, LA LIGA 20/21"),
    (3773523, "BARCELONA 2-1 REAL SOCIEDAD, LA LIGA 20/21"),
    (3773377, "BARCELONA 2-2 VALENCIA, LA LIGA 20/21"),
    (3773571, "REAL VALLADOLID 0-3 BARCELONA, LA LIGA 20/21"),
    (3773552, "HUESCA 0-1 BARCELONA, LA LIGA 20/21"),
    (3764661, "ATHLETIC CLUB 2-3 BARCELONA, LA LIGA 20/21"),
    (3773565, "GRANADA 0-4 BARCELONA, LA LIGA 20/21"),
    (3773403, "BARCELONA 2-1 ATHLETIC CLUB, LA LIGA 20/21"),
    (3773631, "REAL BETIS 2-3 BARCELONA, LA LIGA 20/21"),
    (3773526, "BARCELONA 5-1 DEPORTIVO ALAVÉS, LA LIGA 20/21"),
    (3773415, "BARCELONA 1-1 CÁDIZ, LA LIGA 20/21"),
    (3764440, "BARCELONA 3-0 ELCHE, LA LIGA 20/21"),
    (3773625, "SEVILLA 0-2 BARCELONA, LA LIGA 20/21"),
    (3773665, "OSASUNA 0-2 BARCELONA, LA LIGA 20/21"),
    (3773369, "BARCELONA 4-1 HUESCA, LA LIGA 20/21"),
    (3773597, "REAL SOCIEDAD 1-6 BARCELONA, LA LIGA 20/21"),
    (3773474, "BARCELONA 1-0 REAL VALLADOLID, LA LIGA 20/21"),
    (3773497, "REAL MADRID 2-1 BARCELONA, LA LIGA 20/21"),
    (3773661, "BARCELONA 5-2 GETAFE, LA LIGA 20/21"),
    (3773689, "VILLARREAL 1-2 BARCELONA, LA LIGA 20/21"),
    (3773586, "BARCELONA 1-2 GRANADA, LA LIGA 20/21"),
    (3773695, "VALENCIA 2-3 BARCELONA, LA LIGA 20/21"),
    (3773372, "BARCELONA 0-0 ATLÉTICO MADRID, LA LIGA 20/21"),
    (3773387, "LEVANTE UD 3-3 BARCELONA, LA LIGA 20/21"),
    (3773457, "BARCELONA 1-2 CELTA VIGO, LA LIGA 20/21"),
];

const NIGHT: &str = r#"
fn sdbox(p: vec2<f32>, b: vec2<f32>) -> f32 {
  let q = abs(p) - b;
  return length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0);
}
fn backdrop(px: vec2<f32>, world: vec2<f32>) -> vec4<f32> {
  // world here: metres, x up the screen (p0.x = 1 when portrait)
  let o = (px - u.size * 0.5) / u.scale;
  var w = vec2<f32>(o.x, -o.y);
  if (u.p0.x > 0.5) { w = vec2<f32>(-o.y, -o.x); }
  w = w - u.center;
  let a = abs(w);
  var d = abs(sdbox(w, vec2<f32>(52.5, 34.0)));
  d = min(d, length(vec2<f32>(w.x, max(a.y - 34.0, 0.0))));
  d = min(d, abs(length(w) - 9.15));
  d = min(d, abs(sdbox(vec2<f32>(a.x - 44.25, w.y), vec2<f32>(8.25, 20.16))));
  d = min(d, abs(sdbox(vec2<f32>(a.x - 49.75, w.y), vec2<f32>(2.75, 9.16))));
  if (a.x < 36.0) { d = min(d, abs(length(vec2<f32>(a.x - 41.5, w.y)) - 9.15)); }
  let fw = 0.9 / u.scale;
  let line = clamp((0.07 + fw - d) / (2.0 * fw), 0.0, 1.0);
  let v = length(px / u.size - vec2<f32>(0.5));
  var c = mix(vec3<f32>(0.016, 0.02, 0.034), vec3<f32>(0.003, 0.004, 0.009), smoothstep(0.1, 0.75, v));
  c = c + vec3<f32>(0.006, 0.008, 0.012) * noise2(px * 0.5);
  c = c + vec3<f32>(0.10, 0.12, 0.16) * line * (0.08 + 0.06 * u.p0.y);
  return vec4<f32>(c, 1.0);
}
"#;

#[derive(Clone, Copy)]
struct Pass {
    a: V2,
    b: V2,
    ok: bool,
    lofted: bool,
    /// -1 back, 0 sideways, 1 forward
    dir: i8,
    bend: f32,
}

struct Comet {
    pass: Pass,
    u: f32,
    dur: f32,
}

pub struct Light {
    loader: Loader,
    next: usize,
    queue: Vec<Pass>,
    comets: Vec<Comet>,
    painter: Painter,
    night: Backdrop,
    layer: Option<Layer>,
    /// Trace pieces waiting to be drawn into the layer: (a, b, colour, width, soft) in metres.
    pending: Vec<(V2, V2, [f32; 4], f32, f32)>,
    /// Everything drawn so far, to redraw on a new screen size.
    drawn: Vec<(V2, V2, [f32; 4], f32, f32)>,
    redraw: bool,
    spawn: f32,
    passes: usize,
    matches: usize,
    current: Option<&'static str>,
    hurry: bool,
    words: bool,
    clock: f32,
    rng: Rng,
}

fn colour(p: &Pass) -> [f32; 3] {
    match p.dir {
        1 => [1.0, 0.72, 0.38],
        -1 => [0.5, 0.66, 1.0],
        _ => [0.98, 0.9, 0.78],
    }
}

/// A point along a pass's arc (a gentle bow; lofted ones bow more).
fn arc(p: &Pass, u: f32) -> V2 {
    let d = p.b - p.a;
    let n = d.perp().norm();
    let bow = (u * std::f32::consts::PI).sin() * p.bend * d.len();
    p.a + d * u + n * bow
}

fn write(pt: &mut Painter, text: &str, at: [f32; 2], cell: f32, color: [f32; 4]) {
    let (dots, w) = real::dots::layout(text);
    let x0 = at[0] - w * cell * 0.5;
    for d in dots {
        pt.disc([x0 + d[0] * cell, at[1] + d[1] * cell], cell * 0.4, cell * 0.3, color);
    }
}

impl Light {
    fn frame(&self, ctx: &Ctx) -> (f32, bool) {
        let (w, h) = (ctx.width as f32, ctx.height as f32);
        let portrait = h > w;
        let ppp = ctx.ppp as f32;
        let (long, short) = if portrait { (h - 120.0 * ppp, w - 20.0 * ppp) } else { (w - 40.0 * ppp, h - 110.0 * ppp) };
        ((long / 108.0).min(short / 70.0), portrait)
    }
    fn screen(&self, ctx: &Ctx, q: V2) -> [f32; 2] {
        let (s, portrait) = self.frame(ctx);
        let (w, h) = (ctx.width as f32, ctx.height as f32);
        let cy = h * 0.5 + 4.0 * ctx.ppp as f32;
        if portrait {
            [w * 0.5 - q.y * s, cy - q.x * s]
        } else {
            [w * 0.5 + q.x * s, cy - q.y * s]
        }
    }
    fn take_match(&mut self, g: &Game) {
        let Some(b) = g.find(BUSQUETS) else { return };
        let mut n = 0;
        for e in &g.events {
            if e.kind != EvKind::Pass || e.who != Some(b) {
                continue;
            }
            let (Some(a), Some(end)) = (e.at, e.end) else { continue };
            let d = end - a;
            let ang = d.y.atan2(d.x).abs().to_degrees();
            let dir = if ang < 45.0 { 1 } else if ang > 135.0 { -1 } else { 0 };
            let bend = self.rng.s() * if e.height > 0 { 0.16 } else { 0.05 };
            self.queue.push(Pass { a, b: end, ok: e.ok, lofted: e.height > 0, dir, bend });
            n += 1;
        }
        if n > 0 {
            self.matches += 1;
        }
    }
}

impl Toy for Light {
    /// The rules mark: the few rules that make the scene, and what is real and what is drawn.
    fn rules(&self) -> Option<kit::info::Rules> {
        Some(kit::info::Rules::new(
            &[
                "Each comet is one pass Sergio Busquets made; its trace stays.",
                "Warm: forward. Pale: sideways. Cool: backward. An ember: it didn't arrive.",
            ],
            "Real: every pass (StatsBomb open data). Drawn: the light.",
        ))
    }

    /// The info mark: a finger held down hurries the passes.
    fn demo(&self, ctx: &Ctx) -> Option<kit::info::Demo> {
        use kit::info::{Demo, Stroke};
        let [w, h] = ctx.points();
        Some(Demo::new("Every pass Sergio Busquets made in the open data, as light.", vec![Stroke::hold(0.4, 2.2, [w * 0.5, h * 0.5])]))
    }

    fn new(ctx: &mut Ctx) -> anyhow::Result<Light> {
        let loader = Loader::new();
        loader.ask(&format!("events/{}.json", MATCHES[0].0));
        Ok(Light {
            loader,
            next: 0,
            queue: vec![],
            comets: vec![],
            painter: Painter::new(&ctx.gpu),
            night: Backdrop::new(&ctx.gpu, "light-night", NIGHT),
            layer: None,
            pending: vec![],
            drawn: vec![],
            redraw: true,
            spawn: 0.0,
            passes: 0,
            matches: 0,
            current: None,
            hurry: false,
            words: true,
            clock: 0.0,
            rng: Rng::new(5),
        })
    }

    fn resize(&mut self, _ctx: &mut Ctx) {
        self.layer = None;
        self.redraw = true;
    }

    fn pointer(&mut self, _ctx: &mut Ctx, e: PointerEvent) {
        match e.phase {
            Phase::Down => {
                self.hurry = true;
                self.spawn = 0.0;
            }
            Phase::Up | Phase::Cancel => {
                if self.hurry && self.clock - self.spawn < 0.3 {
                    self.words = !self.words;
                }
                self.hurry = false;
            }
            _ => {}
        }
        self.spawn = self.clock;
    }

    fn update(&mut self, _ctx: &mut Ctx, _t: f64, dt: f64) {
        let dt = dt as f32;
        self.clock += dt;
        // the next match, when the last has been read and the queue runs low
        if self.next < MATCHES.len() {
            let rel = format!("events/{}.json", MATCHES[self.next].0);
            match self.loader.get(&rel) {
                Got::Bytes(b) if self.queue.len() < 40 => {
                    if let Ok(g) = Game::read(&b, None, BUSQUETS) {
                        self.take_match(&g);
                        self.current = Some(MATCHES[self.next].1);
                    }
                    self.loader.forget(&rel);
                    self.next += 1;
                    if self.next < MATCHES.len() {
                        self.loader.ask(&format!("events/{}.json", MATCHES[self.next].0));
                    }
                }
                Got::Failed(_) => {
                    self.next += 1;
                    if self.next < MATCHES.len() {
                        self.loader.ask(&format!("events/{}.json", MATCHES[self.next].0));
                    }
                }
                _ => {}
            }
        }
        // the pace: slow enough to follow at first, then a stream
        let base = if self.passes < 24 { 1.4 } else if self.passes < 150 { 4.0 } else { 9.0 + (self.passes as f32 / 400.0).min(10.0) };
        let rate = if self.hurry { base * 6.0 } else { base };
        let mut budget = rate * dt;
        while budget > 0.0 && !self.queue.is_empty() {
            if self.rng.f() < budget.min(1.0) {
                let p = self.queue.remove(0);
                let len = p.a.dist(p.b);
                let slow = if self.passes < 24 { 1.6 } else { 1.0 };
                self.comets.push(Comet { pass: p, u: 0.0, dur: (0.45 + len / 28.0) * slow });
                self.passes += 1;
            }
            budget -= 1.0;
        }
        // the comets fly and leave their trace, fainter as the traces pile up, so the lanes he used most
        // glow and the rest stays a haze rather than a white-out
        let thin = (260.0 / (260.0 + self.passes as f32)).powf(0.75).max(0.08);
        for c in &mut self.comets {
            let u0 = c.u;
            let speed = if self.hurry { 3.0 } else { 1.0 };
            c.u = (c.u + dt * speed / c.dur).min(1.0);
            let col = colour(&c.pass);
            let steps = 3;
            for k in 0..steps {
                let (ua, ub) = (u0 + (c.u - u0) * k as f32 / steps as f32, u0 + (c.u - u0) * (k + 1) as f32 / steps as f32);
                let fade = if c.pass.ok { 1.0 } else { 1.0 - 0.6 * ub };
                let a = if c.pass.lofted { 0.06 } else { 0.08 } * thin;
                self.pending.push((arc(&c.pass, ua), arc(&c.pass, ub), [col[0], col[1], col[2], a * fade], 0.05, 0.28));
            }
            if c.u >= 1.0 {
                let end = c.pass.b;
                if c.pass.ok {
                    self.pending.push((end, end, [col[0], col[1], col[2], 0.06], 0.1, 0.5));
                }
                // where he stood: a grain of light
                self.pending.push((c.pass.a, c.pass.a, [1.0, 0.97, 0.9, 0.05], 0.08, 0.15));
            }
        }
        self.comets.retain(|c| c.u < 1.0);
    }

    fn draw(&mut self, ctx: &mut Ctx, target: &wgpu::TextureView) {
        let size = [ctx.width, ctx.height];
        let (w, h, ppp) = (ctx.width as f32, ctx.height as f32, ctx.ppp as f32);
        let (s, portrait) = self.frame(ctx);
        let mut enc = ctx.gpu.device.create_command_encoder(&Default::default());
        if self.layer.is_none() {
            self.layer = Some(Layer::new(&ctx.gpu, size));
            self.redraw = true;
        }
        let pieces: Vec<_> = if self.redraw {
            self.drawn.iter().chain(self.pending.iter()).copied().collect()
        } else {
            self.pending.clone()
        };
        let layer = self.layer.as_ref().unwrap();
        if self.redraw {
            layer.clear(&mut enc);
        }
        for chunk in pieces.chunks(2000) {
            for &(a, b, c, wd, soft) in chunk {
                let (pa, pb) = (self.screen(ctx, a), self.screen(ctx, b));
                self.painter.glow(pa, pb, wd * s, soft * s, c);
            }
            self.painter.render(&ctx.gpu, &mut enc, &layer.view, size, None);
        }
        self.redraw = false;
        self.drawn.append(&mut self.pending);

        // the pitch's centre sits where screen() puts it
        let off = (h * 0.5 - (h * 0.5 + 4.0 * ppp)) / s;
        let centre = if portrait { [off, 0.0] } else { [0.0, off] };
        let breathe = 0.5 + 0.5 * (self.clock * 0.4).sin();
        self.night.render(&ctx.gpu, &mut enc, target, size, &BackdropParams { time: self.clock, scale: s, center: centre, p: [if portrait { 1.0 } else { 0.0 }, breathe, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0] });
        self.layer.as_ref().unwrap().composite(&mut enc, target);

        // the comets' heads
        let heads: Vec<([f32; 2], [f32; 3], f32)> = self.comets.iter().map(|c| (self.screen(ctx, arc(&c.pass, c.u)), colour(&c.pass), c.u)).collect();
        let p = &mut self.painter;
        for (sc, col, u) in heads {
            let k = (u * std::f32::consts::PI).sin().max(0.25);
            p.glow(sc, sc, 0.25 * s, 1.6 * s, [col[0], col[1], col[2], 0.35 * k]);
            p.disc(sc, 0.22 * s, 0.12 * s, [1.0, 0.98, 0.94, 0.9 * k]);
        }
        if self.words {
            let ink = [0.85, 0.82, 0.76, 0.6];
            let dim = [0.6, 0.62, 0.68, 0.45];
            write(p, "SERGIO BUSQUETS", [w * 0.5, 22.0 * ppp], 1.9 * ppp, ink);
            write(p, &format!("{} PASS{}, {} MATCH{}", self.passes, if self.passes == 1 { "" } else { "ES" }, self.matches, if self.matches == 1 { "" } else { "ES" }), [w * 0.5, 42.0 * ppp], 1.3 * ppp, dim);
            if let Some(m) = self.current {
                write(p, m, [w * 0.5, h - 44.0 * ppp], 1.15 * ppp, dim);
            }
            write(p, "DATA: STATSBOMB OPEN DATA", [w * 0.5, h - 26.0 * ppp], 1.0 * ppp, [0.5, 0.52, 0.58, 0.4]);
            if self.passes == 0 {
                let pulse = 0.5 + 0.5 * (self.clock * 2.5).sin();
                let c = [w * 0.5, h * 0.5];
                p.glow(c, c, 3.0 * ppp, 20.0 * ppp, [1.0, 0.9, 0.7, 0.1 + 0.2 * pulse]);
            }
        }
        p.render(&ctx.gpu, &mut enc, target, size, None);
        ctx.gpu.queue.submit([enc.finish()]);
    }
}

#[cfg(feature = "app")]
kit::ios_shell!(Light);
