//! Where the data lives, and the loader. Nothing is kept in this repository: on the web the viewer's
//! browser fetches it from StatsBomb's own GitHub (their user agreement: no redistribution); the
//! headless check reads a local cache (`tools/fetch-statsbomb.sh`, outside git); on the device the
//! Swift shell's URLSession fetches it (`toy_fetch`, ios/Sources/main.swift), at run time too.
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

pub const BASE: &str = "https://raw.githubusercontent.com/statsbomb/open-data/master/data";

pub fn events_url(id: u32) -> String {
    format!("{BASE}/events/{id}.json")
}
pub fn frames_url(id: u32) -> String {
    format!("{BASE}/three-sixty/{id}.json")
}

/// A path's state.
#[derive(Clone)]
pub enum Got {
    Waiting,
    Bytes(Rc<Vec<u8>>),
    Failed(String),
}

/// Asks for files and tells you, frame by frame, which have come.
#[derive(Clone, Default)]
pub struct Loader {
    got: Rc<RefCell<HashMap<String, Got>>>,
}

impl Loader {
    pub fn new() -> Loader {
        Loader::default()
    }

    /// `rel` is relative to the data root ("events/3794686.json").
    pub fn ask(&self, rel: &str) {
        if self.got.borrow().contains_key(rel) {
            return;
        }
        self.got.borrow_mut().insert(rel.to_string(), Got::Waiting);
        self.start(rel.to_string());
    }

    pub fn get(&self, rel: &str) -> Got {
        #[cfg(target_os = "ios")]
        device::collect(&self.got);
        self.got.borrow().get(rel).cloned().unwrap_or(Got::Waiting)
    }

    /// Lets go of a file's bytes once read.
    pub fn forget(&self, rel: &str) {
        self.got.borrow_mut().insert(rel.to_string(), Got::Failed("used".into()));
    }

    #[cfg(target_os = "ios")]
    fn start(&self, rel: String) {
        device::start(rel);
    }

    #[cfg(all(not(target_arch = "wasm32"), not(target_os = "ios")))]
    fn start(&self, rel: String) {
        // the headless check: a local cache (tools/fetch-statsbomb.sh), synchronously
        let dir = std::env::var("BUSQUETS_DATA").unwrap_or_else(|_| "data/statsbomb".into());
        let path = std::path::Path::new(&dir).join(&rel);
        let v = match std::fs::read(&path) {
            Ok(b) => Got::Bytes(Rc::new(b)),
            Err(e) => Got::Failed(format!("{}: {e}", path.display())),
        };
        self.got.borrow_mut().insert(rel, v);
    }

    #[cfg(target_arch = "wasm32")]
    fn start(&self, rel: String) {
        use wasm_bindgen::JsCast;
        let got = self.got.clone();
        wasm_bindgen_futures::spawn_local(async move {
            let url = format!("{BASE}/{rel}");
            let r: Result<Vec<u8>, String> = async {
                let w = web_sys::window().ok_or("no window")?;
                let resp = wasm_bindgen_futures::JsFuture::from(w.fetch_with_str(&url)).await.map_err(|e| format!("{e:?}"))?;
                let resp: web_sys::Response = resp.dyn_into().map_err(|_| "not a response")?;
                if !resp.ok() {
                    return Err(format!("{} {}", resp.status(), url));
                }
                let buf = wasm_bindgen_futures::JsFuture::from(resp.array_buffer().map_err(|e| format!("{e:?}"))?).await.map_err(|e| format!("{e:?}"))?;
                Ok(js_sys::Uint8Array::new(&buf).to_vec())
            }
            .await;
            got.borrow_mut().insert(rel, match r {
                Ok(b) => Got::Bytes(Rc::new(b)),
                Err(e) => Got::Failed(e),
            });
        });
    }
}

/// On the device: the Swift shell fetches (URLSession) and calls back on its own thread; what has come
/// waits here until the next `get` on the frame's thread.
#[cfg(target_os = "ios")]
mod device {
    use super::{Got, BASE};
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::ffi::{c_char, CString};
    use std::rc::Rc;
    use std::sync::Mutex;

    type Done = extern "C" fn(token: u64, bytes: *const u8, len: usize, ok: bool);
    extern "C" {
        /// ios/Sources/main.swift: fetches `url` and calls `done(token, …)` when it has.
        fn toy_fetch(url: *const c_char, token: u64, done: Done);
    }

    static ASKED: Mutex<Vec<(u64, String)>> = Mutex::new(Vec::new());
    static CAME: Mutex<Vec<(u64, Result<Vec<u8>, String>)>> = Mutex::new(Vec::new());
    static NEXT: Mutex<u64> = Mutex::new(1);

    extern "C" fn done(token: u64, bytes: *const u8, len: usize, ok: bool) {
        let r = if ok && !bytes.is_null() { Ok(unsafe { std::slice::from_raw_parts(bytes, len) }.to_vec()) } else { Err("the fetch failed".to_string()) };
        if let Ok(mut c) = CAME.lock() {
            c.push((token, r));
        }
    }

    pub fn start(rel: String) {
        let token = {
            let mut n = NEXT.lock().unwrap();
            *n += 1;
            *n
        };
        let url = CString::new(format!("{BASE}/{rel}")).unwrap();
        ASKED.lock().unwrap().push((token, rel));
        unsafe { toy_fetch(url.as_ptr(), token, done) };
    }

    pub fn collect(got: &Rc<RefCell<HashMap<String, Got>>>) {
        let came: Vec<_> = match CAME.lock() {
            Ok(mut c) => c.drain(..).collect(),
            Err(_) => return,
        };
        if came.is_empty() {
            return;
        }
        let mut asked = ASKED.lock().unwrap();
        for (token, r) in came {
            if let Some(i) = asked.iter().position(|a| a.0 == token) {
                let (_, rel) = asked.remove(i);
                got.borrow_mut().insert(rel, match r {
                    Ok(b) => Got::Bytes(Rc::new(b)),
                    Err(e) => Got::Failed(e),
                });
            }
        }
    }
}
