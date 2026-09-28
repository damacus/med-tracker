use medtracker_contract_tests::{fixture, Fixture, Target};
use reqwest::blocking::Response;
use serde_json::{json, Value};
use std::env;
use std::time::{Duration, Instant};
use std::time::{SystemTime, UNIX_EPOCH};

fn body(response: Response) -> Value {
    response.json().expect("JSON response")
}

fn tokens_path(fixture: &Fixture) -> String {
    format!(
        "/api/v1/households/{}/admin/app_tokens",
        fixture.household_id
    )
}

fn request_id(response: &Response) -> String {
    response.headers()["x-request-id"]
        .to_str()
        .expect("request ID")
        .to_owned()
}

fn assert_error(response: Response, status: u16, code: &str) -> Value {
    assert_eq!(response.status().as_u16(), status);
    assert_eq!(
        response.headers()["content-type"]
            .to_str()
            .expect("JSON content type")
            .split(';')
            .next()
            .unwrap(),
        "application/json"
    );
    let id = request_id(&response);
    let payload = body(response);
    assert_eq!(payload["error"]["code"], code);
    assert_eq!(payload["error"]["request_id"], id);
    assert!(payload["error"]["message"]
        .as_str()
        .is_some_and(|s| !s.is_empty()));
    assert!(payload.get("data").is_none());
    payload
}

fn assert_rate_limited(response: Response) {
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
    assert_eq!(response.headers()["content-type"], "application/json");
    let payload = body(response);
    assert_eq!(payload["error"]["code"], "rate_limited");
    assert!(payload["error"]["message"]
        .as_str()
        .is_some_and(|s| !s.is_empty()));
}

fn create_token(target: &Target, fixture: &Fixture, name: &str) -> (i64, String) {
    let response = target.post_json_authorized(
        &tokens_path(fixture),
        &fixture.access_token,
        &json!({"api_app_token": {"name": name}}),
    );
    assert_eq!(response.status().as_u16(), 201);
    let created = body(response);
    let data = created["data"].as_object().expect("created token object");
    assert_eq!(data.len(), 7);
    for field in [
        "id",
        "name",
        "last_used_at",
        "revoked_at",
        "permissions_version",
        "token",
        "expires_at",
    ] {
        assert!(data.contains_key(field), "missing created field {field}");
    }
    assert!(data["id"].is_i64());
    assert_eq!(data["name"], name);
    assert!(data["last_used_at"].is_null() || data["last_used_at"].is_string());
    assert!(data["revoked_at"].is_null());
    assert!(data["permissions_version"].is_u64());
    assert!(data["expires_at"].is_string());
    let raw = data["token"].as_str().expect("one-time bearer token");
    assert!(raw.starts_with("mt_app_"));
    (data["id"].as_i64().unwrap(), raw.to_owned())
}

fn database() -> postgres::Client {
    postgres::Client::connect(
        &env::var("CONTRACT_AUDIT_DATABASE_URL").expect("contract database URL"),
        postgres::NoTls,
    )
    .expect("contract database")
}

fn idempotency_receipt(household_id: i64, key: &str) -> String {
    database()
        .query_one(
            "SELECT response_body::text FROM api_idempotency_keys WHERE household_id = $1 AND key = $2",
            &[&household_id, &key],
        )
        .expect("keyed app-token receipt")
        .get(0)
}

fn security_audit(household_id: i64, event_type: &str) -> (i64, String) {
    let row = database()
        .query_one(
            "SELECT count(*), coalesce(string_agg(metadata::text, ' '), '') FROM security_audit_events WHERE household_id = $1 AND event_type = $2",
            &[&household_id, &event_type],
        )
        .expect("security audit events");
    (row.get(0), row.get(1))
}

#[test]
fn app_token_request_schema_rejects_missing_and_unknown_fields_without_mutation() {
    let target = Target::from_env();
    let fixture = fixture();
    let path = tokens_path(&fixture);
    let before = body(target.get(&path, Some(&fixture.access_token)))["data"].clone();

    assert_error(
        target.post_json_authorized(&path, &fixture.access_token, &json!({})),
        400,
        "bad_request",
    );
    for payload in [
        json!({"api_app_token": {"name": "unknown root"}, "extra": true}),
        json!({"api_app_token": {"name": "unknown inner", "extra": true}}),
        json!({"api_app_token": {"name": null}}),
        json!({"api_app_token": {"name": 17}}),
        json!({"api_app_token": {"name": "null expiry", "expires_at": null}}),
        json!({"api_app_token": {"name": "bad expiry", "expires_at": "tomorrow"}}),
    ] {
        assert_error(
            target.post_json_authorized(&path, &fixture.access_token, &payload),
            422,
            "validation_failed",
        );
    }

    let after = body(target.get(&path, Some(&fixture.access_token)))["data"].clone();
    assert_eq!(after, before);
}

#[test]
fn app_token_create_and_collection_keep_secret_confined_to_create_response() {
    let target = Target::from_env();
    let fixture = fixture();
    let path = tokens_path(&fixture);
    let (id, raw) = create_token(&target, &fixture, "OpenAPI contract probe");

    let list = body(target.get(&path, Some(&fixture.access_token)));
    let rows = list["data"].as_array().expect("token collection");
    let summary = rows
        .iter()
        .find(|row| row["id"] == id)
        .expect("created token listed");
    let fields = summary.as_object().expect("summary object");
    assert_eq!(fields.len(), 6);
    for field in [
        "id",
        "name",
        "last_used_at",
        "revoked_at",
        "permissions_version",
        "expires_at",
    ] {
        assert!(fields.contains_key(field), "missing summary field {field}");
    }
    assert!(summary["id"].is_i64());
    assert!(summary["name"].is_string());
    assert!(summary["last_used_at"].is_null() || summary["last_used_at"].is_string());
    assert!(summary["revoked_at"].is_null() || summary["revoked_at"].is_string());
    assert!(summary["permissions_version"].is_u64());
    assert!(summary["expires_at"].is_string());
    assert!(summary.get("token").is_none());
    assert!(summary.get("token_digest").is_none());
    assert!(!list.to_string().contains(&raw));

    let item = format!("{path}/{id}");
    let revoked = target.delete(&item, Some(&fixture.access_token));
    assert_eq!(revoked.status().as_u16(), 204);
    let household_me = format!("/api/v1/households/{}/me", fixture.household_id);
    assert_error(target.get(&household_me, Some(&raw)), 401, "unauthorized");
}

#[test]
fn app_token_delete_hides_malformed_and_negative_ids_with_json_not_found() {
    let target = Target::from_env();
    let fixture = fixture();
    let path = tokens_path(&fixture);
    for id in ["-1", "not-a-number"] {
        assert_error(
            target.delete(&format!("{path}/{id}"), Some(&fixture.access_token)),
            404,
            "not_found",
        );
    }
}

#[test]
fn keyed_app_token_issue_never_replays_or_persists_the_one_time_secret() {
    let target = Target::from_env();
    let fixture = fixture();
    let path = tokens_path(&fixture);
    let key = format!(
        "app-token-{}-{}",
        fixture.household_id,
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock")
            .as_nanos()
    );
    let payload = json!({"api_app_token": {"name": "Keyed token receipt probe"}});
    let created_before = security_audit(fixture.household_id, "auth_token/api_app_token/created").0;

    let first = target.post_json_with_key(&path, &fixture.access_token, &key, &payload);
    assert_eq!(first.status().as_u16(), 201);
    assert!(first.headers().get("idempotency-replayed").is_none());
    let issued = body(first);
    let id = issued["data"]["id"].as_i64().expect("issued token ID");
    let raw = issued["data"]["token"]
        .as_str()
        .expect("first response includes raw token")
        .to_owned();

    let receipt = idempotency_receipt(fixture.household_id, &key);
    assert!(!receipt.contains(&raw));
    let created_after = security_audit(fixture.household_id, "auth_token/api_app_token/created");
    assert_eq!(created_after.0, created_before + 1);
    assert!(!created_after.1.contains(&raw));

    assert_error(
        target.post_json_with_key(&path, &raw, &key, &payload),
        409,
        "token_already_issued",
    );
    assert_error(
        target.post_json_with_key(
            &path,
            &fixture.access_token,
            &key,
            &json!({"api_app_token": {"name": "Changed keyed payload"}}),
        ),
        409,
        "idempotency_key_reused",
    );

    let listing = body(target.get(&path, Some(&fixture.access_token)));
    let matches = listing["data"]
        .as_array()
        .expect("token list")
        .iter()
        .filter(|row| row["id"] == id)
        .count();
    assert_eq!(matches, 1);
    assert!(!listing.to_string().contains(&raw));

    let item = format!("{path}/{id}");
    let revoked_before = security_audit(fixture.household_id, "auth_token/api_app_token/revoked").0;
    assert_eq!(
        target
            .delete(&item, Some(&fixture.access_token))
            .status()
            .as_u16(),
        204
    );
    assert_eq!(
        target
            .delete(&item, Some(&fixture.access_token))
            .status()
            .as_u16(),
        204
    );
    let revoked_after = security_audit(fixture.household_id, "auth_token/api_app_token/revoked");
    assert_eq!(revoked_after.0, revoked_before + 1);
    assert!(!revoked_after.1.contains(&raw));
}

#[test]
fn all_three_app_token_operations_use_the_documented_shared_rate_limit() {
    let fixture = fixture();
    let base_url = env::var("CONTRACT_RATE_BASE_URL").expect("rate-limited API base URL");
    let tokens = format!(
        "{base_url}/api/v1/households/{}/admin/app_tokens",
        fixture.household_id
    );
    let client = reqwest::blocking::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(5))
        .build()
        .expect("app-token rate limit client");
    let started = Instant::now();
    let mut rejected = None;
    for _ in 0..601 {
        let response = client
            .get(&tokens)
            .bearer_auth(&fixture.access_token)
            .send()
            .expect("app-token list rate request");
        if response.status().as_u16() == 429 {
            rejected = Some(response);
            break;
        }
        assert_eq!(response.status().as_u16(), 200);
    }
    assert!(started.elapsed() < Duration::from_secs(60));
    assert_rate_limited(rejected.expect("app-token list rate limit"));
    assert_rate_limited(
        client
            .post(&tokens)
            .bearer_auth(&fixture.access_token)
            .json(&json!({"api_app_token": {"name": "Rate limited no-op"}}))
            .send()
            .expect("app-token create rate request"),
    );
    assert_rate_limited(
        client
            .delete(format!("{tokens}/{}", fixture.manager_app_token_id))
            .bearer_auth(&fixture.access_token)
            .send()
            .expect("app-token delete rate request"),
    );
}
