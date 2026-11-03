//! Watch (placeholder until the sketch lands).
use kit::{Ctx, PointerEvent, Toy};
pub struct Watch;
impl Toy for Watch {
    fn new(_c: &mut Ctx) -> anyhow::Result<Self> { Ok(Watch) }
    fn pointer(&mut self, _c: &mut Ctx, _e: PointerEvent) {}
    fn update(&mut self, _c: &mut Ctx, _t: f64, _d: f64) {}
    fn draw(&mut self, _c: &mut Ctx, _v: &wgpu::TextureView) {}
}
kit::ios_shell!(Watch);
