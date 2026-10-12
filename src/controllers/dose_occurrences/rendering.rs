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
    let labels = if context["labels"].is_object() {
        context["labels"].clone()
    } else {
        report_pdf::translations("en").unwrap_or_default()
    };
    let ui = &labels["dose_records"];
    data["ui"] = ui.clone();
    data["outcomes"] = labels["dose_outcomes"].clone();
    data["reasons"] = labels["dashboard"]["outcomes"]["reasons"].clone();
    data["lang"] = json!(context["locale"].as_str().unwrap_or("en"));
    let person = context["person"]["name"].as_str().unwrap_or("Person");
    data["title"] = json!(
        ui["title"]
            .as_str()
            .unwrap_or("Dose records for %{name}")
            .replace("%{name}", person)
    );
    data["source"] = context["source"].clone();
    data["medication_name"] = context["medication_name"].clone();
    data["person_name"] = context["person"]["name"].clone();
    data["medications"] = context["medications"].clone();
    data["start_date"] = json!(query.start_date);
    data["end_date"] = json!(query.end_date);
    let mut errors = match error {
        Some(OperationError::Validation { details }) => {
            details.get("errors").cloned().unwrap_or(json!({}))
        }
        _ => json!({}),
    };
    if context["locale"]
        .as_str()
        .is_some_and(|locale| locale != "en")
        && let Some(fields) = errors.as_object_mut()
    {
        for (field, messages) in fields {
            let key = match field.as_str() {
                "dose_amount" => "amount_error",
                "taken_at" => "time_error",
                "taken_from_medication_id" => "stock_error",
                "date_range" => "range_hint",
                _ => "invalid",
            };
            *messages = json!([ui[key]]);
        }
    }
    let message = error.map(|error| match error {
        OperationError::Conflict { code, .. } if code == "sync_conflict" => {
            ui["conflict"].as_str().unwrap_or_default().to_owned()
        }
        OperationError::Conflict { code, .. } if code == "precondition_required" => {
            ui["review"].as_str().unwrap_or_default().to_owned()
        }
        OperationError::Validation { details } if details["code"] == "invalid_occurrence" => {
            ui["unavailable"].as_str().unwrap_or_default().to_owned()
        }
        OperationError::Validation { .. } => ui["invalid"].as_str().unwrap_or_default().to_owned(),
        _ if context["locale"]
            .as_str()
            .is_some_and(|locale| locale != "en") =>
        {
            ui["invalid"].as_str().unwrap_or_default().to_owned()
        }
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
                                .format(
                                    if context["locale"]
                                        .as_str()
                                        .is_some_and(|locale| locale != "en")
                                    {
                                        "%Y-%m-%d, %H:%M:%S %Z"
                                    } else {
                                        "%d %b %Y, %H:%M:%S %Z"
                                    }
                                )
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
                row["open_action"] = if matches {
                    json!(draft.and_then(|draft| draft.get("intent")))
                } else {
                    Value::Null
                };
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
    fn non_english_dates_do_not_include_english_month_names() {
        for locale in ["es", "pt", "cy", "ga"] {
            let rendered = index(
                json!({"locale":locale,"person":{"name":"Synthetic adult"},"source":{"dose_amount":"2"}}),
                json!({"data":[{"scheduled_at":"2026-01-06T23:30:00Z"}]}),
                &dose_occurrences::RangeQuery::default(),
                chrono_tz::UTC,
                None,
                None,
            );
            assert_eq!(
                rendered["rows"][0]["scheduled_at_display"],
                "2026-01-06, 23:30:00 UTC"
            );
        }
    }

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
