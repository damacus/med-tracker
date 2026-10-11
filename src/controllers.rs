pub mod reports;
use loco_rs::prelude::*;

pub mod administration;
pub mod api;
pub mod auth;
pub mod dosage_options;
pub mod dose_occurrences;
pub mod home;
pub mod identity_onboarding;
pub mod invitations;
pub mod locations;
pub mod medication_orders;
pub mod medications;
pub mod nhs_dmd;
pub mod oauth_server;
pub mod people;
pub mod signup;
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
}

pub fn browser_routes() -> Routes {
    Routes::new().add("/", get(home::index))
}
