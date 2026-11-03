//! Visual directions (placeholder until the study lands): four looks, `Night`, `Paper`, `Lines`, `Wash`.
use kit::{Ctx, PointerEvent, Toy};
pub struct Blank;
impl Toy for Blank {
    fn new(_c: &mut Ctx) -> anyhow::Result<Self> { Ok(Blank) }
    fn pointer(&mut self, _c: &mut Ctx, _e: PointerEvent) {}
    fn update(&mut self, _c: &mut Ctx, _t: f64, _d: f64) {}
    fn draw(&mut self, _c: &mut Ctx, _v: &wgpu::TextureView) {}
}
pub type Night = Blank;
pub type Paper = Blank;
pub type Lines = Blank;
pub type Wash = Blank;
