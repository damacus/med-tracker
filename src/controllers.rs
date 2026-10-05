use loco_rs::prelude::*;

async fn index(ViewEngine(view): ViewEngine<TeraView>) -> Result<Response> {
    format::render().view(
        &view,
        "home/index.html",
        data!({"title": "MedTracker migration foundation"}),
    )
}

async fn health() -> Result<Response> {
    format::json(serde_json::json!({"status": "ok", "application": "med-tracker"}))
}

pub fn routes() -> Routes {
    Routes::new()
        .add("/", get(index))
        .add("/up", get(health))
        .add("/health", get(health))
}
