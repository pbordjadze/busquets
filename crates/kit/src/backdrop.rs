//! A full-screen backdrop: a take's own fragment shader over the whole target, given the screen, the
//! time, a camera (world point at the centre, pixels a world unit) and eight free parameters. The
//! take writes `fn backdrop(px: vec2<f32>, world: vec2<f32>) -> vec4<f32>` (premultiplied colour);
//! `u` is in scope.

use crate::Gpu;

pub const PRELUDE: &str = r#"
struct U { size: vec2<f32>, time: f32, scale: f32, center: vec2<f32>, _q: vec2<f32>, p0: vec4<f32>, p1: vec4<f32> };
@group(0) @binding(0) var<uniform> u: U;
struct VO { @builtin(position) pos: vec4<f32>, @location(0) uv: vec2<f32> };
@vertex fn vs_main(@builtin(vertex_index) vi: u32) -> VO {
  let p = array<vec2<f32>, 3>(vec2(-1.0, -1.0), vec2(3.0, -1.0), vec2(-1.0, 3.0))[vi];
  var o: VO; o.pos = vec4<f32>(p, 0.0, 1.0); o.uv = vec2<f32>((p.x + 1.0) * 0.5, (1.0 - p.y) * 0.5); return o;
}
fn hash2(p: vec2<f32>) -> f32 { return fract(sin(dot(p, vec2<f32>(127.1, 311.7))) * 43758.5453); }
fn noise2(p: vec2<f32>) -> f32 {
  let i = floor(p); let f = fract(p); let w = f * f * (3.0 - 2.0 * f);
  return mix(mix(hash2(i), hash2(i + vec2(1.0, 0.0)), w.x), mix(hash2(i + vec2(0.0, 1.0)), hash2(i + vec2(1.0, 1.0)), w.x), w.y);
}
@fragment fn fs_main(i: VO) -> @location(0) vec4<f32> {
  let px = i.uv * u.size;
  let world = u.center + (px - u.size * 0.5) / u.scale;
  return backdrop(px, world);
}
"#;

#[derive(Clone, Copy, Default)]
pub struct BackdropParams {
    pub time: f32,
    pub scale: f32,
    pub center: [f32; 2],
    pub p: [f32; 8],
}

pub struct Backdrop {
    pipeline: wgpu::RenderPipeline,
    bind: wgpu::BindGroup,
    uniform: wgpu::Buffer,
}

impl Backdrop {
    /// `body` defines `fn backdrop(px, world) -> vec4<f32>`.
    pub fn new(gpu: &Gpu, label: &str, body: &str) -> Backdrop {
        let src = format!("{body}\n{PRELUDE}");
        let shader = gpu.shader(label, &src);
        let uniform = gpu.device.create_buffer(&wgpu::BufferDescriptor { label: Some(label), size: 80, usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
        let bgl = gpu.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some(label),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None },
                count: None,
            }],
        });
        let bind = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor { label: Some(label), layout: &bgl, entries: &[wgpu::BindGroupEntry { binding: 0, resource: uniform.as_entire_binding() }] });
        let pipeline = gpu.fullscreen_pipeline(label, &shader, &[&bgl], None);
        Backdrop { pipeline, bind, uniform }
    }

    pub fn render(&self, gpu: &Gpu, enc: &mut wgpu::CommandEncoder, view: &wgpu::TextureView, size: [u32; 2], p: &BackdropParams) {
        let data: [f32; 20] = [
            size[0] as f32, size[1] as f32, p.time, p.scale, p.center[0], p.center[1], 0.0, 0.0,
            p.p[0], p.p[1], p.p[2], p.p[3], p.p[4], p.p[5], p.p[6], p.p[7], 0.0, 0.0, 0.0, 0.0,
        ];
        gpu.queue.write_buffer(&self.uniform, 0, bytemuck::cast_slice(&data));
        let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("backdrop"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment { view, resolve_target: None, depth_slice: None, ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::BLACK), store: wgpu::StoreOp::Store } })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind, &[]);
        pass.draw(0..3, 0..1);
    }
}
