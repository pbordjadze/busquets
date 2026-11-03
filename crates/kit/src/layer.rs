//! A persistent layer: a texture the size of the screen that keeps what is painted into it (nothing
//! clears it unless asked), laid over the frame with `over`. The garden's drawing lives in one.

use crate::Gpu;

const WGSL: &str = r#"
@group(0) @binding(0) var t: texture_2d<f32>;
@group(0) @binding(1) var s: sampler;
struct VO { @builtin(position) pos: vec4<f32>, @location(0) uv: vec2<f32> };
@vertex fn vs_main(@builtin(vertex_index) vi: u32) -> VO {
  let p = array<vec2<f32>, 3>(vec2(-1.0, -1.0), vec2(3.0, -1.0), vec2(-1.0, 3.0))[vi];
  var o: VO; o.pos = vec4<f32>(p, 0.0, 1.0); o.uv = vec2<f32>((p.x + 1.0) * 0.5, (1.0 - p.y) * 0.5); return o;
}
@fragment fn fs_main(i: VO) -> @location(0) vec4<f32> { return textureSample(t, s, i.uv); }
"#;

pub struct Layer {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub size: [u32; 2],
    pipeline: wgpu::RenderPipeline,
    bind: wgpu::BindGroup,
}

impl Layer {
    pub fn new(gpu: &Gpu, size: [u32; 2]) -> Layer {
        let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("layer"),
            size: wgpu::Extent3d { width: size[0].max(1), height: size[1].max(1), depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: gpu.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&Default::default());
        let sampler = gpu.device.create_sampler(&wgpu::SamplerDescriptor { mag_filter: wgpu::FilterMode::Linear, min_filter: wgpu::FilterMode::Linear, ..Default::default() });
        let bgl = gpu.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("layer"),
            entries: &[
                wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: true }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 1, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering), count: None },
            ],
        });
        let bind = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("layer"),
            layout: &bgl,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&view) }, wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&sampler) }],
        });
        let shader = gpu.shader("layer", WGSL);
        let pipeline = gpu.fullscreen_pipeline("layer", &shader, &[&bgl], Some(Gpu::over()));
        Layer { texture, view, size, pipeline, bind }
    }

    /// Empties the layer (transparent).
    pub fn clear(&self, enc: &mut wgpu::CommandEncoder) {
        let _ = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("layer clear"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment { view: &self.view, resolve_target: None, depth_slice: None, ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT), store: wgpu::StoreOp::Store } })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
    }

    /// Lays the layer over `target`.
    pub fn composite(&self, enc: &mut wgpu::CommandEncoder, target: &wgpu::TextureView) {
        let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("layer over"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment { view: target, resolve_target: None, depth_slice: None, ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store } })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind, &[]);
        pass.draw(0..3, 0..1);
    }
}
