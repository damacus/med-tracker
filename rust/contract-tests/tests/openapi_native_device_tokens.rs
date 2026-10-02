use medtracker_contract_tests::{fixture, Target};
use serde_json::{json, Value};
use std::env;
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

fn path(household_id: i64) -> String {
    format!("/api/v1/households/{household_id}/native_device_tokens")
}

fn unique_token() -> String {
    format!(
        "contract-device-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock")
            .as_nanos()
    )
}

fn database() -> postgres::Client {
    postgres::Client::connect(
        &env::var("CONTRACT_AUDIT_DATABASE_URL").expect("contract database URL"),
        postgres::NoTls,
    )
    .expect("contract database")
}

fn request_id(response: &reqwest::blocking::Response) -> String {
    response.headers()["x-request-id"]
        .to_str()
        .expect("request ID")
        .to_owned()
}

fn assert_error(response: reqwest::blocking::Response, status: u16, code: &str) -> Value {
    assert_eq!(response.status().as_u16(), status);
    let request_header = response
        .headers()
        .get("x-request-id")
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let body: Value = response.json().expect("error JSON");
    assert_eq!(body["error"]["code"], code);
    let request_body = body["error"]["request_id"]
        .as_str()
        .expect("error request ID");
    assert!(!request_body.is_empty());
    if let Some(request_header) = request_header {
        assert_eq!(request_body, request_header);
    }
    body
}

fn row_count(account_id: i64, device_token: &str) -> i64 {
    database()
        .query_one(
            "SELECT count(*) FROM native_device_tokens WHERE account_id = $1 AND device_token = $2",
            &[&account_id, &device_token],
        )
        .expect("native device token row count")
        .get(0)
}

fn account_for_membership(membership_id: i64) -> i64 {
    database()
        .query_one(
            "SELECT account_id FROM household_memberships WHERE id = $1",
            &[&membership_id],
        )
        .expect("household membership account")
        .get(0)
}

fn assert_token_absent_from_audit(request: &str, device_token: &str) {
    let rows = database()
        .query(
            "SELECT metadata::text FROM security_audit_events WHERE request_id = $1",
            &[&request],
        )
        .expect("request audit metadata");
    assert!(!rows.is_empty());
    assert!(rows
        .iter()
        .all(|row| !row.get::<_, String>(0).contains(device_token)));
}

#[test]
fn native_device_token_registration_returns_201_without_a_response_body() {
    let fixture = fixture();
    let target = Target::from_env();
    let path = path(fixture.household_id);
    let token = unique_token();
    let response = target.post_json_authorized(
        &path,
        &fixture.access_token,
        &json!({"native_device_token": {
            "device_token": token,
            "platform": "ios",
            "apns_environment": "sandbox"
        }}),
    );
    let first_request = request_id(&response);
    assert_eq!(response.status().as_u16(), 201);
    assert!(response.text().expect("empty response body").is_empty());
    let replay = target.post_json_with_header(
        &path,
        &fixture.access_token,
        "User-Agent",
        "MedTracker contract replay/2",
        &json!({"native_device_token": {
            "device_token": token,
            "platform": "android",
            "apns_environment": "production"
        }}),
    );
    let replay_request = request_id(&replay);
    assert_eq!(replay.status().as_u16(), 201);
    assert!(replay.text().expect("empty replay body").is_empty());
    let row = database()
        .query_one(
            "SELECT count(*), max(platform), max(apns_environment), max(user_agent) FROM native_device_tokens WHERE account_id = $1 AND device_token = $2",
            &[&fixture.account_id, &token],
        )
        .expect("replayed token");
    assert_eq!(row.get::<_, i64>(0), 1);
    assert_eq!(row.get::<_, Option<String>>(1).as_deref(), Some("android"));
    assert_eq!(
        row.get::<_, Option<String>>(2).as_deref(),
        Some("production")
    );
    assert_eq!(
        row.get::<_, Option<String>>(3).as_deref(),
        Some("MedTracker contract replay/2")
    );
    let omitted_apns = target.post_json_with_header(
        &path,
        &fixture.access_token,
        "User-Agent",
        "MedTracker contract replay/2",
        &json!({"native_device_token": {
            "device_token": token,
            "platform": "ios"
        }}),
    );
    let omitted_request = request_id(&omitted_apns);
    assert_eq!(omitted_apns.status().as_u16(), 201);
    assert!(omitted_apns.text().expect("empty repeated body").is_empty());
    let row = database()
        .query_one(
            "SELECT platform, apns_environment, user_agent FROM native_device_tokens WHERE account_id = $1 AND device_token = $2",
            &[&fixture.account_id, &token],
        )
        .expect("replayed token values");
    assert_eq!(row.get::<_, String>(0), "ios");
    assert_eq!(
        row.get::<_, Option<String>>(1).as_deref(),
        Some("production")
    );
    assert_eq!(
        row.get::<_, Option<String>>(2).as_deref(),
        Some("MedTracker contract replay/2")
    );
    assert_token_absent_from_audit(&first_request, &token);
    assert_token_absent_from_audit(&replay_request, &token);
    assert_token_absent_from_audit(&omitted_request, &token);
    let deleted = target.delete(&format!("{path}/{token}"), Some(&fixture.access_token));
    let delete_request = request_id(&deleted);
    assert_eq!(deleted.status().as_u16(), 204);
    assert!(deleted.text().expect("empty delete response").is_empty());
    assert_token_absent_from_audit(&delete_request, &token);
    assert_eq!(row_count(fixture.account_id, &token), 0);
}

#[test]
fn native_device_token_validation_and_authority_fail_without_persistence() {
    let fixture = fixture();
    let target = Target::from_env();
    let path = path(fixture.household_id);
    let token = unique_token();

    for attributes in [
        json!({"device_token": "", "platform": "ios"}),
        json!({"device_token": "   ", "platform": "ios"}),
        json!({"device_token": token}),
        json!({"device_token": token, "platform": "windows"}),
        json!({"device_token": token, "platform": "ios", "apns_environment": "staging"}),
        json!({"device_token": token, "platform": "ios", "apns_environment": null}),
        json!({"device_token": token, "platform": "ios", "unexpected": true}),
    ] {
        let device_token = attributes["device_token"]
            .as_str()
            .expect("token attribute");
        let error = assert_error(
            target.post_json_authorized(
                &path,
                &fixture.access_token,
                &json!({"native_device_token": attributes}),
            ),
            422,
            "validation_failed",
        );
        if !device_token.trim().is_empty() {
            assert!(!error.to_string().contains(device_token));
        }
        assert_eq!(row_count(fixture.account_id, device_token), 0);
    }

    let wrapper_error = assert_error(
        target.post_json_authorized(
            &path,
            &fixture.access_token,
            &json!({"wrong_wrapper": {"device_token": token, "platform": "ios"}}),
        ),
        400,
        "bad_request",
    );
    assert!(!wrapper_error.to_string().contains(&token));

    let request = json!({"native_device_token": {"device_token": token, "platform": "ios"}});
    assert_error(target.post_json(&path, &request), 401, "unauthorized");
    assert_error(
        target.post_json_authorized(&path, &fixture.expired_access_token, &request),
        401,
        "unauthorized",
    );
    assert_error(
        target.post_json_authorized(&path, &fixture.foreign_access_token, &request),
        403,
        "forbidden",
    );
    assert_eq!(row_count(fixture.account_id, &token), 0);
}

#[test]
fn native_device_token_delete_is_idempotent_and_account_scoped() {
    let fixture = fixture();
    let target = Target::from_env();
    let owner_path = path(fixture.household_id);
    let token = unique_token();
    let body = json!({"native_device_token": {"device_token": token, "platform": "ios"}});
    let created = target.post_json_authorized(&owner_path, &fixture.access_token, &body);
    assert_eq!(created.status().as_u16(), 201);
    let request = format!("{owner_path}/{token}");
    assert_error(target.delete(&request, None), 401, "unauthorized");
    assert_error(
        target.delete(&request, Some(&fixture.foreign_access_token)),
        403,
        "forbidden",
    );
    let foreign_request = format!("{}/{}", path(fixture.foreign_household_id), token);
    assert_eq!(
        target
            .delete(&foreign_request, Some(&fixture.foreign_access_token))
            .status()
            .as_u16(),
        204
    );
    assert_eq!(row_count(fixture.account_id, &token), 1);
    for _ in 0..2 {
        let response = target.delete(&request, Some(&fixture.access_token));
        assert_eq!(response.status().as_u16(), 204);
        assert!(response.text().expect("empty delete response").is_empty());
    }
    let missing = target.delete(
        &format!("{owner_path}/{}", unique_token()),
        Some(&fixture.access_token),
    );
    assert_eq!(missing.status().as_u16(), 204);
    assert_eq!(row_count(fixture.account_id, &token), 0);
}

#[test]
fn native_device_token_claims_are_globally_unique_and_races_have_one_winner() {
    let fixture = fixture();
    let target = Target::from_env();
    let token = unique_token();
    let owner_path = path(fixture.household_id);
    let foreign_path = path(fixture.foreign_household_id);
    let body = json!({"native_device_token": {"device_token": token, "platform": "android"}});
    let created = target.post_json_authorized(&owner_path, &fixture.access_token, &body);
    assert_eq!(created.status().as_u16(), 201);
    let conflict = assert_error(
        target.post_json_authorized(&foreign_path, &fixture.foreign_access_token, &body),
        422,
        "validation_failed",
    );
    assert!(!conflict.to_string().contains(&token));
    assert_eq!(row_count(fixture.account_id, &token), 1);
    let foreign_account = account_for_membership(fixture.foreign_membership_id);
    assert_eq!(row_count(foreign_account, &token), 0);
    assert_eq!(
        target
            .delete(
                &format!("{owner_path}/{token}"),
                Some(&fixture.access_token)
            )
            .status()
            .as_u16(),
        204
    );
    let claimed = target.post_json_authorized(&foreign_path, &fixture.foreign_access_token, &body);
    assert_eq!(claimed.status().as_u16(), 201);
    assert!(claimed.text().expect("empty foreign claim body").is_empty());
    assert_eq!(row_count(foreign_account, &token), 1);

    let raced_token = unique_token();
    let raced_body =
        json!({"native_device_token": {"device_token": raced_token, "platform": "ios"}});
    let barrier = Arc::new(Barrier::new(2));
    let requests = [
        (owner_path.clone(), fixture.access_token.clone()),
        (foreign_path.clone(), fixture.foreign_access_token.clone()),
    ];
    let results = thread::scope(|scope| {
        let handles = requests
            .into_iter()
            .map(|(path, access_token)| {
                let barrier = barrier.clone();
                let body = raced_body.clone();
                scope.spawn(move || {
                    let target = Target::from_env();
                    barrier.wait();
                    let response = target.post_json_authorized(&path, &access_token, &body);
                    (
                        response.status().as_u16(),
                        response.text().unwrap_or_default(),
                    )
                })
            })
            .collect::<Vec<_>>();
        handles
            .into_iter()
            .map(|join| join.join().expect("claim request thread"))
            .collect::<Vec<_>>()
    });
    assert_eq!(
        results.iter().filter(|(status, _)| *status == 201).count(),
        1
    );
    assert_eq!(
        results.iter().filter(|(status, _)| *status == 422).count(),
        1
    );
    assert!(results
        .iter()
        .all(|(_, text)| text.is_empty() || !text.contains(&raced_token)));
    let conflict_body: Value = serde_json::from_str(
        &results
            .iter()
            .find(|(status, _)| *status == 422)
            .expect("one rejected claim")
            .1,
    )
    .expect("concurrent conflict JSON");
    assert_eq!(conflict_body["error"]["code"], "validation_failed");
    let rows = database()
        .query(
            "SELECT account_id FROM native_device_tokens WHERE device_token = $1",
            &[&raced_token],
        )
        .expect("concurrent token claim rows");
    assert_eq!(rows.len(), 1);
    let winner_account: i64 = rows[0].get(0);
    let (winner_household, winner_access) = if winner_account == fixture.account_id {
        (fixture.household_id, &fixture.access_token)
    } else {
        assert_eq!(
            winner_account,
            account_for_membership(fixture.foreign_membership_id)
        );
        (fixture.foreign_household_id, &fixture.foreign_access_token)
    };
    assert_eq!(
        target
            .delete(
                &format!("{}/{raced_token}", path(winner_household)),
                Some(winner_access),
            )
            .status()
            .as_u16(),
        204
    );
    assert_eq!(row_count(fixture.account_id, &token), 0);
    assert_eq!(row_count(foreign_account, &token), 1);
    let foreign_delete = format!("{foreign_path}/{token}");
    assert_eq!(
        target
            .delete(&foreign_delete, Some(&fixture.foreign_access_token))
            .status()
            .as_u16(),
        204
    );
    assert_eq!(row_count(foreign_account, &token), 0);
}

#[test]
fn native_device_token_create_and_delete_are_rate_limited() {
    let fixture = fixture();
    let base = env::var("CONTRACT_RATE_BASE_URL").expect("nonloopback API URL");
    let client = reqwest::blocking::Client::builder()
        .no_proxy()
        .timeout(std::time::Duration::from_secs(5))
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
    assert!(limited && started.elapsed() < std::time::Duration::from_secs(60));
    let path = format!(
        "{base}/api/v1/households/{}/native_device_tokens",
        fixture.household_id
    );
    let token = unique_token();
    let create = client
        .post(&path)
        .bearer_auth(&fixture.access_token)
        .json(&json!({"native_device_token": {"device_token": token, "platform": "ios"}}))
        .send()
        .expect("rate limited registration");
    assert_eq!(create.status().as_u16(), 429);
    let delete = client
        .delete(format!("{path}/{token}"))
        .bearer_auth(&fixture.access_token)
        .send()
        .expect("rate limited revocation");
    assert_eq!(delete.status().as_u16(), 429);
}
