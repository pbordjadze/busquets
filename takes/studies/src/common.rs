//! What the four looks share: the self-playing portrait rondo, its camera, and the memory of passes.

use kit::{Ctx, PointerEvent};
use pitch::{v2, Ball, Config, Event, View, World, V2};

/// The pitch card every look marks out (half-width, half-length, m) and its centre circle.
pub const HALF: [f32; 2] = [11.5, 24.0];
pub const CIRCLE: f32 = 5.0;

/// The backdrop's world point for a pixel, turned like the painter's view (`u.p1.w` the view's turn).
pub const WPOS: &str = r#"
fn wpos(px: vec2<f32>) -> vec2<f32> {
  let r = vec2<f32>((px.x - u.size.x * 0.5) / u.scale, -(px.y - u.size.y * 0.5) / u.scale);
  let s = sin(-u.p1.w); let c = cos(-u.p1.w);
  return u.center + vec2<f32>(r.x * c - r.y * s, r.x * s + r.y * c);
}
fn rot(d: vec2<f32>) -> vec2<f32> { let s = sin(-u.p1.w); let c = cos(-u.p1.w); return vec2<f32>(d.x * c - d.y * s, d.x * s + d.y * c); }
fn pitch_lines(w: vec2<f32>) -> f32 {
  let q = abs(w) - vec2<f32>(11.5, 24.0);
  let rect = abs(length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0));
  let half = select(9.0, abs(w.y), abs(w.x) < 11.5);
  return min(min(rect, half), min(abs(length(w) - 5.0), length(w) - 0.15));
}
"#;

/// One pass remembered: from where to where the ball went (`b` follows it while it rolls).
pub struct Pass {
    pub a: V2,
    pub b: V2,
    pub from: usize,
    pub to: usize,
    pub born: f32,
    pub live: bool,
    /// How many passes on this route (either way) are remembered, this one included.
    pub uses: u32,
}

pub struct Sim {
    pub world: World,
    pub cam: V2,
    pub turn: f32,
    pub time: f32,
    pub passes: Vec<Pass>,
    pub trail: Vec<V2>,
}

fn config(seed: u64) -> Config {
    let mates = [v2(-6.5, -16.0), v2(6.5, -16.0), v2(-7.5, -3.0), v2(7.5, -3.0), v2(-6.5, 11.0), v2(6.5, 11.0), v2(0.0, 19.0)];
    let pressers = [v2(-3.0, 6.0), v2(3.5, 5.0), v2(0.0, -7.0), v2(4.0, -12.0)];
    let mut c = Config::rondo(v2(0.0, 1.0), &mates, &pressers, 20.0);
    c.half = v2(HALF[0] + 1.5, HALF[1] + 1.5);
    Config { human: None, seed, ..c }
}

impl Sim {
    pub fn new(seed: u64) -> Sim {
        let world = World::new(config(seed));
        Sim { cam: v2(0.0, 1.5), turn: world.turn, world, time: 0.0, passes: Vec::new(), trail: Vec::new() }
    }

    pub fn tick(&mut self, dt: f32) -> Vec<Event> {
        self.time += dt;
        self.world.step(dt);
        let ev = self.world.drain();
        for e in &ev {
            match *e {
                Event::Pass { from, to, at, .. } => {
                    for p in &mut self.passes {
                        p.live = false;
                    }
                    let uses = 1 + self.passes.iter().filter(|p| (p.from, p.to) == (from, to) || (p.from, p.to) == (to, from)).count() as u32;
                    self.passes.push(Pass { a: at, b: at, from, to, born: self.time, live: true, uses });
                }
                Event::Received { .. } | Event::Poked { .. } | Event::Out { .. } => {
                    if let Some(p) = self.passes.last_mut() {
                        p.live = false;
                    }
                }
                _ => {}
            }
        }
        let ball = self.world.ball_pos;
        if let Some(p) = self.passes.last_mut().filter(|p| p.live) {
            p.b = ball;
        }
        let t = self.time;
        self.passes.retain(|p| t - p.born < 90.0);
        if matches!(self.world.ball, Ball::Rolling { .. }) {
            self.trail.push(ball);
        } else if !self.trail.is_empty() {
            self.trail.remove(0);
        }
        if self.trail.len() > 22 {
            self.trail.remove(0);
        }
        let k = 1.0 - (-dt * 0.7).exp();
        let team: Vec<V2> = self.world.players.iter().filter(|p| p.team == 0).map(|p| p.pos).collect();
        let mid = team.iter().fold(V2::ZERO, |a, &b| a + b) * (1.0 / team.len().max(1) as f32);
        self.cam = self.cam.lerp(mid, k);
        self.turn += (self.world.turn - self.turn) * k;
        ev
    }

    pub fn view(&self, ctx: &Ctx) -> View {
        let (w, h) = (ctx.width as f32, ctx.height as f32);
        View { center: self.cam, scale: w.min(h) / 24.0, size: [w, h], turn: -self.turn }
    }

    /// A touch turns the carrier's head toward it (the pressers read it); nothing else.
    pub fn touch(&mut self, ctx: &Ctx, e: PointerEvent) {
        let at = self.view(ctx).to_world([(e.x * ctx.ppp) as f32, (e.y * ctx.ppp) as f32]);
        let d = at - self.world.ball_pos;
        if matches!(e.phase, kit::Phase::Down | kit::Phase::Move) && d.len() > 0.5 && d.len().is_finite() {
            self.world.glance(d.angle());
        }
    }

    /// The backdrop's uniforms: the camera, the view's turn in `p[7]`.
    pub fn params(&self, view: &View, p: [f32; 7]) -> kit::backdrop::BackdropParams {
        kit::backdrop::BackdropParams { time: self.time, scale: view.scale, center: view.center.arr(), p: [p[0], p[1], p[2], p[3], p[4], p[5], p[6], view.turn] }
    }
}

/// Seconds of play before the first frame, so every look opens mid-rondo, its memory already drawn.
pub const WARM: f32 = 14.0;
