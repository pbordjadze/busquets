//! The web build: `start("<take>")` runs that take on the page's `#take` canvas (web/index.html).
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub async fn start(take: String) -> Result<(), JsValue> {
    let key = format!("busquets-{take}");
    match take.as_str() {
        "pivot" => kit::web::run::<pivot::Pivot>("take", &key).await,
        "pulse" => kit::web::run::<pulse::Pulse>("take", &key).await,
        "garden" => kit::web::run::<garden::Garden>("take", &key).await,
        "watch" => kit::web::run::<watch::Watch>("take", &key).await,
        "eyes" => kit::web::run::<eyes::Eyes>("take", &key).await,
        "ghost" => kit::web::run::<ghost::Ghost>("take", &key).await,
        "light" => kit::web::run::<light::Light>("take", &key).await,
        "night" => kit::web::run::<studies::Night>("take", &key).await,
        "paper" => kit::web::run::<studies::Paper>("take", &key).await,
        "lines" => kit::web::run::<studies::Lines>("take", &key).await,
        "wash" => kit::web::run::<studies::Wash>("take", &key).await,
        _ => Err(JsValue::from_str("no such take")),
    }
}
