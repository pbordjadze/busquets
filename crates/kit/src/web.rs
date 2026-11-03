//! The web shell (patterned on adrift-web): one canvas, WebGPU through wgpu, pointer events in, frames
//! out, the take's sound through a ScriptProcessorNode that starts on the first touch (browsers wait
//! for one), its state kept in localStorage when the page is hidden.

use crate::{Ctx, Gpu, Host, MemoryAssets, Phase, PointerEvent, Toy};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

fn now(perf: &web_sys::Performance) -> f64 {
    perf.now() / 1000.0
}

/// Runs take `T` on the canvas `#<canvas_id>` until the page closes; `key` names its saved state.
pub async fn run<T: Toy>(canvas_id: &str, key: &str) -> Result<(), JsValue> {
    console_error_panic_hook::set_once();
    let window = web_sys::window().unwrap();
    let document = window.document().unwrap();
    let perf = window.performance().unwrap();
    let canvas: web_sys::HtmlCanvasElement = document.get_element_by_id(canvas_id).ok_or("no canvas")?.dyn_into()?;
    if js_sys::Reflect::get(&window.navigator(), &"gpu".into())?.is_undefined() {
        return Err("no WebGPU".into());
    }
    let dpr = window.device_pixel_ratio();
    let rect = canvas.get_bounding_client_rect();
    let (w, h) = (((rect.width() * dpr) as u32).max(1), ((rect.height() * dpr) as u32).max(1));
    canvas.set_width(w);
    canvas.set_height(h);

    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor { backends: wgpu::Backends::BROWSER_WEBGPU, ..Default::default() });
    let surface = instance.create_surface(wgpu::SurfaceTarget::Canvas(canvas.clone())).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let gpu = Gpu::new(&instance, Some(&surface)).await.map_err(|e| JsValue::from_str(&e.to_string()))?;
    let mut config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: gpu.format,
        width: w,
        height: h,
        present_mode: wgpu::PresentMode::Fifo,
        desired_maximum_frame_latency: 2,
        alpha_mode: wgpu::CompositeAlphaMode::Opaque,
        view_formats: vec![],
    };
    surface.configure(&gpu.device, &config);
    let mut ctx = Ctx::new(gpu, Box::new(MemoryAssets(HashMap::new())), w, h, dpr);
    if let Ok(Some(mq)) = window.match_media("(prefers-color-scheme: dark)") {
        ctx.night = mq.matches();
    }
    if let Ok(Some(mq)) = window.match_media("(prefers-reduced-motion: reduce)") {
        ctx.reduce_motion = mq.matches();
    }
    let mut host = Host::<T>::new(ctx).map_err(|e| JsValue::from_str(&format!("{e:#}")))?;
    host.toy.settings(&mut host.ctx);
    if let Ok(Some(store)) = window.local_storage() {
        if let Ok(Some(s)) = store.get_item(key) {
            host.toy.restore(&mut host.ctx, &s);
        }
    }
    let host = Rc::new(RefCell::new(host));

    {
        let (h, w2, key) = (host.clone(), window.clone(), key.to_string());
        let keep = Closure::<dyn FnMut()>::new(move || {
            if let (Ok(Some(store)), Some(s)) = (w2.local_storage(), h.borrow().toy.save()) {
                let _ = store.set_item(&key, &s);
            }
        });
        document.add_event_listener_with_callback("visibilitychange", keep.as_ref().unchecked_ref())?;
        window.add_event_listener_with_callback("pagehide", keep.as_ref().unchecked_ref())?;
        keep.forget();
    }

    // the sound starts with the first touch
    let audio: Rc<RefCell<Option<web_sys::AudioContext>>> = Rc::new(RefCell::new(None));
    let start_audio = {
        let (audio, host) = (audio.clone(), host.clone());
        move || {
            if audio.borrow().is_some() {
                return;
            }
            let Some(synth) = host.borrow().synth.clone() else { return };
            let Ok(ac) = web_sys::AudioContext::new() else { return };
            let rate = ac.sample_rate() as f64;
            let Ok(node) = ac.create_script_processor_with_buffer_size_and_number_of_input_channels_and_number_of_output_channels(2048, 0, 2) else { return };
            let mut buf = vec![0.0f32; 4096];
            let cb = Closure::<dyn FnMut(web_sys::AudioProcessingEvent)>::new(move |e: web_sys::AudioProcessingEvent| {
                let Ok(out) = e.output_buffer() else { return };
                let n = out.length() as usize;
                if buf.len() < n * 2 {
                    buf.resize(n * 2, 0.0);
                }
                Host::<T>::audio(Some(&synth), &mut buf[..n * 2], 2, rate);
                let (mut l, mut r) = (vec![0.0f32; n], vec![0.0f32; n]);
                for i in 0..n {
                    l[i] = buf[2 * i];
                    r[i] = buf[2 * i + 1];
                }
                let _ = out.copy_to_channel(&l, 0);
                let _ = out.copy_to_channel(&r, 1);
            });
            node.set_onaudioprocess(Some(cb.as_ref().unchecked_ref()));
            cb.forget();
            let _ = node.connect_with_audio_node(&ac.destination());
            let _ = ac.resume();
            *audio.borrow_mut() = Some(ac);
        }
    };
    let start_audio = Rc::new(start_audio);

    for (name, phase) in [("pointerdown", Phase::Down), ("pointermove", Phase::Move), ("pointerup", Phase::Up), ("pointercancel", Phase::Cancel)] {
        let (h, perf, c, sa) = (host.clone(), perf.clone(), canvas.clone(), start_audio.clone());
        let cb = Closure::<dyn FnMut(web_sys::PointerEvent)>::new(move |e: web_sys::PointerEvent| {
            let r = c.get_bounding_client_rect();
            let (x, y) = (e.client_x() as f64 - r.left(), e.client_y() as f64 - r.top());
            if phase == Phase::Down {
                let _ = c.set_pointer_capture(e.pointer_id());
                sa();
            }
            e.prevent_default();
            h.borrow_mut().pointer(PointerEvent { id: e.pointer_id() as u64, phase, x, y, t: now(&perf) });
        });
        canvas.add_event_listener_with_callback(name, cb.as_ref().unchecked_ref())?;
        cb.forget();
    }

    let f: Rc<RefCell<Option<Closure<dyn FnMut()>>>> = Rc::new(RefCell::new(None));
    let g = f.clone();
    let (canvas2, window2, host2, perf2) = (canvas.clone(), window.clone(), host.clone(), perf.clone());
    *g.borrow_mut() = Some(Closure::new(move || {
        let dpr = window2.device_pixel_ratio();
        let r = canvas2.get_bounding_client_rect();
        let (w, h) = (((r.width() * dpr) as u32).max(1), ((r.height() * dpr) as u32).max(1));
        {
            let mut host = host2.borrow_mut();
            if w != config.width || h != config.height {
                canvas2.set_width(w);
                canvas2.set_height(h);
                config.width = w;
                config.height = h;
                surface.configure(&host.ctx.gpu.device, &config);
                host.resize(w, h, dpr);
            }
            host.update(now(&perf2));
            if let Ok(frame) = surface.get_current_texture() {
                let view = frame.texture.create_view(&Default::default());
                host.draw(&view);
                frame.present();
            }
            if host.take_haptic() {
                let _ = js_sys::Reflect::get(&window2.navigator(), &"vibrate".into())
                    .ok()
                    .and_then(|v| v.dyn_into::<js_sys::Function>().ok())
                    .map(|v| v.call1(&window2.navigator(), &JsValue::from(8)));
            }
        }
        window2.request_animation_frame(f.borrow().as_ref().unwrap().as_ref().unchecked_ref()).unwrap();
    }));
    window.request_animation_frame(g.borrow().as_ref().unwrap().as_ref().unchecked_ref())?;
    Ok(())
}
