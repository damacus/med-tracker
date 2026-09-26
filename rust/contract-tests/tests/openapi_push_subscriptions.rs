use medtracker_contract_tests::{fixture, Target};
use serde_json::{json, Value};
use std::env;
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

fn path(household_id: i64) -> String {
    format!("/api/v1/households/{household_id}/push_subscription")
}

fn unique_endpoint() -> String {
    format!(
        "https://fcm.googleapis.com/fcm/send/contract-{}?subscription=a%2Fb&mode=one",
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

fn row_count(account_id: i64, endpoint: &str) -> i64 {
    database()
        .query_one(
            "SELECT count(*) FROM push_subscriptions WHERE account_id = $1 AND endpoint = $2",
            &[&account_id, &endpoint],
        )
        .expect("push subscription row count")
        .get(0)
}

fn account_for_membership(membership_id: i64) -> i64 {
    database()
        .query_one(
            "SELECT account_id FROM household_memberships WHERE id = $1",
            &[&membership_id],
        )
        .expect("membership account")
        .get(0)
}

fn delete_query(path: &str, endpoint: &str) -> String {
    let query = url::form_urlencoded::Serializer::new(String::new())
        .append_pair("endpoint", endpoint)
        .finish();
    format!("{path}?{query}")
}

fn duplicate_delete_query(path: &str, first: &str, second: &str) -> String {
    let query = url::form_urlencoded::Serializer::new(String::new())
        .append_pair("endpoint", first)
        .append_pair("endpoint", second)
        .finish();
    format!("{path}?{query}")
}

fn subscription(endpoint: &str, p256dh: &str, auth: &str) -> Value {
    json!({"push_subscription": {"endpoint": endpoint, "keys": {
        "p256dh": p256dh, "auth": auth
    }}})
}

fn assert_secrets_absent_from_audit(request: &str, secrets: &[&str]) {
    let rows = database()
        .query(
            "SELECT metadata::text FROM security_audit_events WHERE request_id = $1",
            &[&request],
        )
        .expect("request audit metadata");
    assert!(!rows.is_empty());
    for row in rows {
        let metadata: String = row.get(0);
        for secret in secrets {
            assert!(!metadata.contains(secret));
        }
    }
}

#[test]
fn push_subscription_routes_register_and_revoke_account_owned_subscription() {
    let fixture = fixture();
    let target = Target::from_env();
    let path = path(fixture.household_id);
    let endpoint = unique_endpoint();
    let first_key = "contract-public-key-secret";
    let first_auth = "contract-auth-secret";
    let created = target.post_json_with_header(
        &path,
        &fixture.access_token,
        "User-Agent",
        "MedTracker push contract/1",
        &subscription(&endpoint, first_key, first_auth),
    );
    assert_eq!(created.status().as_u16(), 201);
    let first_request = request_id(&created);
    assert!(created.text().expect("empty create body").is_empty());
    let row = database()
        .query_one(
            "SELECT account_id, p256dh, auth, user_agent FROM push_subscriptions WHERE endpoint = $1",
            &[&endpoint],
        )
        .expect("created subscription");
    assert_eq!(row.get::<_, i64>(0), fixture.account_id);
    assert_eq!(row.get::<_, String>(1), first_key);
    assert_eq!(row.get::<_, String>(2), first_auth);
    assert_eq!(
        row.get::<_, Option<String>>(3).as_deref(),
        Some("MedTracker push contract/1")
    );
    assert_secrets_absent_from_audit(&first_request, &[&endpoint, first_key, first_auth]);

    let replay_key = "contract-updated-public-key";
    let replay_auth = "contract-updated-auth-secret";
    let replay = target.post_json_with_header(
        &path,
        &fixture.access_token,
        "User-Agent",
        "MedTracker push contract/2",
        &subscription(&endpoint, replay_key, replay_auth),
    );
    assert_eq!(replay.status().as_u16(), 201);
    let replay_request = request_id(&replay);
    assert!(replay.text().expect("empty replay body").is_empty());
    let row = database()
        .query_one(
            "SELECT count(*), max(p256dh), max(auth), max(user_agent) FROM push_subscriptions WHERE account_id = $1 AND endpoint = $2",
            &[&fixture.account_id, &endpoint],
        )
        .expect("updated subscription");
    assert_eq!(row.get::<_, i64>(0), 1);
    assert_eq!(row.get::<_, Option<String>>(1).as_deref(), Some(replay_key));
    assert_eq!(
        row.get::<_, Option<String>>(2).as_deref(),
        Some(replay_auth)
    );
    assert_eq!(
        row.get::<_, Option<String>>(3).as_deref(),
        Some("MedTracker push contract/2")
    );
    assert_secrets_absent_from_audit(&replay_request, &[&endpoint, replay_key, replay_auth]);

    let query = delete_query(&path, &endpoint);
    let removed = target.delete(&query, Some(&fixture.access_token));
    assert_eq!(removed.status().as_u16(), 204);
    let delete_request = request_id(&removed);
    assert!(removed.text().expect("empty delete body").is_empty());
    assert_secrets_absent_from_audit(&delete_request, &[&endpoint]);
    assert_eq!(row_count(fixture.account_id, &endpoint), 0);
    let repeated = target.delete(&query, Some(&fixture.access_token));
    assert_eq!(repeated.status().as_u16(), 204);
    assert!(repeated
        .text()
        .expect("empty repeated delete body")
        .is_empty());
}

#[test]
fn push_subscription_requires_valid_wrapper_keys_and_supported_https_providers() {
    let fixture = fixture();
    let target = Target::from_env();
    let path = path(fixture.household_id);
    let endpoint = unique_endpoint();
    let invalid = [
        ("http://fcm.googleapis.com/token", "http scheme"),
        (
            "https://fcm.googleapis.com.evil.test/token",
            "lookalike host",
        ),
        ("https://evilpush.apple.com/token", "Apple suffix boundary"),
        (
            "https://tenant.notify.windows.com.evil.test/token",
            "Windows suffix boundary",
        ),
        ("https://user:pass@fcm.googleapis.com/token", "userinfo"),
        ("https://127.0.0.1/push", "IPv4 address"),
        ("https://[::1]/push", "IPv6 address"),
        ("https://example.com/push", "unsupported host"),
    ];
    for (invalid_endpoint, _) in invalid {
        let error = assert_error(
            target.post_json_authorized(
                &path,
                &fixture.access_token,
                &subscription(invalid_endpoint, "valid-p256dh", "valid-auth"),
            ),
            422,
            "validation_failed",
        );
        assert!(!error.to_string().contains(invalid_endpoint));
        assert_eq!(row_count(fixture.account_id, invalid_endpoint), 0);
    }

    for attributes in [
        json!({"endpoint": endpoint, "keys": {"p256dh": "", "auth": "valid-auth"}}),
        json!({"endpoint": endpoint, "keys": {"p256dh": "valid-key", "auth": " "}}),
        json!({"endpoint": endpoint, "keys": {"p256dh": "valid-key"}}),
        json!({"endpoint": endpoint, "keys": []}),
        json!({"endpoint": endpoint, "keys": {"p256dh": 42, "auth": "valid-auth"}}),
        json!({"endpoint": endpoint, "keys": {"p256dh": "valid-key", "auth": "valid-auth", "extra": true}}),
        json!({"endpoint": endpoint, "keys": {"p256dh": "valid-key", "auth": "valid-auth"}, "extra": true}),
    ] {
        assert_error(
            target.post_json_authorized(
                &path,
                &fixture.access_token,
                &json!({"push_subscription": attributes}),
            ),
            422,
            "validation_failed",
        );
        assert_eq!(row_count(fixture.account_id, &endpoint), 0);
    }
    let wrapper = assert_error(
        target.post_json_authorized(
            &path,
            &fixture.access_token,
            &json!({"wrong_wrapper": {"endpoint": endpoint, "keys": {
                "p256dh": "private-public-key", "auth": "private-auth-key"
            }}}),
        ),
        400,
        "bad_request",
    );
    assert!(!wrapper.to_string().contains(&endpoint));
    assert!(!wrapper.to_string().contains("private-public-key"));
    assert!(!wrapper.to_string().contains("private-auth-key"));

    let auth_endpoint = unique_endpoint();
    let auth_request = subscription(&auth_endpoint, "auth-public-key", "auth-secret");
    let unauthenticated = assert_error(target.post_json(&path, &auth_request), 401, "unauthorized");
    assert!(!unauthenticated.to_string().contains(&auth_endpoint));
    let foreign = assert_error(
        target.post_json_authorized(&path, &fixture.foreign_access_token, &auth_request),
        403,
        "forbidden",
    );
    assert!(!foreign.to_string().contains(&auth_endpoint));
    assert_eq!(row_count(fixture.account_id, &auth_endpoint), 0);

    let valid_providers = [
        "https://fcm.googleapis.com/fcm/send/contract-valid",
        "https://updates.push.services.mozilla.com/wpush/v2/contract-valid",
        "https://web.push.apple.com/webpush/contract-valid",
        "https://tenant.notify.windows.com/wpush/contract-valid",
        "https://tenant.push.apple.com/webpush/contract-valid",
    ];
    for (index, valid_endpoint) in valid_providers.iter().enumerate() {
        let response = target.post_json_authorized(
            &path,
            &fixture.access_token,
            &subscription(
                valid_endpoint,
                &format!("provider-p256dh-{index}"),
                &format!("provider-auth-{index}"),
            ),
        );
        assert_eq!(response.status().as_u16(), 201);
        assert!(response.text().expect("empty provider response").is_empty());
        assert_eq!(row_count(fixture.account_id, valid_endpoint), 1);
        let removed = target.delete(
            &delete_query(&path, valid_endpoint),
            Some(&fixture.access_token),
        );
        assert_eq!(removed.status().as_u16(), 204);
    }
}

#[test]
fn push_subscription_revocation_is_account_scoped_and_requires_endpoint() {
    let fixture = fixture();
    let target = Target::from_env();
    let owner_path = path(fixture.household_id);
    let foreign_path = path(fixture.foreign_household_id);
    let endpoint = unique_endpoint();
    let request = subscription(&endpoint, "scoped-public-key", "scoped-auth-key");
    let created = target.post_json_authorized(&owner_path, &fixture.access_token, &request);
    assert_eq!(created.status().as_u16(), 201);
    assert!(created.text().expect("empty create body").is_empty());

    assert_error(
        target.delete(&owner_path, Some(&fixture.access_token)),
        400,
        "bad_request",
    );
    assert_error(
        target.delete(
            &format!("{owner_path}?endpoint="),
            Some(&fixture.access_token),
        ),
        400,
        "bad_request",
    );
    assert_error(
        target.delete(
            &duplicate_delete_query(&owner_path, &endpoint, "https://fcm.googleapis.com/other"),
            Some(&fixture.access_token),
        ),
        400,
        "bad_request",
    );
    let owner_query = delete_query(&owner_path, &endpoint);
    assert_error(target.delete(&owner_query, None), 401, "unauthorized");
    assert_error(
        target.delete(&owner_query, Some(&fixture.foreign_access_token)),
        403,
        "forbidden",
    );
    let foreign_query = delete_query(&foreign_path, &endpoint);
    let foreign_delete = target.delete(&foreign_query, Some(&fixture.foreign_access_token));
    assert_eq!(foreign_delete.status().as_u16(), 204);
    assert!(foreign_delete
        .text()
        .expect("empty foreign delete body")
        .is_empty());
    assert_eq!(row_count(fixture.account_id, &endpoint), 1);

    let owner_delete = target.delete(&owner_query, Some(&fixture.access_token));
    assert_eq!(owner_delete.status().as_u16(), 204);
    assert!(owner_delete
        .text()
        .expect("empty owner delete body")
        .is_empty());
    assert_eq!(row_count(fixture.account_id, &endpoint), 0);

    let unsupported_endpoint = format!(
        "https://unsupported.example.test/push/{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock")
            .as_nanos()
    );
    database()
        .execute(
            "INSERT INTO push_subscriptions (account_id, endpoint, p256dh, auth, created_at, updated_at) VALUES ($1, $2, $3, $4, now(), now())",
            &[&fixture.account_id, &unsupported_endpoint, &"seed-public-key", &"seed-auth-key"],
        )
        .expect("seed unsupported endpoint");
    let unsupported_delete = target.delete(
        &delete_query(&owner_path, &unsupported_endpoint),
        Some(&fixture.access_token),
    );
    assert_eq!(unsupported_delete.status().as_u16(), 204);
    assert!(unsupported_delete
        .text()
        .expect("empty unsupported endpoint delete body")
        .is_empty());
    assert_eq!(row_count(fixture.account_id, &unsupported_endpoint), 0);
}

#[test]
fn push_subscription_endpoints_are_globally_unique_and_concurrent_claim_has_one_winner() {
    let fixture = fixture();
    let target = Target::from_env();
    let owner_path = path(fixture.household_id);
    let foreign_path = path(fixture.foreign_household_id);
    let foreign_account = account_for_membership(fixture.foreign_membership_id);
    let endpoint = unique_endpoint();
    let owner_request = subscription(&endpoint, "owner-public-key", "owner-auth-key");
    let foreign_request = subscription(&endpoint, "foreign-public-key", "foreign-auth-key");
    let owner_create =
        target.post_json_authorized(&owner_path, &fixture.access_token, &owner_request);
    assert_eq!(owner_create.status().as_u16(), 201);
    assert!(owner_create
        .text()
        .expect("empty owner create body")
        .is_empty());
    let conflict = assert_error(
        target.post_json_authorized(
            &foreign_path,
            &fixture.foreign_access_token,
            &foreign_request,
        ),
        422,
        "validation_failed",
    );
    assert!(!conflict.to_string().contains(&endpoint));
    assert!(!conflict.to_string().contains("foreign-public-key"));
    assert_eq!(row_count(foreign_account, &endpoint), 0);
    let removed = target.delete(
        &delete_query(&owner_path, &endpoint),
        Some(&fixture.access_token),
    );
    assert_eq!(removed.status().as_u16(), 204);

    let racing_endpoint = unique_endpoint();
    let barrier = Arc::new(Barrier::new(2));
    let base = env::var("CONTRACT_BASE_URL").expect("contract API URL");
    let owner_base = base.clone();
    let owner_token = fixture.access_token.clone();
    let foreign_token = fixture.foreign_access_token.clone();
    let owner_body = subscription(&racing_endpoint, "race-owner-key", "race-owner-auth");
    let foreign_body = subscription(&racing_endpoint, "race-foreign-key", "race-foreign-auth");
    let owner_barrier = barrier.clone();
    let foreign_barrier = barrier.clone();
    let owner_household = fixture.household_id;
    let foreign_household = fixture.foreign_household_id;
    let results = thread::scope(|scope| {
        let owner = scope.spawn(move || {
            let client = reqwest::blocking::Client::new();
            owner_barrier.wait();
            client
                .post(format!(
                    "{owner_base}/api/v1/households/{owner_household}/push_subscription"
                ))
                .bearer_auth(owner_token)
                .json(&owner_body)
                .send()
                .expect("owner registration")
        });
        let foreign = scope.spawn(move || {
            let client = reqwest::blocking::Client::new();
            foreign_barrier.wait();
            client
                .post(format!(
                    "{base}/api/v1/households/{foreign_household}/push_subscription"
                ))
                .bearer_auth(foreign_token)
                .json(&foreign_body)
                .send()
                .expect("foreign registration")
        });
        let owner = owner.join().expect("owner thread");
        let foreign = foreign.join().expect("foreign thread");
        [owner, foreign]
    });
    let statuses: Vec<u16> = results
        .iter()
        .map(|response| response.status().as_u16())
        .collect();
    assert_eq!(statuses.iter().filter(|status| **status == 201).count(), 1);
    assert_eq!(statuses.iter().filter(|status| **status == 422).count(), 1);
    for response in results {
        if response.status().as_u16() == 422 {
            let error = assert_error(response, 422, "validation_failed");
            assert!(!error.to_string().contains(&racing_endpoint));
            assert!(!error.to_string().contains("race-owner-key"));
            assert!(!error.to_string().contains("race-foreign-key"));
            assert!(!error.to_string().contains("race-owner-auth"));
            assert!(!error.to_string().contains("race-foreign-auth"));
        }
    }
    let winner_account = if statuses[0] == 201 {
        fixture.account_id
    } else {
        foreign_account
    };
    assert_eq!(
        row_count(fixture.account_id, &racing_endpoint)
            + row_count(foreign_account, &racing_endpoint),
        1
    );
    assert_eq!(row_count(winner_account, &racing_endpoint), 1);
    let (winner_path, winner_token) = if winner_account == fixture.account_id {
        (owner_path, fixture.access_token)
    } else {
        (foreign_path, fixture.foreign_access_token)
    };
    let deleted = target.delete(
        &delete_query(&winner_path, &racing_endpoint),
        Some(&winner_token),
    );
    assert_eq!(deleted.status().as_u16(), 204);
    assert_eq!(row_count(winner_account, &racing_endpoint), 0);
}

#[test]
fn push_subscription_create_and_delete_share_rate_limits() {
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
            assert_eq!(
                response.json::<Value>().expect("rate limit JSON")["error"]["code"],
                "rate_limited"
            );
            limited = true;
            break;
        }
        assert_eq!(response.status().as_u16(), 200);
    }
    assert!(limited && started.elapsed() < std::time::Duration::from_secs(60));
    let subscription_path = format!(
        "{base}/api/v1/households/{}/push_subscription",
        fixture.household_id
    );
    let endpoint = unique_endpoint();
    let body = subscription(&endpoint, "rate-public-key", "rate-auth-key");
    let create = client
        .post(&subscription_path)
        .bearer_auth(&fixture.access_token)
        .json(&body)
        .send()
        .expect("rate limited create");
    assert_eq!(create.status().as_u16(), 429);
    let delete = client
        .delete(delete_query(&subscription_path, &endpoint))
        .bearer_auth(&fixture.access_token)
        .send()
        .expect("rate limited delete");
    assert_eq!(delete.status().as_u16(), 429);
}
