use super::*;
use chrono::{LocalResult, NaiveDateTime, TimeZone, Utc};

pub(super) fn range(
    query: dose_occurrences::RangeQuery,
    zone: chrono_tz::Tz,
) -> dose_occurrences::RangeQuery {
    let today = Utc::now().with_timezone(&zone).date_naive().to_string();
    dose_occurrences::RangeQuery {
        start_date: query.start_date.or(Some(today.clone())),
        end_date: query.end_date.or(Some(today)),
    }
}

pub(super) fn payload(
    source_path: (&str, &str),
    action: &str,
    draft: &HashMap<String, String>,
    zone: chrono_tz::Tz,
) -> Result<dose_occurrences::ChangeInput, OperationError> {
    let key = browser_forms::field(draft, "key");
    match action {
        "not_taken" => Ok(dose_occurrences::ChangeInput::not_taken_strings(
            key,
            browser_forms::optional(draft, "reason").as_deref(),
            browser_forms::optional(draft, "note").as_deref(),
        )),
        "reopen" => Ok(dose_occurrences::ChangeInput::reopen(key)),
        "take" => {
            let invalid = || OperationError::Validation {
                details: json!({"errors":{"taken_at":["must be a valid local date and time"]}}),
            };
            let local = NaiveDateTime::parse_from_str(
                browser_forms::field(draft, "taken_at"),
                "%Y-%m-%dT%H:%M:%S%.f",
            )
            .or_else(|_| {
                NaiveDateTime::parse_from_str(
                    browser_forms::field(draft, "taken_at"),
                    "%Y-%m-%dT%H:%M",
                )
            })
            .map_err(|_| invalid())?;
            let LocalResult::Single(time) = zone.from_local_datetime(&local) else {
                return Err(invalid());
            };
            let stock = browser_forms::field(draft, "taken_from_medication_id").parse::<i64>().map_err(|_| OperationError::Validation { details: json!({"errors":{"taken_from_medication_id":["must be a positive integer"]}}) })?;
            Ok(dose_occurrences::ChangeInput::take(
                source_path,
                key,
                &time.to_rfc3339(),
                Some(browser_forms::field(draft, "client_uuid")),
                Some(browser_forms::field(draft, "dose_amount")),
                Some(stock),
            ))
        }
        _ => Err(OperationError::NotFound),
    }
}

pub(super) fn defaults(
    row: &Value,
    source: &Value,
    zone: chrono_tz::Tz,
) -> HashMap<String, String> {
    HashMap::from([
        ("key".into(), row["key"].as_str().unwrap_or_default().into()),
        (
            "etag".into(),
            row["etag"].as_str().unwrap_or_default().into(),
        ),
        ("client_uuid".into(), uuid::Uuid::new_v4().to_string()),
        (
            "dose_amount".into(),
            row["prescribed_amount"].as_str().unwrap_or_default().into(),
        ),
        (
            "taken_from_medication_id".into(),
            source["medication_id"].to_string(),
        ),
        (
            "taken_at".into(),
            Utc::now()
                .with_timezone(&zone)
                .format("%Y-%m-%dT%H:%M:%S%.3f")
                .to_string(),
        ),
        ("reason".into(), String::new()),
        ("note".into(), String::new()),
    ])
}
