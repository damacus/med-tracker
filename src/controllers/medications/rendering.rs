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
    presentation: DosePresentation<'_>,
) -> Response {
    detail_with_state(
        view,
        token,
        slug,
        detail,
        presentation.zone,
        presentation.language,
        DetailState::Dose(presentation.draft, presentation.error, presentation.open),
    )
}

pub struct DosePresentation<'a> {
    pub zone: chrono_tz::Tz,
    pub language: &'a str,
    pub draft: &'a HashMap<String, String>,
    pub error: Option<&'a OperationError>,
    pub open: bool,
}

pub fn refill_failure(
    view: &TeraView,
    token: &CsrfToken,
    slug: &str,
    detail: Detail,
    presentation: RefillPresentation<'_>,
) -> Response {
    detail_with_state(
        view,
        token,
        slug,
        detail,
        presentation.zone,
        presentation.language,
        DetailState::Refill(presentation.draft, presentation.error),
    )
}

pub struct RefillPresentation<'a> {
    pub zone: chrono_tz::Tz,
    pub language: &'a str,
    pub draft: &'a HashMap<String, String>,
    pub error: &'a OperationError,
}

pub fn management_failure(
    view: &TeraView,
    token: &CsrfToken,
    slug: &str,
    detail: Detail,
    zone: chrono_tz::Tz,
    language: &str,
    error: &OperationError,
) -> Response {
    detail_with_state(
        view,
        token,
        slug,
        detail,
        zone,
        language,
        DetailState::Management(error),
    )
}

pub fn adjustment_failure(
    view: &TeraView,
    token: &CsrfToken,
    slug: &str,
    detail: Detail,
    presentation: AdjustmentPresentation<'_>,
) -> Response {
    detail_with_state(
        view,
        token,
        slug,
        detail,
        presentation.zone,
        presentation.language,
        DetailState::Adjustment(presentation.draft, presentation.error),
    )
}

pub struct AdjustmentPresentation<'a> {
    pub zone: chrono_tz::Tz,
    pub language: &'a str,
    pub draft: &'a HashMap<String, String>,
    pub error: &'a OperationError,
}

enum DetailState<'a> {
    Dose(
        &'a HashMap<String, String>,
        Option<&'a OperationError>,
        bool,
    ),
    Refill(&'a HashMap<String, String>, &'a OperationError),
    Management(&'a OperationError),
    Adjustment(&'a HashMap<String, String>, &'a OperationError),
}

fn detail_with_state(
    view: &TeraView,
    token: &CsrfToken,
    slug: &str,
    detail: Detail,
    zone: chrono_tz::Tz,
    requested_language: &str,
    state: DetailState<'_>,
) -> Response {
    let language = crate::models::care::report_pdf::locale(requested_language);
    let Ok(labels) = crate::models::care::report_pdf::translations(language) else {
        return super::unavailable();
    };
    let dose_default = super::forms::dose(&detail, zone);
    let (dose_draft, dose_error, dose_open, refill_failure, management_error, adjustment_failure) =
        match state {
            DetailState::Dose(draft, error, open) => (draft, error, open, None, None, None),
            DetailState::Refill(draft, error) => {
                (&dose_default, None, false, Some((draft, error)), None, None)
            }
            DetailState::Management(error) => (&dose_default, None, false, None, Some(error), None),
            DetailState::Adjustment(draft, error) => {
                (&dose_default, None, false, None, None, Some((draft, error)))
            }
        };
    let mut data = context(
        slug,
        &detail.stock,
        dose_draft,
        dose_error.map(super::forms::message),
    );
    data["dose_lang"] = json!(language);
    data["dose_i18n"] = labels["medications"]["dose_dialog"].clone();
    data["dose_cancel"] = labels["dose_outcomes"]["cancel"].clone();
    data["dose_open"] = json!(dose_open);
    if dose_error.is_some() {
        data["error"] = Value::Null;
    }
    data["etag"] = json!(detail.stock.etag);
    if let Some((draft, error)) = refill_failure {
        data["refill_open"] = json!(true);
        data["refill_quantity"] = json!(super::forms::field(draft, "quantity"));
        data["refill_etag"] = json!(super::forms::field(draft, "etag"));
        data["refill_date"] = json!(super::forms::field(draft, "restock_date"));
        data["refill_conflict"] = json!(matches!(error, OperationError::Conflict { .. }));
        let field_errors = match error {
            OperationError::Validation { details } => details.get("errors"),
            _ => None,
        };
        data["refill_quantity_error"] = json!(
            field_errors
                .and_then(|errors| errors.get("quantity"))
                .is_some()
        );
        data["refill_date_error"] = json!(
            field_errors
                .and_then(|errors| errors.get("restock_date"))
                .is_some()
        );
        data["refill_error"] = json!(if matches!(error, OperationError::Conflict { .. }) {
            "Stock changed while this form was open. Review the latest stock before saving."
                .to_string()
        } else {
            super::forms::message(error)
        });
    }
    if let Some(error) = management_error {
        data["delete_open"] = json!(true);
        data["delete_error"] = json!(super::forms::message(error));
        if !detail.can_adjust {
            data["error"] = json!(super::forms::message(error));
        }
    }
    if let Some((draft, error)) = adjustment_failure {
        data["adjust_open"] = json!(true);
        data["adjust_new_quantity"] = json!(super::forms::field(draft, "new_quantity"));
        data["adjust_reason"] = json!(super::forms::field(draft, "reason"));
        data["adjust_etag"] = json!(super::forms::field(draft, "etag"));
        data["adjust_conflict"] = json!(matches!(error, OperationError::Conflict { .. }));
        data["adjust_error"] = json!(if matches!(error, OperationError::Conflict { .. }) {
            "Stock changed while this form was open. Review the latest stock before saving."
                .to_string()
        } else {
            super::forms::message(error)
        });
        if !detail.can_adjust {
            data["error"] = json!(super::forms::message(error));
        }
    }
    data["can_adjust"] = json!(detail.can_adjust);
    data["can_record_any"] = json!(detail.assignments.iter().any(|source| source.can_record));
    data["location_name"] = json!(detail.location_name);
    let medication = &detail.stock.medication;
    data["description"] = json!(
        medication
            .description
            .as_deref()
            .filter(|value| !value.trim().is_empty())
    );
    data["warnings"] = json!(
        medication
            .warnings
            .as_deref()
            .filter(|value| !value.trim().is_empty())
    );
    data["dose_label"] = json!(medication.dose_amount.and_then(|amount| {
        medication.dose_unit.as_deref().map(|unit| {
            let amount = amount.to_string();
            format!("{} {}", amount, unit_label(unit, &amount))
        })
    }));
    data["reorder_label"] = json!(format!(
        "Reorder at {} {}",
        medication.reorder_threshold.normalize(),
        unit_label(
            medication.dose_unit.as_deref().unwrap_or("units"),
            &medication.reorder_threshold.normalize().to_string()
        )
    ));
    data["stock_status"] = json!(if medication.current_supply.is_none() {
        "Supply not tracked"
    } else if detail.stock.representation["data"]["out_of_stock"] == true {
        "Out of stock"
    } else if detail.stock.representation["data"]["low_stock"] == true {
        "Low stock"
    } else {
        "In stock"
    });
    data["low_stock"] = detail.stock.representation["data"]["low_stock"].clone();
    data["out_of_stock"] = detail.stock.representation["data"]["out_of_stock"].clone();
    data["days_until_low_stock"] =
        detail.stock.representation["data"]["days_until_low_stock"].clone();
    data["days_until_out_of_stock"] =
        detail.stock.representation["data"]["days_until_out_of_stock"].clone();
    data["reorder_status"] = detail.stock.representation["data"]["reorder_status"].clone();
    data["order_supplier"] = json!(medication.order_supplier);
    data["order_quantity"] = json!(
        medication
            .order_quantity
            .map(|value| value.normalize().to_string())
    );
    data["expected_arrival_on"] = json!(
        medication
            .expected_arrival_on
            .map(|value| value.format("%d %b %Y").to_string())
    );
    data["expected_arrival_on_iso"] = json!(
        medication
            .expected_arrival_on
            .map(|value| value.to_string())
    );
    data["restock_date"] = json!(
        chrono::Utc::now()
            .with_timezone(&zone)
            .format("%Y-%m-%d")
            .to_string()
    );
    data["dose_options"] = json!(detail.dose_options.iter().map(|option| {
        let amount = option.amount.normalize().to_string();
        let supply = option.current_supply.map(|value| value.normalize().to_string());
        let cycle = match option.default_dose_cycle { 1 => "week", 2 => "month", _ => "day" };
        let maximum = (option.default_max_daily_doses > 0).then(|| format!(
            "Maximum {} {} a {}",
            option.default_max_daily_doses,
            if option.default_max_daily_doses == 1 { "dose" } else { "doses" },
            cycle,
        ));
        let spacing = (option.default_min_hours_between_doses > sea_orm::prelude::Decimal::ZERO)
            .then(|| {
                let hours = option.default_min_hours_between_doses.normalize().to_string();
                format!("At least {} {} between doses", hours, if hours == "1" { "hour" } else { "hours" })
            });
        json!({
            "id": option.id,
            "description": option.description.as_deref().filter(|value| !value.trim().is_empty()),
            "dose_label": format!("{} {}", amount, unit_label(&option.unit, &amount)),
            "frequency": option.frequency,
            "supply": supply,
            "supply_unit": supply.as_deref().map(|value| unit_label(&option.unit, value)),
            "default_for_adults": option.default_for_adults,
            "default_for_children": option.default_for_children,
            "maximum": maximum,
            "spacing": spacing,
        })
    }).collect::<Vec<_>>());
    data["dose_history"] = json!(detail.dose_history.iter().map(|take| {
        let dose_label = take.amount.as_deref().zip(take.unit.as_deref())
            .map(|(amount, unit)| format!("{} {}", amount, unit_label(unit, amount)));
        json!({
            "person_name": take.person_name,
            "dose_label": dose_label,
            "taken_at": take.taken_at.map(|value| value.and_utc().with_timezone(&zone).format("%d %b %Y, %H:%M").to_string()),
            "taken_at_iso": take.taken_at.map(|value| value.and_utc().to_rfc3339()),
        })
    }).collect::<Vec<_>>());
    let selected = super::forms::field(dose_draft, "source_id")
        .parse::<i64>()
        .ok();
    let selected_type = super::forms::field(dose_draft, "source_type");
    let selected_recordable = detail.assignments.iter().any(|source| {
        source.can_record && selected == Some(source.id) && selected_type == source.source_type
    });
    if !selected_recordable && let Some(error) = dose_error {
        let (message, message_language) = dose_error_label(error, &labels, language);
        data["dose_unmatched_error"] = json!(message);
        data["dose_unmatched_error_lang"] = json!(message_language);
    }
    let first_recordable = if dose_open {
        detail.assignments.iter().find(|source| {
            source.can_record && selected == Some(source.id) && selected_type == source.source_type
        })
    } else {
        None
    }
    .or_else(|| detail.assignments.iter().find(|source| source.can_record))
    .map(|source| (source.source_type, source.id));
    data["assignments"] = json!(detail.assignments.iter().map(|source| {
        let submitted = selected == Some(source.id) && selected_type == source.source_type;
        let source_draft = if submitted { dose_draft.clone() } else { super::forms::dose_for(source, detail.stock.medication.id, zone) };
        let time_error = submitted && dose_error.is_some_and(|error| super::forms::message(error) == super::forms::INVALID_TAKEN_AT);
        let source_error = submitted && dose_error.is_some() && !time_error;
        let dose_changed = submitted && matches!(dose_error, Some(OperationError::Conflict { code, .. }) if code == "dose_changed");
        let source_error_label = dose_error.filter(|_| source_error).map(|error| dose_error_label(error, &labels, language));
        let dose_label = format!("{} {}", source.amount, unit_label(&source.unit, &source.amount));
        json!({ "id":source.id, "source_type":source.source_type, "person_name":source.person_name, "can_record":source.can_record, "available":source.available,
            "amount":source.amount, "unit":source.unit,
            "dose_label":dose_label, "draft":source_draft, "time_error":time_error,
            "autofocus": first_recordable == Some((source.source_type, source.id)),
            "dose_changed":dose_changed,
            "source_error":source_error_label.as_ref().map(|(message, _)| message),
            "source_error_lang":source_error_label.as_ref().map(|(_, language)| language) })
    }).collect::<Vec<_>>());
    render(
        view,
        token,
        "medications/show.html",
        data,
        dose_error
            .or(refill_failure.map(|(_, error)| error))
            .or(management_error)
            .or(adjustment_failure.map(|(_, error)| error))
            .map(super::forms::status)
            .unwrap_or(StatusCode::OK),
    )
}

fn dose_error_label(error: &OperationError, labels: &Value, language: &str) -> (String, String) {
    let key = match error {
        OperationError::Forbidden => Some("forbidden"),
        OperationError::Conflict { code, .. } if code == "dose_changed" => Some("dose_changed"),
        OperationError::Conflict { .. } => Some("conflict"),
        _ => None,
    };
    if let Some(message) = key.and_then(|key| labels["medications"]["dose_dialog"][key].as_str()) {
        (message.to_owned(), language.to_owned())
    } else {
        (super::forms::message(error), "en".into())
    }
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

pub(super) fn unit_label(unit: &str, amount: &str) -> String {
    if amount != "1" && matches!(unit, "tablet" | "capsule" | "puff" | "drop") {
        format!("{unit}s")
    } else {
        unit.into()
    }
}
