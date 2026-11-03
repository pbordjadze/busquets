//! The device's C ABI, the same for every toy (`toys/ios/Sources/toy.h`; the Swift shell is
//! `toys/ios/Sources/main.swift`). A toy's crate says `toy_kit::ios_shell!(MyToy);` once and its
//! static library exports these `toy_*` functions.

use crate::{block_on, Ctx, DirAssets, Gpu, Host, Phase, PointerEvent, SharedSynth, Toy};

/// A toy on a CAMetalLayer.
pub struct Shell<T: Toy> {
    _instance: wgpu::Instance,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    pub host: Host<T>,
    store: Option<std::path::PathBuf>,
    stats_at: f64,
    frames: u32,
    cost: f64,
}

impl<T: Toy> Shell<T> {
    /// # Safety
    /// `layer` is a live CAMetalLayer.
    pub unsafe fn new(layer: *mut std::ffi::c_void, width: u32, height: u32, scale: f64, assets: &str) -> Option<Shell<T>> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor { backends: wgpu::Backends::METAL, ..Default::default() });
        let surface = match instance.create_surface_unsafe(wgpu::SurfaceTargetUnsafe::CoreAnimationLayer(layer)) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("toy: no surface: {e}");
                return None;
            }
        };
        let gpu = match block_on(Gpu::new(&instance, Some(&surface))) {
            Ok(g) => g,
            Err(e) => {
                eprintln!("toy: no device: {e}");
                return None;
            }
        };
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: gpu.format,
            width: width.max(1),
            height: height.max(1),
            present_mode: wgpu::PresentMode::Fifo,
            desired_maximum_frame_latency: 2,
            alpha_mode: wgpu::CompositeAlphaMode::Opaque,
            view_formats: vec![],
        };
        surface.configure(&gpu.device, &config);
        let ctx = Ctx::new(gpu, Box::new(DirAssets(assets.into())), width.max(1), height.max(1), scale);
        match Host::<T>::new(ctx) {
            Ok(host) => Some(Shell { _instance: instance, surface, config, host, store: None, stats_at: 0.0, frames: 0, cost: 0.0 }),
            Err(e) => {
                eprintln!("toy: {e:#}");
                None
            }
        }
    }

    pub fn resize(&mut self, width: u32, height: u32, scale: f64) {
        self.config.width = width.max(1);
        self.config.height = height.max(1);
        self.surface.configure(&self.host.ctx.gpu.device, &self.config);
        self.host.resize(width, height, scale);
    }

    pub fn frame(&mut self, t: f64) {
        let start = std::time::Instant::now();
        self.host.update(t);
        let frame = match self.surface.get_current_texture() {
            Ok(f) => f,
            Err(wgpu::SurfaceError::Outdated) | Err(wgpu::SurfaceError::Lost) => {
                self.surface.configure(&self.host.ctx.gpu.device, &self.config);
                return;
            }
            Err(e) => {
                eprintln!("toy: frame: {e}");
                return;
            }
        };
        let view = frame.texture.create_view(&Default::default());
        self.host.draw(&view);
        frame.present();
        self.frames += 1;
        self.cost += start.elapsed().as_secs_f64();
        // the frame's CPU cost every five seconds, in the device's log
        if t - self.stats_at >= 5.0 {
            if self.frames > 0 {
                eprintln!("toy: {} frames, {:.2} ms a frame on the CPU", self.frames, self.cost * 1000.0 / self.frames as f64);
            }
            (self.stats_at, self.frames, self.cost) = (t, 0, 0.0);
        }
    }

    pub fn pointer(&mut self, id: u64, phase: u32, x: f64, y: f64, t: f64) {
        let phase = match phase {
            0 => Phase::Down,
            1 => Phase::Move,
            2 => Phase::Up,
            _ => Phase::Cancel,
        };
        self.host.pointer(PointerEvent { id, phase, x, y, t });
    }

    /// Where to keep the toy's state; what is there is taken back now.
    pub fn store(&mut self, path: &str) {
        let p = std::path::PathBuf::from(path);
        if let Ok(s) = std::fs::read_to_string(&p) {
            self.host.toy.restore(&mut self.host.ctx, &s);
        }
        self.store = Some(p);
    }

    pub fn pause(&mut self) {
        self.host.pause();
        if let (Some(p), Some(s)) = (&self.store, self.host.toy.save()) {
            let _ = std::fs::write(p, s);
        }
    }

    pub fn synth(&self) -> Option<SharedSynth> {
        self.host.synth.clone()
    }
}
