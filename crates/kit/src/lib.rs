//! kit: what every take stands on. A take is one type implementing [`Toy`]; the kit runs it on the
//! device (`ios_shell!`, the C ABI the Swift shell in `ios/` calls), in a browser (`web::run`, WebGPU),
//! headless on Linux (`check::main`, plans of fingers played to PNG and WAV on Mesa's software
//! Vulkan), and gives it the GPU, a painter of soft 2D shapes (`paint`), a full-screen backdrop
//! (`backdrop`), persistent layers, and a sound out (`Synth`, `sound::Bells`).
//!
//! Vendored from pbordjadze/terra-incognita `toys/crates/toy-kit` at 6bf0613 (Nov 3, 2026), the
//! scaffold Adrift's toys stand on (NOTICE.md): `Toy`, `Host`, `Ctx`, `Synth`, `ios_shell!`, the check
//! and the GPU are theirs, trimmed of the globe; `paint`, `backdrop`, `layer`, `sound` and `web` are
//! new here. Change it here freely: nothing upstream depends on it.

pub mod assets;
pub mod backdrop;
#[cfg(not(target_arch = "wasm32"))]
pub mod check;
pub mod gpu;
pub mod layer;
pub mod model;
pub mod paint;
#[cfg(target_os = "ios")]
pub mod shell;
pub mod sound;
#[cfg(target_arch = "wasm32")]
pub mod web;

pub use assets::{Assets, DirAssets, MemoryAssets};
pub use gpu::{Gpu, Offscreen};
pub use model::*;

use std::sync::{Arc, Mutex};

/// What a toy is given: the device, its data, the screen, the reader's settings, and a way back to
/// the shell (a haptic tap).
pub struct Ctx {
    pub gpu: Gpu,
    pub assets: Box<dyn Assets>,
    /// The target's size in pixels, and pixels a point.
    pub width: u32,
    pub height: u32,
    pub ppp: f64,
    /// The system's dark appearance.
    pub night: bool,
    /// The reader's Reduce Motion.
    pub reduce_motion: bool,
    haptic: bool,
}

impl Ctx {
    pub fn new(gpu: Gpu, assets: Box<dyn Assets>, width: u32, height: u32, ppp: f64) -> Ctx {
        Ctx { gpu, assets, width, height, ppp, night: false, reduce_motion: false, haptic: false }
    }

    /// Asks the shell for one light haptic tap this frame.
    pub fn haptic(&mut self) {
        self.haptic = true;
    }

    /// The screen's size in points.
    pub fn points(&self) -> [f64; 2] {
        [self.width as f64 / self.ppp, self.height as f64 / self.ppp]
    }
}

/// One toy. Everything it shows it draws itself into the target, every pixel (there is no chrome
/// around it); everything it hears is a finger.
pub trait Toy: 'static {
    fn new(ctx: &mut Ctx) -> anyhow::Result<Self>
    where
        Self: Sized;
    /// The screen changed size (`ctx.width`, `height`, `ppp` are the new ones).
    fn resize(&mut self, _ctx: &mut Ctx) {}
    /// A finger, in points from the top left.
    fn pointer(&mut self, ctx: &mut Ctx, e: PointerEvent);
    /// Time moves: `t` the shell's seconds, `dt` since the last frame (capped at 0.1 s). Simulation
    /// lives here (GPU compute too), never in `draw`, so a headless check that draws only some frames
    /// sees the same world.
    fn update(&mut self, ctx: &mut Ctx, t: f64, dt: f64);
    /// Draws the frame into `target` (format `ctx.gpu.format`), every pixel.
    fn draw(&mut self, ctx: &mut Ctx, target: &wgpu::TextureView);
    /// The toy's sound, once, at start: it runs on the audio thread from then on (talk to it through
    /// an `Arc` of atomics or a `Mutex` the toy keeps a clone of). None: a silent toy.
    fn synth(&mut self) -> Option<Box<dyn Synth>> {
        None
    }
    /// The device was shaken.
    fn shaken(&mut self, _ctx: &mut Ctx) {}
    /// The appearance or Reduce Motion changed (read `ctx.night`, `ctx.reduce_motion`).
    fn settings(&mut self, _ctx: &mut Ctx) {}
    /// What to keep when the app goes to the background (any text; JSON is usual), and to take back.
    fn save(&self) -> Option<String> {
        None
    }
    fn restore(&mut self, _ctx: &mut Ctx, _saved: &str) {}
}

/// A toy's sound: fills interleaved samples (`channels` of them a frame, at `rate` Hz), -1..1.
pub trait Synth: Send {
    fn render(&mut self, out: &mut [f32], channels: usize, rate: f64);
}

pub type SharedSynth = Arc<Mutex<Box<dyn Synth>>>;

/// A toy and its context, as every shell drives it.
pub struct Host<T: Toy> {
    pub ctx: Ctx,
    pub toy: T,
    pub synth: Option<SharedSynth>,
    last_t: Option<f64>,
}

impl<T: Toy> Host<T> {
    pub fn new(mut ctx: Ctx) -> anyhow::Result<Host<T>> {
        let mut toy = T::new(&mut ctx)?;
        let synth = toy.synth().map(|s| Arc::new(Mutex::new(s)));
        Ok(Host { ctx, toy, synth, last_t: None })
    }

    pub fn resize(&mut self, width: u32, height: u32, ppp: f64) {
        (self.ctx.width, self.ctx.height, self.ctx.ppp) = (width.max(1), height.max(1), ppp);
        self.toy.resize(&mut self.ctx);
    }

    pub fn pointer(&mut self, e: PointerEvent) {
        self.toy.pointer(&mut self.ctx, e);
    }

    /// Time to `t`; the first call only sets the clock.
    pub fn update(&mut self, t: f64) {
        let dt = match self.last_t {
            Some(l) => (t - l).clamp(0.0, 0.1),
            None => 0.0,
        };
        self.last_t = Some(t);
        self.toy.update(&mut self.ctx, t, dt);
    }

    pub fn draw(&mut self, target: &wgpu::TextureView) {
        self.toy.draw(&mut self.ctx, target);
    }

    /// Forgets the clock (after the app was away: the next frame's dt is 0, not the time away).
    pub fn pause(&mut self) {
        self.last_t = None;
    }

    pub fn set_night(&mut self, on: bool) {
        self.ctx.night = on;
        self.toy.settings(&mut self.ctx);
    }

    pub fn set_reduce_motion(&mut self, on: bool) {
        self.ctx.reduce_motion = on;
        self.toy.settings(&mut self.ctx);
    }

    /// Whether the toy asked for a haptic tap since the last call.
    pub fn take_haptic(&mut self) -> bool {
        std::mem::take(&mut self.ctx.haptic)
    }

    /// Sound for the next `out.len() / channels` frames (silence without a synth or while locked).
    pub fn audio(synth: Option<&SharedSynth>, out: &mut [f32], channels: usize, rate: f64) {
        out.fill(0.0);
        if let Some(s) = synth {
            if let Ok(mut s) = s.try_lock() {
                s.render(out, channels, rate);
            }
        }
    }
}

/// A small executor for wgpu's futures (they are ready at once, or nearly).
pub fn block_on<F: std::future::Future>(f: F) -> F::Output {
    use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};
    fn noop(_: *const ()) {}
    fn clone(p: *const ()) -> RawWaker {
        RawWaker::new(p, &VTABLE)
    }
    static VTABLE: RawWakerVTable = RawWakerVTable::new(clone, noop, noop, noop);
    let waker = unsafe { Waker::from_raw(RawWaker::new(std::ptr::null(), &VTABLE)) };
    let mut cx = Context::from_waker(&waker);
    let mut f = std::pin::pin!(f);
    loop {
        if let Poll::Ready(v) = f.as_mut().poll(&mut cx) {
            return v;
        }
        std::thread::yield_now();
    }
}

/// The C ABI over a toy type: `toy_kit::ios_shell!(MyToy);` in the toy's `lib.rs`. Compiled only for
/// iOS (the static library the shell links).
#[macro_export]
macro_rules! ios_shell {
    ($toy:ty) => {
        #[cfg(target_os = "ios")]
        pub mod ffi {
            use super::*;
            use std::ffi::{c_char, c_void, CStr};
            type S = $crate::shell::Shell<$toy>;

            #[no_mangle]
            pub unsafe extern "C" fn toy_new(layer: *mut c_void, width: u32, height: u32, scale: f64, assets: *const c_char) -> *mut S {
                let dir = CStr::from_ptr(assets).to_string_lossy().into_owned();
                match S::new(layer, width, height, scale, &dir) {
                    Some(s) => Box::into_raw(Box::new(s)),
                    None => std::ptr::null_mut(),
                }
            }
            #[no_mangle]
            pub unsafe extern "C" fn toy_resize(s: *mut S, width: u32, height: u32, scale: f64) {
                if let Some(s) = s.as_mut() { s.resize(width, height, scale) }
            }
            #[no_mangle]
            pub unsafe extern "C" fn toy_frame(s: *mut S, t: f64) {
                if let Some(s) = s.as_mut() { s.frame(t) }
            }
            #[no_mangle]
            pub unsafe extern "C" fn toy_pointer(s: *mut S, id: u64, phase: u32, x: f64, y: f64, t: f64) {
                if let Some(s) = s.as_mut() { s.pointer(id, phase, x, y, t) }
            }
            #[no_mangle]
            pub unsafe extern "C" fn toy_shaken(s: *mut S) {
                if let Some(s) = s.as_mut() { s.host.toy.shaken(&mut s.host.ctx) }
            }
            #[no_mangle]
            pub unsafe extern "C" fn toy_take_haptic(s: *mut S) -> bool {
                s.as_mut().map(|s| s.host.take_haptic()).unwrap_or(false)
            }
            #[no_mangle]
            pub unsafe extern "C" fn toy_set_night(s: *mut S, on: bool) {
                if let Some(s) = s.as_mut() { s.host.set_night(on) }
            }
            #[no_mangle]
            pub unsafe extern "C" fn toy_set_reduce_motion(s: *mut S, on: bool) {
                if let Some(s) = s.as_mut() { s.host.set_reduce_motion(on) }
            }
            #[no_mangle]
            pub unsafe extern "C" fn toy_store(s: *mut S, path: *const c_char) {
                if let Some(s) = s.as_mut() { s.store(&CStr::from_ptr(path).to_string_lossy()) }
            }
            #[no_mangle]
            pub unsafe extern "C" fn toy_pause(s: *mut S) {
                if let Some(s) = s.as_mut() { s.pause() }
            }
            /// The toy's sound, for the audio thread (null: silent). Freed by `toy_synth_free`.
            #[no_mangle]
            pub unsafe extern "C" fn toy_synth(s: *mut S) -> *mut $crate::SharedSynth {
                match s.as_ref().and_then(|s| s.synth()) {
                    Some(a) => Box::into_raw(Box::new(a)),
                    None => std::ptr::null_mut(),
                }
            }
            /// Fills `frames` interleaved frames of `channels` floats at `rate` Hz (the audio thread).
            #[no_mangle]
            pub unsafe extern "C" fn toy_audio(synth: *mut $crate::SharedSynth, out: *mut f32, frames: u32, channels: u32, rate: f64) {
                let out = std::slice::from_raw_parts_mut(out, (frames * channels) as usize);
                $crate::Host::<$toy>::audio(synth.as_ref(), out, channels as usize, rate);
            }
            #[no_mangle]
            pub unsafe extern "C" fn toy_synth_free(synth: *mut $crate::SharedSynth) {
                if !synth.is_null() { drop(Box::from_raw(synth)) }
            }
            #[no_mangle]
            pub unsafe extern "C" fn toy_free(s: *mut S) {
                if !s.is_null() { drop(Box::from_raw(s)) }
            }
        }
    };
}
