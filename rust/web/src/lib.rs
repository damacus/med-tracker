use axum::{Router, http::header, response::Html, routing::get};
use leptos::prelude::*;

pub use loom_ui::BROWSER_RUNTIME as UI_BROWSER_RUNTIME;

mod auth;
mod document;
mod medication;

pub mod admin;
pub mod dashboard;
pub mod dosage_options;
pub mod household;
pub mod household_i18n;
pub mod locations;
pub mod medication_management;
pub mod notifications;
pub mod people;
pub mod profile_advanced;
pub mod profile_security;
pub mod rails_time_zones;
pub mod reports;
pub mod settings;
pub mod stock;
pub mod treatments;

use auth::BrandPanel;
use document::{authenticated_document, medication_document};

pub use auth::{PasskeyLogin, render_consent, render_login, render_reset_unavailable};
pub use medication::{
    DoseFormState, DoseSource, MedicationCard, MedicationDetail, MedicationDetailRender,
    render_medication_detail, render_medication_detail_with_management,
    render_medication_detail_with_stock, render_medication_detail_with_stock_inventory,
    render_medication_list, render_medication_list_with_management,
    render_medication_list_with_stock,
};

pub fn render_dashboard(household_name: &str, csrf: &str, empty: bool) -> String {
    let body = view! {
        <main class="auth-page">
            <div class="auth-shell">
                <BrandPanel/>
                <section class="form-panel" aria-label="Household dashboard">
                    <p class="eyebrow">"YOUR HOUSEHOLD"</p>
                    <h1>{household_name.to_owned()}</h1>
                    {empty.then(|| view! { <p class="form-intro">"You do not have an active household yet."</p> })}
                    <form action="/logout" method="post">
                        <input type="hidden" name="authenticity_token" value=csrf.to_owned()/>
                        <button class="primary-button" type="submit">"Sign out"</button>
                    </form>
                </section>
            </div>
        </main>
    }
    .to_html();
    authenticated_document("Dashboard", csrf, body)
}

#[derive(Clone)]
pub struct HistoryRow {
    pub medication_name: String,
    pub amount: String,
    pub unit: String,
}

pub fn render_journey_dashboard(
    household_name: &str,
    slug: &str,
    csrf: &str,
    history: Vec<HistoryRow>,
) -> String {
    let prefix = format!("/households/{slug}");
    let body = view! {
        <main class="med-app">
            <header class="med-topbar"><a class="med-brand" href=format!("{prefix}/dashboard")>"MedTracker"</a><span>{household_name.to_owned()}</span></header>
            <div class="med-layout"><nav class="med-sidebar" aria-label="Household"><a aria-current="page" href=format!("{prefix}/dashboard")>"Dashboard"</a><a href=format!("{prefix}/medications")>"Inventory"</a></nav>
                <section class="med-content"><p class="med-eyebrow">"TODAY"</p><h1>{household_name.to_owned()}</h1><a class="med-button" href=format!("{prefix}/medications")>"View inventory"</a>
                    <section class="med-card med-history" data-testid="dashboard-today-dose-history"><h2>"Previous Doses Today"</h2>
                        {history.into_iter().map(|row| view! { <div class="med-history-row"><strong>{row.medication_name}</strong><span>{row.amount}" "{row.unit}</span></div> }).collect_view()}
                    </section>
                    <form action="/logout" method="post"><input type="hidden" name="authenticity_token" value=csrf.to_owned()/><button type="submit" class="med-text-button">"Sign out"</button></form>
                </section>
            </div>
        </main>
    }.to_html();
    medication_document("Dashboard", csrf, body)
}

include!(concat!(env!("OUT_DIR"), "/rails-fonts.rs"));

pub fn rails_design_stylesheet() -> &'static str {
    include_str!(concat!(env!("OUT_DIR"), "/rails-design.css"))
}

pub fn stylesheet() -> &'static str {
    include_str!("auth.css")
}

pub fn medication_stylesheet() -> &'static str {
    include_str!("medication.css")
}

pub fn medication_script() -> &'static str {
    include_str!("medication.js")
}

pub fn passkey_script() -> &'static str {
    include_str!("auth-passkey.js")
}

async fn login() -> Html<String> {
    Html(render_login("", "", None))
}

async fn styles() -> ([(header::HeaderName, &'static str); 1], &'static str) {
    (
        [(header::CONTENT_TYPE, "text/css; charset=utf-8")],
        stylesheet(),
    )
}

async fn health() -> &'static str {
    "ok"
}

pub fn app() -> Router {
    Router::new()
        .route("/login", get(login))
        .route("/auth.css", get(styles))
        .route("/health", get(health))
}
