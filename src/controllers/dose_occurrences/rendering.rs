use super::*;

pub(super) fn index(
    context: Value,
    rows: Value,
    query: &dose_occurrences::RangeQuery,
    zone: chrono_tz::Tz,
    draft: Option<&HashMap<String, String>>,
    error: Option<&OperationError>,
) -> Value {
    let mut data = browser::rendering::appearance_context();
    let person = context["person"]["name"].as_str().unwrap_or("Person");
    data["title"] = json!(format!("Dose records for {person}"));
    data["source"] = context["source"].clone();
    data["medication_name"] = context["medication_name"].clone();
    data["medications"] = context["medications"].clone();
    data["start_date"] = json!(query.start_date);
    data["end_date"] = json!(query.end_date);
    let errors = match error {
        Some(OperationError::Validation { details }) => {
            details.get("errors").cloned().unwrap_or(json!({}))
        }
        _ => json!({}),
    };
    let message = error.map(|error| match error {
        OperationError::Conflict {code,..} if code == "sync_conflict" => "Dose record changed while this form was open. Review the latest dose record before saving.".into(),
        OperationError::Conflict {code,..} if code == "precondition_required" => "Review the latest dose record before saving.".into(),
        OperationError::Validation { details } => details.get("message").and_then(Value::as_str).map(str::to_owned).unwrap_or_else(|| browser_forms::message(error)),
        _ => browser_forms::message(error),
    });
    data["error"] = json!(message);
    data["errors"] = errors.clone();
    data["range_invalid"] = json!(errors.get("date_range").is_some());
    data["rows"] = json!(
        rows["data"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|row| {
                let mut row = row.clone();
                for field in ["scheduled_at", "resolved_at"] {
                    row[format!("{field}_display")] = json!(
                        row[field]
                            .as_str()
                            .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
                            .map(|time| time
                                .with_timezone(&zone)
                                .format("%d %b %Y, %H:%M:%S %Z")
                                .to_string())
                    );
                }
                let mut fields = forms::defaults(&row, &context["source"], zone);
                let matches = draft.is_some_and(|draft| {
                    browser_forms::field(draft, "key") == row["key"].as_str().unwrap_or_default()
                });
                if matches && let Some(draft) = draft {
                    fields.extend(draft.clone());
                    fields.insert("key".into(), row["key"].as_str().unwrap_or_default().into());
                    fields.insert(
                        "etag".into(),
                        row["etag"].as_str().unwrap_or_default().into(),
                    );
                }
                row["draft"] = json!(fields);
                row["errors"] = if matches { errors.clone() } else { json!({}) };
                row["error"] = if matches { json!(message) } else { Value::Null };
                row
            })
            .collect::<Vec<_>>()
    );
    data
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn occurrence_timestamps_use_account_zone_across_midnight() {
        let context = json!({"person":{"name":"Synthetic adult"},"source":{"medication_id":1,"dose_amount":"2"}});
        let rows = json!({"data":[{"scheduled_at":"2026-10-06T23:30:00Z","resolved_at":"2026-10-06T23:45:00Z"}]});
        for (zone, scheduled, resolved) in [
            (
                chrono_tz::UTC,
                "06 Oct 2026, 23:30:00 UTC",
                "06 Oct 2026, 23:45:00 UTC",
            ),
            (
                chrono_tz::Europe::London,
                "07 Oct 2026, 00:30:00 BST",
                "07 Oct 2026, 00:45:00 BST",
            ),
        ] {
            let rendered = index(
                context.clone(),
                rows.clone(),
                &dose_occurrences::RangeQuery::default(),
                zone,
                None,
                None,
            );
            assert_eq!(rendered["rows"][0]["scheduled_at_display"], scheduled);
            assert_eq!(rendered["rows"][0]["resolved_at_display"], resolved);
            assert_eq!(
                rendered["rows"][0]["scheduled_at"],
                rows["data"][0]["scheduled_at"]
            );
            assert_eq!(
                rendered["rows"][0]["resolved_at"],
                rows["data"][0]["resolved_at"]
            );
        }
    }
}
