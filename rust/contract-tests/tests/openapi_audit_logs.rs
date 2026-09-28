use medtracker_contract_tests::{Fixture, Target, fixture};
use reqwest::Method;
use reqwest::blocking::{Client, Response};
use serde_json::Value;
use std::env;
use std::time::Duration as StdDuration;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

fn body(response: Response) -> Value {
    response.json().expect("JSON response")
}

fn audit_path(household_id: i64) -> String {
    format!("/api/v1/households/{household_id}/admin/audit_logs")
}

fn assert_error(response: Response, status: u16) {
    assert_eq!(response.status().as_u16(), status);
    let error = body(response);
    assert!(error["error"]["code"].is_string());
    assert!(error["error"]["request_id"].is_string());
    assert!(error["error"]["message"].is_string());
    assert!(error.get("data").is_none());
}

fn assert_event(row: &Value) -> OffsetDateTime {
    let mut keys: Vec<_> = row
        .as_object()
        .expect("audit event object")
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    let mut expected = [
        "actor_account_id",
        "actor_membership_id",
        "created_at",
        "event_type",
        "id",
        "metadata",
        "request_id",
    ];
    expected.sort_unstable();
    assert_eq!(keys, expected);
    assert!(row["id"].is_i64());
    assert!(
        row["event_type"]
            .as_str()
            .is_some_and(|event_type| !event_type.is_empty())
    );
    assert!(row["actor_account_id"].is_null() || row["actor_account_id"].is_i64());
    assert!(row["actor_membership_id"].is_null() || row["actor_membership_id"].is_i64());
    assert!(row["request_id"].is_null() || row["request_id"].is_string());
    assert!(row["metadata"].is_object());
    OffsetDateTime::parse(
        row["created_at"].as_str().expect("audit timestamp"),
        &Rfc3339,
    )
    .expect("RFC3339 audit timestamp")
}

fn audit_collection(target: &Target, household_id: i64, token: &str) -> Value {
    let response = target.get(&audit_path(household_id), Some(token));
    assert_eq!(response.status().as_u16(), 200);
    let collection = body(response);
    assert_eq!(collection.as_object().unwrap().len(), 1);
    assert!(collection["data"].is_array());
    let rows = collection["data"].as_array().unwrap();
    assert!(rows.len() <= 100);
    let mut previous = None;
    for row in rows {
        let created_at = assert_event(row);
        if let Some(previous) = previous {
            assert!(previous >= created_at, "audit events are not newest first");
        }
        previous = Some(created_at);
    }
    collection
}

fn rate_limited(response: Response) {
    assert_eq!(response.status().as_u16(), 429);
    for header in [
        "retry-after",
        "ratelimit-limit",
        "ratelimit-remaining",
        "ratelimit-reset",
    ] {
        assert!(response.headers().get(header).is_some(), "missing {header}");
    }
    assert_eq!(response.headers()["ratelimit-limit"], "300");
    assert_eq!(response.headers()["ratelimit-remaining"], "0");
    assert_eq!(body(response)["error"]["code"], "rate_limited");
}

fn request(client: &Client, base_url: &str, path: &str, token: &str) -> Response {
    client
        .request(
            Method::GET,
            format!("{}{}", base_url.trim_end_matches('/'), path),
        )
        .bearer_auth(token)
        .send()
        .expect("audit log rate-limit request")
}

#[test]
fn list_audit_logs_has_strict_rows_and_enforces_household_manager_scope() {
    let target = Target::from_env();
    let fixture = fixture();
    let household_path = audit_path(fixture.household_id);
    assert_error(target.get(&household_path, None), 401);
    let owner_rows = audit_collection(&target, fixture.household_id, &fixture.access_token);
    let admin_rows = audit_collection(&target, fixture.household_id, &fixture.manager_access_token);
    assert!(!owner_rows["data"].as_array().unwrap().is_empty());
    assert!(!admin_rows["data"].as_array().unwrap().is_empty());
    assert_error(
        target.get(&household_path, Some(&fixture.view_access_token)),
        403,
    );
    assert_error(
        target.get(&household_path, Some(&fixture.foreign_access_token)),
        403,
    );
    assert_error(
        target.get(
            &audit_path(fixture.foreign_household_id),
            Some(&fixture.access_token),
        ),
        403,
    );

    let foreign_rows = audit_collection(
        &target,
        fixture.foreign_household_id,
        &fixture.foreign_access_token,
    );
    let owner_ids: Vec<_> = owner_rows["data"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["id"].as_i64().unwrap())
        .collect();
    assert!(
        foreign_rows["data"]
            .as_array()
            .unwrap()
            .iter()
            .all(|row| { !owner_ids.contains(&row["id"].as_i64().unwrap()) })
    );
    let owner_text = owner_rows.to_string();
    assert!(!owner_text.contains(&fixture.access_token));
    assert!(!owner_text.contains(&fixture.manager_app_token));
}

#[test]
fn list_audit_logs_uses_the_shared_rate_limiter() {
    let fixture: Fixture = fixture();
    let base_url = env::var("CONTRACT_RATE_BASE_URL").expect("non-loopback API base URL");
    let client = Client::builder()
        .no_proxy()
        .timeout(StdDuration::from_secs(5))
        .build()
        .expect("rate-limit client");
    let mut limited = None;
    for _ in 0..601 {
        let response = request(
            &client,
            &base_url,
            "/api/v1/capabilities",
            &fixture.access_token,
        );
        if response.status().as_u16() == 429 {
            limited = Some(response);
            break;
        }
        assert_eq!(response.status().as_u16(), 200);
    }
    rate_limited(limited.expect("shared rate limit response"));
    rate_limited(request(
        &client,
        &base_url,
        &audit_path(fixture.household_id),
        &fixture.access_token,
    ));
}
