use medtracker_contract_tests::{fixture, Target};
use postgres::{Client, NoTls};
use serde_json::{json, Value};
use std::env;
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

fn database() -> Client {
    Client::connect(
        &env::var("CONTRACT_AUDIT_DATABASE_URL").expect("contract database URL"),
        NoTls,
    )
    .expect("contract database")
}

fn keys(value: &Value) -> Vec<&str> {
    value
        .as_object()
        .expect("object")
        .keys()
        .map(String::as_str)
        .collect()
}

fn error(response: reqwest::blocking::Response, expected_status: u16) -> Value {
    assert_eq!(response.status().as_u16(), expected_status);
    let body: Value = response.json().expect("error JSON");
    assert_eq!(keys(&body), ["error"]);
    assert!(keys(&body["error"])
        .iter()
        .all(|key| ["code", "message", "request_id", "errors"].contains(key)));
    for field in ["code", "message", "request_id"] {
        assert!(body["error"][field]
            .as_str()
            .is_some_and(|value| !value.is_empty()));
    }
    if let Some(errors) = body["error"].get("errors") {
        for messages in errors.as_object().expect("validation errors").values() {
            assert!(messages
                .as_array()
                .expect("error messages")
                .iter()
                .all(Value::is_string));
        }
    }
    body
}

fn location_body(response: reqwest::blocking::Response, status: u16) -> (Value, String) {
    assert_eq!(response.status().as_u16(), status);
    let etag = response.headers()["etag"]
        .to_str()
        .expect("ETag")
        .to_owned();
    let body: Value = response.json().expect("location JSON");
    assert_eq!(keys(&body), ["data"]);
    assert_eq!(
        keys(&body["data"]),
        ["description", "id", "name", "portable_id", "updated_at"]
    );
    assert!(body["data"]["id"].as_u64().is_some_and(|id| id > 0));
    OffsetDateTime::parse(
        body["data"]["updated_at"].as_str().expect("timestamp"),
        &Rfc3339,
    )
    .expect("RFC3339 timestamp");
    (body, etag)
}

fn delete_json_with_key(path: &str, token: &str, key: &str) -> reqwest::blocking::Response {
    let origin =
        url::Url::parse(&env::var("CONTRACT_BASE_URL").expect("CONTRACT_BASE_URL is required"))
            .expect("target URL");
    let local = match origin.host() {
        Some(url::Host::Domain("localhost")) => true,
        Some(url::Host::Ipv4(ip)) => ip.is_loopback(),
        Some(url::Host::Ipv6(ip)) => ip.is_loopback(),
        _ => false,
    };
    assert!(local, "contract write requests require a loopback target");
    assert!(path.starts_with('/') && !path.starts_with("//"));
    let url = origin.join(path).expect("request path");
    assert_eq!(url.origin(), origin.origin());
    reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .redirect(reqwest::redirect::Policy::none())
        .no_proxy()
        .build()
        .expect("HTTP client")
        .delete(url)
        .header("Accept", "application/json")
        .bearer_auth(token)
        .header("Idempotency-Key", key)
        .send()
        .expect("target must respond")
}

struct TemporaryLocation {
    db: Client,
    id: i64,
    portable_id: String,
}

impl TemporaryLocation {
    fn new(household_id: i64, name: &str) -> Self {
        let mut db = database();
        let row = db.query_one(
            "INSERT INTO locations (household_id, name, created_at, updated_at) VALUES ($1, $2, now(), now()) RETURNING id, portable_id",
            &[&household_id, &name],
        ).expect("temporary fixture location");
        Self {
            db,
            id: row.get(0),
            portable_id: row.get(1),
        }
    }

    fn from_created(id: i64) -> Self {
        let mut db = database();
        let row = db
            .query_one("SELECT portable_id FROM locations WHERE id = $1", &[&id])
            .expect("created location");
        Self {
            db,
            id,
            portable_id: row.get(0),
        }
    }

    fn path(&self, household_id: i64) -> String {
        format!(
            "/api/v1/households/{household_id}/locations/{}",
            self.portable_id
        )
    }

    fn name(&mut self) -> String {
        self.db
            .query_one("SELECT name FROM locations WHERE id = $1", &[&self.id])
            .expect("location row")
            .get(0)
    }

    fn membership_count(&mut self, person_id: i64) -> i64 {
        self.db.query_one(
            "SELECT count(*) FROM location_memberships WHERE location_id = $1 AND person_id = $2",
            &[&self.id, &person_id],
        ).expect("membership count").get(0)
    }
}

impl Drop for TemporaryLocation {
    fn drop(&mut self) {
        let _ = self.db.execute(
            "DELETE FROM location_memberships WHERE location_id = $1",
            &[&self.id],
        );
        let _ = self
            .db
            .execute("DELETE FROM locations WHERE id = $1", &[&self.id]);
    }
}

struct TemporaryMedicationGraph {
    location: TemporaryLocation,
    medication_id: i64,
    schedule_id: i64,
    assignment_id: i64,
    dosage_id: i64,
    membership_id: i64,
}

impl TemporaryMedicationGraph {
    fn new(household_id: i64, person_id: i64, name: &str) -> Self {
        let mut location = TemporaryLocation::new(household_id, name);
        let medication_id: i64 = location.db.query_one(
            "INSERT INTO medications (household_id, location_id, name, created_at, updated_at) VALUES ($1, $2, $3, now(), now()) RETURNING id",
            &[&household_id, &location.id, &name],
        ).expect("temporary medication").get(0);
        let dosage_id: i64 = location.db.query_one(
            "INSERT INTO dosages (household_id, medication_id, amount, unit, frequency, default_dose_cycle, default_max_daily_doses, default_min_hours_between_doses, created_at, updated_at) VALUES ($1, $2, 1.0, 'tablet', 'daily', 0, 1, 0.0, now(), now()) RETURNING id",
            &[&household_id, &medication_id],
        ).expect("temporary dosage option").get(0);
        let schedule_id: i64 = location.db.query_one(
            "INSERT INTO schedules (household_id, medication_id, person_id, source_dosage_option_id, created_at, updated_at) VALUES ($1, $2, $3, $4, now(), now()) RETURNING id",
            &[&household_id, &medication_id, &person_id, &dosage_id],
        ).expect("temporary schedule").get(0);
        let assignment_id: i64 = location.db.query_one(
            "INSERT INTO person_medications (household_id, medication_id, person_id, source_dosage_option_id, position, created_at, updated_at) VALUES ($1, $2, $3, $4, 900001, now(), now()) RETURNING id",
            &[&household_id, &medication_id, &person_id, &dosage_id],
        ).expect("temporary person assignment").get(0);
        let membership_id: i64 = location.db.query_one(
            "INSERT INTO location_memberships (household_id, location_id, person_id, created_at, updated_at) VALUES ($1, $2, $3, now(), now()) RETURNING id",
            &[&household_id, &location.id, &person_id],
        ).expect("temporary location membership").get(0);
        Self {
            location,
            medication_id,
            schedule_id,
            assignment_id,
            dosage_id,
            membership_id,
        }
    }

    fn count(&mut self, table: &str, id: i64) -> i64 {
        let statement = format!("SELECT count(*) FROM {table} WHERE id = $1");
        self.location
            .db
            .query_one(&statement, &[&id])
            .expect("graph row count")
            .get(0)
    }
}

impl Drop for TemporaryMedicationGraph {
    fn drop(&mut self) {
        let db = &mut self.location.db;
        let _ = db.execute("DELETE FROM medication_dose_occurrences WHERE schedule_id = $1 OR person_medication_id = $2", &[&self.schedule_id, &self.assignment_id]);
        let _ = db.execute(
            "DELETE FROM medication_takes WHERE schedule_id = $1 OR person_medication_id = $2",
            &[&self.schedule_id, &self.assignment_id],
        );
        let _ = db.execute(
            "DELETE FROM medication_takes WHERE taken_from_location_id = $1",
            &[&self.location.id],
        );
        let _ = db.execute("DELETE FROM medication_pause_periods WHERE schedule_id = $1 OR person_medication_id = $2", &[&self.schedule_id, &self.assignment_id]);
        let _ = db.execute("DELETE FROM schedules WHERE id = $1", &[&self.schedule_id]);
        let _ = db.execute(
            "DELETE FROM person_medications WHERE id = $1",
            &[&self.assignment_id],
        );
        let _ = db.execute("DELETE FROM dosages WHERE id = $1", &[&self.dosage_id]);
        let _ = db.execute(
            "DELETE FROM medications WHERE id = $1",
            &[&self.medication_id],
        );
    }
}

#[test]
fn create_location_obeys_documented_request_and_response() {
    let fixture = fixture();
    let target = Target::from_env();
    let base = format!("/api/v1/households/{}/locations", fixture.household_id);
    let name = "OpenAPI created location";
    error(
        target.post_json(&base, &json!({"location":{"name":name}})),
        401,
    );
    error(
        target.post_json_authorized(
            &base,
            &fixture.access_token,
            &json!({"location":{"name":""}}),
        ),
        422,
    );
    error(
        target.post_json_authorized(
            &base,
            &fixture.access_token,
            &json!({"location":{"name":name,"unexpected":true}}),
        ),
        422,
    );
    error(
        target.post_json_authorized(
            &base,
            &fixture.access_token,
            &json!({"location":{"name":name},"unexpected":true}),
        ),
        422,
    );
    error(
        target.post_json_authorized(&base, &fixture.access_token, &json!({})),
        422,
    );
    error(
        target.post_json_authorized(&base, &fixture.access_token, &json!({"location":{}})),
        422,
    );
    error(
        target.post_raw_json_authorized(&base, &fixture.access_token, ""),
        400,
    );
    error(
        target.post_raw_json_authorized(&base, &fixture.access_token, "{"),
        400,
    );

    let (body, _) = location_body(
        target.post_json_authorized(
            &base,
            &fixture.access_token,
            &json!({"location":{"name":name,"description":"Contract shelf"}}),
        ),
        201,
    );
    let id = body["data"]["id"].as_i64().expect("location ID");
    let temporary = TemporaryLocation::from_created(id);
    assert_eq!(body["data"]["portable_id"], temporary.portable_id);
    assert_eq!(body["data"]["name"], name);
    assert_eq!(body["data"]["description"], "Contract shelf");
    let (read, _) = location_body(
        target.get(
            &temporary.path(fixture.household_id),
            Some(&fixture.access_token),
        ),
        200,
    );
    assert_eq!(read["data"], body["data"]);

    let duplicate = error(
        target.post_json_authorized(
            &base,
            &fixture.access_token,
            &json!({"location":{"name":"oPENaPI CREATED LOCATION"}}),
        ),
        422,
    );
    assert_eq!(duplicate["error"]["code"], "validation_failed");
    assert!(duplicate["error"]["errors"]["name"]
        .as_array()
        .is_some_and(|messages| messages.iter().any(|message| {
            message
                .as_str()
                .is_some_and(|text| text.contains("already been taken"))
        })));
    let duplicate_count: i64 = database()
        .query_one(
            "SELECT count(*) FROM locations WHERE household_id = $1 AND lower(name) = lower($2)",
            &[&fixture.household_id, &name],
        )
        .expect("case-insensitive duplicate location count")
        .get(0);
    assert_eq!(duplicate_count, 1);
}

#[test]
fn keyed_location_create_replays_without_duplicate_resource_or_domain_events() {
    let fixture = fixture();
    let target = Target::from_env();
    let base = format!("/api/v1/households/{}/locations", fixture.household_id);
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock")
        .as_nanos();
    let name = format!("OpenAPI keyed location {suffix}");
    let key = format!("location-create-{}-{suffix}", fixture.household_id);
    let payload = json!({"location":{"name":name,"description":"Keyed create replay"}});

    let first = target.post_json_with_key(&base, &fixture.access_token, &key, &payload);
    let (created, _) = location_body(first, 201);
    let id = created["data"]["id"].as_i64().expect("created location ID");
    let mut temporary = TemporaryLocation::from_created(id);
    assert_eq!(created["data"]["name"], name);
    assert_eq!(created["data"]["description"], "Keyed create replay");
    let sync_count: i64 = temporary
        .db
        .query_one(
            "SELECT count(*) FROM api_change_events WHERE record_type = 'Location' AND record_id = $1",
            &[&id],
        )
        .expect("created location sync event count")
        .get(0);
    let version_count: i64 = temporary
        .db
        .query_one(
            "SELECT count(*) FROM versions WHERE item_type = 'Location' AND item_id = $1",
            &[&id],
        )
        .expect("created location version count")
        .get(0);
    assert_eq!(sync_count, 1);
    assert_eq!(version_count, 1);

    let replay = target.post_json_with_key(&base, &fixture.access_token, &key, &payload);
    assert_eq!(replay.status().as_u16(), 201);
    assert_eq!(replay.headers()["idempotency-replayed"], "true");
    let replay: Value = replay.json().expect("location create replay JSON");
    assert_eq!(replay, created);
    let resource_count: i64 = temporary
        .db
        .query_one(
            "SELECT count(*) FROM locations WHERE household_id = $1 AND name = $2",
            &[&fixture.household_id, &name],
        )
        .expect("single keyed location row")
        .get(0);
    assert_eq!(resource_count, 1);
    let sync_count_after: i64 = temporary
        .db
        .query_one(
            "SELECT count(*) FROM api_change_events WHERE record_type = 'Location' AND record_id = $1",
            &[&id],
        )
        .expect("location sync count after replay")
        .get(0);
    let version_count_after: i64 = temporary
        .db
        .query_one(
            "SELECT count(*) FROM versions WHERE item_type = 'Location' AND item_id = $1",
            &[&id],
        )
        .expect("location version count after replay")
        .get(0);
    assert_eq!(sync_count_after, sync_count);
    assert_eq!(version_count_after, version_count);

    let changed = target.post_json_with_key(
        &base,
        &fixture.access_token,
        &key,
        &json!({"location":{"name":format!("{name} changed")}}),
    );
    let changed = error(changed, 409);
    assert_eq!(changed["error"]["code"], "idempotency_key_reused");
    assert_eq!(temporary.name(), name);
}

#[test]
fn invalid_keyed_location_create_replays_without_mutation() {
    let fixture = fixture();
    let target = Target::from_env();
    let base = format!("/api/v1/households/{}/locations", fixture.household_id);
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock")
        .as_nanos();
    let key = format!("location-invalid-create-{}-{suffix}", fixture.household_id);
    let invalid = json!({"location":{"name":""}});
    let mut db = database();
    let locations_before: i64 = db
        .query_one(
            "SELECT count(*) FROM locations WHERE household_id = $1",
            &[&fixture.household_id],
        )
        .expect("location count before invalid request")
        .get(0);
    let versions_before: i64 = db
        .query_one(
            "SELECT count(*) FROM versions WHERE household_id = $1 AND item_type = 'Location'",
            &[&fixture.household_id],
        )
        .expect("location version count before invalid request")
        .get(0);
    let sync_before: i64 = db
        .query_one(
            "SELECT count(*) FROM api_change_events WHERE household_id = $1 AND record_type = 'Location'",
            &[&fixture.household_id],
        )
        .expect("location sync count before invalid request")
        .get(0);

    let first_response = target.post_json_with_key(&base, &fixture.access_token, &key, &invalid);
    let first_header_request_id = first_response.headers()["x-request-id"]
        .to_str()
        .expect("first request ID header")
        .to_owned();
    let first = error(first_response, 422);
    assert_eq!(first["error"]["code"], "validation_failed");
    let replay = target.post_json_with_key(&base, &fixture.access_token, &key, &invalid);
    assert_eq!(replay.status().as_u16(), 422);
    assert_eq!(replay.headers()["idempotency-replayed"], "true");
    let replay_header_request_id = replay.headers()["x-request-id"]
        .to_str()
        .expect("replay request ID header")
        .to_owned();
    let replay: Value = replay.json().expect("invalid location replay JSON");
    assert_eq!(first["error"]["request_id"], first_header_request_id);
    assert_eq!(replay["error"]["request_id"], replay_header_request_id);
    assert_ne!(replay["error"]["request_id"], first["error"]["request_id"]);
    for field in ["code", "message", "errors"] {
        assert_eq!(replay["error"][field], first["error"][field]);
    }

    let changed = target.post_json_with_key(
        &base,
        &fixture.access_token,
        &key,
        &json!({"location":{"name":format!("Changed invalid key {suffix}")}}),
    );
    let changed = error(changed, 409);
    assert_eq!(changed["error"]["code"], "idempotency_key_reused");

    let locations_after: i64 = db
        .query_one(
            "SELECT count(*) FROM locations WHERE household_id = $1",
            &[&fixture.household_id],
        )
        .expect("location count after invalid request")
        .get(0);
    let versions_after: i64 = db
        .query_one(
            "SELECT count(*) FROM versions WHERE household_id = $1 AND item_type = 'Location'",
            &[&fixture.household_id],
        )
        .expect("location version count after invalid request")
        .get(0);
    let sync_after: i64 = db
        .query_one(
            "SELECT count(*) FROM api_change_events WHERE household_id = $1 AND record_type = 'Location'",
            &[&fixture.household_id],
        )
        .expect("location sync count after invalid request")
        .get(0);
    assert_eq!(locations_after, locations_before);
    assert_eq!(versions_after, versions_before);
    assert_eq!(sync_after, sync_before);
}

#[test]
fn patch_and_put_require_current_etag_and_preserve_failed_mutations() {
    let fixture = fixture();
    let target = Target::from_env();
    let mut temporary = TemporaryLocation::new(fixture.household_id, "OpenAPI update source");
    temporary
        .db
        .execute(
            "UPDATE locations SET description = $1 WHERE id = $2",
            &[&"Original description", &temporary.id],
        )
        .expect("fixture description");
    let path = temporary.path(fixture.household_id);
    let (_, initial_etag) = location_body(target.get(&path, Some(&fixture.access_token)), 200);

    error(
        target.patch_raw_json_if_match(&path, &fixture.access_token, "{", &initial_etag),
        400,
    );
    error(
        target.put_raw_json_if_match(&path, &fixture.access_token, "", &initial_etag),
        400,
    );
    error(
        target.patch_json_if_match(&path, &fixture.access_token, &json!({}), &initial_etag),
        422,
    );
    error(
        target.patch_json_if_match(
            &path,
            &fixture.access_token,
            &json!({"location":{"description":"x"},"extra":true}),
            &initial_etag,
        ),
        422,
    );

    for id in [
        "+1",
        "00000000000000000000000000000000",
        "00000000-0000-0000-0000-000000000000",
    ] {
        error(
            target.patch_json_if_match(
                &format!("/api/v1/households/{}/locations/{id}", fixture.household_id),
                &fixture.access_token,
                &json!({"location":{"name":"Invalid identifier"}}),
                &initial_etag,
            ),
            400,
        );
    }

    error(
        target.patch_json(
            &path,
            &fixture.access_token,
            &json!({"location":{"name":"No precondition"}}),
        ),
        428,
    );
    error(
        target.put_json(
            &path,
            &fixture.access_token,
            &json!({"location":{"name":"No precondition"}}),
        ),
        428,
    );
    error(
        target.patch_json_if_match(
            &path,
            &fixture.access_token,
            &json!({"location":{"name":"Stale"}}),
            "\"stale\"",
        ),
        409,
    );
    assert_eq!(temporary.name(), "OpenAPI update source");

    let (patched, patched_etag) = location_body(
        target.patch_json_if_match(
            &path,
            &fixture.access_token,
            &json!({"location":{"name":"OpenAPI patched"}}),
            &initial_etag,
        ),
        200,
    );
    assert_eq!(patched["data"]["name"], "OpenAPI patched");
    assert_eq!(patched["data"]["description"], "Original description");
    assert_ne!(patched_etag, initial_etag);
    error(
        target.patch_json_if_match(
            &path,
            &fixture.access_token,
            &json!({"location":{"name":"Stale retry"}}),
            &initial_etag,
        ),
        409,
    );
    error(
        target.put_json_if_match(
            &path,
            &fixture.access_token,
            &json!({"location":{"name":"Stale PUT"}}),
            &initial_etag,
        ),
        409,
    );
    assert_eq!(temporary.name(), "OpenAPI patched");
    error(
        target.put_json_if_match(
            &path,
            &fixture.access_token,
            &json!({"location":{"name":""}}),
            &patched_etag,
        ),
        422,
    );
    error(
        target.put_json_if_match(
            &path,
            &fixture.access_token,
            &json!({"location":{"name":"Valid","unexpected":true}}),
            &patched_etag,
        ),
        422,
    );
    error(
        target.put_json_if_match(
            &path,
            &fixture.access_token,
            &json!({"location":{}}),
            &patched_etag,
        ),
        422,
    );
    error(
        target.put_json_if_match(
            &path,
            &fixture.access_token,
            &json!({"location":{"description":"x"},"unexpected":true}),
            &patched_etag,
        ),
        422,
    );
    assert_eq!(temporary.name(), "OpenAPI patched");

    let (replaced, new_etag) = location_body(
        target.put_json_if_match(
            &path,
            &fixture.access_token,
            &json!({"location":{"description":"Updated through PUT"}}),
            &patched_etag,
        ),
        200,
    );
    assert_eq!(replaced["data"]["name"], "OpenAPI patched");
    assert_eq!(replaced["data"]["description"], "Updated through PUT");
    assert_ne!(new_etag, patched_etag);
}

#[test]
fn put_location_updates_description_with_current_version() {
    let fixture = fixture();
    let target = Target::from_env();
    let temporary = TemporaryLocation::new(fixture.household_id, "OpenAPI PUT source");
    database()
        .execute(
            "UPDATE locations SET description = $1 WHERE id = $2",
            &[&"Before null", &temporary.id],
        )
        .expect("fixture description");
    let path = temporary.path(fixture.household_id);
    let (_, etag) = location_body(target.get(&path, Some(&fixture.access_token)), 200);
    let (updated, _) = location_body(
        target.put_json_if_match(
            &path,
            &fixture.access_token,
            &json!({"location":{"description":null}}),
            &etag,
        ),
        200,
    );
    assert_eq!(updated["data"]["name"], "OpenAPI PUT source");
    assert!(updated["data"]["description"].is_null());
}

#[test]
fn foreign_household_location_mutations_leave_rows_unchanged() {
    let fixture = fixture();
    let target = Target::from_env();
    let local_base = format!("/api/v1/households/{}/locations", fixture.household_id);
    let foreign_base = format!(
        "/api/v1/households/{}/locations",
        fixture.foreign_household_id
    );
    let foreign_path = format!("{local_base}/{}", fixture.foreign_location_portable_id);
    let original_name: String = database()
        .query_one(
            "SELECT name FROM locations WHERE id = $1",
            &[&fixture.foreign_location_id],
        )
        .expect("foreign location")
        .get(0);

    let create = target.post_json_authorized(
        &foreign_base,
        &fixture.access_token,
        &json!({"location":{"name":"OpenAPI forbidden foreign create"}}),
    );
    assert_eq!(create.status().as_u16(), 403);
    let update = target.patch_json_if_match(
        &foreign_path,
        &fixture.access_token,
        &json!({"location":{"name":"OpenAPI forbidden foreign update"}}),
        "\"stale\"",
    );
    assert_eq!(update.status().as_u16(), 404);
    let replacement = target.put_json_if_match(
        &foreign_path,
        &fixture.access_token,
        &json!({"location":{"name":"OpenAPI forbidden foreign replacement"}}),
        "\"stale\"",
    );
    assert_eq!(replacement.status().as_u16(), 404);
    let deletion = target.delete_if_match(&foreign_path, &fixture.access_token, "\"stale\"");
    assert_eq!(deletion.status().as_u16(), 404);
    let membership = target.post_json_authorized(
        &format!("{foreign_path}/location_memberships"),
        &fixture.access_token,
        &json!({"location_membership":{"person_id":fixture.managed_person_portable_id}}),
    );
    assert_eq!(membership.status().as_u16(), 404);
    error(
        target.post_json_authorized(
            &format!(
                "{foreign_base}/{}/location_memberships",
                fixture.foreign_location_portable_id
            ),
            &fixture.access_token,
            &json!({"location_membership":{"person_id":fixture.managed_person_portable_id}}),
        ),
        403,
    );
    error(
        target.delete(
            &format!(
                "{foreign_base}/{}/location_memberships/1",
                fixture.foreign_location_portable_id
            ),
            Some(&fixture.access_token),
        ),
        403,
    );
    error(
        target.delete(
            &format!("{foreign_path}/location_memberships/1"),
            Some(&fixture.access_token),
        ),
        404,
    );

    let mut db = database();
    let current_name: String = db
        .query_one(
            "SELECT name FROM locations WHERE id = $1",
            &[&fixture.foreign_location_id],
        )
        .expect("foreign location retained")
        .get(0);
    assert_eq!(current_name, original_name);
    let created: i64 = db
        .query_one(
            "SELECT count(*) FROM locations WHERE household_id = $1 AND name = $2",
            &[
                &fixture.foreign_household_id,
                &"OpenAPI forbidden foreign create",
            ],
        )
        .expect("foreign create count")
        .get(0);
    assert_eq!(created, 0);
}

#[test]
fn concurrent_location_updates_admit_only_one_matching_version() {
    let fixture = fixture();
    let mut temporary = TemporaryLocation::new(fixture.household_id, "OpenAPI racing source");
    let path = temporary.path(fixture.household_id);
    let (_, etag) = location_body(
        Target::from_env().get(&path, Some(&fixture.access_token)),
        200,
    );
    let barrier = Arc::new(Barrier::new(3));
    let mut handles = Vec::new();
    for name in ["OpenAPI racer A", "OpenAPI racer B"] {
        let path = path.clone();
        let token = fixture.access_token.clone();
        let etag = etag.clone();
        let barrier = Arc::clone(&barrier);
        handles.push(thread::spawn(move || {
            let target = Target::from_env();
            barrier.wait();
            target
                .patch_json_if_match(&path, &token, &json!({"location":{"name":name}}), &etag)
                .status()
                .as_u16()
        }));
    }
    barrier.wait();
    let mut statuses: Vec<u16> = handles
        .into_iter()
        .map(|handle| handle.join().expect("request thread"))
        .collect();
    statuses.sort_unstable();
    assert_eq!(statuses, [200, 409]);
    assert!(["OpenAPI racer A", "OpenAPI racer B"].contains(&temporary.name().as_str()));
}

#[test]
fn delete_location_requires_version_and_removes_unreferenced_storage() {
    let fixture = fixture();
    let target = Target::from_env();
    let temporary = TemporaryLocation::new(fixture.household_id, "OpenAPI deletable location");
    let path = temporary.path(fixture.household_id);
    let (_, etag) = location_body(target.get(&path, Some(&fixture.access_token)), 200);
    error(
        target.delete_if_match(
            &format!("/api/v1/households/{}/locations/+1", fixture.household_id),
            &fixture.access_token,
            &etag,
        ),
        400,
    );
    error(target.delete(&path, Some(&fixture.access_token)), 428);
    error(
        target.delete_if_match(&path, &fixture.access_token, "\"stale\""),
        409,
    );
    assert_eq!(
        target
            .get(&path, Some(&fixture.access_token))
            .status()
            .as_u16(),
        200
    );
    assert_eq!(
        {
            let response = target.delete_if_match(&path, &fixture.access_token, &etag);
            let status = response.status().as_u16();
            assert!(response.text().expect("empty deleted response").is_empty());
            status
        },
        204
    );
    error(target.get(&path, Some(&fixture.access_token)), 404);
}

#[test]
fn membership_create_replays_and_delete_is_scoped_to_location() {
    let fixture = fixture();
    let target = Target::from_env();
    let mut temporary = TemporaryLocation::new(fixture.household_id, "OpenAPI membership location");
    let other = TemporaryLocation::new(fixture.household_id, "OpenAPI other membership location");
    let base = format!(
        "{}/location_memberships",
        temporary.path(fixture.household_id)
    );
    let payload = json!({"location_membership":{"person_id":fixture.managed_person_portable_id}});
    error(target.post_json(&base, &payload), 401);
    error(
        target.post_json_authorized(
            &base,
            &fixture.access_token,
            &json!({"location_membership":{"person_id":"not-an-id"}}),
        ),
        422,
    );

    let response = target.post_json_authorized(&base, &fixture.access_token, &payload);
    assert_eq!(response.status().as_u16(), 201);
    let body: Value = response.json().expect("membership JSON");
    assert_eq!(keys(&body), ["data"]);
    assert_eq!(
        keys(&body["data"]),
        [
            "created_at",
            "id",
            "location_id",
            "location_portable_id",
            "person_id",
            "person_portable_id"
        ]
    );
    let id = body["data"]["id"]
        .as_str()
        .expect("membership ID")
        .to_owned();
    assert_eq!(body["data"]["location_id"], temporary.id.to_string());
    assert_eq!(body["data"]["location_portable_id"], temporary.portable_id);
    assert_eq!(
        body["data"]["person_id"],
        fixture.managed_person_id.to_string()
    );
    assert_eq!(
        body["data"]["person_portable_id"],
        fixture.managed_person_portable_id
    );
    OffsetDateTime::parse(
        body["data"]["created_at"].as_str().expect("timestamp"),
        &Rfc3339,
    )
    .expect("RFC3339 timestamp");

    let replay = target.post_json_authorized(&base, &fixture.access_token, &payload);
    assert_eq!(replay.status().as_u16(), 201);
    let replay_body: Value = replay.json().expect("replayed membership JSON");
    assert_eq!(replay_body["data"]["id"], id);
    assert_eq!(temporary.membership_count(fixture.managed_person_id), 1);

    let wrong_path = format!(
        "{}/location_memberships/{id}",
        other.path(fixture.household_id)
    );
    error(target.delete(&wrong_path, Some(&fixture.access_token)), 404);
    error(
        target.delete(&format!("{base}/+1"), Some(&fixture.access_token)),
        400,
    );
    assert_eq!(temporary.membership_count(fixture.managed_person_id), 1);
    let path = format!("{base}/{id}");
    let membership_id = id.parse::<i64>().expect("membership numeric ID");
    let versions_before: i64 = temporary
        .db
        .query_one(
            "SELECT count(*) FROM versions WHERE item_type = 'LocationMembership' AND item_id = $1",
            &[&membership_id],
        )
        .expect("membership version count before delete")
        .get(0);
    let sync_before: i64 = temporary
        .db
        .query_one(
            "SELECT count(*) FROM api_change_events WHERE record_type = 'Person' AND record_id = $1",
            &[&fixture.managed_person_id],
        )
        .expect("person sync count before delete")
        .get(0);
    let key = format!(
        "location-membership-delete-{}-{membership_id}",
        fixture.household_id
    );
    let removed = delete_json_with_key(&path, &fixture.access_token, &key);
    assert_eq!(removed.status().as_u16(), 204);
    assert!(removed
        .text()
        .expect("empty membership delete response")
        .is_empty());
    assert_eq!(temporary.membership_count(fixture.managed_person_id), 0);

    let versions_after_delete: i64 = temporary
        .db
        .query_one(
            "SELECT count(*) FROM versions WHERE item_type = 'LocationMembership' AND item_id = $1",
            &[&membership_id],
        )
        .expect("membership version count after delete")
        .get(0);
    let sync_after_delete: i64 = temporary
        .db
        .query_one(
            "SELECT count(*) FROM api_change_events WHERE record_type = 'Person' AND record_id = $1",
            &[&fixture.managed_person_id],
        )
        .expect("person sync count after delete")
        .get(0);
    assert_eq!(versions_after_delete, versions_before + 1);
    assert_eq!(sync_after_delete, sync_before + 1);

    let replay = delete_json_with_key(&path, &fixture.access_token, &key);
    assert_eq!(replay.status().as_u16(), 204);
    assert_eq!(replay.headers()["idempotency-replayed"], "true");
    assert!(replay
        .text()
        .expect("empty membership delete replay")
        .is_empty());
    assert_eq!(temporary.membership_count(fixture.managed_person_id), 0);
    let versions_after_replay: i64 = temporary
        .db
        .query_one(
            "SELECT count(*) FROM versions WHERE item_type = 'LocationMembership' AND item_id = $1",
            &[&membership_id],
        )
        .expect("membership version count after replay")
        .get(0);
    let sync_after_replay: i64 = temporary
        .db
        .query_one(
            "SELECT count(*) FROM api_change_events WHERE record_type = 'Person' AND record_id = $1",
            &[&fixture.managed_person_id],
        )
        .expect("person sync count after replay")
        .get(0);
    assert_eq!(versions_after_replay, versions_after_delete);
    assert_eq!(sync_after_replay, sync_after_delete);
    error(target.delete(&path, Some(&fixture.access_token)), 404);
}

#[test]
fn membership_delete_removes_a_fixture_assignment() {
    let fixture = fixture();
    let target = Target::from_env();
    let mut temporary = TemporaryLocation::new(fixture.household_id, "OpenAPI membership deletion");
    let id: i64 = temporary.db.query_one(
        "INSERT INTO location_memberships (household_id, location_id, person_id, created_at, updated_at) VALUES ($1, $2, $3, now(), now()) RETURNING id",
        &[&fixture.household_id, &temporary.id, &fixture.managed_person_id],
    ).expect("temporary fixture membership").get(0);
    let path = format!(
        "{}/location_memberships/{id}",
        temporary.path(fixture.household_id)
    );
    assert_eq!(
        target
            .delete(&path, Some(&fixture.access_token))
            .status()
            .as_u16(),
        204
    );
    assert_eq!(temporary.membership_count(fixture.managed_person_id), 0);
}

#[test]
fn location_permissions_distinguish_active_members_managers_and_households() {
    let fixture = fixture();
    let target = Target::from_env();
    let base = format!("/api/v1/households/{}/locations", fixture.household_id);
    let mut location = TemporaryLocation::new(fixture.household_id, "OpenAPI role fixture");
    let path = location.path(fixture.household_id);

    let list = target.get(&base, Some(&fixture.view_access_token));
    assert_eq!(list.status().as_u16(), 200);
    let list: Value = list.json().expect("member list JSON");
    assert!(list["data"]
        .as_array()
        .expect("locations")
        .iter()
        .any(|item| item["portable_id"] == fixture.hidden_location_portable_id));
    let read = target.get(&path, Some(&fixture.view_access_token));
    assert_eq!(read.status().as_u16(), 200);
    let (_, etag) = location_body(target.get(&path, Some(&fixture.manager_access_token)), 200);

    error(
        target.post_json_authorized(
            &base,
            &fixture.view_access_token,
            &json!({"location":{"name":"Member denied"}}),
        ),
        403,
    );
    error(
        target.patch_json_if_match(
            &path,
            &fixture.view_access_token,
            &json!({"location":{"name":"Member denied"}}),
            &etag,
        ),
        403,
    );
    error(
        target.put_json_if_match(
            &path,
            &fixture.view_access_token,
            &json!({"location":{"name":"Member denied"}}),
            &etag,
        ),
        403,
    );
    error(
        target.delete_if_match(&path, &fixture.view_access_token, &etag),
        403,
    );
    assert_eq!(location.name(), "OpenAPI role fixture");

    let (created, _) = location_body(
        target.post_json_authorized(
            &base,
            &fixture.manager_access_token,
            &json!({"location":{"name":"OpenAPI administrator created"}}),
        ),
        201,
    );
    let administrative_location =
        TemporaryLocation::from_created(created["data"]["id"].as_i64().expect("created ID"));
    let (updated, etag) = location_body(
        target.patch_json_if_match(
            &path,
            &fixture.manager_access_token,
            &json!({"location":{"name":"Administrator patched"}}),
            &etag,
        ),
        200,
    );
    assert_eq!(updated["data"]["name"], "Administrator patched");
    let (_, etag) = location_body(
        target.put_json_if_match(
            &path,
            &fixture.manager_access_token,
            &json!({"location":{"description":"Administrator updated"}}),
            &etag,
        ),
        200,
    );
    assert_eq!(
        target
            .delete_if_match(&path, &fixture.manager_access_token, &etag)
            .status()
            .as_u16(),
        204
    );
    drop(administrative_location);

    let foreign_base = format!(
        "/api/v1/households/{}/locations",
        fixture.foreign_household_id
    );
    error(target.get(&foreign_base, Some(&fixture.access_token)), 403);
    error(
        target.post_json_authorized(
            &foreign_base,
            &fixture.access_token,
            &json!({"location":{"name":"Wrong household"}}),
        ),
        403,
    );
    error(
        target.get(
            &format!("{base}/{}", fixture.foreign_location_portable_id),
            Some(&fixture.access_token),
        ),
        404,
    );
}

#[test]
fn location_membership_requires_manager_and_person_manage_grant() {
    let fixture = fixture();
    let target = Target::from_env();
    let mut location = TemporaryLocation::new(fixture.household_id, "OpenAPI grant fixture");
    let base = format!(
        "{}/location_memberships",
        location.path(fixture.household_id)
    );
    let payload = json!({"location_membership":{"person_id":fixture.managed_person_portable_id}});

    error(
        target.post_json_authorized(&base, &fixture.delegated_access_token, &payload),
        403,
    );
    error(
        target.post_json_authorized(&base, &fixture.manager_access_token, &payload),
        404,
    );
    error(
        target.post_json_authorized(&base, &fixture.view_owner_access_token, &payload),
        403,
    );
    error(
        target.post_json_authorized(
            &base,
            &fixture.access_token,
            &json!({"location_membership":{"person_id":fixture.hidden_person_portable_id}}),
        ),
        404,
    );
    error(
        target.post_raw_json_authorized(&base, &fixture.access_token, "{"),
        400,
    );
    error(target.post_json_authorized(&base, &fixture.access_token, &json!({"location_membership":{"person_id":fixture.managed_person_portable_id},"extra":true})), 422);

    let grant_id: i64 = location.db.query_one(
        "INSERT INTO person_access_grants (household_id, household_membership_id, person_id, access_level, relationship_type, created_at, updated_at) VALUES ($1, $2, $3, 'manage', 'family_member', now(), now()) RETURNING id",
        &[&fixture.household_id, &fixture.manager_membership_id, &fixture.managed_person_id],
    ).expect("administrator manage grant fixture").get(0);
    let assigned = target.post_json_authorized(&base, &fixture.manager_access_token, &payload);
    let status = assigned.status().as_u16();
    let body: Value = assigned.json().expect("assignment JSON");
    location
        .db
        .execute(
            "DELETE FROM person_access_grants WHERE id = $1",
            &[&grant_id],
        )
        .expect("grant cleanup");
    assert_eq!(status, 201);
    let membership_id = body["data"]["id"].as_str().expect("membership ID");
    let path = format!("{base}/{membership_id}");
    error(
        target.delete(&path, Some(&fixture.view_owner_access_token)),
        403,
    );
    error(
        target.delete(&path, Some(&fixture.manager_access_token)),
        404,
    );
    assert_eq!(location.membership_count(fixture.managed_person_id), 1);
    assert_eq!(
        target
            .delete(&path, Some(&fixture.access_token))
            .status()
            .as_u16(),
        204
    );
}

#[test]
fn every_location_operation_rejects_invalid_bearer_credentials() {
    let fixture = fixture();
    let target = Target::from_env();
    let base = format!("/api/v1/households/{}/locations", fixture.household_id);
    let location = TemporaryLocation::new(fixture.household_id, "OpenAPI invalid bearer fixture");
    let path = location.path(fixture.household_id);
    let membership_base = format!("{path}/location_memberships");
    let invalid = "invalid-bearer";
    error(target.get(&base, Some(invalid)), 401);
    error(target.get(&path, Some(invalid)), 401);
    error(
        target.post_json_authorized(&base, invalid, &json!({"location":{"name":"Denied"}})),
        401,
    );
    error(
        target.patch_json_if_match(
            &path,
            invalid,
            &json!({"location":{"name":"Denied"}}),
            "\"stale\"",
        ),
        401,
    );
    error(
        target.put_json_if_match(
            &path,
            invalid,
            &json!({"location":{"name":"Denied"}}),
            "\"stale\"",
        ),
        401,
    );
    error(target.delete_if_match(&path, invalid, "\"stale\""), 401);
    error(
        target.post_json_authorized(
            &membership_base,
            invalid,
            &json!({"location_membership":{"person_id":fixture.managed_person_portable_id}}),
        ),
        401,
    );
    error(
        target.delete(&format!("{membership_base}/1"), Some(invalid)),
        401,
    );
}

#[test]
fn every_location_operation_requires_authentication() {
    let fixture = fixture();
    let target = Target::from_env();
    let base = format!("/api/v1/households/{}/locations", fixture.household_id);
    let location = TemporaryLocation::new(fixture.household_id, "OpenAPI missing bearer fixture");
    let path = location.path(fixture.household_id);
    let membership_base = format!("{path}/location_memberships");
    error(target.get(&base, None), 401);
    error(target.get(&path, None), 401);
    error(
        target.post_json(&base, &json!({"location":{"name":"Denied"}})),
        401,
    );
    error(
        target.patch_json_without_auth(&path, &json!({"location":{"name":"Denied"}})),
        401,
    );
    error(
        target.put_json_without_auth(&path, &json!({"location":{"name":"Denied"}})),
        401,
    );
    error(target.delete(&path, None), 401);
    error(
        target.post_json(
            &membership_base,
            &json!({"location_membership":{"person_id":fixture.managed_person_portable_id}}),
        ),
        401,
    );
    error(target.delete(&format!("{membership_base}/1"), None), 401);
}

#[test]
fn location_delete_blocks_each_retained_administration_history_source() {
    let fixture = fixture();
    let target = Target::from_env();
    for mode in [
        "direct_take",
        "scheduled_take",
        "assignment_take",
        "saved_outcome",
        "assignment_outcome",
        "pause_period",
        "assignment_pause",
    ] {
        let name = format!("OpenAPI retained {mode}");
        let mut graph =
            TemporaryMedicationGraph::new(fixture.household_id, fixture.managed_person_id, &name);
        let history_id: i64 = match mode {
            "direct_take" => graph.location.db.query_one(
                "INSERT INTO medication_takes (household_id, schedule_id, taken_from_location_id, created_at, updated_at) VALUES ($1, $2, $3, now(), now()) RETURNING id",
                &[&fixture.household_id, &fixture.managed_schedule_id, &graph.location.id],
            ).expect("direct take").get(0),
            "scheduled_take" => graph.location.db.query_one(
                "INSERT INTO medication_takes (household_id, schedule_id, created_at, updated_at) VALUES ($1, $2, now(), now()) RETURNING id",
                &[&fixture.household_id, &graph.schedule_id],
            ).expect("scheduled take").get(0),
            "assignment_take" => graph.location.db.query_one(
                "INSERT INTO medication_takes (household_id, person_medication_id, created_at, updated_at) VALUES ($1, $2, now(), now()) RETURNING id",
                &[&fixture.household_id, &graph.assignment_id],
            ).expect("assignment take").get(0),
            "saved_outcome" => graph.location.db.query_one(
                "INSERT INTO medication_dose_occurrences (household_id, schedule_id, window_starts_on, position, created_at, updated_at) VALUES ($1, $2, current_date, 1, now(), now()) RETURNING id",
                &[&fixture.household_id, &graph.schedule_id],
            ).expect("saved outcome").get(0),
            "assignment_outcome" => graph.location.db.query_one(
                "INSERT INTO medication_dose_occurrences (household_id, person_medication_id, window_starts_on, position, created_at, updated_at) VALUES ($1, $2, current_date, 1, now(), now()) RETURNING id",
                &[&fixture.household_id, &graph.assignment_id],
            ).expect("assignment outcome").get(0),
            "pause_period" => graph.location.db.query_one(
                "INSERT INTO medication_pause_periods (household_id, schedule_id, reason, legacy_context, created_at, updated_at) VALUES ($1, $2, 'reason_not_recorded', true, now(), now()) RETURNING id",
                &[&fixture.household_id, &graph.schedule_id],
            ).expect("retained pause").get(0),
            "assignment_pause" => graph.location.db.query_one(
                "INSERT INTO medication_pause_periods (household_id, person_medication_id, reason, legacy_context, created_at, updated_at) VALUES ($1, $2, 'reason_not_recorded', true, now(), now()) RETURNING id",
                &[&fixture.household_id, &graph.assignment_id],
            ).expect("retained assignment pause").get(0),
            _ => unreachable!(),
        };
        let path = graph.location.path(fixture.household_id);
        let (_, etag) = location_body(target.get(&path, Some(&fixture.access_token)), 200);
        error(
            target.delete_if_match(&path, &fixture.access_token, &etag),
            422,
        );
        assert_eq!(graph.count("locations", graph.location.id), 1);
        assert_eq!(graph.count("medications", graph.medication_id), 1);
        let table = match mode {
            "direct_take" | "scheduled_take" | "assignment_take" => "medication_takes",
            "saved_outcome" | "assignment_outcome" => "medication_dose_occurrences",
            _ => "medication_pause_periods",
        };
        assert_eq!(graph.count(table, history_id), 1);
    }
}

#[test]
fn location_delete_cascades_unretained_storage_in_one_transaction() {
    let fixture = fixture();
    let target = Target::from_env();
    let mut graph = TemporaryMedicationGraph::new(
        fixture.household_id,
        fixture.managed_person_id,
        "OpenAPI cascade location",
    );
    let medication_portable_id: String = graph
        .location
        .db
        .query_one(
            "SELECT portable_id FROM medications WHERE id = $1",
            &[&graph.medication_id],
        )
        .expect("cascade medication portable ID")
        .get(0);
    let dosage_portable_id: String = graph
        .location
        .db
        .query_one(
            "SELECT portable_id FROM dosages WHERE id = $1",
            &[&graph.dosage_id],
        )
        .expect("cascade dosage portable ID")
        .get(0);
    let schedule_portable_id: String = graph
        .location
        .db
        .query_one(
            "SELECT portable_id FROM schedules WHERE id = $1",
            &[&graph.schedule_id],
        )
        .expect("cascade schedule portable ID")
        .get(0);
    let assignment_portable_id: String = graph
        .location
        .db
        .query_one(
            "SELECT portable_id FROM person_medications WHERE id = $1",
            &[&graph.assignment_id],
        )
        .expect("cascade assignment portable ID")
        .get(0);
    let person_sync_before: i64 = graph
        .location
        .db
        .query_one(
            "SELECT count(*) FROM api_change_events WHERE household_id = $1 AND record_type = 'Person' AND record_id = $2",
            &[&fixture.household_id, &fixture.managed_person_id],
        )
        .expect("person sync count before cascade")
        .get(0);
    let path = graph.location.path(fixture.household_id);
    let (_, etag) = location_body(target.get(&path, Some(&fixture.access_token)), 200);
    let response = target.delete_if_match(&path, &fixture.access_token, &etag);
    assert_eq!(response.status().as_u16(), 204);
    assert!(response.text().expect("empty delete body").is_empty());
    for (table, id) in [
        ("locations", graph.location.id),
        ("medications", graph.medication_id),
        ("dosages", graph.dosage_id),
        ("schedules", graph.schedule_id),
        ("person_medications", graph.assignment_id),
        ("location_memberships", graph.membership_id),
    ] {
        assert_eq!(graph.count(table, id), 0, "{table} should be deleted");
    }

    for (record_type, portable_id) in [
        ("Medication", medication_portable_id.as_str()),
        ("MedicationDosageOption", dosage_portable_id.as_str()),
        ("Schedule", schedule_portable_id.as_str()),
        ("PersonMedication", assignment_portable_id.as_str()),
    ] {
        let tombstones: i64 = graph
            .location
            .db
            .query_one(
                "SELECT count(*) FROM api_tombstones WHERE household_id = $1 AND record_type = $2 AND record_portable_id = $3 AND action = 'delete'",
                &[&fixture.household_id, &record_type, &portable_id],
            )
            .expect("cascade tombstone count")
            .get(0);
        assert_eq!(tombstones, 1, "{record_type} delete tombstone");
    }
    for (record_type, portable_id) in [
        ("Schedule", &schedule_portable_id),
        ("PersonMedication", &assignment_portable_id),
    ] {
        let metadata_text: String = graph
            .location
            .db
            .query_one(
                "SELECT metadata::text FROM api_tombstones WHERE household_id = $1 AND record_type = $2 AND record_portable_id = $3",
                &[&fixture.household_id, &record_type, portable_id],
            )
            .expect("source tombstone metadata")
            .get(0);
        let metadata: Value = serde_json::from_str(&metadata_text).expect("source tombstone JSON");
        assert_eq!(
            metadata["person_portable_id"],
            fixture.managed_person_portable_id,
            "{record_type} person visibility"
        );
    }
    let medication_metadata_text: String = graph
        .location
        .db
        .query_one(
            "SELECT metadata::text FROM api_tombstones WHERE household_id = $1 AND record_type = 'Medication' AND record_portable_id = $2",
            &[&fixture.household_id, &medication_portable_id],
        )
        .expect("medication tombstone metadata")
        .get(0);
    let medication_metadata: Value =
        serde_json::from_str(&medication_metadata_text).expect("medication tombstone JSON");
    assert_eq!(
        medication_metadata["sync_person_portable_ids"],
        json!([fixture.managed_person_portable_id])
    );
    assert!(!medication_metadata
        .to_string()
        .contains(&fixture.foreign_email));
    let location_tombstones: i64 = graph
        .location
        .db
        .query_one(
            "SELECT count(*) FROM api_tombstones WHERE household_id = $1 AND record_type = 'Location' AND record_portable_id = $2 AND action = 'delete'",
            &[&fixture.household_id, &graph.location.portable_id],
        )
        .expect("location tombstone count")
        .get(0);
    assert_eq!(location_tombstones, 1);

    for (item_type, item_id) in [
        ("MedicationDosageOption", graph.dosage_id),
        ("Schedule", graph.schedule_id),
        ("PersonMedication", graph.assignment_id),
        ("LocationMembership", graph.membership_id),
    ] {
        let destroys: i64 = graph
            .location
            .db
            .query_one(
                "SELECT count(*) FROM versions WHERE household_id = $1 AND item_type = $2 AND item_id = $3 AND event = 'destroy'",
                &[&fixture.household_id, &item_type, &item_id],
            )
            .expect("cascade destroy version count")
            .get(0);
        assert_eq!(destroys, 1, "{item_type} destroy version");
    }

    let person_sync_after: i64 = graph
        .location
        .db
        .query_one(
            "SELECT count(*) FROM api_change_events WHERE household_id = $1 AND record_type = 'Person' AND record_id = $2",
            &[&fixture.household_id, &fixture.managed_person_id],
        )
        .expect("person sync count after cascade")
        .get(0);
    assert_eq!(person_sync_after, person_sync_before);
}

#[test]
fn location_delete_rolls_back_cascade_when_a_late_foreign_key_blocks_it() {
    let fixture = fixture();
    let target = Target::from_env();
    let mut graph = TemporaryMedicationGraph::new(
        fixture.household_id,
        fixture.managed_person_id,
        "OpenAPI rollback location",
    );
    let link_id: i64 = graph.location.db.query_one(
        "INSERT INTO health_event_medications (health_event_id, household_id, medication_id, medication_name, created_at, updated_at) VALUES ($1, $2, $3, 'Retained reference', now(), now()) RETURNING id",
        &[&fixture.managed_health_event_id, &fixture.household_id, &graph.medication_id],
    ).expect("late foreign key fixture").get(0);
    let path = graph.location.path(fixture.household_id);
    let (_, etag) = location_body(target.get(&path, Some(&fixture.access_token)), 200);
    let response = target.delete_if_match(&path, &fixture.access_token, &etag);
    let status = response.status().as_u16();
    let body: Value = response.json().expect("failure JSON");
    let retained = [
        ("locations", graph.location.id),
        ("medications", graph.medication_id),
        ("dosages", graph.dosage_id),
        ("schedules", graph.schedule_id),
        ("person_medications", graph.assignment_id),
        ("location_memberships", graph.membership_id),
        ("health_event_medications", link_id),
    ]
    .map(|(table, id)| (table, graph.count(table, id)));
    graph
        .location
        .db
        .execute(
            "DELETE FROM health_event_medications WHERE id = $1",
            &[&link_id],
        )
        .expect("reference cleanup");
    assert_eq!(status, 422);
    assert_eq!(body["error"]["code"], "validation_failed");
    for (table, count) in retained {
        assert_eq!(count, 1, "{table} should survive rollback");
    }
}

#[test]
fn location_delete_and_new_take_cannot_both_commit() {
    let fixture = fixture();
    let target = Target::from_env();
    let mut graph = TemporaryMedicationGraph::new(
        fixture.household_id,
        fixture.managed_person_id,
        "OpenAPI concurrent history location",
    );
    let path = graph.location.path(fixture.household_id);
    let (_, etag) = location_body(target.get(&path, Some(&fixture.access_token)), 200);
    let barrier = Arc::new(Barrier::new(3));
    let delete_barrier = barrier.clone();
    let delete_token = fixture.access_token.clone();
    let delete = thread::spawn(move || {
        delete_barrier.wait();
        Target::from_env()
            .delete_if_match(&path, &delete_token, &etag)
            .status()
            .as_u16()
    });
    let insert_barrier = barrier.clone();
    let household_id = fixture.household_id;
    let schedule_id = graph.schedule_id;
    let insert = thread::spawn(move || {
        let mut db = database();
        insert_barrier.wait();
        db.query_one(
            "INSERT INTO medication_takes (household_id, schedule_id, created_at, updated_at) VALUES ($1, $2, now(), now()) RETURNING id",
            &[&household_id, &schedule_id],
        )
        .map(|row| row.get::<usize, i64>(0))
    });
    barrier.wait();
    let delete_status = delete.join().expect("delete thread");
    let take = insert.join().expect("insert thread");
    match (delete_status, take) {
        (422, Ok(take_id)) => {
            assert_eq!(graph.count("locations", graph.location.id), 1);
            assert_eq!(graph.count("medication_takes", take_id), 1);
        }
        (204, Err(error)) if error.code().is_some_and(|code| code.code() == "23503") => {
            assert_eq!(graph.count("locations", graph.location.id), 0);
        }
        (status, result) => panic!("unsafe delete/history race: {status}, take={result:?}"),
    }
}

#[test]
fn revoked_expired_and_platform_admin_credentials_do_not_bypass_location_policy() {
    let fixture = fixture();
    let target = Target::from_env();
    let base = format!("/api/v1/households/{}/locations", fixture.household_id);
    error(target.get(&base, Some(&fixture.expired_access_token)), 401);
    error(
        target.get(
            &format!(
                "/api/v1/households/{}/locations",
                fixture.auth_inactive_household_id
            ),
            Some(&fixture.auth_inactive_access_token),
        ),
        401,
    );
    error(
        target.get(&base, Some(&fixture.auth_inactive_access_token)),
        401,
    );
    error(
        target.get(
            &format!(
                "/api/v1/households/{}/locations",
                fixture.portable_target_household_id
            ),
            Some(&fixture.portable_revoked_access_token),
        ),
        401,
    );
    error(
        target.get(&base, Some(&fixture.portable_revoked_access_token)),
        401,
    );
    let mut db = database();
    db.execute(
        "UPDATE household_memberships SET status = 'suspended', updated_at = now() WHERE id = $1",
        &[&fixture.auth_role_member_membership_id],
    )
    .expect("suspend membership");
    let suspended = target.get(
        &format!(
            "/api/v1/households/{}/locations",
            fixture.auth_role_household_id
        ),
        Some(&fixture.auth_role_member_access_token),
    );
    db.execute(
        "UPDATE household_memberships SET status = 'active', updated_at = now() WHERE id = $1",
        &[&fixture.auth_role_member_membership_id],
    )
    .expect("restore membership");
    error(suspended, 401);
    error(target.get(&base, Some(&fixture.platform_access_token)), 403);
    error(
        target.post_json_authorized(
            &base,
            &fixture.platform_access_token,
            &json!({"location":{"name":"Denied platform admin"}}),
        ),
        403,
    );
}

#[test]
fn membership_person_grant_must_be_current_and_manage_level() {
    let fixture = fixture();
    let target = Target::from_env();
    let mut location =
        TemporaryLocation::new(fixture.household_id, "OpenAPI grant lifetime fixture");
    let path = format!(
        "{}/location_memberships",
        location.path(fixture.household_id)
    );
    let payload = json!({"location_membership":{"person_id":fixture.managed_person_portable_id}});
    let grant_id: i64 = location.db.query_one(
        "INSERT INTO person_access_grants (household_id, household_membership_id, person_id, access_level, relationship_type, revoked_at, created_at, updated_at) VALUES ($1, $2, $3, 'manage', 'family_member', now(), now(), now()) RETURNING id",
        &[&fixture.household_id, &fixture.manager_membership_id, &fixture.managed_person_id],
    ).expect("revoked grant fixture").get(0);
    error(
        target.post_json_authorized(&path, &fixture.manager_access_token, &payload),
        404,
    );
    location.db.execute("UPDATE person_access_grants SET revoked_at = NULL, expires_at = now() - interval '1 minute' WHERE id = $1", &[&grant_id]).expect("expire grant");
    error(
        target.post_json_authorized(&path, &fixture.manager_access_token, &payload),
        404,
    );
    location.db.execute("UPDATE person_access_grants SET expires_at = now() + interval '1 hour', access_level = 'view' WHERE id = $1", &[&grant_id]).expect("view grant");
    error(
        target.post_json_authorized(&path, &fixture.manager_access_token, &payload),
        403,
    );
    location
        .db
        .execute(
            "DELETE FROM person_access_grants WHERE id = $1",
            &[&grant_id],
        )
        .expect("grant cleanup");
    assert_eq!(location.membership_count(fixture.managed_person_id), 0);
}
