use super::api_client::html_cookie_only;
use super::response::page_status;
use super::time::{dashboard_now, is_today_in_zone};
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use chrono::NaiveDate;

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
