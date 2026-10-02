use medtracker_contract_tests::{Fixture, Target, fixture};
use postgres::{Client as PgClient, NoTls};
use reqwest::Method;
use reqwest::blocking::{Client, Response, multipart};
use serde_json::{Value, json};
use std::env;
use std::time::{SystemTime, UNIX_EPOCH};

fn profile_path(fixture: &Fixture) -> String {
    format!(
        "/api/v1/households/{}/profile",
        fixture.profile_household_id
    )
}

fn avatar_path(fixture: &Fixture) -> String {
    format!(
        "/api/v1/households/{}/profile/avatar",
        fixture.avatar_invalid_household_id
    )
}

fn body(response: Response) -> Value {
    response.json().expect("JSON response")
}

fn assert_no_store(response: &Response) {
    assert!(
        response.headers()["cache-control"]
            .to_str()
            .expect("cache control")
            .contains("no-store")
    );
}

fn profile_from_response(response: Response) -> Value {
    assert_eq!(response.status().as_u16(), 200);
    assert_no_store(&response);
    let payload = body(response);
    assert_eq!(payload.as_object().expect("profile envelope").len(), 1);
    let profile = &payload["data"];
    let object = profile.as_object().expect("profile resource");
    assert_eq!(object.len(), 7);
    for field in [
        "person_id",
        "account_id",
        "date_of_birth",
        "time_zone",
        "gravatar_enabled",
        "mobile_shortcuts",
        "avatar_attached",
    ] {
        assert!(object.contains_key(field), "missing profile field {field}");
    }
    assert!(profile["person_id"].as_str().is_some_and(|v| !v.is_empty()));
    assert!(
        profile["account_id"]
            .as_str()
            .is_some_and(|v| !v.is_empty())
    );
    assert!(profile["date_of_birth"].is_string());
    assert!(profile["time_zone"].is_string());
    assert!(profile["gravatar_enabled"].is_boolean());
    let shortcuts = profile["mobile_shortcuts"]
        .as_array()
        .expect("mobile shortcuts");
    assert!((1..=3).contains(&shortcuts.len()));
    assert!(shortcuts.iter().all(Value::is_string));
    assert!(profile["avatar_attached"].is_boolean());
    profile.clone()
}

fn assert_error(response: Response, status: u16, code: &str) -> Value {
    assert_eq!(response.status().as_u16(), status);
    let request_id = response.headers()["x-request-id"]
        .to_str()
        .expect("request ID")
        .to_owned();
    let payload = body(response);
    assert_eq!(payload["error"]["code"], code);
    assert_eq!(payload["error"]["request_id"], request_id);
    assert!(
        payload["error"]["message"]
            .as_str()
            .is_some_and(|v| !v.is_empty())
    );
    assert!(payload.get("data").is_none());
    payload
}

fn request_id(response: &Response) -> String {
    response.headers()["x-request-id"]
        .to_str()
        .expect("request ID")
        .to_owned()
}

fn avatar_form(bytes: Vec<u8>, filename: &str, mime: &str) -> multipart::Form {
    let part = multipart::Part::bytes(bytes)
        .file_name(filename.to_owned())
        .mime_str(mime)
        .expect("valid MIME type");
    multipart::Form::new().part("avatar", part)
}

fn assert_avatar_download(target: &Target, path: &str, token: &str, bytes: &[u8], filename: &str) {
    let response = target.get(path, Some(token));
    assert_eq!(response.status().as_u16(), 200);
    assert_no_store(&response);
    assert_eq!(response.headers()["content-type"], "image/png");
    assert!(
        response.headers()["content-disposition"]
            .to_str()
            .expect("content disposition")
            .contains(filename)
    );
    assert_eq!(response.bytes().expect("avatar bytes").as_ref(), bytes);
}

struct AccountPreferencesSnapshot {
    db: PgClient,
    account_id: i64,
    preferences: String,
}

impl AccountPreferencesSnapshot {
    fn with_marker(account_id: i64) -> Self {
        let mut db = PgClient::connect(
            &env::var("CONTRACT_AUDIT_DATABASE_URL").expect("contract database URL"),
            NoTls,
        )
        .expect("contract database");
        let preferences: String = db
            .query_one(
                "SELECT preferences::text FROM accounts WHERE id = $1",
                &[&account_id],
            )
            .expect("account preferences")
            .get(0);
        db.execute(
            "UPDATE accounts SET preferences = COALESCE(preferences, '{}'::jsonb) || jsonb_build_object('contract_profile_preservation', 'preserve-me') WHERE id = $1",
            &[&account_id],
        )
        .expect("set unrelated profile preference");
        Self {
            db,
            account_id,
            preferences,
        }
    }

    fn marker(&mut self) -> Option<String> {
        self.db
            .query_one(
                "SELECT preferences ->> 'contract_profile_preservation' FROM accounts WHERE id = $1",
                &[&self.account_id],
            )
            .expect("read unrelated profile preference")
            .get(0)
    }
}

impl Drop for AccountPreferencesSnapshot {
    fn drop(&mut self) {
        self.db
            .execute(
                "UPDATE accounts SET preferences = $2::text::jsonb WHERE id = $1",
                &[&self.account_id, &self.preferences],
            )
            .expect("restore account preferences");
    }
}

fn profile_database() -> PgClient {
    PgClient::connect(
        &env::var("CONTRACT_AUDIT_DATABASE_URL").expect("contract database URL"),
        NoTls,
    )
    .expect("contract database")
}

fn keyed_profile_request(
    method: Method,
    path: &str,
    token: &str,
    key: &str,
    payload: &Value,
) -> Response {
    let _validated_target = Target::from_env();
    let base = env::var("CONTRACT_BASE_URL").expect("contract API base URL");
    let origin = url::Url::parse(&base).expect("contract API origin");
    let url = origin.join(path).expect("profile path");
    assert_eq!(url.origin(), origin.origin());
    Client::builder()
        .no_proxy()
        .build()
        .expect("keyed profile HTTP client")
        .request(method, url)
        .header("Accept", "application/json")
        .bearer_auth(token)
        .header("Idempotency-Key", key)
        .json(payload)
        .send()
        .expect("keyed profile request")
}

struct AvatarCleanup {
    target: Target,
    path: String,
    token: String,
}

impl Drop for AvatarCleanup {
    fn drop(&mut self) {
        let _ = self.target.delete(&self.path, Some(&self.token));
    }
}

#[test]
fn profile_contract_has_exact_shape_partial_updates_and_atomic_validation() {
    let target = Target::from_env();
    let fixture = fixture();
    let path = profile_path(&fixture);
    let initial = profile_from_response(target.get(&path, Some(&fixture.profile_access_token)));

    let empty = target.patch_json(
        &path,
        &fixture.profile_access_token,
        &json!({"profile": {}}),
    );
    assert_eq!(profile_from_response(empty), initial);

    let other_zone = if initial["time_zone"] == "UTC" {
        "Europe/London"
    } else {
        "UTC"
    };
    let patch = target.patch_json(
        &path,
        &fixture.profile_access_token,
        &json!({"profile": {"time_zone": other_zone}}),
    );
    let patched = profile_from_response(patch);
    assert_eq!(patched["time_zone"], other_zone);
    for field in [
        "person_id",
        "account_id",
        "date_of_birth",
        "gravatar_enabled",
        "mobile_shortcuts",
        "avatar_attached",
    ] {
        assert_eq!(
            patched[field], initial[field],
            "PATCH changed omitted {field}"
        );
    }

    let put = target.put_json(
        &path,
        &fixture.profile_access_token,
        &json!({"profile": {"gravatar_enabled": !initial["gravatar_enabled"].as_bool().unwrap()}}),
    );
    let put = profile_from_response(put);
    assert_eq!(
        put["gravatar_enabled"],
        !initial["gravatar_enabled"].as_bool().unwrap()
    );
    for field in [
        "person_id",
        "account_id",
        "date_of_birth",
        "time_zone",
        "mobile_shortcuts",
        "avatar_attached",
    ] {
        assert_eq!(put[field], patched[field], "PUT changed omitted {field}");
    }

    let restore = target.patch_json(
        &path,
        &fixture.profile_access_token,
        &json!({"profile": {
            "time_zone": initial["time_zone"],
            "gravatar_enabled": initial["gravatar_enabled"]
        }}),
    );
    assert_eq!(profile_from_response(restore), initial);

    let invalid = [
        (json!({}), 400, "bad_request"),
        (
            json!({"profile": {"time_zone": initial["time_zone"]}, "extra": true}),
            422,
            "unprocessable_content",
        ),
        (
            json!({"profile": {"time_zone": initial["time_zone"], "extra": true}}),
            422,
            "unprocessable_content",
        ),
        (
            json!({"profile": {"date_of_birth": null}}),
            422,
            "unprocessable_content",
        ),
        (
            json!({"profile": {"gravatar_enabled": null}}),
            422,
            "unprocessable_content",
        ),
        (
            json!({"profile": {"mobile_shortcuts": ["not-a-shortcut"]}}),
            422,
            "validation_failed",
        ),
    ];
    for (payload, status, code) in invalid {
        assert_error(
            target.patch_json(&path, &fixture.profile_access_token, &payload),
            status,
            code,
        );
        assert_eq!(
            profile_from_response(target.get(&path, Some(&fixture.profile_access_token))),
            initial,
            "invalid profile request changed stored profile"
        );
    }
}

#[test]
fn date_of_birth_update_changes_person_sync_version_and_updated_at() {
    let target = Target::from_env();
    let fixture = fixture();
    let path = profile_path(&fixture);
    let initial = profile_from_response(target.get(&path, Some(&fixture.profile_access_token)));
    let mut db = profile_database();
    let previous_updated_at: String = db
        .query_one(
            "SELECT EXTRACT(EPOCH FROM updated_at)::text FROM people WHERE id = $1",
            &[&fixture.profile_person_id],
        )
        .expect("person timestamp before update")
        .get(0);
    let previous_sync_count: i64 = db
        .query_one(
            "SELECT count(*) FROM api_change_events WHERE record_type = 'Person' AND record_id = $1",
            &[&fixture.profile_person_id],
        )
        .expect("person sync count before update")
        .get(0);
    let date_of_birth = if initial["date_of_birth"] == "1984-07-16" {
        "1984-07-17"
    } else {
        "1984-07-16"
    };
    let response = target.patch_json(
        &path,
        &fixture.profile_access_token,
        &json!({"profile": {"date_of_birth": date_of_birth}}),
    );
    let request_id = request_id(&response);
    let updated = profile_from_response(response);
    assert_eq!(updated["date_of_birth"], date_of_birth);
    assert_eq!(
        profile_from_response(target.get(&path, Some(&fixture.profile_access_token))),
        updated
    );

    let updated_at: String = db
        .query_one(
            "SELECT EXTRACT(EPOCH FROM updated_at)::text FROM people WHERE id = $1",
            &[&fixture.profile_person_id],
        )
        .expect("person timestamp after update")
        .get(0);
    assert!(
        updated_at.parse::<f64>().unwrap() > previous_updated_at.parse::<f64>().unwrap(),
        "date-of-birth update must touch the Person"
    );
    let sync_count: i64 = db
        .query_one(
            "SELECT count(*) FROM api_change_events WHERE record_type = 'Person' AND record_id = $1",
            &[&fixture.profile_person_id],
        )
        .expect("person sync count after update")
        .get(0);
    assert_eq!(sync_count, previous_sync_count + 1);
    assert!(db
        .query_one(
            "SELECT EXISTS (SELECT 1 FROM api_change_events WHERE request_id = $1 AND record_type = 'Person' AND record_id = $2 AND action = 'update')",
            &[&request_id, &fixture.profile_person_id],
        )
        .expect("request-correlated Person sync event")
        .get::<_, bool>(0));
    assert!(db
        .query_one(
            "SELECT EXISTS (SELECT 1 FROM versions WHERE request_id = $1 AND item_type = 'Person' AND item_id = $2 AND event = 'update')",
            &[&request_id, &fixture.profile_person_id],
        )
        .expect("request-correlated Person audit version")
        .get::<_, bool>(0));
    let restored = target.patch_json(
        &path,
        &fixture.profile_access_token,
        &json!({"profile": {"date_of_birth": initial["date_of_birth"]}}),
    );
    assert_eq!(profile_from_response(restored), initial);
}

#[test]
fn partial_profile_update_preserves_unrelated_account_preferences() {
    let target = Target::from_env();
    let fixture = fixture();
    let path = profile_path(&fixture);
    let mut preferences = AccountPreferencesSnapshot::with_marker(fixture.profile_account_id);
    let initial = profile_from_response(target.get(&path, Some(&fixture.profile_access_token)));
    let time_zone = if initial["time_zone"] == "UTC" {
        "Europe/London"
    } else {
        "UTC"
    };
    let response = target.patch_json(
        &path,
        &fixture.profile_access_token,
        &json!({"profile": {"time_zone": time_zone}}),
    );
    let updated = profile_from_response(response);
    assert_eq!(updated["time_zone"], time_zone);
    assert_eq!(updated["date_of_birth"], initial["date_of_birth"]);
    assert_eq!(updated["gravatar_enabled"], initial["gravatar_enabled"]);
    assert_eq!(updated["mobile_shortcuts"], initial["mobile_shortcuts"]);
    assert_eq!(preferences.marker().as_deref(), Some("preserve-me"));
}

#[test]
fn keyed_profile_patch_and_put_replay_once_and_reject_changed_payloads() {
    let target = Target::from_env();
    let fixture = fixture();
    let path = profile_path(&fixture);
    let initial = profile_from_response(target.get(&path, Some(&fixture.profile_access_token)));
    let key_seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock")
        .as_nanos();
    let mut db = profile_database();
    let person_id = fixture.profile_person_id;

    let patch_payload = json!({"profile": {"time_zone": if initial["time_zone"] == "UTC" {
        "Europe/London"
    } else {
        "UTC"
    }}});
    let patch_key = format!("profile-patch-{person_id}-{key_seed}");
    let patch_event_count: i64 = db
        .query_one(
            "SELECT count(*) FROM api_change_events WHERE record_type = 'Person' AND record_id = $1",
            &[&person_id],
        )
        .expect("Person sync count before PATCH")
        .get(0);
    let patch_version_count: i64 = db
        .query_one(
            "SELECT count(*) FROM versions WHERE item_type = 'Person' AND item_id = $1",
            &[&person_id],
        )
        .expect("Person version count before PATCH")
        .get(0);
    let patch_first = keyed_profile_request(
        Method::PATCH,
        &path,
        &fixture.profile_access_token,
        &patch_key,
        &patch_payload,
    );
    assert_eq!(patch_first.status().as_u16(), 200);
    assert!(patch_first.headers().get("idempotency-replayed").is_none());
    let patch_first: Value = body(patch_first);
    let patch_replay = keyed_profile_request(
        Method::PATCH,
        &path,
        &fixture.profile_access_token,
        &patch_key,
        &patch_payload,
    );
    assert_eq!(patch_replay.status().as_u16(), 200);
    assert_eq!(patch_replay.headers()["idempotency-replayed"], "true");
    assert_eq!(body(patch_replay), patch_first);
    let mut changed_patch = patch_payload.clone();
    changed_patch["profile"]["time_zone"] = json!(initial["time_zone"]);
    let changed = assert_error(
        keyed_profile_request(
            Method::PATCH,
            &path,
            &fixture.profile_access_token,
            &patch_key,
            &changed_patch,
        ),
        409,
        "idempotency_key_reused",
    );
    assert_eq!(changed["error"]["code"], "idempotency_key_reused");
    let patch_event_count_after: i64 = db
        .query_one(
            "SELECT count(*) FROM api_change_events WHERE record_type = 'Person' AND record_id = $1",
            &[&person_id],
        )
        .expect("Person sync count after PATCH replay")
        .get(0);
    let patch_version_count_after: i64 = db
        .query_one(
            "SELECT count(*) FROM versions WHERE item_type = 'Person' AND item_id = $1",
            &[&person_id],
        )
        .expect("Person version count after PATCH replay")
        .get(0);
    assert_eq!(patch_event_count_after, patch_event_count);
    assert_eq!(patch_version_count_after, patch_version_count);

    let put_payload =
        json!({"profile": {"gravatar_enabled": !initial["gravatar_enabled"].as_bool().unwrap()}});
    let put_key = format!("profile-put-{person_id}-{key_seed}");
    let put_first = keyed_profile_request(
        Method::PUT,
        &path,
        &fixture.profile_access_token,
        &put_key,
        &put_payload,
    );
    assert_eq!(put_first.status().as_u16(), 200);
    let put_first: Value = body(put_first);
    let put_replay = keyed_profile_request(
        Method::PUT,
        &path,
        &fixture.profile_access_token,
        &put_key,
        &put_payload,
    );
    assert_eq!(put_replay.status().as_u16(), 200);
    assert_eq!(put_replay.headers()["idempotency-replayed"], "true");
    assert_eq!(body(put_replay), put_first);
    let restored = target.patch_json(
        &path,
        &fixture.profile_access_token,
        &json!({"profile": {
            "time_zone": initial["time_zone"],
            "gravatar_enabled": initial["gravatar_enabled"],
            "mobile_shortcuts": initial["mobile_shortcuts"]
        }}),
    );
    assert_eq!(profile_from_response(restored), initial);
}

#[test]
fn avatar_required_part_and_delete_preserve_private_metadata_and_attachment_state() {
    let target = Target::from_env();
    let fixture = fixture();
    let path = avatar_path(&fixture);
    let token = &fixture.avatar_invalid_access_token;
    let bytes = b"profile avatar bytes\0kept private".to_vec();
    let upload = target.put_multipart(
        &path,
        token,
        avatar_form(bytes.clone(), "profile-check.png", "image/png"),
    );
    let uploaded = profile_from_response(upload);
    assert_eq!(uploaded["avatar_attached"], true);
    assert_avatar_download(&target, &path, token, &bytes, "profile-check.png");

    assert_error(
        target.put_multipart(&path, token, multipart::Form::new()),
        400,
        "bad_request",
    );
    assert_avatar_download(&target, &path, token, &bytes, "profile-check.png");

    let deleted = target.delete(&path, Some(token));
    assert_eq!(deleted.status().as_u16(), 204);
    assert_no_store(&deleted);
    let deleted_request_id = request_id(&deleted);
    assert!(deleted.bytes().expect("empty delete response").is_empty());
    assert_eq!(
        profile_from_response(target.get(
            &format!(
                "/api/v1/households/{}/profile",
                fixture.avatar_invalid_household_id
            ),
            Some(token),
        ))["avatar_attached"],
        false
    );
    assert_error(target.get(&path, Some(token)), 404, "not_found");

    let audit_path = format!(
        "/api/v1/households/{}/admin/audit_logs",
        fixture.avatar_invalid_household_id
    );
    let audit = body(target.get(&audit_path, Some(token)));
    let removal = audit["data"]
        .as_array()
        .expect("audit events")
        .iter()
        .find(|row| {
            row["request_id"] == deleted_request_id && row["event_type"] == "profile.avatar.removed"
        })
        .expect("request-correlated avatar removal audit");
    assert_eq!(
        removal["metadata"]["person_id"].as_i64().unwrap().to_string(),
        uploaded["person_id"].as_str().unwrap()
    );
}

#[test]
fn avatar_storage_failure_preserves_existing_attachment_and_profile_state() {
    let target = Target::from_env();
    let fixture = fixture();
    let path = avatar_path(&fixture);
    let token = &fixture.avatar_invalid_access_token;
    let baseline_bytes = b"baseline avatar remains readable".to_vec();
    let upload = target.put_multipart(
        &path,
        token,
        avatar_form(baseline_bytes.clone(), "baseline.png", "image/png"),
    );
    assert_eq!(profile_from_response(upload)["avatar_attached"], true);
    let cleanup = AvatarCleanup {
        target: Target::from_env(),
        path: path.clone(),
        token: token.clone(),
    };
    assert_avatar_download(&target, &path, token, &baseline_bytes, "baseline.png");

    let profile_path = format!(
        "/api/v1/households/{}/profile",
        fixture.avatar_invalid_household_id
    );
    let profile_before = profile_from_response(target.get(&profile_path, Some(token)));
    let person_id: i64 = profile_before["person_id"]
        .as_str()
        .expect("person ID")
        .parse()
        .expect("numeric person ID");
    let mut db = profile_database();
    let baseline_blob_id: i64 = db
        .query_one(
            "SELECT blob_id FROM active_storage_attachments WHERE record_type = 'Person' AND record_id = $1 AND name = 'avatar'",
            &[&person_id],
        )
        .expect("baseline avatar attachment")
        .get(0);
    let blob_count: i64 = db
        .query_one("SELECT count(*) FROM active_storage_blobs", &[])
        .expect("baseline blob count")
        .get(0);
    let failure_base =
        env::var("CONTRACT_AVATAR_FAILURE_BASE_URL").expect("storage failure API base URL");
    let client = Client::builder().no_proxy().build().expect("HTTP client");
    let failed_download = client
        .get(format!("{failure_base}{path}"))
        .bearer_auth(token)
        .send()
        .expect("storage failure download response");
    assert_no_store(&failed_download);
    assert_error(failed_download, 503, "avatar_unavailable");
    assert_avatar_download(&target, &path, token, &baseline_bytes, "baseline.png");
    let failed_upload = client
        .put(format!("{failure_base}{path}"))
        .bearer_auth(token)
        .multipart(avatar_form(
            b"replacement that cannot be stored".to_vec(),
            "replacement.png",
            "image/png",
        ))
        .send()
        .expect("storage failure API response");
    assert_error(failed_upload, 503, "avatar_unavailable");

    let profile_after = profile_from_response(target.get(&profile_path, Some(token)));
    assert_eq!(profile_after, profile_before);
    assert_avatar_download(&target, &path, token, &baseline_bytes, "baseline.png");
    let retained_blob_id: i64 = db
        .query_one(
            "SELECT blob_id FROM active_storage_attachments WHERE record_type = 'Person' AND record_id = $1 AND name = 'avatar'",
            &[&person_id],
        )
        .expect("retained avatar attachment")
        .get(0);
    assert_eq!(retained_blob_id, baseline_blob_id);
    let blob_count_after: i64 = db
        .query_one("SELECT count(*) FROM active_storage_blobs", &[])
        .expect("blob count after failure")
        .get(0);
    assert_eq!(blob_count_after, blob_count);
    drop(cleanup);
}
