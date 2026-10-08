use crate::models::{
    access::TenantTransaction,
    care::{
        browser_query::{Assignment, Detail},
        doses::Take,
    },
    errors::OperationError,
};
use axum::http::StatusCode;
use chrono::{LocalResult, NaiveDateTime, TimeZone, Utc};
use std::collections::HashMap;

pub const INVALID_TAKEN_AT: &str = "Taken at is invalid.";

pub fn field<'a>(draft: &'a HashMap<String, String>, key: &str) -> &'a str {
    draft.get(key).map(String::as_str).unwrap_or_default()
}
pub fn optional(draft: &HashMap<String, String>, key: &str) -> Option<String> {
    let value = field(draft, key);
    (!value.is_empty()).then(|| value.into())
}

pub fn dose(detail: &Detail, zone: chrono_tz::Tz) -> HashMap<String, String> {
    let source = detail.assignments.iter().find(|source| source.can_record);
    source
        .map(|source| dose_for(source, detail.stock.medication.id, zone))
        .unwrap_or_default()
}

pub fn dose_for(
    source: &Assignment,
    medication_id: i64,
    zone: chrono_tz::Tz,
) -> HashMap<String, String> {
    HashMap::from([
        ("client_uuid".into(), uuid::Uuid::new_v4().to_string()),
        ("source_type".into(), source.source_type.into()),
        ("source_id".into(), source.id.to_string()),
        ("dose_amount".into(), source.amount.clone()),
        ("dose_unit".into(), source.unit.clone()),
        (
            "taken_at".into(),
            Utc::now()
                .with_timezone(&zone)
                .format("%Y-%m-%dT%H:%M")
                .to_string(),
        ),
        ("taken_from_medication_id".into(), medication_id.to_string()),
    ])
}

pub fn take(
    detail: &Detail,
    draft: &HashMap<String, String>,
    zone: chrono_tz::Tz,
) -> Result<Take, OperationError> {
    if !detail.assignments.iter().any(|source| {
        source.can_record
            && source.source_type == field(draft, "source_type")
            && source.id.to_string() == field(draft, "source_id")
    }) {
        return Err(OperationError::Forbidden);
    }
    let invalid = || OperationError::Validation {
        details: serde_json::json!({"error":INVALID_TAKEN_AT}),
    };
    let local = NaiveDateTime::parse_from_str(field(draft, "taken_at"), "%Y-%m-%dT%H:%M")
        .map_err(|_| invalid())?;
    let LocalResult::Single(time) = zone.from_local_datetime(&local) else {
        return Err(invalid());
    };
    let stock_id = field(draft, "taken_from_medication_id")
        .parse()
        .map_err(|_| OperationError::NotFound)?;
    let confirmed_amount =
        optional(draft, "dose_amount").ok_or_else(|| OperationError::Validation {
            details: serde_json::json!({"error":"Dose confirmation is incomplete."}),
        })?;
    let confirmed_unit =
        optional(draft, "dose_unit").ok_or_else(|| OperationError::Validation {
            details: serde_json::json!({"error":"Dose confirmation is incomplete."}),
        })?;
    Ok(Take {
        client_uuid: optional(draft, "client_uuid"),
        source_type: field(draft, "source_type").into(),
        source_id: field(draft, "source_id").into(),
        taken_at: time.to_rfc3339(),
        dose_amount: Some(confirmed_amount.clone()),
        dose_unit: Some(confirmed_unit.clone()),
        taken_from_medication_id: Some(stock_id),
        expected_effective_amount: Some(confirmed_amount),
        expected_effective_unit: Some(confirmed_unit),
    })
}

pub fn can_adjust(tenant: &TenantTransaction) -> bool {
    crate::models::care::browser_query::can_adjust(tenant)
}

pub fn status(error: &OperationError) -> StatusCode {
    match error {
        OperationError::Unauthenticated => StatusCode::UNAUTHORIZED,
        OperationError::Forbidden => StatusCode::FORBIDDEN,
        OperationError::NotFound => StatusCode::NOT_FOUND,
        OperationError::Validation { .. } => StatusCode::UNPROCESSABLE_ENTITY,
        OperationError::Conflict { .. } => StatusCode::CONFLICT,
        OperationError::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
    }
}

pub fn message(error: &OperationError) -> String {
    match error {
        OperationError::Validation { details } | OperationError::Conflict { details, .. } => {
            details
                .get("error")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned)
                .unwrap_or_else(|| error.to_string())
        }
        _ => error.to_string(),
    }
}
