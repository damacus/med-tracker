use loco_rs::prelude::*;

pub mod api;
pub mod auth;
pub mod home;
pub mod medications;

async fn health() -> Result<Response> {
    format::json(serde_json::json!({"status": "ok", "application": "med-tracker"}))
}

pub fn routes() -> Routes {
    Routes::new()
        .add("/up", get(health))
        .add("/health", get(health))
}

pub fn browser_routes() -> Routes {
    Routes::new().add("/", get(home::index))
}
