use crate::{oauth, AppState};
use axum::extract::{Form, Path, Query, State};
use axum::http::{header, HeaderMap, HeaderValue, Method, StatusCode};
use axum::response::Response;
use axum::routing::{get, post};
use axum::Router;
use serde_json::{json, Value};
use std::collections::HashMap;

use api_client::WebApi;
use response::{error, failure, login_redirect, page, page_status, redirect, PageError};
pub(crate) use time::dashboard_now;

mod admin;
mod api_client;
mod assets;
mod dashboard;
mod dashboard_projection;
mod dosage_options;
mod doses;
mod inventory;
mod locations;
mod medications;
mod notifications;
mod people;
mod profile_advanced;
mod profile_security;
mod response;
mod settings;
mod stock;
mod time;
mod treatments;

#[cfg(test)]
mod tests;

pub fn routes() -> Router<AppState> {
    Router::new()
        .merge(people::routes())
        .merge(locations::routes())
        .merge(medications::routes())
        .merge(notifications::routes())
        .merge(dosage_options::routes())
        .merge(stock::routes())
        .merge(treatments::routes())
        .merge(admin::routes())
        .merge(settings::routes())
        .merge(profile_security::routes())
        .merge(profile_advanced::routes())
        .route("/household.css", get(assets::household_styles))
        .route("/profile.css", get(assets::profile_styles))
        .route("/profile.js", get(assets::profile_script))
        .route("/households/{slug}/dashboard", get(dashboard::dashboard))
        .route(
            "/households/{slug}/medications",
            get(inventory::medications),
        )
        .route(
            "/households/{slug}/medications/{id}",
            get(inventory::medication),
        )
        .route(
            "/households/{slug}/medications/{id}/doses",
            post(doses::record_dose),
        )
        .route("/medication.css", get(assets::styles))
        .route("/medication.js", get(assets::script))
        .route("/dashboard.css", get(assets::dashboard_styles))
        .route("/dashboard.js", get(assets::dashboard_script))
        .route("/leptodon.css", get(assets::leptodon_styles))
        .route(
            "/dashboard-hydrate.js",
            get(assets::dashboard_hydrate_script),
        )
        .route(
            "/dashboard-hydrate-pkg.js",
            get(assets::dashboard_hydrate_package),
        )
        .route(
            "/dashboard-hydrate.wasm",
            get(assets::dashboard_hydrate_wasm),
        )
        .route("/sw.js", get(assets::dashboard_worker))
        .route("/manifest.webmanifest", get(assets::dashboard_manifest))
        .route("/offline", get(assets::dashboard_offline))
        .route("/reconnect", get(assets::dashboard_reconnect))
        .route("/icons/icon-192.png", get(assets::dashboard_icon_192))
        .route("/icons/icon-512.png", get(assets::dashboard_icon_512))
        .route("/fonts/inter-regular.woff2", get(assets::inter_regular))
        .route("/fonts/inter-500.woff2", get(assets::inter_500))
        .route("/fonts/inter-800.woff2", get(assets::inter_800))
        .route("/fonts/inter-600.woff2", get(assets::inter_600))
        .route("/fonts/inter-700.woff2", get(assets::inter_700))
        .route("/fonts/plus-jakarta-sans/{file}", get(assets::plus_jakarta_font))
}

fn field<'a>(value: &'a Value, key: &str) -> &'a str {
    value.get(key).and_then(Value::as_str).unwrap_or("")
}

fn numeric(value: &Value, key: &str) -> Option<i64> {
    value.get(key).and_then(Value::as_i64)
}
