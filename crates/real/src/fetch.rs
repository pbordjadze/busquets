//! Where the data lives, and the loader. Nothing is kept in this repository: on the web the viewer's
//! browser fetches it from StatsBomb's own GitHub (their user agreement: no redistribution); the
//! headless check reads a local cache (`tools/fetch-statsbomb.sh`, outside git).
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
        self.got.borrow().get(rel).cloned().unwrap_or(Got::Waiting)
    }

    /// Lets go of a file's bytes once read.
    pub fn forget(&self, rel: &str) {
        self.got.borrow_mut().insert(rel.to_string(), Got::Failed("used".into()));
    }

    #[cfg(not(target_arch = "wasm32"))]
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
