//! A painter of soft 2D shapes, in pixels from the top left: discs, rings and round-capped strokes,
//! each with a soft (gaussian) edge that is antialiasing at one pixel and a glow at forty. One instanced
//! draw a blend: `over` (premultiplied) shapes, then `add` (light) shapes. Everything the takes draw
//! that is not a backdrop is these.

use crate::Gpu;
use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

/// One shape: the segment `a`..`b` (a disc when they are equal) thickened by `r`; with `ring > 0` only
/// a band of that width at distance `r` is filled. `soft` is the edge's gaussian width, pixels.
/// `color` is premultiplied RGBA.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Debug)]
pub struct Shape {
    pub a: [f32; 2],
    pub b: [f32; 2],
    pub r: f32,
    pub ring: f32,
    pub soft: f32,
    pub _pad: f32,
    pub color: [f32; 4],
}

/// Premultiplies a straight RGBA colour.
pub fn pm(c: [f32; 4]) -> [f32; 4] {
    [c[0] * c[3], c[1] * c[3], c[2] * c[3], c[3]]
}

const WGSL: &str = r#"
struct U { size: vec2<f32>, _p: vec2<f32> };
@group(0) @binding(0) var<uniform> u: U;
struct S { @location(0) a: vec2<f32>, @location(1) b: vec2<f32>, @location(2) rrs: vec4<f32>, @location(3) color: vec4<f32> };
struct O { @builtin(position) pos: vec4<f32>, @location(0) p: vec2<f32>, @location(1) a: vec2<f32>, @location(2) b: vec2<f32>, @location(3) rrs: vec4<f32>, @location(4) color: vec4<f32> };
@vertex fn vs_main(@builtin(vertex_index) vi: u32, s: S) -> O {
  let reach = s.rrs.x + s.rrs.y * 0.5 + s.rrs.z * 1.6 + 1.5;
  var d = s.b - s.a;
  let len = length(d);
  var dir = vec2<f32>(1.0, 0.0);
  if (len > 1e-4) { dir = d / len; }
  let nrm = vec2<f32>(-dir.y, dir.x);
  let corner = array<vec2<f32>, 6>(vec2(-1.0,-1.0), vec2(1.0,-1.0), vec2(1.0,1.0), vec2(-1.0,-1.0), vec2(1.0,1.0), vec2(-1.0,1.0))[vi];
  let along = select(s.a - dir * reach, s.b + dir * reach, corner.x > 0.0);
  let p = along + nrm * reach * corner.y;
  var o: O;
  o.pos = vec4<f32>(p.x / u.size.x * 2.0 - 1.0, 1.0 - p.y / u.size.y * 2.0, 0.0, 1.0);
  o.p = p; o.a = s.a; o.b = s.b; o.rrs = s.rrs; o.color = s.color;
  return o;
}
@fragment fn fs_main(i: O) -> @location(0) vec4<f32> {
  let ab = i.b - i.a;
  let h = clamp(dot(i.p - i.a, ab) / max(dot(ab, ab), 1e-6), 0.0, 1.0);
  var d = length(i.p - i.a - ab * h);
  if (i.rrs.y > 0.0) { d = abs(d - i.rrs.x) - i.rrs.y * 0.5; } else { d = d - i.rrs.x; }
  let s = max(i.rrs.z, 0.75);
  let k = max(d, 0.0) / s;
  let a = exp(-4.0 * k * k);
  return i.color * a;
}
"#;

/// The shapes of a frame and the pipelines that draw them.
pub struct Painter {
    over: wgpu::RenderPipeline,
    add: wgpu::RenderPipeline,
    bind: wgpu::BindGroup,
    uniform: wgpu::Buffer,
    pub shapes_over: Vec<Shape>,
    pub shapes_add: Vec<Shape>,
}

impl Painter {
    pub fn new(gpu: &Gpu) -> Painter {
        Painter::with_format(gpu, gpu.format)
    }

    pub fn with_format(gpu: &Gpu, format: wgpu::TextureFormat) -> Painter {
        let dev = &gpu.device;
        let shader = gpu.shader("paint", WGSL);
        let uniform = dev.create_buffer(&wgpu::BufferDescriptor {
            label: Some("paint u"),
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bgl = dev.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("paint"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None },
                count: None,
            }],
        });
        let bind = dev.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("paint"),
            layout: &bgl,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: uniform.as_entire_binding() }],
        });
        let layout = dev.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: Some("paint"), bind_group_layouts: &[&bgl], push_constant_ranges: &[] });
        let attrs = wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2, 2 => Float32x4, 3 => Float32x4];
        let pipe = |blend: wgpu::BlendState| {
            dev.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("paint"),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    buffers: &[wgpu::VertexBufferLayout { array_stride: std::mem::size_of::<Shape>() as u64, step_mode: wgpu::VertexStepMode::Instance, attributes: &attrs }],
                    compilation_options: Default::default(),
                },
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState { format, blend: Some(blend), write_mask: wgpu::ColorWrites::ALL })],
                    compilation_options: Default::default(),
                }),
                multiview: None,
                cache: None,
            })
        };
        let add = wgpu::BlendState {
            color: wgpu::BlendComponent { src_factor: wgpu::BlendFactor::One, dst_factor: wgpu::BlendFactor::One, operation: wgpu::BlendOperation::Add },
            alpha: wgpu::BlendComponent { src_factor: wgpu::BlendFactor::Zero, dst_factor: wgpu::BlendFactor::One, operation: wgpu::BlendOperation::Add },
        };
        Painter { over: pipe(Gpu::over()), add: pipe(add), bind, uniform, shapes_over: Vec::new(), shapes_add: Vec::new() }
    }

    /// A filled disc (straight colour; premultiplied here).
    pub fn disc(&mut self, c: [f32; 2], r: f32, soft: f32, color: [f32; 4]) {
        self.shapes_over.push(Shape { a: c, b: c, r, ring: 0.0, soft, _pad: 0.0, color: pm(color) });
    }
    /// A ring of `width` at radius `r`.
    pub fn ring(&mut self, c: [f32; 2], r: f32, width: f32, soft: f32, color: [f32; 4]) {
        self.shapes_over.push(Shape { a: c, b: c, r, ring: width.max(0.01), soft, _pad: 0.0, color: pm(color) });
    }
    /// A round-capped stroke of half-width `r`.
    pub fn line(&mut self, a: [f32; 2], b: [f32; 2], r: f32, soft: f32, color: [f32; 4]) {
        self.shapes_over.push(Shape { a, b, r, ring: 0.0, soft, _pad: 0.0, color: pm(color) });
    }
    /// Light: the same shapes added (`color` straight; its alpha is the strength).
    pub fn glow(&mut self, a: [f32; 2], b: [f32; 2], r: f32, soft: f32, color: [f32; 4]) {
        self.shapes_add.push(Shape { a, b, r, ring: 0.0, soft, _pad: 0.0, color: pm(color) });
    }
    pub fn glow_ring(&mut self, c: [f32; 2], r: f32, width: f32, soft: f32, color: [f32; 4]) {
        self.shapes_add.push(Shape { a: c, b: c, r, ring: width.max(0.01), soft, _pad: 0.0, color: pm(color) });
    }

    pub fn is_empty(&self) -> bool {
        self.shapes_over.is_empty() && self.shapes_add.is_empty()
    }

    /// Draws (and forgets) the shapes into `view` of `size` pixels; `clear` clears it first.
    pub fn render(&mut self, gpu: &Gpu, enc: &mut wgpu::CommandEncoder, view: &wgpu::TextureView, size: [u32; 2], clear: Option<[f64; 4]>) {
        gpu.queue.write_buffer(&self.uniform, 0, bytemuck::cast_slice(&[size[0] as f32, size[1] as f32, 0.0, 0.0]));
        let over = (!self.shapes_over.is_empty()).then(|| gpu.device.create_buffer_init(&wgpu::util::BufferInitDescriptor { label: Some("shapes"), contents: bytemuck::cast_slice(&self.shapes_over), usage: wgpu::BufferUsages::VERTEX }));
        let add = (!self.shapes_add.is_empty()).then(|| gpu.device.create_buffer_init(&wgpu::util::BufferInitDescriptor { label: Some("light"), contents: bytemuck::cast_slice(&self.shapes_add), usage: wgpu::BufferUsages::VERTEX }));
        {
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("paint"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: match clear {
                            Some(c) => wgpu::LoadOp::Clear(wgpu::Color { r: c[0], g: c[1], b: c[2], a: c[3] }),
                            None => wgpu::LoadOp::Load,
                        },
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_bind_group(0, &self.bind, &[]);
            if let Some(b) = &over {
                pass.set_pipeline(&self.over);
                pass.set_vertex_buffer(0, b.slice(..));
                pass.draw(0..6, 0..self.shapes_over.len() as u32);
            }
            if let Some(b) = &add {
                pass.set_pipeline(&self.add);
                pass.set_vertex_buffer(0, b.slice(..));
                pass.draw(0..6, 0..self.shapes_add.len() as u32);
            }
        }
        self.shapes_over.clear();
        self.shapes_add.clear();
    }
}
