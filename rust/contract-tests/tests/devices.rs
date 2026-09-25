use medtracker_contract_tests::{fixture, Fixture, Target};
use reqwest::blocking::Response;
use serde_json::{json, Value};
use url::form_urlencoded;

fn path(fixture: &Fixture, resource: &str) -> String {
    format!("/api/v1/households/{}/{resource}", fixture.household_id)
}

fn response_body(response: Response) -> Value {
    response.json().expect("JSON response")
}

fn request_id(response: &Response) -> String {
    response.headers()["x-request-id"]
        .to_str()
        .expect("request ID")
        .to_owned()
}

fn assert_error(response: Response, status: u16, code: &str) -> Value {
    assert_eq!(response.status().as_u16(), status);
    let id = request_id(&response);
    let body = response_body(response);
    assert_eq!(body["error"]["code"], code);
    assert_eq!(body["error"]["request_id"], id);
    assert!(body["error"]["message"].is_string());
    body
}

fn audit_for(target: &Target, fixture: &Fixture, id: &str) -> Value {
    let response = target.get(
        &path(fixture, "admin/audit_logs"),
        Some(&fixture.access_token),
    );
    assert_eq!(response.status().as_u16(), 200);
    response_body(response)["data"]
        .as_array()
        .expect("audit entries")
        .iter()
        .find(|row| row["request_id"] == id && row["event_type"] == "api.request")
        .cloned()
        .expect("request-correlated API audit")
}

#[test]
fn notification_preference_get_patch_and_put_follow_rails_partial_update_contract() {
    let target = Target::from_env();
    let fixture = fixture();
    let url = path(&fixture, "notification_preference");
    assert_error(target.get(&url, None), 401, "unauthorized");
    assert_error(
        target.get(&url, Some(&fixture.foreign_access_token)),
        403,
        "forbidden",
    );
    let missing = target.get(&url, Some(&fixture.access_token));
    assert_error(missing, 404, "not_found");

    let patch = target.patch_json(
        &url,
        &fixture.access_token,
        &json!({"notification_preference": {
            "enabled": true,
            "dose_due_enabled": true,
            "missed_dose_enabled": false,
            "low_stock_enabled": true,
            "private_text_enabled": false,
            "morning_time": "08:15",
            "afternoon_time": "13:30",
            "evening_time": "18:45",
            "night_time": "22:00"
        }}),
    );
    assert_eq!(patch.status().as_u16(), 200);
    let patch_id = request_id(&patch);
    let created = response_body(patch)["data"].clone();
    assert_eq!(created["person_id"], fixture.user_person_id);
    assert_eq!(created["enabled"], true);
    assert_eq!(created["dose_due_enabled"], true);
    assert_eq!(created["missed_dose_enabled"], false);
    assert_eq!(created["low_stock_enabled"], true);
    assert_eq!(created["private_text_enabled"], false);
    assert_eq!(created["morning_time"], "08:15:00");
    assert_eq!(created["afternoon_time"], "13:30:00");
    assert_eq!(created["evening_time"], "18:45:00");
    assert_eq!(created["night_time"], "22:00:00");

    let read = target.get(&url, Some(&fixture.access_token));
    assert_eq!(read.status().as_u16(), 200);
    assert_eq!(response_body(read)["data"], created);
    let put = target.put_json(
        &url,
        &fixture.access_token,
        &json!({"notification_preference": {"enabled": false, "night_time": "21:10"}}),
    );
    assert_eq!(put.status().as_u16(), 200);
    let replaced = response_body(put)["data"].clone();
    assert_eq!(replaced["enabled"], false);
    assert_eq!(replaced["night_time"], "21:10:00");
    assert_eq!(replaced["dose_due_enabled"], true);
    assert_eq!(replaced["morning_time"], "08:15:00");
    assert_eq!(replaced["id"], created["id"]);
    assert_eq!(
        response_body(target.get(&url, Some(&fixture.access_token)))["data"],
        replaced
    );
    let audit = audit_for(&target, &fixture, &patch_id);
    assert_eq!(audit["actor_account_id"], fixture.account_id);
    assert_eq!(audit["actor_membership_id"], fixture.owner_membership_id);
    assert_eq!(audit["metadata"]["http_method"], "PATCH");
    assert_eq!(audit["metadata"]["status"], 200);
}

#[test]
fn notification_preference_rejects_missing_wrapper_without_partial_persistence() {
    let target = Target::from_env();
    let fixture = fixture();
    let url = path(&fixture, "notification_preference");
    let initial = target.patch_json(
        &url,
        &fixture.access_token,
        &json!({"notification_preference": {"enabled": true, "morning_time": "07:00"}}),
    );
    assert_eq!(initial.status().as_u16(), 200);
    let before = response_body(initial)["data"].clone();
    let invalid = target.put_json(
        &url,
        &fixture.access_token,
        &json!({"wrong_wrapper": {"enabled": false, "morning_time": "09:00"}}),
    );
    assert_error(invalid, 400, "bad_request");
    let after = response_body(target.get(&url, Some(&fixture.access_token)));
    assert_eq!(after["data"]["enabled"], true);
    assert_eq!(after["data"]["morning_time"], before["morning_time"]);
}

#[test]
fn device_tokens_are_account_owned_idempotent_and_secret_free() {
    let target = Target::from_env();
    let fixture = fixture();
    let url = path(&fixture, "native_device_tokens");
    let token = format!("contract-device-{}", fixture.account_id);
    let request = json!({"native_device_token": {
        "device_token": token,
        "platform": "ios",
        "apns_environment": "sandbox"
    }});
    let first = target.post_json_authorized(&url, &fixture.access_token, &request);
    assert_eq!(first.status().as_u16(), 201);
    let first_id = request_id(&first);
    assert!(first.text().expect("empty created body").is_empty());
    let repeated = target.post_json_authorized(&url, &fixture.access_token, &request);
    assert_eq!(repeated.status().as_u16(), 201);
    assert!(repeated.text().expect("empty repeated body").is_empty());
    let audit = audit_for(&target, &fixture, &first_id);
    assert_eq!(audit["actor_account_id"], fixture.account_id);
    assert_eq!(audit["metadata"]["http_method"], "POST");
    assert!(!audit.to_string().contains(&token));

    let foreign_url = format!(
        "/api/v1/households/{}/native_device_tokens",
        fixture.foreign_household_id
    );
    let foreign_duplicate =
        target.post_json_authorized(&foreign_url, &fixture.foreign_access_token, &request);
    assert_error(foreign_duplicate, 422, "validation_failed");
    let delete = format!("{url}/{token}");
    assert_eq!(
        target
            .delete(&delete, Some(&fixture.foreign_access_token))
            .status()
            .as_u16(),
        403
    );
    assert_eq!(
        target
            .delete(&delete, Some(&fixture.access_token))
            .status()
            .as_u16(),
        204
    );
    assert_eq!(
        target
            .delete(&delete, Some(&fixture.access_token))
            .status()
            .as_u16(),
        204
    );
    assert!(!response_body(target.get(
        &path(&fixture, "mobile_snapshot"),
        Some(&fixture.access_token)
    ))
    .to_string()
    .contains(&token));
}

#[test]
fn device_token_validation_and_authority_fail_before_persistence() {
    let target = Target::from_env();
    let fixture = fixture();
    let url = path(&fixture, "native_device_tokens");
    let invalid = target.post_json_authorized(
        &url,
        &fixture.access_token,
        &json!({"native_device_token": {"device_token": "", "platform": "unknown"}}),
    );
    assert_error(invalid, 422, "validation_failed");
    let token = format!("contract-denied-device-{}", fixture.account_id);
    let request = json!({"native_device_token": {"device_token": token, "platform": "android"}});
    assert_error(target.post_json(&url, &request), 401, "unauthorized");
    assert_error(
        target.post_json_authorized(&url, &fixture.expired_access_token, &request),
        401,
        "unauthorized",
    );
    assert_error(
        target.post_json_authorized(&url, &fixture.foreign_access_token, &request),
        403,
        "forbidden",
    );
    assert_eq!(
        target
            .delete(&format!("{url}/{token}"), Some(&fixture.access_token))
            .status()
            .as_u16(),
        204
    );
}

#[test]
fn push_subscriptions_register_repeat_and_revoke_by_endpoint() {
    let target = Target::from_env();
    let fixture = fixture();
    let url = path(&fixture, "push_subscription");
    let endpoint = format!(
        "https://fcm.googleapis.com/fcm/send/contract-{}",
        fixture.account_id
    );
    let request = json!({"push_subscription": {"endpoint": endpoint, "keys": {
        "p256dh": "contract-p256dh-secret", "auth": "contract-auth-secret"
    }}});
    let created = target.post_json_authorized(&url, &fixture.access_token, &request);
    assert_eq!(created.status().as_u16(), 201);
    let id = request_id(&created);
    assert!(created.text().expect("empty created body").is_empty());
    assert_eq!(
        target
            .post_json_authorized(&url, &fixture.access_token, &request)
            .status()
            .as_u16(),
        201
    );
    let audit = audit_for(&target, &fixture, &id);
    assert!(!audit.to_string().contains("contract-p256dh-secret"));
    assert!(!audit.to_string().contains("contract-auth-secret"));
    let foreign_url = format!(
        "/api/v1/households/{}/push_subscription",
        fixture.foreign_household_id
    );
    assert_error(
        target.post_json_authorized(&foreign_url, &fixture.foreign_access_token, &request),
        422,
        "validation_failed",
    );
    let delete = format!(
        "{url}?{}",
        form_urlencoded::Serializer::new(String::new())
            .append_pair("endpoint", &endpoint)
            .finish()
    );
    assert_eq!(
        target
            .delete(&delete, Some(&fixture.foreign_access_token))
            .status()
            .as_u16(),
        403
    );
    assert_eq!(
        target
            .delete(&delete, Some(&fixture.access_token))
            .status()
            .as_u16(),
        204
    );
    assert_eq!(
        target
            .delete(&delete, Some(&fixture.access_token))
            .status()
            .as_u16(),
        204
    );
}

#[test]
fn push_subscription_validation_missing_endpoint_and_test_http_outcome() {
    let target = Target::from_env();
    let fixture = fixture();
    let url = path(&fixture, "push_subscription");
    let invalid = target.post_json_authorized(
        &url,
        &fixture.access_token,
        &json!({"push_subscription": {"endpoint": "http://push.example.test/invalid", "keys": {
            "p256dh": "", "auth": ""
        }}}),
    );
    assert_error(invalid, 422, "validation_failed");
    assert_error(
        target.delete(&url, Some(&fixture.access_token)),
        400,
        "bad_request",
    );
    let test_url = format!("{url}/test");
    assert_error(target.post_json(&test_url, &json!({})), 401, "unauthorized");
    assert_error(
        target.post_json_authorized(&test_url, &fixture.expired_access_token, &json!({})),
        401,
        "unauthorized",
    );
    assert_error(
        target.post_json_authorized(&test_url, &fixture.foreign_access_token, &json!({})),
        403,
        "forbidden",
    );
    let response = target.post_json_authorized(&test_url, &fixture.access_token, &json!({}));
    assert_eq!(response.status().as_u16(), 204);
    assert!(response.text().expect("empty test response").is_empty());
}
