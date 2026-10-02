use medtracker_contract_tests::{fixture, Target};
use reqwest::blocking::{Client, RequestBuilder, Response};
use serde_json::Value;
use std::env;
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::Duration;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

fn settings_path(household_id: i64) -> String {
    format!("/api/v1/households/{household_id}/admin/settings")
}

fn assert_api_error(response: Response, status: u16) -> Value {
    assert_eq!(response.status().as_u16(), status);
    let body: Value = response.json().expect("error JSON");
    assert!(body["error"]["code"].is_string());
    assert!(body["error"]["request_id"].is_string());
    assert!(body["error"]["message"].is_string());
    assert!(body.get("data").is_none());
    body
}

fn assert_settings(data: &Value, household_id: i64) {
    let data = data.as_object().expect("settings data object");
    assert_eq!(data.len(), 6);
    for key in [
        "id",
        "name",
        "slug",
        "timezone",
        "subscription_plan",
        "updated_at",
    ] {
        assert!(data.contains_key(key), "missing settings field {key}");
    }
    assert_eq!(data["id"], household_id);
    assert!(data["name"]
        .as_str()
        .is_some_and(|value| !value.trim().is_empty()));
    assert!(data["slug"].as_str().is_some_and(|value| !value.is_empty()));
    assert!(data["timezone"]
        .as_str()
        .is_some_and(|value| !value.is_empty()));
    assert!(matches!(
        data["subscription_plan"].as_str(),
        Some("free" | "family_plus")
    ));
    OffsetDateTime::parse(
        data["updated_at"].as_str().expect("settings timestamp"),
        &Rfc3339,
    )
    .expect("RFC3339 settings timestamp");
}

fn assert_settings_envelope(body: &Value, household_id: i64) {
    assert_eq!(body.as_object().expect("settings response object").len(), 1);
    assert_settings(&body["data"], household_id);
}

fn request_id(response: &Response) -> String {
    response.headers()["x-request-id"]
        .to_str()
        .expect("request ID")
        .to_owned()
}

fn admin_audit_events(target: &Target, household_id: i64, token: &str, request_id: &str) -> usize {
    let response = target.get(
        &format!("/api/v1/households/{household_id}/admin/audit_logs"),
        Some(token),
    );
    assert_eq!(response.status().as_u16(), 200);
    let body: Value = response.json().expect("audit log JSON");
    body["data"]
        .as_array()
        .expect("audit log array")
        .iter()
        .filter(|event| {
            event["request_id"] == request_id
                && event["event_type"] == "api/admin/household_settings/updated"
        })
        .count()
}

fn settings_audit_count(household_id: i64) -> i64 {
    let mut db = postgres::Client::connect(
        &env::var("CONTRACT_AUDIT_DATABASE_URL").expect("audit database URL"),
        postgres::NoTls,
    )
    .expect("contract database");
    db.query_one(
        "SELECT count(*) FROM security_audit_events WHERE household_id = $1 AND event_type = 'api/admin/household_settings/updated'",
        &[&household_id],
    )
    .expect("settings security audit count")
    .get(0)
}

struct MembershipRoleGuard {
    db: postgres::Client,
    membership_id: i64,
    original_role: String,
}

impl MembershipRoleGuard {
    fn demote_to_member(membership_id: i64) -> Self {
        let mut db = postgres::Client::connect(
            &env::var("CONTRACT_AUDIT_DATABASE_URL").expect("audit database URL"),
            postgres::NoTls,
        )
        .expect("contract database");
        let original_role: String = db
            .query_one(
                "SELECT role FROM household_memberships WHERE id = $1",
                &[&membership_id],
            )
            .expect("manager membership role")
            .get(0);
        assert_eq!(original_role, "administrator");
        db.execute(
            "UPDATE household_memberships SET role = 'member' WHERE id = $1",
            &[&membership_id],
        )
        .expect("temporarily demote administrator");
        Self {
            db,
            membership_id,
            original_role,
        }
    }
}

impl Drop for MembershipRoleGuard {
    fn drop(&mut self) {
        self.db
            .execute(
                "UPDATE household_memberships SET role = $2 WHERE id = $1",
                &[&self.membership_id, &self.original_role],
            )
            .expect("restore administrator role");
    }
}

fn idempotent_request(method: &str, path: &str, token: &str, key: &str, body: &Value) -> Response {
    let base = env::var("CONTRACT_BASE_URL").expect("API origin");
    let client = Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(10))
        .build()
        .expect("idempotency client");
    let url = format!("{base}{path}");
    let request: RequestBuilder = match method {
        "PATCH" => client.patch(url),
        "PUT" => client.put(url),
        _ => panic!("unsupported settings method"),
    };
    request
        .bearer_auth(token)
        .header("Idempotency-Key", key)
        .json(body)
        .send()
        .expect("idempotent settings request")
}

fn malformed_json_request(method: &str, path: &str, token: &str) -> Response {
    let base = env::var("CONTRACT_BASE_URL").expect("API origin");
    let client = Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(10))
        .build()
        .expect("malformed JSON client");
    let url = format!("{base}{path}");
    let request: RequestBuilder = match method {
        "PATCH" => client.patch(url),
        "PUT" => client.put(url),
        _ => panic!("unsupported settings method"),
    };
    request
        .bearer_auth(token)
        .header("Content-Type", "application/json")
        .body("{".to_owned())
        .send()
        .expect("malformed JSON request")
}

#[test]
fn owner_can_read_the_exact_household_admin_settings_response() {
    let fixture = fixture();
    let target = Target::from_env();
    let response = target.get(
        &settings_path(fixture.household_id),
        Some(&fixture.access_token),
    );
    assert_eq!(response.status().as_u16(), 200);
    let body: Value = response.json().expect("settings JSON");
    assert_settings_envelope(&body, fixture.household_id);
}

#[test]
fn get_enforces_owner_admin_member_foreign_and_missing_household_boundaries() {
    let fixture = fixture();
    let target = Target::from_env();
    let path = settings_path(fixture.household_id);
    for token in [&fixture.access_token, &fixture.manager_access_token] {
        let response = target.get(&path, Some(token));
        assert_eq!(response.status().as_u16(), 200);
        let body: Value = response.json().expect("settings JSON");
        assert_settings_envelope(&body, fixture.household_id);
    }
    assert_api_error(target.get(&path, None), 401);
    assert_api_error(target.get(&path, Some("invalid-token")), 401);
    assert_api_error(target.get(&path, Some(&fixture.view_access_token)), 403);
    assert_api_error(target.get(&path, Some(&fixture.foreign_access_token)), 403);
    assert_api_error(
        target.get(
            &settings_path(fixture.foreign_household_id),
            Some(&fixture.access_token),
        ),
        403,
    );
    assert_api_error(
        target.get(&settings_path(i64::MAX), Some(&fixture.access_token)),
        404,
    );
}

#[test]
fn patch_and_put_merge_allowed_fields_and_preserve_slug_and_omitted_values() {
    let fixture = fixture();
    let target = Target::from_env();
    let path = settings_path(fixture.household_id);
    let original_response = target.get(&path, Some(&fixture.access_token));
    assert_eq!(original_response.status().as_u16(), 200);
    let original_body: Value = original_response.json().expect("original settings");
    let original = original_body["data"].clone();

    let patch = target.patch_json(
        &path,
        &fixture.manager_access_token,
        &serde_json::json!({"household": {"name": "Contract admin settings", "subscription_plan": "family_plus"}}),
    );
    assert_eq!(patch.status().as_u16(), 200);
    let patch_id = request_id(&patch);
    let patch_body: Value = patch.json().expect("PATCH settings JSON");
    assert_settings_envelope(&patch_body, fixture.household_id);
    assert_eq!(patch_body["data"]["name"], "Contract admin settings");
    assert_eq!(patch_body["data"]["subscription_plan"], "family_plus");
    assert_eq!(patch_body["data"]["slug"], original["slug"]);
    assert_eq!(patch_body["data"]["timezone"], original["timezone"]);
    assert_eq!(
        admin_audit_events(
            &target,
            fixture.household_id,
            &fixture.access_token,
            &patch_id
        ),
        1
    );

    let put = target.put_json(
        &path,
        &fixture.access_token,
        &serde_json::json!({"household": {"timezone": "Europe/Paris"}}),
    );
    assert_eq!(put.status().as_u16(), 200);
    let put_id = request_id(&put);
    let put_body: Value = put.json().expect("PUT settings JSON");
    assert_settings_envelope(&put_body, fixture.household_id);
    assert_eq!(put_body["data"]["name"], "Contract admin settings");
    assert_eq!(put_body["data"]["timezone"], "Europe/Paris");
    assert_eq!(put_body["data"]["subscription_plan"], "family_plus");
    assert_eq!(put_body["data"]["slug"], original["slug"]);
    assert_eq!(
        admin_audit_events(
            &target,
            fixture.household_id,
            &fixture.access_token,
            &put_id
        ),
        1
    );

    let restore = target.patch_json(
        &path,
        &fixture.access_token,
        &serde_json::json!({"household": {
            "name": original["name"],
            "timezone": original["timezone"],
            "subscription_plan": original["subscription_plan"]
        }}),
    );
    assert_eq!(restore.status().as_u16(), 200);
    let restored: Value = restore.json().expect("restored settings");
    assert_eq!(restored["data"]["name"], original["name"]);
    assert_eq!(restored["data"]["timezone"], original["timezone"]);
    assert_eq!(
        restored["data"]["subscription_plan"],
        original["subscription_plan"]
    );
    assert_eq!(restored["data"]["slug"], original["slug"]);
}

#[test]
fn patch_and_put_reject_malformed_and_invalid_settings_without_mutation() {
    let fixture = fixture();
    let target = Target::from_env();
    let path = settings_path(fixture.household_id);
    let before_response = target.get(&path, Some(&fixture.access_token));
    assert_eq!(before_response.status().as_u16(), 200);
    let before: Value = before_response
        .json()
        .expect("settings before invalid requests");
    let invalid = [
        (serde_json::json!({}), 400),
        (serde_json::json!({"household": {}}), 422),
        (
            serde_json::json!({"household": {"name": "Unknown key", "extra": true}}),
            422,
        ),
        (serde_json::json!({"household": {"name": " "}}), 422),
        (serde_json::json!({"household": {"timezone": ""}}), 422),
        (
            serde_json::json!({"household": {"subscription_plan": "invalid"}}),
            422,
        ),
        (serde_json::json!({"household": {"name": 7}}), 422),
        (
            serde_json::json!({"household": {"name": "Bad root", "unknown": true}, "extra": 1}),
            422,
        ),
    ];
    for method in ["PATCH", "PUT"] {
        for (payload, status) in &invalid {
            let response = if method == "PATCH" {
                target.patch_json(&path, &fixture.access_token, payload)
            } else {
                target.put_json(&path, &fixture.access_token, payload)
            };
            assert_api_error(response, *status);
            let after_response = target.get(&path, Some(&fixture.access_token));
            assert_eq!(after_response.status().as_u16(), 200);
            let after: Value = after_response
                .json()
                .expect("settings after invalid request");
            assert_eq!(after["data"], before["data"]);
        }
    }
}

#[test]
fn patch_and_put_apply_authentication_and_household_boundaries_without_mutation() {
    let fixture = fixture();
    let target = Target::from_env();
    let path = settings_path(fixture.household_id);
    let before_response = target.get(&path, Some(&fixture.access_token));
    assert_eq!(before_response.status().as_u16(), 200);
    let before: Value = before_response
        .json()
        .expect("settings before denied writes");
    let payload = serde_json::json!({"household": {"name": "Denied settings write"}});

    for method in ["PATCH", "PUT"] {
        let missing_auth = if method == "PATCH" {
            target.patch_json_without_auth(&path, &payload)
        } else {
            target.put_json_without_auth(&path, &payload)
        };
        assert_api_error(missing_auth, 401);

        for (token, request_path, status) in [
            ("invalid-token", path.clone(), 401),
            (fixture.view_access_token.as_str(), path.clone(), 403),
            (fixture.foreign_access_token.as_str(), path.clone(), 403),
            (
                fixture.access_token.as_str(),
                settings_path(fixture.foreign_household_id),
                403,
            ),
            (fixture.access_token.as_str(), settings_path(i64::MAX), 404),
        ] {
            let response = if method == "PATCH" {
                target.patch_json(&request_path, token, &payload)
            } else {
                target.put_json(&request_path, token, &payload)
            };
            assert_api_error(response, status);
            let after_response = target.get(&path, Some(&fixture.access_token));
            assert_eq!(after_response.status().as_u16(), 200);
            let after: Value = after_response.json().expect("settings after denied write");
            assert_eq!(after["data"], before["data"]);
        }

        assert_api_error(
            malformed_json_request(method, &path, &fixture.access_token),
            400,
        );
        let after_response = target.get(&path, Some(&fixture.access_token));
        assert_eq!(after_response.status().as_u16(), 200);
        let after: Value = after_response
            .json()
            .expect("settings after malformed JSON");
        assert_eq!(after["data"], before["data"]);
    }
}

#[test]
fn put_idempotency_replays_and_rejects_a_changed_payload() {
    let fixture = fixture();
    let target = Target::from_env();
    let path = settings_path(fixture.household_id);
    let initial = target.get(&path, Some(&fixture.access_token));
    assert_eq!(initial.status().as_u16(), 200);
    let original: Value = initial.json().expect("original settings");
    let key = format!("admin-settings-put-{}", fixture.household_id);
    let payload = serde_json::json!({"household": {"name": "Idempotent PUT"}});
    let first = idempotent_request("PUT", &path, &fixture.manager_access_token, &key, &payload);
    assert_eq!(first.status().as_u16(), 200);
    let first_body: Value = first.json().expect("first PUT response");
    let replay = idempotent_request("PUT", &path, &fixture.manager_app_token, &key, &payload);
    assert_eq!(replay.status().as_u16(), 200);
    assert_eq!(replay.headers()["idempotency-replayed"], "true");
    let replay_body: Value = replay.json().expect("replayed PUT response");
    assert_eq!(replay_body, first_body);
    assert_api_error(
        idempotent_request(
            "PUT",
            &path,
            &fixture.manager_access_token,
            &key,
            &serde_json::json!({"household": {"name": "Changed PUT"}}),
        ),
        409,
    );
    let restore = target.patch_json(
        &path,
        &fixture.access_token,
        &serde_json::json!({"household": {
            "name": original["data"]["name"],
            "timezone": original["data"]["timezone"],
            "subscription_plan": original["data"]["subscription_plan"]
        }}),
    );
    assert_eq!(restore.status().as_u16(), 200);
}

#[test]
fn idempotency_replays_once_conflicts_on_reuse_and_rechecks_current_authority() {
    let fixture = fixture();
    let target = Target::from_env();
    let path = settings_path(fixture.household_id);
    let initial = target.get(&path, Some(&fixture.access_token));
    assert_eq!(initial.status().as_u16(), 200);
    let original: Value = initial.json().expect("original settings");
    let key = format!("admin-settings-{}", fixture.household_id);
    let payload = serde_json::json!({"household": {"name": "Idempotent settings"}});
    let audits_before = settings_audit_count(fixture.household_id);

    let first = idempotent_request(
        "PATCH",
        &path,
        &fixture.manager_access_token,
        &key,
        &payload,
    );
    assert_eq!(first.status().as_u16(), 200);
    let first_id = request_id(&first);
    let first_body: Value = first.json().expect("first idempotent response");
    assert_eq!(first_body["data"]["name"], "Idempotent settings");
    assert_eq!(
        settings_audit_count(fixture.household_id),
        audits_before + 1
    );
    assert_eq!(
        admin_audit_events(
            &target,
            fixture.household_id,
            &fixture.access_token,
            &first_id
        ),
        1
    );

    let replay = idempotent_request("PATCH", &path, &fixture.manager_app_token, &key, &payload);
    assert_eq!(replay.status().as_u16(), 200);
    assert_eq!(replay.headers()["idempotency-replayed"], "true");
    let replay_body: Value = replay.json().expect("replayed response");
    assert_eq!(replay_body, first_body);
    assert_eq!(
        settings_audit_count(fixture.household_id),
        audits_before + 1
    );
    assert_eq!(
        admin_audit_events(
            &target,
            fixture.household_id,
            &fixture.access_token,
            &first_id
        ),
        1
    );

    assert_api_error(
        idempotent_request(
            "PATCH",
            &path,
            &fixture.manager_access_token,
            &key,
            &serde_json::json!({"household": {"name": "Different payload"}}),
        ),
        409,
    );
    assert_api_error(
        idempotent_request("PATCH", &path, &fixture.access_token, &key, &payload),
        409,
    );
    assert_api_error(
        idempotent_request("PATCH", &path, &fixture.view_access_token, &key, &payload),
        403,
    );
    {
        let _role_guard = MembershipRoleGuard::demote_to_member(fixture.manager_membership_id);
        assert_api_error(
            idempotent_request(
                "PATCH",
                &path,
                &fixture.manager_access_token,
                &key,
                &payload,
            ),
            403,
        );
    }
    let after_conflict = target.get(&path, Some(&fixture.access_token));
    assert_eq!(after_conflict.status().as_u16(), 200);
    let after_conflict_body: Value = after_conflict.json().expect("settings after conflict");
    assert_eq!(after_conflict_body["data"]["name"], "Idempotent settings");

    let restore = target.patch_json(
        &path,
        &fixture.access_token,
        &serde_json::json!({"household": {
            "name": original["data"]["name"],
            "timezone": original["data"]["timezone"],
            "subscription_plan": original["data"]["subscription_plan"]
        }}),
    );
    assert_eq!(restore.status().as_u16(), 200);
}

#[test]
fn concurrent_identical_idempotency_requests_write_one_settings_audit() {
    let fixture = fixture();
    let target = Target::from_env();
    let path = settings_path(fixture.household_id);
    let initial = target.get(&path, Some(&fixture.access_token));
    assert_eq!(initial.status().as_u16(), 200);
    let original: Value = initial.json().expect("original settings");
    let key = format!("admin-settings-concurrent-{}", fixture.household_id);
    let payload = serde_json::json!({"household": {"name": "Concurrent idempotent settings"}});
    let audits_before = settings_audit_count(fixture.household_id);
    let barrier = Arc::new(Barrier::new(3));
    let handles: Vec<_> = (0..2)
        .map(|_| {
            let barrier = Arc::clone(&barrier);
            let path = path.clone();
            let key = key.clone();
            let payload = payload.clone();
            let token = fixture.manager_access_token.clone();
            thread::spawn(move || {
                barrier.wait();
                let response = idempotent_request("PATCH", &path, &token, &key, &payload);
                let status = response.status().as_u16();
                let replayed = response
                    .headers()
                    .get("idempotency-replayed")
                    .and_then(|value| value.to_str().ok())
                    == Some("true");
                let body: Value = response.json().expect("concurrent response JSON");
                (status, replayed, body)
            })
        })
        .collect();
    barrier.wait();
    let responses: Vec<_> = handles
        .into_iter()
        .map(|handle| handle.join().expect("idempotency request worker"))
        .collect();
    assert!(responses.iter().all(|response| response.0 == 200));
    assert!(responses
        .iter()
        .all(|response| response.2 == responses[0].2));
    assert_eq!(responses.iter().filter(|response| response.1).count(), 1);
    assert_eq!(
        settings_audit_count(fixture.household_id),
        audits_before + 1
    );
    assert_eq!(
        responses[0].2["data"]["name"],
        "Concurrent idempotent settings"
    );

    let restore = target.patch_json(
        &path,
        &fixture.access_token,
        &serde_json::json!({"household": {
            "name": original["data"]["name"],
            "timezone": original["data"]["timezone"],
            "subscription_plan": original["data"]["subscription_plan"]
        }}),
    );
    assert_eq!(restore.status().as_u16(), 200);
}

#[test]
fn idempotency_key_can_be_reused_after_its_expiry() {
    let fixture = fixture();
    let target = Target::from_env();
    let path = settings_path(fixture.household_id);
    let initial = target.get(&path, Some(&fixture.access_token));
    assert_eq!(initial.status().as_u16(), 200);
    let original: Value = initial.json().expect("original settings");
    let key = format!("admin-settings-expiry-{}", fixture.household_id);
    let first = idempotent_request(
        "PATCH",
        &path,
        &fixture.manager_access_token,
        &key,
        &serde_json::json!({"household": {"name": "Before expiry"}}),
    );
    assert_eq!(first.status().as_u16(), 200);

    let mut db = postgres::Client::connect(
        &env::var("CONTRACT_AUDIT_DATABASE_URL").expect("audit database URL"),
        postgres::NoTls,
    )
    .expect("contract database");
    let changed = db
        .execute(
            "UPDATE api_idempotency_keys SET expires_at = now() - interval '1 second' WHERE household_id = $1 AND key = $2",
            &[&fixture.household_id, &key],
        )
        .expect("expire idempotency key");
    assert_eq!(changed, 1);

    let after_expiry = idempotent_request(
        "PATCH",
        &path,
        &fixture.manager_access_token,
        &key,
        &serde_json::json!({"household": {"name": "After expiry"}}),
    );
    assert_eq!(after_expiry.status().as_u16(), 200);
    let body: Value = after_expiry.json().expect("post-expiry response");
    assert_eq!(body["data"]["name"], "After expiry");

    let restore = target.patch_json(
        &path,
        &fixture.access_token,
        &serde_json::json!({"household": {
            "name": original["data"]["name"],
            "timezone": original["data"]["timezone"],
            "subscription_plan": original["data"]["subscription_plan"]
        }}),
    );
    assert_eq!(restore.status().as_u16(), 200);
}

#[test]
fn z_rate_limiter_wires_get_patch_and_put() {
    let fixture = fixture();
    let base = env::var("CONTRACT_RATE_BASE_URL").expect("rate-limited API URL");
    let client = Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(5))
        .build()
        .expect("rate limit client");
    let started = std::time::Instant::now();
    let mut limited = false;
    for _ in 0..601 {
        let response = client
            .get(format!("{base}/api/v1/capabilities"))
            .send()
            .expect("rate limit request");
        if response.status().as_u16() == 429 {
            assert_eq!(response.headers()["ratelimit-limit"], "300");
            assert_eq!(response.headers()["ratelimit-remaining"], "0");
            assert!(response.headers().get("retry-after").is_some());
            let body: Value = response.json().expect("rate limit JSON");
            assert_eq!(body["error"]["code"], "rate_limited");
            limited = true;
            break;
        }
        assert_eq!(response.status().as_u16(), 200);
    }
    assert!(limited && started.elapsed() < Duration::from_secs(60));
    let path = format!("{base}{}", settings_path(fixture.household_id));
    for response in [
        client.get(&path).bearer_auth(&fixture.access_token).send(),
        client
            .patch(&path)
            .bearer_auth(&fixture.access_token)
            .json(&serde_json::json!({"household": {"name": "Rate denied"}}))
            .send(),
        client
            .put(&path)
            .bearer_auth(&fixture.access_token)
            .json(&serde_json::json!({"household": {"name": "Rate denied"}}))
            .send(),
    ] {
        assert_eq!(
            response
                .expect("rate limited settings endpoint")
                .status()
                .as_u16(),
            429
        );
    }
}
