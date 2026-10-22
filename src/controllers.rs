pub mod reports;
pub mod review_prompts;
pub mod searches;
use loco_rs::prelude::*;

pub mod administration;
pub mod api;
pub mod auth;
pub mod dashboard;
pub mod dosage_options;
pub mod dose_occurrences;
pub mod guardian_declarations;
pub mod home;
pub mod identity_onboarding;
pub mod invitations;
pub mod locations;
pub mod medication_orders;
pub mod medications;
pub mod nhs_dmd;
pub mod oauth_server;
pub mod people;
pub mod platform;
pub mod profile;
pub mod treatments;

#[allow(
    clippy::result_large_err,
    reason = "Loco's framework error type contains ordered JSON values"
)]
async fn health() -> Result<Response> {
    format::json(serde_json::json!({"status": "ok", "application": "med-tracker"}))
}

pub fn routes() -> Routes {
    Routes::new()
        .add("/up", get(health))
        .add("/health", get(health))
        .add("/profile-service-worker.js", get(profile_worker))
}

async fn profile_worker() -> impl axum::response::IntoResponse {
    (
        [
            (axum::http::header::CONTENT_TYPE, "application/javascript"),
            (axum::http::header::CACHE_CONTROL, "no-store"),
        ],
        include_str!("../assets/static/js/profile-push-worker.js"),
    )
}

pub fn browser_routes() -> Routes {
    Routes::new()
        .add("/", get(home::index))
        .add("/households/choose", get(home::choose))
        .add("/households", post(home::create))
        .add("/households/select", post(home::select))
}
