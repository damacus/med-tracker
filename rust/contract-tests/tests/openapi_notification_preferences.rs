use medtracker_contract_tests::{fixture, Target};
use serde_json::{json, Value};
use std::env;

fn database() -> postgres::Client {
    postgres::Client::connect(
        &env::var("CONTRACT_AUDIT_DATABASE_URL").expect("contract database URL"),
        postgres::NoTls,
    )
    .expect("contract database")
}

struct OwnerViewGuard {
    db: postgres::Client,
    membership_id: i64,
    original_role: String,
    grant: Option<(i64, String, Option<String>)>,
    grant_id: i64,
    restored: bool,
}

impl OwnerViewGuard {
    fn new(fixture: &medtracker_contract_tests::Fixture) -> Self {
        let mut db = database();
        let original_role: String = db
            .query_one(
                "SELECT role FROM household_memberships WHERE id = $1 AND household_id = $2",
                &[&fixture.owner_membership_id, &fixture.household_id],
            )
            .expect("owner membership role")
            .get(0);
        assert_eq!(original_role, "owner");
        let grant: Option<(i64, String, Option<String>)> = db
            .query_opt(
                "SELECT id, access_level, revoked_at::text FROM person_access_grants WHERE household_id = $1 AND household_membership_id = $2 AND person_id = $3 ORDER BY id LIMIT 1",
                &[&fixture.household_id, &fixture.owner_membership_id, &fixture.user_person_id],
            )
            .expect("owner self grant")
            .map(|row| {
                (
                    row.get::<_, i64>(0),
                    row.get::<_, String>(1),
                    row.get::<_, Option<String>>(2),
                )
            });
        db.execute(
            "UPDATE household_memberships SET role = 'member' WHERE id = $1",
            &[&fixture.owner_membership_id],
        )
        .expect("temporarily restrict owner role");
        let grant_id = if let Some((id, _, _)) = &grant {
            db.execute(
                "UPDATE person_access_grants SET access_level = 'view', revoked_at = now() WHERE id = $1",
                &[id],
            )
            .expect("temporarily revoke own view grant");
            *id
        } else {
            db.query_one(
                "INSERT INTO person_access_grants (household_id, household_membership_id, person_id, granted_by_membership_id, access_level, relationship_type, revoked_at, created_at, updated_at) VALUES ($1, $2, $3, $2, 'view', 'self', now(), now(), now()) RETURNING id",
                &[&fixture.household_id, &fixture.owner_membership_id, &fixture.user_person_id],
            )
            .expect("temporary self view grant")
            .get(0)
        };
        Self {
            db,
            membership_id: fixture.owner_membership_id,
            original_role,
            grant,
            grant_id,
            restored: false,
        }
    }

    fn expose_view(&mut self) {
        self.db
            .execute(
                "UPDATE person_access_grants SET access_level = 'view', revoked_at = NULL WHERE id = $1",
                &[&self.grant_id],
            )
            .expect("expose self view grant");
    }

    fn restore(&mut self) {
        if self.restored {
            return;
        }
        if let Some((id, access_level, revoked_at)) = &self.grant {
            self.db
                .execute(
                    "UPDATE person_access_grants SET access_level = $2, revoked_at = $3::text::timestamptz WHERE id = $1",
                    &[id, access_level, revoked_at],
                )
                .expect("restore original self grant");
        } else {
            self.db
                .execute(
                    "DELETE FROM person_access_grants WHERE id = $1",
                    &[&self.grant_id],
                )
                .expect("remove temporary self grant");
        }
        self.db
            .execute(
                "UPDATE household_memberships SET role = $2 WHERE id = $1",
                &[&self.membership_id, &self.original_role],
            )
            .expect("restore original membership role");
        self.restored = true;
    }
}

impl Drop for OwnerViewGuard {
    fn drop(&mut self) {
        self.restore();
    }
}

fn request_id(response: &reqwest::blocking::Response) -> String {
    response.headers()["x-request-id"]
        .to_str()
        .expect("request ID")
        .to_owned()
}

fn assert_sync_event(request: &str, action: &str, preference: &Value) {
    let rows = database()
        .query(
            "SELECT metadata::text FROM api_change_events WHERE request_id = $1 AND record_type = 'NotificationPreference' AND action = $2 AND record_id = $3",
            &[&request, &action, &preference["id"].as_i64().unwrap()],
        )
        .expect("notification preference sync event");
    assert_eq!(rows.len(), 1);
    let metadata: Value = serde_json::from_str(&rows[0].get::<_, String>(0)).unwrap();
    assert_eq!(
        metadata["person_portable_id"],
        preference["person_portable_id"]
    );
}

fn api_error(response: reqwest::blocking::Response, status: u16) {
    assert_eq!(response.status().as_u16(), status);
    let body: Value = response.json().expect("error JSON");
    assert_eq!(body.as_object().unwrap().len(), 1);
    assert!(body["error"]["code"].is_string());
}

fn assert_preference(value: &Value) {
    let object = value.as_object().expect("preference object");
    let fields = [
        "id",
        "portable_id",
        "person_id",
        "person_portable_id",
        "enabled",
        "dose_due_enabled",
        "missed_dose_enabled",
        "low_stock_enabled",
        "private_text_enabled",
        "morning_time",
        "afternoon_time",
        "evening_time",
        "night_time",
        "updated_at",
    ];
    assert_eq!(object.len(), fields.len());
    for field in fields {
        assert!(object.contains_key(field), "missing {field}");
    }
    assert!(value["id"].as_u64().is_some_and(|id| id > 0));
    let portable_id = value["portable_id"].as_str().expect("portable ID");
    assert!(portable_id.len() == 36);
    assert!([8, 13, 18, 23]
        .iter()
        .all(|index| portable_id.as_bytes()[*index] == b'-'));
    assert!(portable_id
        .bytes()
        .enumerate()
        .all(|(index, byte)| { [8, 13, 18, 23].contains(&index) || byte.is_ascii_hexdigit() }));
    assert!(value["person_id"].as_u64().is_some_and(|id| id > 0));
    assert!(value["person_portable_id"].is_string());
    for field in [
        "enabled",
        "dose_due_enabled",
        "missed_dose_enabled",
        "low_stock_enabled",
        "private_text_enabled",
    ] {
        assert!(value[field].is_boolean(), "{field} must be boolean");
    }
    for field in [
        "morning_time",
        "afternoon_time",
        "evening_time",
        "night_time",
    ] {
        assert!(
            value[field].is_null()
                || value[field].as_str().is_some_and(|time| {
                    let bytes = time.as_bytes();
                    bytes.len() == 8
                        && bytes[2] == b':'
                        && bytes[5] == b':'
                        && bytes
                            .iter()
                            .enumerate()
                            .all(|(index, byte)| [2, 5].contains(&index) || byte.is_ascii_digit())
                        && time[0..2].parse::<u8>().is_ok_and(|hour| hour <= 23)
                        && time[3..5].parse::<u8>().is_ok_and(|minute| minute <= 59)
                        && time[6..8].parse::<u8>().is_ok_and(|second| second <= 59)
                })
        );
    }
    let updated_at = value["updated_at"].as_str().expect("updated_at");
    assert!(time::OffsetDateTime::parse(
        updated_at,
        &time::format_description::well_known::Rfc3339
    )
    .is_ok());
}

fn change_counts(preference_id: i64) -> (i64, i64) {
    let mut db = database();
    let events = db
        .query_one(
            "SELECT count(*) FROM api_change_events WHERE record_type = 'NotificationPreference' AND record_id = $1",
            &[&preference_id],
        )
        .expect("preference event count")
        .get(0);
    let versions = db
        .query_one(
            "SELECT count(*) FROM versions WHERE item_type = 'NotificationPreference' AND item_id = $1",
            &[&preference_id],
        )
        .expect("preference version count")
        .get(0);
    (events, versions)
}

fn path(household_id: i64) -> String {
    format!("/api/v1/households/{household_id}/notification_preference")
}

#[test]
fn notification_preference_routes_read_update_and_replace_the_signed_in_person_resource() {
    let fixture = fixture();
    let target = Target::from_env();
    let path = path(fixture.household_id);

    assert_eq!(
        target
            .get(&path, Some(&fixture.access_token))
            .status()
            .as_u16(),
        404
    );

    api_error(target.get(&path, None), 401);
    api_error(target.get(&path, Some(&fixture.foreign_access_token)), 403);

    let patched = target.patch_json(
        &path,
        &fixture.access_token,
        &json!({"notification_preference": {"enabled": false}}),
    );
    assert_eq!(patched.status().as_u16(), 200);
    assert!(patched.headers().get("etag").is_some());
    let patch_request_id = request_id(&patched);
    let patched_body: Value = patched.json().expect("PATCH response JSON");
    assert_eq!(patched_body.as_object().unwrap().len(), 1);
    assert_preference(&patched_body["data"]);
    assert_eq!(patched_body["data"]["person_id"], fixture.user_person_id);
    assert_eq!(patched_body["data"]["enabled"], false);
    assert_eq!(patched_body["data"]["dose_due_enabled"], true);
    assert_eq!(patched_body["data"]["missed_dose_enabled"], true);
    assert_eq!(patched_body["data"]["low_stock_enabled"], true);
    assert_eq!(patched_body["data"]["private_text_enabled"], false);
    assert_eq!(patched_body["data"]["morning_time"], "08:00:00");
    assert_eq!(patched_body["data"]["afternoon_time"], "14:00:00");
    assert_eq!(patched_body["data"]["evening_time"], "18:00:00");
    assert_eq!(patched_body["data"]["night_time"], "22:00:00");
    assert_sync_event(&patch_request_id, "create", &patched_body["data"]);

    let read = target.get(&path, Some(&fixture.access_token));
    assert_eq!(read.status().as_u16(), 200);
    assert!(read.headers().get("etag").is_some());
    let read_body: Value = read.json().expect("GET response JSON");
    assert_eq!(read_body["data"]["enabled"], false);

    let replaced = target.put_json(
        &path,
        &fixture.access_token,
        &json!({"notification_preference": {
            "dose_due_enabled": false,
            "morning_time": "07:05",
            "night_time": null
        }}),
    );
    assert_eq!(replaced.status().as_u16(), 200);
    assert!(replaced.headers().get("etag").is_some());
    let put_etag = replaced.headers()["etag"].to_str().unwrap().to_owned();
    let put_request_id = request_id(&replaced);
    let replaced_body: Value = replaced.json().expect("PUT response JSON");
    assert_preference(&replaced_body["data"]);
    assert_eq!(replaced_body["data"]["enabled"], false);
    assert_eq!(replaced_body["data"]["dose_due_enabled"], false);
    assert_eq!(replaced_body["data"]["morning_time"], "07:05:00");
    assert!(replaced_body["data"]["night_time"].is_null());
    assert_sync_event(&put_request_id, "update", &replaced_body["data"]);

    let preference_id = replaced_body["data"]["id"].as_i64().unwrap();
    let counts_before_noop = change_counts(preference_id);
    let no_op = target.patch_json(
        &path,
        &fixture.access_token,
        &json!({"notification_preference": {
            "dose_due_enabled": false,
            "morning_time": "07:05:00",
            "night_time": null
        }}),
    );
    assert_eq!(no_op.status().as_u16(), 200);
    assert_eq!(no_op.headers()["etag"], put_etag);
    let no_op_body: Value = no_op.json().expect("no-op PATCH response");
    assert_eq!(no_op_body["data"], replaced_body["data"]);
    assert_eq!(change_counts(preference_id), counts_before_noop);

    let mut viewer = OwnerViewGuard::new(&fixture);
    api_error(target.get(&path, Some(&fixture.access_token)), 404);
    viewer.expose_view();
    let visible: Value = target
        .get(&path, Some(&fixture.access_token))
        .json()
        .expect("read preference with view grant");
    assert_eq!(visible["data"], replaced_body["data"]);
    api_error(
        target.patch_json(
            &path,
            &fixture.access_token,
            &json!({"notification_preference": {"enabled": true}}),
        ),
        403,
    );
    let after_view_denial: Value = target
        .get(&path, Some(&fixture.access_token))
        .json()
        .expect("read preference after denied update");
    assert_eq!(after_view_denial["data"], replaced_body["data"]);
    viewer.restore();

    let denied = json!({"notification_preference": {"enabled": true}});
    api_error(target.patch_json_without_auth(&path, &denied), 401);
    api_error(
        target.put_json(&path, &fixture.foreign_access_token, &denied),
        403,
    );
    let unchanged: Value = target
        .get(&path, Some(&fixture.access_token))
        .json()
        .expect("read after denied writes");
    assert_eq!(unchanged["data"], replaced_body["data"]);

    let invalid_requests = [
        json!({"notification_preference": {}}),
        json!({"notification_preference": {"enabled": "false"}}),
        json!({"notification_preference": {"enabled": true, "morning_time": "24:00"}}),
        json!({"notification_preference": {"enabled": true, "morning_time": "23:59:60"}}),
        json!({"notification_preference": {"enabled": true, "morning_time": "12:60"}}),
        json!({"notification_preference": {"enabled": true, "unknown": false}}),
        json!({"wrong_wrapper": {"enabled": true}}),
        json!({"notification_preference": {"enabled": true, "person_id": fixture.user_person_id}}),
    ];
    for invalid in invalid_requests {
        for response in [
            target.patch_json(&path, &fixture.access_token, &invalid),
            target.put_json(&path, &fixture.access_token, &invalid),
        ] {
            let status = if invalid.get("wrong_wrapper").is_some() {
                400
            } else {
                422
            };
            api_error(response, status);
            let after: Value = target
                .get(&path, Some(&fixture.access_token))
                .json()
                .expect("read after rejected write");
            assert_eq!(after["data"], replaced_body["data"]);
        }
    }
}

#[test]
fn notification_preference_rate_limit_covers_get_patch_and_put() {
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
        "{base}/api/v1/households/{}/notification_preference",
        fixture.household_id
    );
    for response in [
        client.get(&path).bearer_auth(&fixture.access_token).send(),
        client
            .patch(&path)
            .bearer_auth(&fixture.access_token)
            .json(&json!({"notification_preference": {"enabled": false}}))
            .send(),
        client
            .put(&path)
            .bearer_auth(&fixture.access_token)
            .json(&json!({"notification_preference": {"enabled": false}}))
            .send(),
    ] {
        assert_eq!(
            response.expect("rate limited endpoint").status().as_u16(),
            429
        );
    }
}
