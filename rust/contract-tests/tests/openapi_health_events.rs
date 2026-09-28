use medtracker_contract_tests::{fixture, Fixture, Target};
use postgres::{Client as PostgresClient, NoTls};
use reqwest::blocking::{Client, Response};
use serde_json::{json, Value};
use std::env;
use std::time::{Duration, Instant};

fn body(response: Response) -> Value {
    response.json().expect("JSON response")
}

fn health_path(fixture: &Fixture) -> String {
    format!("/api/v1/households/{}/health_events", fixture.household_id)
}

fn database() -> PostgresClient {
    PostgresClient::connect(
        &env::var("CONTRACT_AUDIT_DATABASE_URL").expect("contract database URL"),
        NoTls,
    )
    .expect("contract database")
}

struct MedicationNameRestore {
    db: PostgresClient,
    household_id: i64,
    medication_id: i64,
    original_name: String,
}

impl MedicationNameRestore {
    fn rename(household_id: i64, medication_id: i64, suffix: &str) -> Self {
        let mut db = database();
        let original_name: String = db
            .query_one(
                "SELECT name FROM medications WHERE household_id = $1 AND id = $2",
                &[&household_id, &medication_id],
            )
            .expect("fixture medication name")
            .get(0);
        db.execute(
            "UPDATE medications SET name = $1 WHERE household_id = $2 AND id = $3",
            &[
                &format!("{original_name} {suffix}"),
                &household_id,
                &medication_id,
            ],
        )
        .expect("rename fixture medication");
        Self {
            db,
            household_id,
            medication_id,
            original_name,
        }
    }
}

impl Drop for MedicationNameRestore {
    fn drop(&mut self) {
        self.db
            .execute(
                "UPDATE medications SET name = $1 WHERE household_id = $2 AND id = $3",
                &[&self.original_name, &self.household_id, &self.medication_id],
            )
            .expect("restore fixture medication name");
    }
}

fn request_id(response: &Response) -> String {
    response.headers()["x-request-id"]
        .to_str()
        .expect("request ID")
        .to_owned()
}

fn assert_error(response: Response, status: u16) -> Value {
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
    assert_eq!(payload["error"]["request_id"], id);
    assert!(payload["error"]["code"].is_string());
    assert!(payload["error"]["message"]
        .as_str()
        .is_some_and(|s| !s.is_empty()));
    assert!(payload.get("data").is_none());
    payload
}

fn assert_health_event(row: &Value) {
    let object = row.as_object().expect("health event object");
    assert_eq!(object.len(), 13);
    for field in [
        "id",
        "portable_id",
        "person_id",
        "person_portable_id",
        "event_kind",
        "severity",
        "title",
        "notes",
        "started_on",
        "ended_on",
        "updated_at",
        "medication_ids",
        "medication_portable_ids",
    ] {
        assert!(
            object.contains_key(field),
            "missing health-event field {field}"
        );
    }
    assert!(row["id"].is_i64());
    assert!(row["portable_id"].is_string());
    assert!(row["person_id"].is_i64());
    assert!(row["person_portable_id"].is_string());
    assert!(matches!(
        row["event_kind"].as_str(),
        Some("illness" | "suspected_side_effect")
    ));
    assert!(
        row["severity"].is_null()
            || matches!(
                row["severity"].as_str(),
                Some("mild" | "moderate" | "severe")
            )
    );
    assert!(row["title"].as_str().is_some_and(|v| !v.is_empty()));
    assert!(row["notes"].is_null() || row["notes"].is_string());
    assert!(row["started_on"].is_string());
    assert!(row["ended_on"].is_null() || row["ended_on"].is_string());
    assert!(row["updated_at"].is_string());
    assert!(row["medication_ids"].is_array());
    assert!(row["medication_portable_ids"].is_array());
}

fn create_event(target: &Target, fixture: &Fixture, title: &str) -> (Value, String, String) {
    let response = target.post_json_authorized(
        &health_path(fixture),
        &fixture.access_token,
        &json!({"health_event": {
            "person_id": fixture.managed_person_portable_id,
            "event_kind": "illness",
            "severity": "mild",
            "title": title,
            "notes": "Observed at home",
            "started_on": "2026-02-25",
            "medication_ids": [fixture.managed_medication_portable_id]
        }}),
    );
    assert_eq!(response.status().as_u16(), 201);
    let request_id = request_id(&response);
    let tag = response.headers()["etag"]
        .to_str()
        .expect("created ETag")
        .to_owned();
    let created = body(response)["data"].clone();
    assert_health_event(&created);
    (created, tag, request_id)
}

fn assert_health_event_effect(household_id: i64, event_id: i64, request_id: &str, action: &str) {
    let mut db = database();
    let versions: i64 = db
        .query_one(
            "SELECT count(*) FROM versions WHERE household_id = $1 AND item_type = 'HealthEvent' AND item_id = $2 AND event = $3 AND request_id = $4",
            &[&household_id, &event_id, &action, &request_id],
        )
        .expect("health event version count")
        .get(0);
    let sync_events: i64 = db
        .query_one(
            "SELECT count(*) FROM api_change_events WHERE household_id = $1 AND record_type = 'HealthEvent' AND record_id = $2 AND action = $3 AND request_id = $4",
            &[&household_id, &event_id, &action, &request_id],
        )
        .expect("health event sync count")
        .get(0);
    assert_eq!(versions, 1, "one matching HealthEvent version");
    assert_eq!(sync_events, 1, "one matching HealthEvent sync event");
}

#[test]
fn health_event_requests_enforce_schema_without_changing_existing_records() {
    let target = Target::from_env();
    let fixture = fixture();
    let collection = health_path(&fixture);
    let (created, tag, _) = create_event(&target, &fixture, "Health schema validation probe");
    let item = format!("{collection}/{}", created["portable_id"].as_str().unwrap());
    let before = body(target.get(&item, Some(&fixture.access_token)));
    let count_before = body(target.get(&collection, Some(&fixture.access_token)))["data"]
        .as_array()
        .expect("health event collection")
        .len();
    assert_health_event(&before["data"]);
    assert_eq!(
        target.get(&item, Some(&fixture.access_token)).headers()["etag"],
        tag
    );

    let invalid_creates = [
        (json!({}), 400),
        (
            json!({"health_event": {
            "person_id": fixture.managed_person_portable_id,
            "event_kind": "illness", "title": "Unknown root field", "started_on": "2026-02-25"
        }, "extra": true}),
            422,
        ),
        (
            json!({"health_event": {
                "person_id": fixture.managed_person_portable_id,
                "event_kind": "illness", "title": "Unknown inner field", "started_on": "2026-02-25", "extra": true
            }}),
            422,
        ),
        (
            json!({"health_event": {
                "person_id": fixture.managed_person_portable_id,
                "event_kind": "illness", "title": "Null notes", "notes": null, "started_on": "2026-02-25"
            }}),
            422,
        ),
        (
            json!({"health_event": {
                "person_id": fixture.managed_person_portable_id,
                "event_kind": "illness", "title": "Bad date", "started_on": "25 February 2026"
            }}),
            422,
        ),
        (
            json!({"health_event": {
                "person_id": fixture.managed_person_portable_id,
                "event_kind": "unsupported", "title": "Bad enum", "started_on": "2026-02-25"
            }}),
            422,
        ),
    ];
    for (payload, status) in invalid_creates {
        assert_error(
            target.post_json_authorized(&collection, &fixture.access_token, &payload),
            status,
        );
    }
    let count_after = body(target.get(&collection, Some(&fixture.access_token)))["data"]
        .as_array()
        .expect("health event collection")
        .len();
    assert_eq!(count_after, count_before);

    for method in ["patch", "put"] {
        let payload = json!({"health_event": {"title": "Changed", "extra": true}});
        let response = if method == "patch" {
            target.patch_json(&item, &fixture.access_token, &payload)
        } else {
            target.put_json(&item, &fixture.access_token, &payload)
        };
        assert_error(response, 422);
        let current = body(target.get(&item, Some(&fixture.access_token)));
        assert_eq!(current, before);
        assert_eq!(
            target.get(&item, Some(&fixture.access_token)).headers()["etag"],
            tag
        );
    }
}

#[test]
fn health_event_create_get_list_and_update_match_exact_resource_shape() {
    let target = Target::from_env();
    let fixture = fixture();
    let collection = health_path(&fixture);
    let (created, tag, _) = create_event(&target, &fixture, "Exact health event response");
    assert_eq!(
        created["medication_ids"],
        json!([fixture.managed_medication_id])
    );
    assert_eq!(
        created["medication_portable_ids"],
        json!([fixture.managed_medication_portable_id])
    );
    let item = format!("{collection}/{}", created["portable_id"].as_str().unwrap());
    let read = target.get(&item, Some(&fixture.access_token));
    assert_eq!(read.status().as_u16(), 200);
    assert_eq!(read.headers()["etag"], tag);
    assert_eq!(body(read)["data"], created);

    let listing = body(target.get(&collection, Some(&fixture.access_token)));
    let envelope = listing.as_object().expect("collection envelope");
    assert_eq!(envelope.len(), 2);
    assert!(envelope["data"].is_array());
    let meta = envelope["meta"].as_object().expect("pagination metadata");
    assert_eq!(meta.len(), 3);
    for field in ["page", "per_page", "total_count"] {
        assert!(meta.contains_key(field), "missing pagination field {field}");
    }
    assert_eq!(meta["page"], 1);
    assert_eq!(meta["per_page"], 20);
    assert!(listing["data"].as_array().unwrap().iter().any(|row| {
        assert_health_event(row);
        row["id"] == created["id"]
    }));

    let updated = target.patch_json_if_match(
        &item,
        &fixture.access_token,
        &json!({"health_event": {"title": "Updated exact response"}}),
        &tag,
    );
    assert_eq!(updated.status().as_u16(), 200);
    let updated_tag = updated.headers()["etag"]
        .to_str()
        .expect("updated ETag")
        .to_owned();
    let updated_body = body(updated);
    assert_health_event(&updated_body["data"]);
    assert_eq!(updated_body["data"]["title"], "Updated exact response");
    assert_eq!(
        updated_body["data"]["medication_ids"],
        json!([fixture.managed_medication_id])
    );
    assert_eq!(
        updated_body["data"]["medication_portable_ids"],
        json!([fixture.managed_medication_portable_id])
    );
    assert_ne!(updated_tag, tag);

    let replaced = target.put_json_if_match(
        &item,
        &fixture.access_token,
        &json!({"health_event": {"severity": "moderate"}}),
        &updated_tag,
    );
    assert_eq!(replaced.status().as_u16(), 200);
    let replaced_tag = replaced.headers()["etag"]
        .to_str()
        .expect("replacement ETag")
        .to_owned();
    let replaced_body = body(replaced);
    assert_health_event(&replaced_body["data"]);
    assert_eq!(
        replaced_body["data"]["medication_ids"],
        json!([fixture.managed_medication_id])
    );
    assert_eq!(
        replaced_body["data"]["medication_portable_ids"],
        json!([fixture.managed_medication_portable_id])
    );

    let patch_clear = target.patch_json_if_match(
        &item,
        &fixture.access_token,
        &json!({"health_event": {"medication_ids": []}}),
        &replaced_tag,
    );
    assert_eq!(patch_clear.status().as_u16(), 200);
    let patch_clear_body = body(patch_clear);
    assert_eq!(patch_clear_body["data"]["medication_ids"], json!([]));
    assert_eq!(
        patch_clear_body["data"]["medication_portable_ids"],
        json!([])
    );
    let patch_clear_tag = target.get(&item, Some(&fixture.access_token)).headers()["etag"]
        .to_str()
        .expect("cleared ETag")
        .to_owned();

    let put_restore = target.put_json_if_match(
        &item,
        &fixture.access_token,
        &json!({"health_event": {"medication_ids": [fixture.managed_medication_portable_id]}}),
        &patch_clear_tag,
    );
    assert_eq!(put_restore.status().as_u16(), 200);
    let put_restore_tag = put_restore.headers()["etag"]
        .to_str()
        .expect("restored ETag")
        .to_owned();
    assert_eq!(
        body(put_restore)["data"]["medication_ids"],
        json!([fixture.managed_medication_id])
    );

    let put_clear = target.put_json_if_match(
        &item,
        &fixture.access_token,
        &json!({"health_event": {"medication_ids": []}}),
        &put_restore_tag,
    );
    assert_eq!(put_clear.status().as_u16(), 200);
    let put_clear_body = body(put_clear);
    assert_eq!(put_clear_body["data"]["medication_ids"], json!([]));
    assert_eq!(put_clear_body["data"]["medication_portable_ids"], json!([]));
}

#[test]
fn updating_event_with_same_medication_preserves_link_snapshot() {
    let target = Target::from_env();
    let fixture = fixture();
    let (created, etag, create_request_id) =
        create_event(&target, &fixture, "Medication snapshot before rename");
    let event_id = created["id"].as_i64().expect("health event ID");
    assert_health_event_effect(fixture.household_id, event_id, &create_request_id, "create");
    let medication_id = fixture.managed_medication_id;
    let before_row = database()
        .query_one(
            "SELECT id, medication_name, created_at::text FROM health_event_medications WHERE household_id = $1 AND health_event_id = $2 AND medication_id = $3",
            &[&fixture.household_id, &event_id, &medication_id],
        )
        .expect("health event medication snapshot before rename");
    let before = (
        before_row.get::<_, i64>(0),
        before_row.get::<_, String>(1),
        before_row.get::<_, String>(2),
    );
    assert_eq!(before.1, fixture.managed_medication_name);

    let _restore = MedicationNameRestore::rename(
        fixture.household_id,
        medication_id,
        "snapshot regression rename",
    );
    let item = format!(
        "{}/{}",
        health_path(&fixture),
        created["portable_id"].as_str().unwrap()
    );
    let updated = target.patch_json_if_match(
        &item,
        &fixture.access_token,
        &json!({"health_event": {
            "title": "Medication snapshot after rename",
            "medication_ids": [fixture.managed_medication_portable_id]
        }}),
        &etag,
    );
    assert_eq!(updated.status().as_u16(), 200);
    let update_request_id = request_id(&updated);
    let updated: Value = body(updated);
    assert_eq!(updated["data"]["title"], "Medication snapshot after rename");
    assert_eq!(
        updated["data"]["medication_ids"],
        json!([fixture.managed_medication_id])
    );
    assert_health_event_effect(fixture.household_id, event_id, &update_request_id, "update");

    let after_row = database()
        .query_one(
            "SELECT id, medication_name, created_at::text FROM health_event_medications WHERE household_id = $1 AND health_event_id = $2 AND medication_id = $3",
            &[&fixture.household_id, &event_id, &medication_id],
        )
        .expect("health event medication snapshot after update");
    let after = (
        after_row.get::<_, i64>(0),
        after_row.get::<_, String>(1),
        after_row.get::<_, String>(2),
    );
    assert_eq!(after, before);
}

#[test]
fn health_event_person_reassignment_requires_manage_on_both_people() {
    let target = Target::from_env();
    let fixture = fixture();
    let manager = &fixture.manager_mobile_oauth_token;
    let viewer = fixture.medication_mobile_oauth_tokens["view"]
        .as_str()
        .expect("view member mobile token");
    let suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("current time")
        .as_nanos();
    let people_path = format!("/api/v1/households/{}/people", fixture.household_id);
    let create_person = |label: &str| {
        let response = target.post_json_authorized(
            &people_path,
            manager,
            &json!({"person": {
                "name": format!("Health reassignment {label} {suffix}"),
                "date_of_birth": "1980-01-01",
                "person_type": "adult",
                "has_capacity": true
            }}),
        );
        assert_eq!(response.status().as_u16(), 201);
        body(response)["data"].clone()
    };
    let old_person = create_person("old");
    let new_person = create_person("new");
    let old_id = old_person["portable_id"].as_str().expect("old portable ID");
    let new_id = new_person["portable_id"].as_str().expect("new portable ID");
    let viewer_membership_id: i64 = database()
        .query_one(
            "SELECT id FROM household_memberships WHERE household_id = $1 AND account_id = $2",
            &[&fixture.household_id, &fixture.view_account_id],
        )
        .expect("view membership")
        .get(0);
    let grants_path = format!(
        "/api/v1/households/{}/admin/person_access_grants",
        fixture.household_id
    );
    let create_grant = |person_id: i64, level: &str| {
        let response = target.post_json_authorized(
            &grants_path,
            manager,
            &json!({"person_access_grant": {
                "household_membership_id": viewer_membership_id,
                "person_id": person_id,
                "access_level": level,
                "relationship_type": "family_member"
            }}),
        );
        assert_eq!(response.status().as_u16(), 201);
        body(response)["data"]["id"].as_i64().expect("grant ID")
    };
    let revoke_grant = |grant_id: i64| {
        let response = target.delete(&format!("{grants_path}/{grant_id}"), Some(manager));
        assert_eq!(response.status().as_u16(), 204);
    };
    let old_grant_id = create_grant(old_person["id"].as_i64().unwrap(), "manage");
    let mut new_grant_id = create_grant(new_person["id"].as_i64().unwrap(), "view");
    for person_id in [old_id, new_id] {
        let response = target.get(&format!("{people_path}/{person_id}"), Some(viewer));
        assert_eq!(response.status().as_u16(), 200);
    }

    let create_for_person = |person_id: &str, title: &str| {
        let response = target.post_json_authorized(
            &health_path(&fixture),
            manager,
            &json!({"health_event": {
                "person_id": person_id,
                "event_kind": "illness",
                "severity": "mild",
                "title": title,
                "started_on": "2026-02-25"
            }}),
        );
        assert_eq!(response.status().as_u16(), 201);
        let tag = response.headers()["etag"].to_str().unwrap().to_owned();
        (body(response)["data"].clone(), tag)
    };
    let (patch_event, patch_tag) = create_for_person(old_id, "PATCH reassignment");
    let patch_path = format!(
        "{}/{}",
        health_path(&fixture),
        patch_event["portable_id"].as_str().unwrap()
    );
    let denied_patch = target.patch_json_if_match(
        &patch_path,
        viewer,
        &json!({"health_event": {"person_id": new_id}}),
        &patch_tag,
    );
    assert_error(denied_patch, 403);
    let denied_put = target.put_json_if_match(
        &patch_path,
        viewer,
        &json!({"health_event": {"person_id": new_id}}),
        &patch_tag,
    );
    assert_error(denied_put, 403);
    let unchanged = target.get(&patch_path, Some(viewer));
    assert_eq!(unchanged.headers()["etag"], patch_tag);
    assert_eq!(body(unchanged)["data"], patch_event);

    let batch_path = format!("/api/v1/households/{}/sync/batches", fixture.household_id);
    let batch_key = format!("health-reassignment-{suffix}");
    let batch_payload = json!({"batch": {"operations": [{
        "resource_type": "health_event", "action": "update",
        "id": patch_event["portable_id"], "if_match": patch_tag,
        "attributes": {"title": "Batch update under old manage grant"}
    }]}});
    let applied = target.post_json_with_key(&batch_path, viewer, &batch_key, &batch_payload);
    assert_eq!(applied.status().as_u16(), 201);
    let replay = target.post_json_with_key(&batch_path, viewer, &batch_key, &batch_payload);
    assert_eq!(replay.status().as_u16(), 201);
    assert_eq!(replay.headers()["idempotency-replayed"], "true");
    let patch_tag = target.get(&patch_path, Some(manager)).headers()["etag"]
        .to_str()
        .unwrap()
        .to_owned();
    let cursor = body(target.get(
        &format!("/api/v1/households/{}/sync/snapshot", fixture.household_id),
        Some(viewer),
    ))["data"]["cursor"]
        .as_str()
        .expect("sync cursor")
        .to_owned();
    let patched = target.patch_json_if_match(
        &patch_path,
        manager,
        &json!({"health_event": {"person_id": new_id}}),
        &patch_tag,
    );
    assert_eq!(patched.status().as_u16(), 200);
    let patch_updated_tag = patched.headers()["etag"].to_str().unwrap().to_owned();
    let patched = body(patched)["data"].clone();
    assert_health_event(&patched);
    assert_eq!(patched["person_portable_id"], new_id);
    assert_ne!(patch_updated_tag, patch_tag);
    let read = target.get(&patch_path, Some(manager));
    assert_eq!(read.status().as_u16(), 200);
    assert_eq!(read.headers()["etag"], patch_updated_tag);
    assert_eq!(body(read)["data"], patched);
    let same_person = target.patch_json_if_match(
        &patch_path,
        manager,
        &json!({"health_event": {"person_id": new_id, "title": "Same person update"}}),
        &patch_updated_tag,
    );
    assert_eq!(same_person.status().as_u16(), 200);
    let same_person = body(same_person)["data"].clone();
    assert_eq!(same_person["person_portable_id"], new_id);

    let (put_event, put_tag) = create_for_person(old_id, "PUT reassignment");
    let put_path = format!(
        "{}/{}",
        health_path(&fixture),
        put_event["portable_id"].as_str().unwrap()
    );
    let replaced = target.put_json_if_match(
        &put_path,
        manager,
        &json!({"health_event": {"person_id": new_id}}),
        &put_tag,
    );
    assert_eq!(replaced.status().as_u16(), 200);
    let put_tag = replaced.headers()["etag"].to_str().unwrap().to_owned();
    let replaced = body(replaced)["data"].clone();
    assert_health_event(&replaced);
    assert_eq!(replaced["person_portable_id"], new_id);
    let read = target.get(&put_path, Some(manager));
    assert_eq!(read.status().as_u16(), 200);
    assert_eq!(read.headers()["etag"], put_tag);
    assert_eq!(body(read)["data"], replaced);
    let feed_path = format!(
        "/api/v1/households/{}/sync/changes?cursor={cursor}",
        fixture.household_id
    );
    let feed_entries = |token: &str| {
        let response = target.get(&feed_path, Some(token));
        assert_eq!(response.status().as_u16(), 200);
        let feed = body(response);
        let matching = |kind: &str| {
            feed["data"][kind]
                .as_array()
                .unwrap()
                .iter()
                .any(|row| row["record_portable_id"] == patch_event["portable_id"])
        };
        (matching("changes"), matching("tombstones"))
    };
    assert_eq!(
        feed_entries(viewer),
        (true, false),
        "both grants see reassigned event"
    );
    revoke_grant(new_grant_id);
    assert_eq!(
        feed_entries(viewer),
        (false, true),
        "old grant sees reassignment tombstone"
    );
    new_grant_id = create_grant(new_person["id"].as_i64().unwrap(), "view");
    revoke_grant(old_grant_id);
    assert_error(
        target.post_json_with_key(&batch_path, viewer, &batch_key, &batch_payload),
        403,
    );
    assert_eq!(
        feed_entries(viewer),
        (true, false),
        "new grant sees reassigned event"
    );
    revoke_grant(new_grant_id);
    assert_eq!(
        feed_entries(viewer),
        (false, false),
        "no grant hides reassigned event"
    );

    let (denied_event, denied_tag) = create_for_person(old_id, "Hidden target denial");
    let denied_path = format!(
        "{}/{}",
        health_path(&fixture),
        denied_event["portable_id"].as_str().unwrap()
    );
    for person_id in [
        fixture.hidden_person_portable_id.as_str(),
        fixture.foreign_person_portable_id.as_str(),
    ] {
        for response in [
            target.patch_json_if_match(
                &denied_path,
                manager,
                &json!({"health_event": {"person_id": person_id}}),
                &denied_tag,
            ),
            target.put_json_if_match(
                &denied_path,
                manager,
                &json!({"health_event": {"person_id": person_id}}),
                &denied_tag,
            ),
        ] {
            assert_error(response, 404);
        }
        let read = target.get(&denied_path, Some(manager));
        assert_eq!(read.status().as_u16(), 200);
        assert_eq!(read.headers()["etag"], denied_tag);
        assert_eq!(body(read)["data"], denied_event);
    }
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
    assert_eq!(response.headers()["content-type"], "application/json");
    assert_eq!(body(response)["error"]["code"], "rate_limited");
}

#[test]
fn all_five_health_event_operations_share_the_documented_rate_limit() {
    let fixture = fixture();
    let base = env::var("CONTRACT_RATE_BASE_URL").expect("rate-limited API base URL");
    let collection = format!(
        "{}/api/v1/households/{}/health_events",
        base.trim_end_matches('/'),
        fixture.household_id
    );
    let item = format!("{collection}/{}", fixture.managed_health_event_portable_id);
    let client = Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(5))
        .build()
        .expect("health-event rate client");
    let started = Instant::now();
    let mut rejected = None;
    for _ in 0..601 {
        let response = client
            .get(&collection)
            .bearer_auth(&fixture.access_token)
            .send()
            .expect("health-event list rate request");
        if response.status().as_u16() == 429 {
            rejected = Some(response);
            break;
        }
        assert_eq!(response.status().as_u16(), 200);
    }
    assert!(started.elapsed() < Duration::from_secs(60));
    rate_limited(rejected.expect("health-event list rate limit"));

    for response in [
        client
            .post(&collection)
            .bearer_auth(&fixture.access_token)
            .json(&json!({"health_event": {
                "person_id": fixture.managed_person_portable_id,
                "event_kind": "illness", "title": "Rate no-op", "started_on": "2026-02-25"
            }}))
            .send(),
        client.get(&item).bearer_auth(&fixture.access_token).send(),
        client
            .patch(&item)
            .bearer_auth(&fixture.access_token)
            .json(&json!({"health_event": {"title": "Rate no-op"}}))
            .send(),
        client
            .put(&item)
            .bearer_auth(&fixture.access_token)
            .json(&json!({"health_event": {"title": "Rate no-op"}}))
            .send(),
    ] {
        rate_limited(response.expect("health-event operation rate request"));
    }
}
