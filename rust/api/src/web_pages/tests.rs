use super::api_client::{download_headers, html_cookie_only};
use super::reports::{download_error_key, filters};
use super::response::page_status;
use super::time::{dashboard_now, is_today_in_zone};
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use chrono::NaiveDate;
use std::collections::HashMap;

#[test]
fn pagination_rejects_duplicate_ids_and_changing_totals() {
    use super::api_client::PageScan;
    let page = |number, count, ids: &[i64]| {
        serde_json::json!({
            "meta": {"page": number, "per_page": 100, "total_count": count},
            "data": ids.iter().map(|id| serde_json::json!({"id": id})).collect::<Vec<_>>()
        })
    };
    let mut scan = PageScan::default();
    assert!(scan.accept(page(1, 2, &[1])).is_ok());
    assert!(scan.accept(page(2, 2, &[1])).is_err());
    let mut scan = PageScan::default();
    scan.accept(page(1, 2, &[1])).unwrap();
    assert!(scan.accept(page(2, 3, &[2])).is_err());
}

#[test]
fn pagination_rejects_malformed_metadata_and_truncated_or_oversized_results() {
    use super::api_client::PageScan;
    for value in [
        serde_json::json!({"meta": {"page": 2, "per_page": 100, "total_count": 1}, "data": [{"id": 1}]}),
        serde_json::json!({"meta": {"page": "1", "per_page": 100, "total_count": 1}, "data": [{"id": 1}]}),
        serde_json::json!({"meta": {"page": 1, "per_page": 20, "total_count": 1}, "data": [{"id": 1}]}),
        serde_json::json!({"meta": {"page": 1, "per_page": 100, "total_count": 1}, "data": []}),
        serde_json::json!({"meta": {"page": 1, "per_page": 100, "total_count": 0}, "data": [{"id": 1}]}),
    ] {
        assert!(PageScan::default().accept(value).is_err());
    }
}

#[test]
fn malformed_api_json_preserves_the_parse_error() {
    use super::api_client::decode_api_body;
    use super::response::PageError;
    assert!(matches!(
        decode_api_body(StatusCode::OK, b"{invalid"),
        Err(PageError::InvalidJson(_))
    ));
    assert!(decode_api_body(StatusCode::OK, b"").is_err());
    assert!(decode_api_body(StatusCode::NO_CONTENT, b"")
        .unwrap()
        .is_null());
}

#[test]
fn dashboard_clock_uses_real_utc_outside_the_isolated_contract_runner() {
    if std::env::var_os("CONTRACT_PROJECT").is_some() {
        return;
    }
    let before = chrono::Utc::now();
    let actual = dashboard_now();
    let after = chrono::Utc::now();
    assert!(before <= actual && actual <= after);
}

#[test]
fn london_today_includes_late_utc_previous_day() {
    let today = NaiveDate::from_ymd_opt(2026, 3, 30).unwrap();
    assert!(is_today_in_zone(
        "2026-03-29T23:30:00Z",
        today,
        chrono_tz::Europe::London
    ));
    assert!(!is_today_in_zone(
        "2026-03-29T22:30:00Z",
        today,
        chrono_tz::Europe::London
    ));
}

#[test]
fn rejected_form_keeps_its_validation_status() {
    for status in [
        StatusCode::UNPROCESSABLE_ENTITY,
        StatusCode::FORBIDDEN,
        StatusCode::CONFLICT,
    ] {
        let response = page_status("Invalid dose configured".to_owned(), None, status);
        assert_eq!(response.status(), status);
    }
}

#[test]
fn html_cookie_session_rejects_any_explicit_authorization() {
    let mut headers = HeaderMap::new();
    headers.insert(
        header::COOKIE,
        HeaderValue::from_static("medtracker_session=fake"),
    );
    assert!(html_cookie_only(&headers));
    headers.insert(
        header::AUTHORIZATION,
        HeaderValue::from_static("Bearer fake"),
    );
    assert!(!html_cookie_only(&headers));
}

fn report_query(pairs: &[(&str, &str)]) -> HashMap<String, String> {
    pairs
        .iter()
        .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
        .collect()
}

#[test]
fn report_filters_default_to_today_and_twelve_months_earlier() {
    let today = NaiveDate::from_ymd_opt(2026, 2, 26).unwrap();
    let accepted = filters(&report_query(&[("person_id", "7")]), today).expect("defaults");
    assert_eq!(accepted.person_id, "7");
    assert_eq!(accepted.end.to_string(), "2026-02-26");
    assert_eq!(accepted.start.to_string(), "2025-02-26");
    assert!(!accepted.include_takes);
    let explicit = filters(
        &report_query(&[
            ("person_id", "9"),
            ("start_date", "2026-02-19"),
            ("end_date", "2026-02-26"),
            ("include_medication_takes", "1"),
        ]),
        today,
    )
    .expect("explicit filters");
    assert_eq!(explicit.person_id, "9");
    assert_eq!(explicit.start.to_string(), "2026-02-19");
    assert_eq!(explicit.end.to_string(), "2026-02-26");
    assert!(explicit.include_takes);
    let cleared = filters(
        &report_query(&[("person_id", "7"), ("include_medication_takes", "0")]),
        today,
    )
    .expect("unchecked takes");
    assert!(!cleared.include_takes);
}

#[test]
fn report_filters_default_start_handles_leap_day() {
    let leap = NaiveDate::from_ymd_opt(2024, 2, 29).unwrap();
    let accepted = filters(&report_query(&[("person_id", "7")]), leap).expect("leap day");
    assert_eq!(accepted.end.to_string(), "2024-02-29");
    assert_eq!(accepted.start.to_string(), "2023-02-28");
    let leap_span = filters(
        &report_query(&[
            ("person_id", "7"),
            ("start_date", "2023-02-28"),
            ("end_date", "2024-02-29"),
        ]),
        NaiveDate::from_ymd_opt(2026, 2, 26).unwrap(),
    )
    .expect("366-day span including a leap day");
    assert_eq!(
        (leap_span.end - leap_span.start).num_days(),
        366,
        "leap-day span is allowed at the 366-day boundary"
    );
}

#[test]
fn report_filters_reject_malformed_reversed_oversized_and_unknown_values() {
    let today = NaiveDate::from_ymd_opt(2026, 2, 26).unwrap();
    for pairs in [
        &[][..],
        &[("person_id", "")][..],
        &[("person_id", "7"), ("start_date", "not-a-date")][..],
        &[("person_id", "7"), ("start_date", "2026-2-9")][..],
        &[("person_id", "7"), ("end_date", "2026-13-40")][..],
        &[
            ("person_id", "7"),
            ("start_date", "2026-02-27"),
            ("end_date", "2026-02-26"),
        ][..],
        &[
            ("person_id", "7"),
            ("start_date", "2023-02-27"),
            ("end_date", "2024-02-29"),
        ][..],
        &[("person_id", "7"), ("include_medication_takes", "true")][..],
        &[("person_id", "7"), ("include_medication_takes", "2")][..],
    ] {
        assert!(filters(&report_query(pairs), today).is_err(), "{pairs:?}");
    }
    let errors = filters(&report_query(&[]), today).unwrap_err();
    assert!(errors.iter().any(|(field, _)| field == "person_id"));
    let errors = filters(
        &report_query(&[
            ("person_id", "7"),
            ("start_date", "2026-02-27"),
            ("end_date", "2026-02-26"),
        ]),
        today,
    )
    .unwrap_err();
    assert!(errors.iter().any(|(field, _)| field == "end_date"));
}

#[test]
fn download_headers_only_preserve_pdf_type_attachment_and_request_id() {
    let mut upstream = HeaderMap::new();
    upstream.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/pdf"),
    );
    upstream.insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_static("attachment; filename=\"report.pdf\""),
    );
    upstream.insert("x-request-id", HeaderValue::from_static("request-1"));
    upstream.insert(header::SET_COOKIE, HeaderValue::from_static("mt_session=x"));
    upstream.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    upstream.insert("x-internal-trace", HeaderValue::from_static("private"));
    let forwarded = download_headers(&upstream);
    assert_eq!(forwarded[header::CONTENT_TYPE], "application/pdf");
    assert_eq!(
        forwarded[header::CONTENT_DISPOSITION],
        "attachment; filename=\"report.pdf\""
    );
    assert_eq!(forwarded["x-request-id"], "request-1");
    assert!(!forwarded.contains_key(header::SET_COOKIE));
    assert!(!forwarded.contains_key(header::CACHE_CONTROL));
    assert!(!forwarded.contains_key("x-internal-trace"));
}

#[test]
fn download_error_keys_map_report_api_statuses_neutrally() {
    assert_eq!(download_error_key(StatusCode::FORBIDDEN), "forbidden");
    assert_eq!(download_error_key(StatusCode::NOT_FOUND), "not_found");
    assert_eq!(
        download_error_key(StatusCode::SERVICE_UNAVAILABLE),
        "unavailable"
    );
    assert_eq!(
        download_error_key(StatusCode::UNPROCESSABLE_ENTITY),
        "failed"
    );
    assert_eq!(download_error_key(StatusCode::BAD_GATEWAY), "failed");
}
