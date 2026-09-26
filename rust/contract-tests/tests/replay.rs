use medtracker_contract_tests::{fixture, Fixture, Target};
use reqwest::blocking::Response;
use serde_json::{json, Value};
use std::sync::{Arc, Barrier};
use std::thread;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

fn body(response: Response) -> Value {
    response.json().expect("JSON response")
}

fn path(fixture: &Fixture, route: &str) -> String {
    format!("/api/v1/households/{}/{route}", fixture.household_id)
}

fn uuid(fixture: &Fixture, sequence: u8) -> String {
    format!(
        "33333333-3333-4333-8333-{:010x}{sequence:02x}",
        fixture.household_id
    )
}

fn batch_body(operation: Value) -> Value {
    json!({"batch": {"operations": [operation]}})
}

fn take_operation(source_id: &str, client_uuid: &str, medication_id: i64) -> Value {
    let taken_at = OffsetDateTime::now_utc()
        .replace_nanosecond(0)
        .unwrap()
        .format(&Rfc3339)
        .unwrap();
    json!({"resource_type": "medication_take", "action": "create", "attributes": {
        "source_type": "schedule", "source_id": source_id,
        "client_uuid": client_uuid, "taken_at": taken_at,
        "dose_amount": "1.25", "taken_from_medication_id": medication_id
    }})
}

fn fresh_source(target: &Target, fixture: &Fixture) -> (String, i64) {
    let medication = target.post_json_authorized(
        &path(fixture, "medications"),
        &fixture.access_token,
        &json!({"medication": {
            "name": format!("Replay medicine {}", OffsetDateTime::now_utc().unix_timestamp_nanos()),
            "location_id": fixture.primary_location_id,
            "dose_amount": "1.25", "dose_unit": "ml", "current_supply": "20.0"
        }}),
    );
    assert_eq!(medication.status().as_u16(), 201);
    let medication = body(medication)["data"].clone();
    let schedule = target.post_json_authorized(
        &path(fixture, "schedules"),
        &fixture.access_token,
        &json!({"schedule": {
            "person_id": fixture.managed_person_portable_id,
            "medication_id": medication["portable_id"],
            "dose_amount": "1.25", "dose_unit": "ml", "frequency": "Daily",
            "start_date": "2026-02-25", "end_date": "2099-12-31"
        }}),
    );
    assert_eq!(schedule.status().as_u16(), 201);
    (
        body(schedule)["data"]["portable_id"]
            .as_str()
            .unwrap()
            .to_owned(),
        medication["id"].as_i64().unwrap(),
    )
}

fn stock(target: &Target, fixture: &Fixture, medication_id: i64) -> f64 {
    let response = target.get(
        &path(fixture, &format!("medications/{medication_id}")),
        Some(&fixture.access_token),
    );
    assert_eq!(response.status().as_u16(), 200);
    body(response)["data"]["current_supply"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap()
}

fn take_rows(target: &Target, fixture: &Fixture) -> Vec<Value> {
    let response = target.get(
        &path(fixture, "medication_takes?per_page=100"),
        Some(&fixture.access_token),
    );
    assert_eq!(response.status().as_u16(), 200);
    body(response)["data"].as_array().unwrap().clone()
}

fn feed_cursor(target: &Target, fixture: &Fixture) -> String {
    let response = target.get(&path(fixture, "sync/snapshot"), Some(&fixture.access_token));
    assert_eq!(response.status().as_u16(), 200);
    body(response)["data"]["cursor"]
        .as_str()
        .unwrap()
        .to_owned()
}

fn take_change_count(target: &Target, fixture: &Fixture, cursor: &str, take_id: &str) -> usize {
    let response = target.get(
        &path(fixture, &format!("sync/changes?cursor={cursor}")),
        Some(&fixture.access_token),
    );
    assert_eq!(response.status().as_u16(), 200);
    body(response)["data"]["changes"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| {
            row["record_type"] == "MedicationTake" && row["record_portable_id"] == take_id
        })
        .count()
}

fn request_audit_count(target: &Target, fixture: &Fixture, request_id: &str) -> usize {
    let response = target.get(
        &path(fixture, "admin/audit_logs"),
        Some(&fixture.access_token),
    );
    assert_eq!(response.status().as_u16(), 200);
    body(response)["data"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row["request_id"] == request_id && row["event_type"] == "api.request")
        .count()
}

struct BatchResponse {
    status: u16,
    replay_header: Option<String>,
    request_id: String,
    body: Value,
}

fn concurrent_batches(
    fixture: &Fixture,
    requests: [(Value, Option<String>); 2],
) -> [BatchResponse; 2] {
    let barrier = Arc::new(Barrier::new(3));
    let route = path(fixture, "sync/batches");
    let token = fixture.access_token.clone();
    let client_ip = format!(
        "198.19.{}.{}",
        (fixture.household_id / 256) % 256,
        fixture.household_id % 256
    );
    thread::scope(|scope| {
        let workers: Vec<_> = requests
            .into_iter()
            .map(|(payload, key)| {
                let barrier = Arc::clone(&barrier);
                let route = route.clone();
                let token = token.clone();
                let client_ip = client_ip.clone();
                scope.spawn(move || {
                    let target = Target::from_env();
                    barrier.wait();
                    let response = target.post_json_from_local_client(
                        &route,
                        &token,
                        &client_ip,
                        key.as_deref(),
                        &payload,
                    );
                    let status = response.status().as_u16();
                    let replay_header = response
                        .headers()
                        .get("Idempotency-Replayed")
                        .map(|value| value.to_str().unwrap().to_owned());
                    let request_id = response.headers()["x-request-id"]
                        .to_str()
                        .unwrap()
                        .to_owned();
                    BatchResponse {
                        status,
                        replay_header,
                        request_id,
                        body: body(response),
                    }
                })
            })
            .collect();
        barrier.wait();
        workers
            .into_iter()
            .map(|worker| worker.join().expect("concurrent HTTP worker"))
            .collect::<Vec<_>>()
            .try_into()
            .unwrap_or_else(|_| panic!("two HTTP responses"))
    })
}

fn assert_one_take_effect(
    target: &Target,
    fixture: &Fixture,
    medication_id: i64,
    before_stock: f64,
    cursor: &str,
    client_uuid: &str,
    responses: &[BatchResponse; 2],
) {
    let take_id = responses[0].body["data"]["results"][0]["record_portable_id"]
        .as_str()
        .expect("portable take ID");
    assert_eq!(stock(target, fixture, medication_id), before_stock - 1.25);
    let matching_takes: Vec<_> = take_rows(target, fixture)
        .into_iter()
        .filter(|row| row["client_uuid"] == client_uuid)
        .collect();
    assert_eq!(matching_takes.len(), 1);
    assert_eq!(matching_takes[0]["portable_id"], take_id);
    assert_eq!(take_change_count(target, fixture, cursor, take_id), 1);
    for response in responses {
        assert_eq!(
            request_audit_count(target, fixture, &response.request_id),
            1
        );
    }
}

#[test]
fn client_uuid_replay_is_persistent_without_a_response_cache_key() {
    let target = Target::from_env();
    let fixture = fixture();
    let (source_id, medication_id) = fresh_source(&target, &fixture);
    let cursor = feed_cursor(&target, &fixture);
    let before = stock(&target, &fixture, medication_id);
    let payload = batch_body(take_operation(
        &source_id,
        &uuid(&fixture, 1),
        medication_id,
    ));
    let route = path(&fixture, "sync/batches");
    let first = target.post_json_authorized(&route, &fixture.access_token, &payload);
    assert_eq!(first.status().as_u16(), 201);
    assert!(first.headers().get("Idempotency-Replayed").is_none());
    let first_request_id = first.headers()["x-request-id"].to_str().unwrap().to_owned();
    let first_result = body(first)["data"]["results"][0].clone();
    assert_eq!(first_result["replayed"], false);
    assert_eq!(first_result["record_type"], "MedicationTake");
    let take_id = first_result["record_portable_id"].as_str().unwrap();
    assert_eq!(stock(&target, &fixture, medication_id), before - 1.25);
    let second = target.post_json_authorized(&route, &fixture.access_token, &payload);
    assert_eq!(second.status().as_u16(), 201);
    assert!(second.headers().get("Idempotency-Replayed").is_none());
    let second_request_id = second.headers()["x-request-id"]
        .to_str()
        .unwrap()
        .to_owned();
    let second_result = body(second)["data"]["results"][0].clone();
    assert_eq!(second_result["replayed"], true);
    assert_eq!(second_result["record_portable_id"], take_id);
    assert_eq!(second_result["etag"], first_result["etag"]);
    assert_eq!(stock(&target, &fixture, medication_id), before - 1.25);
    assert_eq!(
        take_rows(&target, &fixture)
            .iter()
            .filter(|row| row["portable_id"] == take_id)
            .count(),
        1
    );
    assert_eq!(take_change_count(&target, &fixture, &cursor, take_id), 1);
    assert_eq!(request_audit_count(&target, &fixture, &first_request_id), 1);
    assert_eq!(
        request_audit_count(&target, &fixture, &second_request_id),
        1
    );
}

#[test]
fn idempotency_key_replays_the_saved_response_and_rejects_changed_request_or_account() {
    let target = Target::from_env();
    let fixture = fixture();
    let (source_id, medication_id) = fresh_source(&target, &fixture);
    let cursor = feed_cursor(&target, &fixture);
    let before = stock(&target, &fixture, medication_id);
    let operation = take_operation(&source_id, &uuid(&fixture, 2), medication_id);
    let payload = batch_body(operation.clone());
    let key = uuid(&fixture, 3);
    let route = path(&fixture, "sync/batches");
    let first = target.post_json_with_key(&route, &fixture.access_token, &key, &payload);
    assert_eq!(first.status().as_u16(), 201);
    assert!(first.headers().get("Idempotency-Replayed").is_none());
    let first_request_id = first.headers()["x-request-id"].to_str().unwrap().to_owned();
    let saved = body(first);
    let result = &saved["data"]["results"][0];
    assert_eq!(result["replayed"], false);
    let take_id = result["record_portable_id"].as_str().unwrap();
    let second = target.post_json_with_key(&route, &fixture.access_token, &key, &payload);
    assert_eq!(second.status().as_u16(), 201);
    assert_eq!(second.headers()["Idempotency-Replayed"], "true");
    let second_request_id = second.headers()["x-request-id"]
        .to_str()
        .unwrap()
        .to_owned();
    assert_ne!(second_request_id, first_request_id);
    assert_eq!(body(second), saved);
    assert_eq!(stock(&target, &fixture, medication_id), before - 1.25);
    assert_eq!(
        take_rows(&target, &fixture)
            .iter()
            .filter(|row| row["portable_id"] == take_id)
            .count(),
        1
    );
    assert_eq!(take_change_count(&target, &fixture, &cursor, take_id), 1);
    assert_eq!(request_audit_count(&target, &fixture, &first_request_id), 1);
    assert_eq!(
        request_audit_count(&target, &fixture, &second_request_id),
        1
    );

    let changed = batch_body(
        json!({"resource_type": "medication_take", "action": "create",
        "attributes": {"source_type": "schedule", "source_id": source_id,
            "client_uuid": uuid(&fixture, 4), "taken_at": OffsetDateTime::now_utc().format(&Rfc3339).unwrap()}}),
    );
    let response = target.post_json_with_key(&route, &fixture.access_token, &key, &changed);
    assert_eq!(response.status().as_u16(), 409);
    assert_eq!(body(response)["error"]["code"], "idempotency_key_reused");
    let response =
        target.post_json_with_key(&route, &fixture.delegated_access_token, &key, &payload);
    assert_eq!(response.status().as_u16(), 409);
    assert_eq!(body(response)["error"]["code"], "idempotency_key_reused");
    assert_eq!(stock(&target, &fixture, medication_id), before - 1.25);
    assert_eq!(take_change_count(&target, &fixture, &cursor, take_id), 1);

    let persistent = target.post_json_authorized(&route, &fixture.access_token, &payload);
    assert_eq!(persistent.status().as_u16(), 201);
    assert!(persistent.headers().get("Idempotency-Replayed").is_none());
    assert_eq!(body(persistent)["data"]["results"][0]["replayed"], true);
}

#[test]
fn invisible_client_uuid_collision_does_not_disclose_the_existing_take() {
    let target = Target::from_env();
    let fixture = fixture();
    let hidden = take_rows(&target, &fixture)
        .into_iter()
        .find(|row| row["portable_id"] == fixture.hidden_take_portable_id)
        .expect("owner can inspect the hidden take");
    let hidden_uuid = hidden["client_uuid"].as_str().unwrap();
    let (source_id, medication_id) = fresh_source(&target, &fixture);
    let before = stock(&target, &fixture, medication_id);
    let response = target.post_json_authorized(
        &path(&fixture, "sync/batches"),
        &fixture.delegated_access_token,
        &batch_body(take_operation(&source_id, hidden_uuid, medication_id)),
    );
    assert_eq!(response.status().as_u16(), 409);
    let result = body(response);
    assert_eq!(result["error"]["code"], "idempotency_key_unavailable");
    let response_text = result.to_string();
    assert!(!response_text.contains(hidden_uuid));
    assert!(!response_text.contains(&fixture.hidden_take_portable_id));
    assert!(!response_text.contains(&fixture.hidden_person_portable_id));
    assert_eq!(stock(&target, &fixture, medication_id), before);
    assert_eq!(
        take_rows(&target, &fixture)
            .iter()
            .filter(|row| row["client_uuid"] == hidden_uuid)
            .count(),
        1
    );
}

#[test]
fn saved_validation_error_replays_without_creating_a_take() {
    let target = Target::from_env();
    let fixture = fixture();
    let key = uuid(&fixture, 5);
    let before = stock(&target, &fixture, fixture.managed_medication_id);
    let before_takes = take_rows(&target, &fixture).len();
    let payload = batch_body(
        json!({"resource_type": "medication_take", "action": "create",
        "attributes": {"source_type": "schedule", "source_id": fixture.managed_schedule_portable_id,
            "client_uuid": "", "taken_at": OffsetDateTime::now_utc().format(&Rfc3339).unwrap()}}),
    );
    let route = path(&fixture, "sync/batches");
    let first = target.post_json_with_key(&route, &fixture.access_token, &key, &payload);
    assert_eq!(first.status().as_u16(), 422);
    let saved = body(first);
    assert_eq!(saved["error"]["code"], "medication_take_invalid");
    assert!(saved["data"].is_null());
    let second = target.post_json_with_key(&route, &fixture.access_token, &key, &payload);
    assert_eq!(second.status().as_u16(), 422);
    assert_eq!(second.headers()["Idempotency-Replayed"], "true");
    assert_eq!(body(second), saved);
    assert_eq!(
        stock(&target, &fixture, fixture.managed_medication_id),
        before
    );
    assert_eq!(take_rows(&target, &fixture).len(), before_takes);
}

#[test]
fn concurrent_matching_request_keys_replay_one_response() {
    let target = Target::from_env();
    let fixture = fixture();
    let (source_id, medication_id) = fresh_source(&target, &fixture);
    let cursor = feed_cursor(&target, &fixture);
    let before_stock = stock(&target, &fixture, medication_id);
    let client_uuid = uuid(&fixture, 7);
    let payload = batch_body(take_operation(&source_id, &client_uuid, medication_id));
    let key = uuid(&fixture, 8);
    let responses = concurrent_batches(
        &fixture,
        [(payload.clone(), Some(key.clone())), (payload, Some(key))],
    );
    assert!(responses.iter().all(|response| response.status == 201));
    assert_eq!(
        responses
            .iter()
            .filter(|response| response.replay_header.as_deref() == Some("true"))
            .count(),
        1
    );
    assert_eq!(
        responses
            .iter()
            .filter(|response| response.replay_header.is_none())
            .count(),
        1
    );
    assert_ne!(responses[0].request_id, responses[1].request_id);
    assert_eq!(responses[0].body, responses[1].body);
    let result = &responses[0].body["data"]["results"][0];
    assert_eq!(result["replayed"], false);
    assert_one_take_effect(
        &target,
        &fixture,
        medication_id,
        before_stock,
        &cursor,
        &client_uuid,
        &responses,
    );
}

#[test]
fn concurrent_matching_client_uuids_create_one_take_without_a_shared_request_key() {
    let target = Target::from_env();
    let fixture = fixture();
    let (source_id, medication_id) = fresh_source(&target, &fixture);
    let cursor = feed_cursor(&target, &fixture);
    let before_stock = stock(&target, &fixture, medication_id);
    let client_uuid = uuid(&fixture, 9);
    let payload = batch_body(take_operation(&source_id, &client_uuid, medication_id));
    let responses = concurrent_batches(
        &fixture,
        [(payload.clone(), Some(uuid(&fixture, 10))), (payload, None)],
    );
    assert!(responses.iter().all(|response| response.status == 201));
    assert!(responses
        .iter()
        .all(|response| response.replay_header.is_none()));
    assert_ne!(responses[0].request_id, responses[1].request_id);
    let results: Vec<_> = responses
        .iter()
        .map(|response| &response.body["data"]["results"][0])
        .collect();
    assert_eq!(
        results
            .iter()
            .filter(|result| result["replayed"] == false)
            .count(),
        1
    );
    assert_eq!(
        results
            .iter()
            .filter(|result| result["replayed"] == true)
            .count(),
        1
    );
    assert_eq!(
        results[0]["record_portable_id"],
        results[1]["record_portable_id"]
    );
    assert_eq!(results[0]["etag"], results[1]["etag"]);
    assert_one_take_effect(
        &target,
        &fixture,
        medication_id,
        before_stock,
        &cursor,
        &client_uuid,
        &responses,
    );
}

#[test]
fn concurrent_stale_edits_with_distinct_keys_conflict() {
    let target = Target::from_env();
    let fixture = fixture();
    let created = target.post_json_authorized(
        &path(&fixture, "locations"),
        &fixture.access_token,
        &json!({"location": {"name": "Concurrent edit baseline"}}),
    );
    assert_eq!(created.status().as_u16(), 201);
    let id = body(created)["data"]["portable_id"]
        .as_str()
        .unwrap()
        .to_owned();
    let read = target.get(
        &path(&fixture, &format!("locations/{id}")),
        Some(&fixture.access_token),
    );
    assert_eq!(read.status().as_u16(), 200);
    let etag = read.headers()["etag"].to_str().unwrap().to_owned();
    let cursor = feed_cursor(&target, &fixture);
    let count_changes = |feed: Response| {
        assert_eq!(feed.status().as_u16(), 200);
        body(feed)["data"]["changes"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|row| row["record_type"] == "Location" && row["record_portable_id"] == id)
            .count()
    };
    let before_changes = count_changes(target.get(
        &path(&fixture, &format!("sync/changes?cursor={cursor}")),
        Some(&fixture.access_token),
    ));
    let payload = |name| {
        batch_body(
            json!({"resource_type": "location", "action": "update", "id": id,
            "if_match": etag, "attributes": {"name": name}}),
        )
    };
    let responses = concurrent_batches(
        &fixture,
        [
            (payload("Concurrent edit A"), Some(uuid(&fixture, 11))),
            (payload("Concurrent edit B"), Some(uuid(&fixture, 12))),
        ],
    );
    let winner = responses
        .iter()
        .position(|response| response.status == 201)
        .unwrap();
    let loser = 1 - winner;
    assert_eq!(responses[loser].status, 409);
    assert_eq!(responses[loser].body["error"]["code"], "sync_conflict");
    assert!(responses
        .iter()
        .all(|response| response.replay_header.is_none()));
    assert_ne!(responses[0].request_id, responses[1].request_id);
    let expected_name = if winner == 0 {
        "Concurrent edit A"
    } else {
        "Concurrent edit B"
    };
    let retained = target.get(
        &path(&fixture, &format!("locations/{id}")),
        Some(&fixture.access_token),
    );
    assert_eq!(retained.status().as_u16(), 200);
    assert_eq!(body(retained)["data"]["name"], expected_name);
    let after_changes = count_changes(target.get(
        &path(&fixture, &format!("sync/changes?cursor={cursor}")),
        Some(&fixture.access_token),
    ));
    assert_eq!(after_changes, before_changes + 1);
    assert_eq!(
        request_audit_count(&target, &fixture, &responses[winner].request_id),
        1
    );
}

#[test]
fn cached_success_rechecks_exact_grant_after_owner_revocation() {
    let target = Target::from_env();
    let fixture = fixture();
    let grants_path = path(&fixture, "admin/person_access_grants");
    let grants = target.get(&grants_path, Some(&fixture.access_token));
    assert_eq!(grants.status().as_u16(), 200);
    let grant_ids: Vec<i64> = body(grants)["data"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| {
            row["id"] == fixture.replay_grant_id
                && row["person_id"] == fixture.managed_person_id
                && row["access_level"] == "manage"
                && row["revoked_at"].is_null()
        })
        .map(|row| row["id"].as_i64().unwrap())
        .collect();
    assert_eq!(grant_ids.len(), 1, "exact delegated grant");
    let grant_id = grant_ids[0];
    let person_route = path(&fixture, &format!("people/{}", fixture.managed_person_id));
    let read = target.get(&person_route, Some(&fixture.replay_mobile_access_token));
    assert_eq!(read.status().as_u16(), 200);
    let tag = read.headers()["etag"].to_str().unwrap().to_owned();
    let person = body(read)["data"].clone();
    let payload = batch_body(json!({"resource_type": "person", "action": "update",
        "id": person["portable_id"], "if_match": tag,
        "attributes": {"name": "Replay revoked care"}}));
    let key = uuid(&fixture, 6);
    let route = path(&fixture, "sync/batches");
    let first =
        target.post_json_with_key(&route, &fixture.replay_mobile_access_token, &key, &payload);
    assert_eq!(first.status().as_u16(), 201);
    let original = body(first);
    let cached =
        target.post_json_with_key(&route, &fixture.replay_mobile_access_token, &key, &payload);
    assert_eq!(cached.status().as_u16(), 201);
    assert_eq!(cached.headers()["Idempotency-Replayed"], "true");
    assert_eq!(body(cached), original);
    let revoked = target.delete(
        &format!("{grants_path}/{grant_id}"),
        Some(&fixture.access_token),
    );
    assert_eq!(revoked.status().as_u16(), 204);
    let old_session = target.get(&path(&fixture, "me"), Some(&fixture.replay_access_token));
    assert_eq!(old_session.status().as_u16(), 401);
    let session = target.get(
        &path(&fixture, "me"),
        Some(&fixture.replay_mobile_access_token),
    );
    assert_eq!(session.status().as_u16(), 200);
    let denied =
        target.post_json_with_key(&route, &fixture.replay_mobile_access_token, &key, &payload);
    assert_eq!(denied.status().as_u16(), 403);
    let denial = body(denied);
    assert_eq!(denial["error"]["code"], "forbidden");
    assert!(denial["data"].is_null());
    assert!(!denial.to_string().contains("Replay revoked care"));
}
