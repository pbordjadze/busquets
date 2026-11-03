//! The web build: `start("<take>")` runs that take on the page's `#take` canvas (web/index.html).
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub async fn start(take: String) -> Result<(), JsValue> {
    start_on(take, "take".into()).await
}

/// Runs `take` on the canvas with id `canvas` (several takes on one page: the studies gallery).
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub async fn start_on(take: String, canvas: String) -> Result<(), JsValue> {
    let key = format!("busquets-{take}");
    match take.as_str() {
        "pivot" => kit::web::run::<pivot::Pivot>(&canvas, &key).await,
        "pulse" => kit::web::run::<pulse::Pulse>(&canvas, &key).await,
        "garden" => kit::web::run::<garden::Garden>(&canvas, &key).await,
        "watch" => kit::web::run::<watch::Watch>(&canvas, &key).await,
        "eyes" => kit::web::run::<eyes::Eyes>(&canvas, &key).await,
        "night" => kit::web::run::<studies::Night>(&canvas, &key).await,
        "paper" => kit::web::run::<studies::Paper>(&canvas, &key).await,
        "lines" => kit::web::run::<studies::Lines>(&canvas, &key).await,
        "wash" => kit::web::run::<studies::Wash>(&canvas, &key).await,
        "ghost" => kit::web::run::<ghost::Ghost>(&canvas, &key).await,
        "light" => kit::web::run::<light::Light>(&canvas, &key).await,
        _ => Err(JsValue::from_str("no such take")),
    }
}
