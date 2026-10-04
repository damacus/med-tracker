use super::api_client::html_cookie_only;
use super::response::page_status;
use super::time::{dashboard_now, is_today_in_zone};
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use chrono::NaiveDate;

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
fn dashboard_uses_a_saved_rails_timezone_label_for_calendar_dates() {
    let profile = serde_json::json!({"data": {"time_zone": "London"}});
    let zone = super::dashboard::profile_timezone(&profile);
    assert_eq!(zone, chrono_tz::Europe::London);
    let now = chrono::DateTime::parse_from_rfc3339("2026-03-29T23:30:00Z")
        .unwrap()
        .with_timezone(&chrono::Utc);
    assert_eq!(
        now.with_timezone(&zone).date_naive(),
        NaiveDate::from_ymd_opt(2026, 3, 30).unwrap()
    );
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

#[test]
fn settings_validation_errors_keep_the_field_and_message() {
    let reply = serde_json::json!({
        "error": {"errors": {"time_zone": ["Time zone is not included in the list"]}}
    });
    let errors = super::settings::profile_errors(&reply);
    assert_eq!(
        errors.get("time_zone"),
        Some(&vec!["Time zone is not included in the list".to_owned()])
    );
}

#[test]
fn settings_edit_permission_requires_manage_access_to_profile_person() {
    let profile = serde_json::json!({"data": {"person_id": "42"}});
    let viewing = serde_json::json!({"data": {"people": {"manage_ids": [7]}}});
    let managing = serde_json::json!({"data": {"people": {"manage_ids": [7, 42]}}});
    assert!(!super::settings::can_edit_profile(&profile, &viewing));
    assert!(super::settings::can_edit_profile(&profile, &managing));
}

#[test]
fn settings_rejects_an_empty_timezone_before_profile_write() {
    assert_eq!(super::settings::time_zone_error(""), Some("can't be blank"));
    assert_eq!(super::settings::time_zone_error("UTC"), None);
}
