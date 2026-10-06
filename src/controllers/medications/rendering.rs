use crate::models::care::{
    browser_query::{Detail, MedicationCard},
    medications::StockSnapshot,
};
use crate::models::errors::OperationError;
use axum::{
    http::{StatusCode, header},
    response::IntoResponse,
};
use axum_csrf::CsrfToken;
use loco_rs::prelude::*;
use serde_json::{Value, json};
use std::collections::HashMap;

fn context(
    slug: &str,
    snapshot: &StockSnapshot,
    draft: &HashMap<String, String>,
    error: Option<String>,
) -> Value {
    let name = snapshot.representation["data"]["display_name"]
        .as_str()
        .unwrap_or("Medication");
    let quantity = snapshot
        .medication
        .current_supply
        .map(|value| value.normalize().to_string())
        .unwrap_or_else(|| "Untracked".into());
    let unit = snapshot.medication.dose_unit.as_deref().unwrap_or_default();
    let mut data = appearance_context();
    data["title"] = json!(name);
    data["slug"] = json!(slug);
    data["medication_id"] = json!(snapshot.medication.id);
    data["name"] = json!(name);
    data["quantity"] = json!(quantity);
    data["unit"] = json!(unit_label(unit, &quantity));
    data["draft"] = json!(draft);
    data["error"] = json!(error);
    data
}

pub(crate) fn appearance_context() -> Value {
    json!({ "allow_palette":true,
        "appearances":[{"id":"light","label":"Light"},{"id":"dark","label":"Dark"},{"id":"system","label":"System"}],
        "palettes":[{"id":"default","label":"Command Centre"},{"id":"serene-sage","label":"Serene Sage"},{"id":"modern-clinical","label":"Modern Clinical"},{"id":"warm-earth","label":"Warm Earth"},{"id":"deep-lavender","label":"Deep Lavender"},{"id":"forest-care","label":"Forest Care"},{"id":"sunset-support","label":"Sunset Support"},{"id":"tech-indigo","label":"Tech Indigo"},{"id":"soft-rose","label":"Soft Rose"},{"id":"minty-fresh","label":"Minty Fresh"}] })
}

pub fn index(
    view: &TeraView,
    token: &CsrfToken,
    slug: &str,
    medications: &[MedicationCard],
    can_create: bool,
    can_manage: bool,
) -> Response {
    let mut data = appearance_context();
    data["title"] = json!("Medications");
    data["slug"] = json!(slug);
    data["medications"] = json!(medications);
    data["can_create"] = json!(can_create);
    data["can_manage"] = json!(can_manage);
    render(view, token, "medications/index.html", data, StatusCode::OK)
}

pub fn detail(
    view: &TeraView,
    token: &CsrfToken,
    slug: &str,
    detail: Detail,
    zone: chrono_tz::Tz,
    draft: &HashMap<String, String>,
    error: Option<&OperationError>,
) -> Response {
    let mut data = context(slug, &detail.stock, draft, error.map(super::forms::message));
    data["etag"] = json!(detail.stock.etag);
    data["can_adjust"] = json!(detail.can_adjust);
    let selected = super::forms::field(draft, "source_id").parse::<i64>().ok();
    data["assignments"] = json!(detail.assignments.iter().map(|source| {
        let submitted = selected == Some(source.id);
        let source_draft = if submitted { draft.clone() } else { super::forms::dose_for(source, detail.stock.medication.id, zone) };
        json!({ "id":source.id, "person_name":source.person_name, "can_record":source.can_record,
            "dose_unit_label":unit_label(super::forms::field(&source_draft, "dose_unit"), super::forms::field(&source_draft, "dose_amount")),
            "draft":source_draft, "error":submitted && error.is_some() })
    }).collect::<Vec<_>>());
    render(
        view,
        token,
        "medications/show.html",
        data,
        error.map(super::forms::status).unwrap_or(StatusCode::OK),
    )
}

pub fn stock(
    view: &TeraView,
    token: &CsrfToken,
    slug: &str,
    snapshot: StockSnapshot,
    draft: &HashMap<String, String>,
    error: Option<String>,
    status: StatusCode,
) -> Response {
    let changed = status == StatusCode::CONFLICT;
    let error = if changed {
        Some(
            "Stock changed while this form was open. Review the latest stock before saving.".into(),
        )
    } else {
        error
    };
    let mut data = context(slug, &snapshot, draft, error);
    data["stock_changed"] = json!(changed);
    render(view, token, "medications/stock.html", data, status)
}

fn render(
    view: &TeraView,
    token: &CsrfToken,
    template: &str,
    mut data: Value,
    status: StatusCode,
) -> Response {
    let Ok(authenticity) = token.authenticity_token() else {
        return super::unavailable();
    };
    data["authenticity_token"] = json!(authenticity);
    match format::render().view(view, template, data) {
        Ok(response) => (
            status,
            token.clone(),
            [(header::CACHE_CONTROL, "no-store")],
            response,
        )
            .into_response(),
        Err(_) => super::unavailable(),
    }
}

fn unit_label(unit: &str, amount: &str) -> String {
    if amount != "1" && matches!(unit, "tablet" | "capsule" | "puff" | "drop") {
        format!("{unit}s")
    } else {
        unit.into()
    }
}
