use medtracker_contract_tests::{fixture, Target};
use postgres::{Client, NoTls};
use serde_json::{json, Value};
use std::env;
use std::sync::{Arc, Barrier};
use std::thread;
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
    assert!([403, 404].contains(&create.status().as_u16()));
    let update = target.patch_json_if_match(
        &foreign_path,
        &fixture.access_token,
        &json!({"location":{"name":"OpenAPI forbidden foreign update"}}),
        "\"stale\"",
    );
    assert!([403, 404].contains(&update.status().as_u16()));
    let deletion = target.delete_if_match(&foreign_path, &fixture.access_token, "\"stale\"");
    assert!([403, 404].contains(&deletion.status().as_u16()));
    let membership = target.post_json_authorized(
        &format!("{foreign_path}/location_memberships"),
        &fixture.access_token,
        &json!({"location_membership":{"person_id":fixture.managed_person_portable_id}}),
    );
    assert!([403, 404].contains(&membership.status().as_u16()));

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
    let removed = target.delete(&path, Some(&fixture.access_token));
    assert_eq!(removed.status().as_u16(), 204);
    assert!(removed
        .text()
        .expect("empty membership delete response")
        .is_empty());
    assert_eq!(temporary.membership_count(fixture.managed_person_id), 0);
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
