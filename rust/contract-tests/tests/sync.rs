use medtracker_contract_tests::{fixture, Fixture, Target};
use reqwest::blocking::Response;
use serde_json::{json, Value};
use time::OffsetDateTime;

fn body(response: Response) -> Value {
    response.json().expect("JSON response")
}

fn snapshot_path(fixture: &Fixture, kind: &str) -> String {
    format!("/api/v1/households/{}/{}", fixture.household_id, kind)
}

fn batch(target: &Target, fixture: &Fixture, token: &str, operations: Value) -> Response {
    target.post_json_authorized(
        &snapshot_path(fixture, "sync/batches"),
        token,
        &json!({"batch": {"operations": operations}}),
    )
}

fn resource(target: &Target, fixture: &Fixture, kind: &str, id: &str) -> (Value, String) {
    let response = target.get(
        &format!("{}/{}", snapshot_path(fixture, kind), id),
        Some(&fixture.access_token),
    );
    assert_eq!(response.status().as_u16(), 200);
    let etag = response.headers()["etag"].to_str().unwrap().to_owned();
    (body(response)["data"].clone(), etag)
}

fn assert_batch_result(result: &Value, index: u64, action: &str, record_type: &str) -> String {
    assert_eq!(result["index"], index);
    assert_eq!(result["action"], action);
    assert_eq!(result["record_type"], record_type);
    let id = result["record_portable_id"]
        .as_str()
        .expect("portable result ID");
    assert_eq!(id.len(), 36, "portable result ID must be a UUID");
    assert!(id.chars().enumerate().all(|(index, character)| {
        if [8, 13, 18, 23].contains(&index) {
            character == '-'
        } else {
            character.is_ascii_hexdigit()
        }
    }));
    assert!(result["etag"].as_str().is_some_and(|tag| !tag.is_empty()));
    id.to_owned()
}

fn assert_batch_error(response: Response, status: u16, code: &str) {
    assert_eq!(response.status().as_u16(), status);
    let content_type = response.headers()["content-type"].to_str().unwrap();
    assert_eq!(content_type.split(';').next().unwrap(), "application/json");
    let request_id = response.headers()["x-request-id"]
        .to_str()
        .unwrap()
        .to_owned();
    let error = body(response)["error"].clone();
    assert_eq!(error["code"], code);
    assert!(error["message"]
        .as_str()
        .is_some_and(|text| !text.is_empty()));
    assert_eq!(error["request_id"], request_id);
}

fn sync_action_path(household_id: i64, kind: &str) -> String {
    format!("/api/v1/households/{household_id}/{kind}")
}

fn sync_action_client_ip(household_id: i64) -> String {
    format!("198.51.100.{}", 20 + household_id % 220)
}

fn sync_action_batch(
    target: &Target,
    household_id: i64,
    token: &str,
    operations: Value,
) -> Response {
    target.post_json_from_local_client(
        &sync_action_path(household_id, "sync/batches"),
        token,
        &sync_action_client_ip(household_id),
        None,
        &json!({"batch": {"operations": operations}}),
    )
}

fn sync_action_resource(
    target: &Target,
    household_id: i64,
    token: &str,
    kind: &str,
    id: &str,
) -> (Value, String) {
    let response = target.get(
        &format!("{}/{id}", sync_action_path(household_id, kind)),
        Some(token),
    );
    assert_eq!(response.status().as_u16(), 200);
    let etag = response.headers()["etag"].to_str().unwrap().to_owned();
    (body(response)["data"].clone(), etag)
}

fn sync_periods(target: &Target, fixture: &Fixture) -> Vec<Value> {
    let response = target.get(
        &sync_action_path(fixture.sync_period_household_id, "medication_pause_periods"),
        Some(&fixture.sync_period_access_token),
    );
    assert_eq!(response.status().as_u16(), 200);
    body(response)["data"].as_array().unwrap().clone()
}

fn sync_action_feed(target: &Target, household_id: i64, token: &str, cursor: &str) -> Value {
    let response = target.get(
        &format!(
            "{}?cursor={cursor}",
            sync_action_path(household_id, "sync/changes")
        ),
        Some(token),
    );
    assert_eq!(response.status().as_u16(), 200);
    body(response)["data"].clone()
}

fn sync_action_audit(target: &Target, household_id: i64, token: &str) -> Vec<Value> {
    let response = target.get(
        &sync_action_path(household_id, "admin/audit_logs"),
        Some(token),
    );
    assert_eq!(response.status().as_u16(), 200);
    body(response)["data"].as_array().unwrap().clone()
}

fn location_feed_identifiers(feed: &Value) -> (Vec<String>, Vec<String>) {
    let identifiers = |kind| {
        let mut ids: Vec<String> = feed[kind]
            .as_array()
            .expect("feed collection")
            .iter()
            .filter(|row| row["record_type"] == "Location")
            .map(|row| {
                row["record_portable_id"]
                    .as_str()
                    .expect("location portable ID")
                    .to_owned()
            })
            .collect();
        ids.sort();
        ids
    };
    (identifiers("changes"), identifiers("tombstones"))
}

#[test]
fn batch_pause_periods_create_close_replay_and_roll_back() {
    let target = Target::from_env();
    let fixture = fixture();
    let household = fixture.sync_period_household_id;
    let token = &fixture.sync_period_access_token;
    let sources = [
        (
            "schedule",
            "schedules",
            &fixture.sync_period_schedule_portable_id,
        ),
        (
            "person_medication",
            "person_medications",
            &fixture.sync_period_assignment_portable_id,
        ),
    ];
    let initial: Vec<(Value, String)> = sources
        .iter()
        .map(|(_, kind, id)| sync_action_resource(&target, household, token, kind, id))
        .collect();
    assert!(sync_periods(&target, &fixture).is_empty());
    let snapshot = target.get(&sync_action_path(household, "sync/snapshot"), Some(token));
    assert_eq!(snapshot.status().as_u16(), 200);
    let cursor = body(snapshot)["data"]["cursor"]
        .as_str()
        .unwrap()
        .to_owned();
    let create = |source_type: &str, id: &str, reason: &str, note: &str| {
        json!({"resource_type": "medication_pause_period", "action": "create", "attributes": {
            "source_type": source_type, "source_id": id, "reason": reason, "note": note
        }})
    };
    let schedule_create = create(
        "schedule",
        &fixture.sync_period_schedule_portable_id,
        "out_of_supply",
        "Delivery tomorrow",
    );
    let assignment_create = create(
        "person_medication",
        &fixture.sync_period_assignment_portable_id,
        "out_of_supply",
        "Delivery tomorrow",
    );

    assert_batch_error(
        sync_action_batch(
            &target,
            household,
            token,
            json!([create(
                "schedule",
                &fixture.sync_period_schedule_portable_id,
                "unsupported",
                "Denied"
            )]),
        ),
        422,
        "unprocessable_content",
    );
    assert_batch_error(
        sync_action_batch(
            &target,
            household,
            &fixture.sync_period_view_access_token,
            json!([schedule_create]),
        ),
        403,
        "forbidden",
    );
    for field in ["started_at", "recorded_by_membership_id"] {
        let mut forged = schedule_create.clone();
        forged["attributes"][field] = if field == "started_at" {
            json!("2026-09-08T12:00:00Z")
        } else {
            json!(fixture.sync_period_membership_id)
        };
        assert_batch_error(
            sync_action_batch(&target, household, token, json!([forged])),
            422,
            "unprocessable_content",
        );
    }
    let mut numeric_create = schedule_create.clone();
    numeric_create["attributes"]["source_id"] =
        json!(initial[0].0["id"].as_i64().unwrap().to_string());
    assert_batch_error(
        sync_action_batch(&target, household, token, json!([numeric_create])),
        404,
        "not_found",
    );
    let feed_before = sync_action_feed(&target, household, token, &cursor);
    let audit_before = sync_action_audit(&target, household, token);
    let rollback = sync_action_batch(
        &target,
        household,
        token,
        json!([schedule_create, {"resource_type": "unsupported", "action": "update",
            "id": "00000000-0000-4000-8000-000000000000", "attributes": {}}]),
    );
    assert_batch_error(rollback, 422, "sync_operation_unsupported");
    assert!(sync_periods(&target, &fixture).is_empty());
    let feed_after = sync_action_feed(&target, household, token, &cursor);
    assert_eq!(feed_after["changes"], feed_before["changes"]);
    assert_eq!(feed_after["tombstones"], feed_before["tombstones"]);
    let audit_after = sync_action_audit(&target, household, token);
    let clinical_audit_ids = |events: &[Value]| -> Vec<Value> {
        events
            .iter()
            .filter(|event| event["event_type"] != "api.request")
            .map(|event| event["id"].clone())
            .collect()
    };
    assert_eq!(
        clinical_audit_ids(&audit_after),
        clinical_audit_ids(&audit_before)
    );
    let rollback_audit: Vec<_> = audit_after
        .iter()
        .filter(|event| event["metadata"]["status"] == 422)
        .collect();
    assert!(rollback_audit
        .iter()
        .all(|event| event["event_type"] == "api.request"));
    for (index, (_, kind, id)) in sources.iter().enumerate() {
        assert_eq!(
            sync_action_resource(&target, household, token, kind, id),
            initial[index]
        );
    }

    let route = sync_action_path(household, "sync/batches");
    let client_ip = sync_action_client_ip(household);
    let create_key = format!("00000000-0000-4002-8000-{:012x}", household);
    let create_request = json!({"batch": {"operations": [schedule_create, assignment_create]}});
    let response = target.post_json_from_local_client(
        &route,
        token,
        &client_ip,
        Some(&create_key),
        &create_request,
    );
    assert_eq!(response.status().as_u16(), 201);
    let created_body = body(response);
    let results = created_body["data"]["results"].as_array().unwrap().clone();
    let cached = target.post_json_from_local_client(
        &route,
        token,
        &client_ip,
        Some(&create_key),
        &create_request,
    );
    assert_eq!(cached.status().as_u16(), 201);
    assert_eq!(cached.headers()["idempotency-replayed"], "true");
    assert_eq!(body(cached), created_body);
    assert_eq!(results.len(), 2);
    let mut period_ids = Vec::new();
    let mut period_etags = Vec::new();
    for (index, (source_type, kind, id)) in sources.iter().enumerate() {
        let period_id = assert_batch_result(
            &results[index],
            index as u64,
            "create",
            "MedicationPausePeriod",
        );
        assert_eq!(results[index]["replayed"], false);
        let (source, _) = sync_action_resource(&target, household, token, kind, id);
        assert_eq!(source["active"], false);
        let period = &source["current_pause_period"];
        assert_eq!(period["portable_id"], period_id);
        assert_eq!(period["source_type"], *source_type);
        assert_eq!(period["source_id"], **id);
        assert_eq!(period["reason"], "out_of_supply");
        assert_eq!(period["note"], "Delivery tomorrow");
        assert_eq!(period["legacy_context"], false);
        assert_eq!(
            period["recorded_by_membership_id"],
            fixture.sync_period_membership_id.to_string()
        );
        assert!(period["started_at"].is_string());
        assert!(period["ended_at"].is_null());
        period_ids.push(period_id);
        period_etags.push(results[index]["etag"].as_str().unwrap().to_owned());
    }
    assert_eq!(sync_periods(&target, &fixture).len(), 2);

    let repeat = sync_action_batch(
        &target,
        household,
        token,
        json!([create(
            "schedule",
            &fixture.sync_period_schedule_portable_id,
            "other",
            "Replacement"
        )]),
    );
    assert_eq!(repeat.status().as_u16(), 201);
    let repeated = body(repeat)["data"]["results"][0].clone();
    assert_eq!(repeated["replayed"], true);
    assert_eq!(repeated["record_portable_id"], period_ids[0]);
    assert_eq!(sync_periods(&target, &fixture).len(), 2);

    let close: Vec<Value> = period_ids
        .iter()
        .zip(period_etags.iter())
        .map(|(id, etag)| {
            json!({"resource_type": "medication_pause_period", "action": "close", "id": id,
                "if_match": etag, "attributes": {}})
        })
        .collect();
    let without_version = json!([{"resource_type": "medication_pause_period", "action": "close",
        "id": period_ids[0], "attributes": {}}]);
    assert_batch_error(
        sync_action_batch(&target, household, token, without_version),
        428,
        "precondition_required",
    );
    assert_batch_error(
        sync_action_batch(
            &target,
            household,
            token,
            json!([{"resource_type": "medication_pause_period", "action": "close",
                "id": period_ids[0], "if_match": "\"stale\"", "attributes": {}}]),
        ),
        409,
        "sync_conflict",
    );
    assert_batch_error(
        sync_action_batch(
            &target,
            household,
            token,
            json!([{"resource_type": "medication_pause_period", "action": "close",
                "id": period_ids[0], "if_match": period_etags[0],
                "attributes": {"reason": "other"}}]),
        ),
        422,
        "unprocessable_content",
    );
    assert_eq!(sync_periods(&target, &fixture).len(), 2);
    let close_key = format!("00000000-0000-4003-8000-{:012x}", household);
    let close_request = json!({"batch": {"operations": close}});
    let response = target.post_json_from_local_client(
        &route,
        token,
        &client_ip,
        Some(&close_key),
        &close_request,
    );
    assert_eq!(response.status().as_u16(), 201);
    let closed_body = body(response);
    let closed = closed_body["data"]["results"].as_array().unwrap().clone();
    let cached = target.post_json_from_local_client(
        &route,
        token,
        &client_ip,
        Some(&close_key),
        &close_request,
    );
    assert_eq!(cached.status().as_u16(), 201);
    assert_eq!(cached.headers()["idempotency-replayed"], "true");
    assert_eq!(body(cached), closed_body);
    for (index, (_, kind, id)) in sources.iter().enumerate() {
        assert_eq!(closed[index]["action"], "close");
        assert_eq!(closed[index]["record_portable_id"], period_ids[index]);
        assert_eq!(closed[index]["replayed"], false);
        let (source, _) = sync_action_resource(&target, household, token, kind, id);
        assert_eq!(source["active"], true);
        assert!(source["current_pause_period"].is_null());
    }
    let periods = sync_periods(&target, &fixture);
    assert_eq!(periods.len(), 2);
    for period in &periods {
        assert!(period["ended_at"].is_string());
        assert_eq!(
            period["resumed_by_membership_id"],
            fixture.sync_period_membership_id.to_string()
        );
        assert_eq!(period["note"], "Delivery tomorrow");
    }

    let newer = sync_action_batch(
        &target,
        household,
        token,
        json!([create(
            "schedule",
            &fixture.sync_period_schedule_portable_id,
            "other",
            "Later pause"
        )]),
    );
    assert_eq!(newer.status().as_u16(), 201);
    let newer_id = body(newer)["data"]["results"][0]["record_portable_id"].clone();
    assert_ne!(newer_id, period_ids[0]);
    let response = sync_action_batch(
        &target,
        household,
        token,
        json!([{"resource_type": "medication_pause_period", "action": "close",
            "id": period_ids[0], "if_match": closed[0]["etag"], "attributes": {}}]),
    );
    assert_eq!(response.status().as_u16(), 201);
    assert_eq!(body(response)["data"]["results"][0]["replayed"], true);
    let (source, _) = sync_action_resource(
        &target,
        household,
        token,
        "schedules",
        &fixture.sync_period_schedule_portable_id,
    );
    assert_eq!(source["current_pause_period"]["portable_id"], newer_id);
    assert_eq!(source["active"], false);
}

#[test]
fn batch_assignment_actions_pause_resume_and_reorder_with_current_versions() {
    let target = Target::from_env();
    let fixture = fixture();
    let household = fixture.sync_action_household_id;
    let token = &fixture.sync_action_access_token;
    let sources = [
        (
            "schedule",
            "schedules",
            "Schedule",
            &fixture.sync_action_source_schedule_portable_id,
        ),
        (
            "person_medication",
            "person_medications",
            "PersonMedication",
            &fixture.sync_action_source_assignment_portable_id,
        ),
    ];
    let initial: Vec<(Value, String)> = sources
        .iter()
        .map(|(_, kind, _, id)| sync_action_resource(&target, household, token, kind, id))
        .collect();
    let pause = |source_type: &str, id: &str, etag: &str| {
        json!({"resource_type": source_type, "action": "pause", "id": id,
            "if_match": etag, "attributes": {"reason": "clinician_advice", "note": "Queued pause"}})
    };
    let schedule_pause = pause(
        "schedule",
        &fixture.sync_action_source_schedule_portable_id,
        &initial[0].1,
    );
    let assignment_pause = pause(
        "person_medication",
        &fixture.sync_action_source_assignment_portable_id,
        &initial[1].1,
    );
    assert_batch_error(
        sync_action_batch(
            &target,
            household,
            token,
            json!([{"resource_type": "schedule", "action": "pause",
                "id": fixture.sync_action_source_schedule_portable_id,
                "attributes": {"reason": "clinician_advice"}}]),
        ),
        428,
        "precondition_required",
    );
    assert_batch_error(
        sync_action_batch(
            &target,
            household,
            token,
            json!([pause(
                "schedule",
                &fixture.sync_action_source_schedule_portable_id,
                "\"stale\""
            )]),
        ),
        409,
        "sync_conflict",
    );
    assert_batch_error(
        sync_action_batch(
            &target,
            household,
            &fixture.sync_action_view_access_token,
            json!([schedule_pause]),
        ),
        403,
        "forbidden",
    );
    for (index, (_, kind, _, id)) in sources.iter().enumerate() {
        assert_eq!(
            sync_action_resource(&target, household, token, kind, id),
            initial[index]
        );
    }

    let key = format!("00000000-0000-4000-8000-{:012x}", household);
    let route = sync_action_path(household, "sync/batches");
    let client_ip = sync_action_client_ip(household);
    let request = json!({"batch": {"operations": [schedule_pause, assignment_pause]}});
    let first = target.post_json_from_local_client(&route, token, &client_ip, Some(&key), &request);
    assert_eq!(first.status().as_u16(), 201);
    let original = body(first);
    let results = original["data"]["results"].as_array().unwrap();
    assert_eq!(results.len(), 2);
    for (index, (_, kind, record_type, id)) in sources.iter().enumerate() {
        assert_eq!(
            assert_batch_result(&results[index], index as u64, "pause", record_type),
            **id
        );
        let (source, tag) = sync_action_resource(&target, household, token, kind, id);
        assert_eq!(results[index]["etag"], tag);
        assert_eq!(source["active"], false);
        assert_eq!(source["current_pause_period"]["reason"], "clinician_advice");
        assert_eq!(source["current_pause_period"]["note"], "Queued pause");
        assert_eq!(source["current_pause_period"]["legacy_context"], false);
    }
    let repeated =
        target.post_json_from_local_client(&route, token, &client_ip, Some(&key), &request);
    assert_eq!(repeated.status().as_u16(), 201);
    assert_eq!(repeated.headers()["idempotency-replayed"], "true");
    assert_eq!(body(repeated), original);

    let resume: Vec<Value> = sources
        .iter()
        .map(|(source_type, kind, _, id)| {
            let (_, tag) = sync_action_resource(&target, household, token, kind, id);
            json!({"resource_type": source_type, "action": "resume", "id": id,
                "if_match": tag, "attributes": {}})
        })
        .collect();
    let response = sync_action_batch(&target, household, token, json!(resume));
    assert_eq!(response.status().as_u16(), 201);
    let resumed = body(response)["data"]["results"]
        .as_array()
        .unwrap()
        .clone();
    for (index, (_, kind, record_type, id)) in sources.iter().enumerate() {
        assert_eq!(
            assert_batch_result(&resumed[index], index as u64, "resume", record_type),
            **id
        );
        let (source, tag) = sync_action_resource(&target, household, token, kind, id);
        assert_eq!(resumed[index]["etag"], tag);
        assert_eq!(source["active"], true);
        assert!(source["current_pause_period"].is_null());
    }

    let first_id = &fixture.sync_action_reorder_first_portable_id;
    let second_id = &fixture.sync_action_reorder_second_portable_id;
    let (first_before, _) =
        sync_action_resource(&target, household, token, "person_medications", first_id);
    let (second_before, second_tag) =
        sync_action_resource(&target, household, token, "person_medications", second_id);
    assert_eq!(
        second_before["position"].as_i64(),
        first_before["position"]
            .as_i64()
            .map(|position| position + 1)
    );
    let reorder = json!({"batch": {"operations": [{"resource_type": "person_medication",
        "action": "reorder", "id": second_id, "if_match": second_tag,
        "attributes": {"direction": "up"}}]}});
    let key = format!("00000000-0000-4001-8000-{:012x}", household);
    let first = target.post_json_from_local_client(&route, token, &client_ip, Some(&key), &reorder);
    assert_eq!(first.status().as_u16(), 201);
    let first_body = body(first);
    assert_eq!(
        assert_batch_result(
            &first_body["data"]["results"][0],
            0,
            "reorder",
            "PersonMedication"
        ),
        *second_id
    );
    let (first_after, _) =
        sync_action_resource(&target, household, token, "person_medications", first_id);
    let (second_after, _) =
        sync_action_resource(&target, household, token, "person_medications", second_id);
    assert_eq!(first_after["position"], second_before["position"]);
    assert_eq!(second_after["position"], first_before["position"]);
    let replay =
        target.post_json_from_local_client(&route, token, &client_ip, Some(&key), &reorder);
    assert_eq!(replay.status().as_u16(), 201);
    assert_eq!(replay.headers()["idempotency-replayed"], "true");
    assert_eq!(body(replay), first_body);
    let (_, current_tag) =
        sync_action_resource(&target, household, token, "person_medications", second_id);
    assert_batch_error(
        sync_action_batch(
            &target,
            household,
            token,
            json!([{"resource_type": "person_medication", "action": "reorder",
                "id": second_id, "if_match": current_tag,
                "attributes": {"direction": "sideways"}}]),
        ),
        422,
        "unprocessable_content",
    );
    let (first_retained, _) =
        sync_action_resource(&target, household, token, "person_medications", first_id);
    let (second_retained, _) =
        sync_action_resource(&target, household, token, "person_medications", second_id);
    assert_eq!(first_retained["position"], first_after["position"]);
    assert_eq!(second_retained["position"], second_after["position"]);
}

#[test]
fn batch_applies_indexed_care_creates_with_portable_results() {
    let target = Target::from_env();
    let fixture = fixture();
    let response = batch(
        &target,
        &fixture,
        &fixture.access_token,
        json!([
            {"resource_type": "location", "action": "create", "attributes": {"name": "Batch location"}},
            {"resource_type": "health_event", "action": "create", "attributes": {
                "person_id": fixture.managed_person_portable_id,
                "title": "Batch event", "event_kind": "illness", "started_on": "2026-02-25"
            }}
        ]),
    );
    assert_eq!(response.status().as_u16(), 201);
    let data = body(response)["data"].clone();
    assert_eq!(data["applied"], true);
    let results = data["results"].as_array().unwrap();
    assert_eq!(results.len(), 2);
    let location_id = assert_batch_result(&results[0], 0, "create", "Location");
    let event_id = assert_batch_result(&results[1], 1, "create", "HealthEvent");
    let (location, location_etag) = resource(&target, &fixture, "locations", &location_id);
    assert_eq!(location["name"], "Batch location");
    assert_eq!(results[0]["etag"], location_etag);
    let (event, event_etag) = resource(&target, &fixture, "health_events", &event_id);
    assert_eq!(event["title"], "Batch event");
    assert_eq!(results[1]["etag"], event_etag);
}

#[test]
fn batch_updates_and_deletes_care_records_with_operation_etags() {
    let target = Target::from_env();
    let fixture = fixture();
    let created = target.post_json_authorized(
        &snapshot_path(&fixture, "locations"),
        &fixture.access_token,
        &json!({"location": {"name": "Batch lifecycle"}}),
    );
    assert_eq!(created.status().as_u16(), 201);
    let id = body(created)["data"]["portable_id"]
        .as_str()
        .unwrap()
        .to_owned();
    let (_, original_etag) = resource(&target, &fixture, "locations", &id);
    let response = batch(
        &target,
        &fixture,
        &fixture.access_token,
        json!([{"resource_type": "location", "action": "update", "id": id,
            "if_match": original_etag, "attributes": {"description": "Updated offline"}}]),
    );
    assert_eq!(response.status().as_u16(), 201);
    let result = body(response)["data"]["results"][0].clone();
    assert_eq!(assert_batch_result(&result, 0, "update", "Location"), id);
    assert_ne!(result["etag"], original_etag);
    let (updated, current_etag) = resource(&target, &fixture, "locations", &id);
    assert_eq!(updated["description"], "Updated offline");
    assert_eq!(result["etag"], current_etag);
    let response = batch(
        &target,
        &fixture,
        &fixture.access_token,
        json!([{"resource_type": "location", "action": "delete", "id": id,
            "if_match": current_etag, "attributes": {}}]),
    );
    assert_eq!(response.status().as_u16(), 201);
    let deleted = body(response)["data"]["results"][0].clone();
    assert_eq!(deleted["index"], 0);
    assert_eq!(deleted["action"], "delete");
    assert_eq!(deleted["record_type"], "Location");
    assert_eq!(deleted["record_portable_id"], id);
    assert!(deleted.get("etag").is_none());
    assert_eq!(
        target
            .get(
                &format!("{}/{}", snapshot_path(&fixture, "locations"), id),
                Some(&fixture.access_token)
            )
            .status()
            .as_u16(),
        404
    );
}

#[test]
fn batch_validates_actions_versions_and_current_permissions() {
    let target = Target::from_env();
    let fixture = fixture();
    let (_, etag) = resource(
        &target,
        &fixture,
        "medications",
        &fixture.managed_medication_portable_id,
    );
    let mutation = json!({"resource_type": "medication", "action": "adjust_inventory",
        "id": fixture.managed_medication_portable_id,
        "attributes": {"new_quantity": "18", "reason": "Counted offline"}});
    let response = batch(&target, &fixture, &fixture.access_token, json!([mutation]));
    assert_eq!(response.status().as_u16(), 428);
    assert_eq!(body(response)["error"]["code"], "precondition_required");
    let response = batch(
        &target,
        &fixture,
        &fixture.access_token,
        json!([{ "resource_type": "medication", "action": "adjust_inventory",
            "id": fixture.managed_medication_portable_id, "if_match": "\"stale\"",
            "attributes": {"new_quantity": "18", "reason": "Counted offline"}}]),
    );
    assert_eq!(response.status().as_u16(), 409);
    assert_eq!(body(response)["error"]["code"], "sync_conflict");
    let response = batch(
        &target,
        &fixture,
        &fixture.access_token,
        json!([{"resource_type": "medication", "action": "adjust_inventory",
            "id": fixture.managed_medication_portable_id, "if_match": etag,
            "attributes": {"new_quantity": "invalid", "reason": "Counted offline"}}]),
    );
    assert_batch_error(response, 422, "unprocessable_content");
    let response = batch(
        &target,
        &fixture,
        &fixture.access_token,
        json!([{"resource_type": "location", "action": "reorder",
            "id": fixture.primary_location_portable_id, "attributes": {}}]),
    );
    assert_batch_error(response, 422, "sync_operation_unsupported");
    let response = batch(
        &target,
        &fixture,
        &fixture.view_access_token,
        json!([{"resource_type": "location", "action": "create", "attributes": {"name": "Denied batch"}}]),
    );
    assert_batch_error(response, 403, "forbidden");
    let response = batch(
        &target,
        &fixture,
        &fixture.view_access_token,
        json!([{"resource_type": "person", "action": "update",
            "id": fixture.hidden_person_portable_id, "if_match": "\"stale\"",
            "attributes": {"name": "Hidden batch"}}]),
    );
    assert_batch_error(response, 404, "not_found");
}

#[test]
fn batch_inventory_action_updates_stock_and_order_status() {
    let target = Target::from_env();
    let fixture = fixture();
    let id = &fixture.managed_medication_portable_id;
    let (_, etag) = resource(&target, &fixture, "medications", id);
    let response = batch(
        &target,
        &fixture,
        &fixture.access_token,
        json!([{"resource_type": "medication", "action": "adjust_inventory", "id": id,
            "if_match": etag, "attributes": {"new_quantity": "18.5", "reason": "Counted offline"}}]),
    );
    assert_eq!(response.status().as_u16(), 201);
    let result = body(response)["data"]["results"][0].clone();
    assert_eq!(
        assert_batch_result(&result, 0, "adjust_inventory", "Medication"),
        *id
    );
    let (adjusted, next_etag) = resource(&target, &fixture, "medications", id);
    assert_eq!(adjusted["current_supply"], "18.5");
    assert_eq!(result["etag"], next_etag);
    let response = batch(
        &target,
        &fixture,
        &fixture.access_token,
        json!([{"resource_type": "medication", "action": "mark_as_ordered", "id": id,
            "if_match": next_etag, "attributes": {"supplier": "Batch pharmacy", "quantity": "20"}}]),
    );
    assert_eq!(response.status().as_u16(), 201);
    let result = body(response)["data"]["results"][0].clone();
    assert_eq!(
        assert_batch_result(&result, 0, "mark_as_ordered", "Medication"),
        *id
    );
    let (ordered, current_etag) = resource(&target, &fixture, "medications", id);
    assert_eq!(ordered["reorder_status"], "ordered");
    assert_eq!(result["etag"], current_etag);
}

#[test]
fn batch_rolls_back_stock_and_feed_after_a_late_invalid_operation() {
    let target = Target::from_env();
    let fixture = fixture();
    let id = &fixture.managed_medication_portable_id;
    let (before, etag) = resource(&target, &fixture, "medications", id);
    let snapshot = body(target.get(
        &snapshot_path(&fixture, "sync/snapshot"),
        Some(&fixture.access_token),
    ));
    let cursor = snapshot["data"]["cursor"].as_str().unwrap();
    let before_feed = changes(&target, &fixture, &fixture.access_token, cursor);
    let before_changes = before_feed["changes"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row["record_portable_id"] == *id)
        .count();
    let before_tombstones = before_feed["tombstones"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row["record_portable_id"] == *id)
        .count();
    let response = batch(
        &target,
        &fixture,
        &fixture.access_token,
        json!([
            {"resource_type": "medication", "action": "adjust_inventory", "id": id,
                "if_match": etag, "attributes": {"new_quantity": "17", "reason": "Counted offline"}},
            {"resource_type": "medication", "action": "remove_stock", "id": id,
                "attributes": {"quantity": "1e1", "reason": "dropped"}}
        ]),
    );
    assert_eq!(response.status().as_u16(), 422);
    let request_id = response.headers()["x-request-id"]
        .to_str()
        .unwrap()
        .to_owned();
    let (after, current_etag) = resource(&target, &fixture, "medications", id);
    assert_eq!(after["current_supply"], before["current_supply"]);
    assert_eq!(current_etag, etag);
    let feed = changes(&target, &fixture, &fixture.access_token, cursor);
    assert_eq!(
        feed["changes"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|row| row["record_portable_id"] == *id)
            .count(),
        before_changes
    );
    assert_eq!(
        feed["tombstones"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|row| row["record_portable_id"] == *id)
            .count(),
        before_tombstones
    );
    let audit = body(target.get(
        &snapshot_path(&fixture, "admin/audit_logs"),
        Some(&fixture.access_token),
    ));
    let related: Vec<_> = audit["data"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|event| event["request_id"] == request_id)
        .collect();
    assert_eq!(related.len(), 1);
    assert_eq!(related[0]["event_type"], "api.request");
    assert_eq!(related[0]["metadata"]["status"], 422);
}

#[test]
fn batch_rolls_back_created_resource_after_a_late_stale_operation() {
    let target = Target::from_env();
    let fixture = fixture();
    let id = &fixture.managed_medication_portable_id;
    let (before, etag) = resource(&target, &fixture, "medications", id);
    let snapshot = body(target.get(
        &snapshot_path(&fixture, "sync/snapshot"),
        Some(&fixture.access_token),
    ));
    let cursor = snapshot["data"]["cursor"].as_str().unwrap();
    let before_feed = changes(&target, &fixture, &fixture.access_token, cursor);
    let before_location_identifiers = location_feed_identifiers(&before_feed);
    let response = batch(
        &target,
        &fixture,
        &fixture.access_token,
        json!([
            {"resource_type": "location", "action": "create", "attributes": {"name": "Discarded batch location"}},
            {"resource_type": "medication", "action": "update", "id": id,
                "if_match": "\"stale\"", "attributes": {"description": "Must not persist"}}
        ]),
    );
    assert_eq!(response.status().as_u16(), 409);
    let request_id = response.headers()["x-request-id"]
        .to_str()
        .unwrap()
        .to_owned();
    let (after, current_etag) = resource(&target, &fixture, "medications", id);
    assert_eq!(after["description"], before["description"]);
    assert_eq!(current_etag, etag);
    let locations = body(target.get(
        &snapshot_path(&fixture, "locations"),
        Some(&fixture.access_token),
    ));
    assert!(locations["data"]
        .as_array()
        .unwrap()
        .iter()
        .all(|row| row["name"] != "Discarded batch location"));
    let feed = changes(&target, &fixture, &fixture.access_token, cursor);
    assert_eq!(
        location_feed_identifiers(&feed),
        before_location_identifiers
    );
    let audit = body(target.get(
        &snapshot_path(&fixture, "admin/audit_logs"),
        Some(&fixture.access_token),
    ));
    let related: Vec<_> = audit["data"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|event| event["request_id"] == request_id)
        .collect();
    assert_eq!(related.len(), 1);
    assert_eq!(related[0]["event_type"], "api.request");
    assert_eq!(related[0]["metadata"]["status"], 409);
}

#[test]
fn location_feed_identifiers_detect_an_event_without_a_record_body() {
    let before = json!({"changes": [], "tombstones": []});
    let leaked_change = json!({"changes": [{
        "record_type": "Location", "record_portable_id": "11111111-1111-4111-8111-111111111111",
        "action": "create", "metadata": {}
    }], "tombstones": []});
    let leaked_tombstone = json!({"changes": [], "tombstones": [{
        "record_type": "Location", "record_portable_id": "22222222-2222-4222-8222-222222222222",
        "action": "delete", "metadata": {}
    }]});
    assert!(leaked_change["changes"]
        .as_array()
        .unwrap()
        .iter()
        .all(|row| row["record"]["name"] != "Discarded batch location"));
    assert_eq!(location_feed_identifiers(&before), (vec![], vec![]));
    assert_eq!(
        location_feed_identifiers(&leaked_change),
        (
            vec!["11111111-1111-4111-8111-111111111111".to_owned()],
            vec![]
        )
    );
    assert_eq!(
        location_feed_identifiers(&leaked_tombstone),
        (
            vec![],
            vec!["22222222-2222-4222-8222-222222222222".to_owned()]
        )
    );
}

fn changes(target: &Target, fixture: &Fixture, token: &str, cursor: &str) -> Value {
    let path = format!("{}?cursor={cursor}", snapshot_path(fixture, "sync/changes"));
    let response = target.get(&path, Some(token));
    assert_eq!(response.status().as_u16(), 200);
    body(response)["data"].clone()
}

fn row_for_portable_id<'a>(rows: &'a Value, portable_id: &str) -> Option<&'a Value> {
    rows.as_array()
        .expect("feed collection")
        .iter()
        .find(|row| row["record_portable_id"] == portable_id)
}

fn contains_identifier(value: &Value, portable_id: &str) -> bool {
    match value {
        Value::String(text) => text.contains(portable_id),
        Value::Array(rows) => rows.iter().any(|row| contains_identifier(row, portable_id)),
        Value::Object(fields) => fields.iter().any(|(key, field)| {
            key.contains(portable_id) || contains_identifier(field, portable_id)
        }),
        _ => false,
    }
}

fn records(snapshot: &Value) -> &Value {
    &snapshot["data"]["records"]
}

fn has_portable_id(rows: &Value, portable_id: &str) -> bool {
    rows.as_array()
        .expect("record collection")
        .iter()
        .any(|row| row["portable_id"] == portable_id)
}

fn assert_portable_records(collections: &Value, require_nonempty: bool) {
    for category in [
        "people",
        "locations",
        "medications",
        "dosage_options",
        "schedules",
        "person_medications",
        "medication_takes",
        "notification_preferences",
        "health_events",
    ] {
        let rows = collections[category].as_array().expect(category);
        if require_nonempty {
            assert!(!rows.is_empty(), "{category} must have a fixture record");
        }
        for row in rows {
            assert!(
                row["portable_id"].as_str().is_some_and(|id| !id.is_empty()),
                "{category}"
            );
            assert!(
                row["etag"].as_str().is_some_and(|tag| !tag.is_empty()),
                "{category}"
            );
            assert!(row.get("id").is_none(), "numeric ID leaked in {category}");
        }
    }
}

fn assert_hidden_and_foreign_records_absent(collections: &Value, fixture: &Fixture) {
    for (category, portable_ids) in [
        (
            "people",
            [
                &fixture.hidden_person_portable_id,
                &fixture.foreign_person_portable_id,
            ],
        ),
        (
            "locations",
            [
                &fixture.hidden_location_portable_id,
                &fixture.foreign_location_portable_id,
            ],
        ),
        (
            "medications",
            [
                &fixture.hidden_medication_portable_id,
                &fixture.foreign_medication_portable_id,
            ],
        ),
        (
            "dosage_options",
            [
                &fixture.hidden_dosage_portable_id,
                &fixture.foreign_dosage_portable_id,
            ],
        ),
        (
            "schedules",
            [
                &fixture.hidden_schedule_portable_id,
                &fixture.foreign_schedule_portable_id,
            ],
        ),
        (
            "person_medications",
            [
                &fixture.hidden_assignment_portable_id,
                &fixture.foreign_assignment_portable_id,
            ],
        ),
        (
            "medication_takes",
            [
                &fixture.hidden_take_portable_id,
                &fixture.foreign_take_portable_id,
            ],
        ),
        (
            "notification_preferences",
            [
                &fixture.hidden_preference_portable_id,
                &fixture.foreign_preference_portable_id,
            ],
        ),
        (
            "health_events",
            [
                &fixture.hidden_health_event_portable_id,
                &fixture.foreign_health_event_portable_id,
            ],
        ),
    ] {
        for id in portable_ids {
            assert!(
                !has_portable_id(&collections[category], id),
                "{category} leaked {id}"
            );
        }
    }
    if let Some(rows) = collections.get("medication_pause_periods") {
        for id in [
            &fixture.hidden_pause_period_id,
            &fixture.foreign_pause_period_id,
        ] {
            assert!(!has_portable_id(rows, id));
        }
    }
    for (category, rows) in collections.as_object().unwrap() {
        for row in rows.as_array().unwrap() {
            for source in [
                &fixture.hidden_schedule_portable_id,
                &fixture.foreign_schedule_portable_id,
                &fixture.hidden_assignment_portable_id,
                &fixture.foreign_assignment_portable_id,
            ] {
                assert_ne!(
                    row["source_portable_id"],
                    source.as_str(),
                    "{category} leaked source {source}"
                );
            }
        }
    }
}

#[test]
fn sync_v2_uses_current_view_scope_and_exposes_portable_clinical_relations() {
    let target = Target::from_env();
    let fixture = fixture();
    let response = target.get(
        &snapshot_path(&fixture, "sync/snapshot"),
        Some(&fixture.view_access_token),
    );
    assert_eq!(response.status().as_u16(), 200);
    let snapshot = body(response);
    let data = &snapshot["data"];
    assert_eq!(data["format"], "medtracker.portable.v2");
    assert_eq!(data["scope"], "single_person");
    assert!(data["cursor"]
        .as_str()
        .is_some_and(|cursor| !cursor.is_empty()));
    let collections = records(&snapshot);
    assert_portable_records(collections, true);
    assert_hidden_and_foreign_records_absent(collections, &fixture);
    assert!(has_portable_id(
        &collections["people"],
        &fixture.managed_person_portable_id
    ));
    assert!(!has_portable_id(
        &collections["people"],
        &fixture.hidden_person_portable_id
    ));
    assert!(!has_portable_id(
        &collections["people"],
        &fixture.foreign_person_portable_id
    ));
    assert!(has_portable_id(
        &collections["health_events"],
        &fixture.managed_health_event_portable_id
    ));
    assert!(!has_portable_id(
        &collections["health_events"],
        &fixture.hidden_health_event_portable_id
    ));
    assert!(!has_portable_id(
        &collections["health_events"],
        &fixture.foreign_health_event_portable_id
    ));
    assert!(has_portable_id(
        &collections["medications"],
        &fixture.managed_medication_portable_id
    ));
    assert!(has_portable_id(
        &collections["dosage_options"],
        &fixture.managed_dosage_portable_id
    ));
    assert!(!has_portable_id(
        &collections["medications"],
        &fixture.hidden_medication_portable_id
    ));
    assert!(!has_portable_id(
        &collections["medications"],
        &fixture.foreign_medication_portable_id
    ));
    assert!(has_portable_id(
        &collections["dose_occurrences"],
        &fixture.managed_occurrence_portable_id
    ));
    let occurrence = collections["dose_occurrences"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["portable_id"] == fixture.managed_occurrence_portable_id)
        .unwrap();
    assert_eq!(occurrence["source_type"], "schedule");
    assert_eq!(
        occurrence["source_portable_id"],
        fixture.historical_schedule_portable_id
    );
    assert!(occurrence["etag"]
        .as_str()
        .is_some_and(|tag| !tag.is_empty()));
    let period = collections["medication_pause_periods"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["portable_id"] == fixture.retired_assignment_period_id)
        .unwrap();
    assert_eq!(period["source_type"], "person_medication");
    assert_eq!(
        period["source_portable_id"],
        fixture.retired_assignment_portable_id
    );
    assert!(period["etag"].as_str().is_some_and(|tag| !tag.is_empty()));
    let schedule = collections["schedules"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["portable_id"] == fixture.managed_schedule_portable_id)
        .unwrap();
    assert_eq!(
        schedule["person_portable_id"],
        fixture.managed_person_portable_id
    );
    assert_eq!(
        schedule["medication_portable_id"],
        fixture.managed_medication_portable_id
    );
    let medication = collections["medications"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["portable_id"] == fixture.managed_medication_portable_id)
        .unwrap();
    assert_eq!(
        medication["location_portable_id"],
        fixture.primary_location_portable_id
    );
    let dosage = collections["dosage_options"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["portable_id"] == fixture.managed_dosage_portable_id)
        .unwrap();
    assert_eq!(
        dosage["medication_portable_id"],
        fixture.managed_medication_portable_id
    );
    let event = collections["health_events"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["portable_id"] == fixture.managed_health_event_portable_id)
        .unwrap();
    assert_eq!(
        event["medication_portable_ids"],
        serde_json::json!([fixture.managed_medication_portable_id])
    );
    let take = collections["medication_takes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["portable_id"] == fixture.managed_take_portable_id)
        .unwrap();
    assert_eq!(
        take["source_portable_id"],
        fixture.historical_schedule_portable_id
    );
    assert_eq!(
        take["taken_from_medication_portable_id"],
        fixture.historical_medication_portable_id
    );
    assert_eq!(
        take["taken_from_location_portable_id"],
        fixture.historical_location_portable_id
    );
    let preference = collections["notification_preferences"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["portable_id"] == fixture.managed_preference_portable_id)
        .unwrap();
    assert_eq!(
        preference["person_portable_id"],
        fixture.managed_person_portable_id
    );
    assert!(collections.get("api_sessions").is_none());
    assert!(collections.get("security_audit_events").is_none());
}

#[test]
fn mobile_v1_uses_manage_scope_and_records_one_correlated_read_audit() {
    let target = Target::from_env();
    let fixture = fixture();
    let path = snapshot_path(&fixture, "mobile_snapshot");
    let response = target.get(&path, Some(&fixture.view_access_token));
    assert_eq!(response.status().as_u16(), 200);
    let request_id = response.headers()["x-request-id"]
        .to_str()
        .unwrap()
        .to_owned();
    let snapshot = body(response);
    let data = &snapshot["data"];
    assert_eq!(data["format"], "medtracker.portable.v1");
    assert_eq!(data["scope"], "single_person");
    assert!(data.get("cursor").is_none());
    let collections = records(&snapshot);
    assert_portable_records(collections, false);
    assert_hidden_and_foreign_records_absent(collections, &fixture);
    assert!(!has_portable_id(
        &collections["people"],
        &fixture.managed_person_portable_id
    ));
    assert!(!has_portable_id(
        &collections["people"],
        &fixture.hidden_person_portable_id
    ));
    assert!(!has_portable_id(
        &collections["people"],
        &fixture.foreign_person_portable_id
    ));
    assert!(collections.get("dose_occurrences").is_none());
    assert!(collections.get("medication_pause_periods").is_none());
    let audit_response = target.get(
        &format!(
            "/api/v1/households/{}/admin/audit_logs",
            fixture.household_id
        ),
        Some(&fixture.access_token),
    );
    assert_eq!(audit_response.status().as_u16(), 200);
    let audit = body(audit_response);
    let matching: Vec<_> = audit["data"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|event| event["request_id"] == request_id)
        .collect();
    assert_eq!(matching.len(), 2);
    let read = matching
        .iter()
        .find(|event| event["event_type"] == "portable_data.mobile_snapshot_read")
        .unwrap();
    assert_eq!(read["actor_account_id"], fixture.view_account_id);
    assert_eq!(read["actor_membership_id"], fixture.view_membership_id);
    assert_eq!(read["metadata"]["export_mode"], "mobile_snapshot");
    assert_eq!(read["metadata"]["encrypted"], false);
    for (category, rows) in collections.as_object().unwrap() {
        assert_eq!(
            read["metadata"]["record_counts"][category],
            rows.as_array().unwrap().len()
        );
    }
    let request = matching
        .iter()
        .find(|event| event["event_type"] == "api.request")
        .unwrap();
    assert_eq!(request["metadata"]["status"], 200);
}

#[test]
fn delegated_mobile_export_contains_manageable_person_and_relations() {
    let target = Target::from_env();
    let fixture = fixture();
    let response = target.get(
        &snapshot_path(&fixture, "mobile_snapshot"),
        Some(&fixture.delegated_access_token),
    );
    assert_eq!(response.status().as_u16(), 200);
    let snapshot = body(response);
    assert_eq!(snapshot["data"]["format"], "medtracker.portable.v1");
    let collections = records(&snapshot);
    assert_portable_records(collections, true);
    assert_hidden_and_foreign_records_absent(collections, &fixture);
    assert!(has_portable_id(
        &collections["people"],
        &fixture.managed_person_portable_id
    ));
    assert!(has_portable_id(
        &collections["health_events"],
        &fixture.managed_health_event_portable_id
    ));
    assert!(!has_portable_id(
        &collections["people"],
        &fixture.hidden_person_portable_id
    ));
    assert!(!has_portable_id(
        &collections["people"],
        &fixture.foreign_person_portable_id
    ));
    assert_eq!(
        collections["health_events"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["portable_id"] == fixture.managed_health_event_portable_id)
            .unwrap()["person_portable_id"],
        fixture.managed_person_portable_id
    );
    let medication = collections["medications"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["portable_id"] == fixture.managed_medication_portable_id)
        .unwrap();
    assert_eq!(
        medication["location_portable_id"],
        fixture.primary_location_portable_id
    );
}

#[test]
fn snapshot_routes_require_authentication_and_reject_foreign_households() {
    let target = Target::from_env();
    let fixture = fixture();
    for kind in ["sync/snapshot", "mobile_snapshot"] {
        let path = snapshot_path(&fixture, kind);
        assert_eq!(target.get(&path, None).status().as_u16(), 401);
        let foreign = format!(
            "/api/v1/households/{}/{}",
            fixture.foreign_household_id, kind
        );
        assert_eq!(
            target
                .get(&foreign, Some(&fixture.access_token))
                .status()
                .as_u16(),
            403
        );
    }
}

#[test]
fn change_feed_retains_inclusive_ordered_writes_and_tombstone_metadata() {
    let target = Target::from_env();
    let fixture = fixture();
    let snapshot = body(target.get(
        &snapshot_path(&fixture, "sync/snapshot"),
        Some(&fixture.access_token),
    ));
    let cursor = snapshot["data"]["cursor"]
        .as_str()
        .expect("snapshot cursor");
    let health_path = snapshot_path(&fixture, "health_events");
    let response = target.post_json_authorized(
        &health_path,
        &fixture.access_token,
        &json!({"health_event": {
            "person_id": fixture.managed_person_portable_id,
            "event_kind": "illness", "title": "Feed event", "started_on": "2026-02-25"
        }}),
    );
    assert_eq!(response.status().as_u16(), 201);
    let created = body(response)["data"].clone();
    let health_id = created["portable_id"].as_str().expect("portable health ID");
    let response = target.patch_json(
        &format!("{health_path}/{health_id}"),
        &fixture.access_token,
        &json!({"health_event": {"title": "Feed event updated"}}),
    );
    assert_eq!(response.status().as_u16(), 200);

    let location_path = snapshot_path(&fixture, "locations");
    let response = target.post_json_authorized(
        &location_path,
        &fixture.access_token,
        &json!({"location": {"name": format!("Feed location {health_id}")}}),
    );
    assert_eq!(response.status().as_u16(), 201);
    let tag = response.headers()["etag"]
        .to_str()
        .expect("location ETag")
        .to_owned();
    let location = body(response)["data"].clone();
    let location_id = location["portable_id"]
        .as_str()
        .expect("portable location ID");
    let response = target.delete_if_match(
        &format!("{location_path}/{location_id}"),
        &fixture.access_token,
        &tag,
    );
    assert_eq!(response.status().as_u16(), 204);
    let response = target.post_json_authorized(
        &location_path,
        &fixture.access_token,
        &json!({"location": {"name": format!("Second feed location {health_id}")}}),
    );
    assert_eq!(response.status().as_u16(), 201);
    let second_tag = response.headers()["etag"]
        .to_str()
        .expect("location ETag")
        .to_owned();
    let second_location = body(response)["data"].clone();
    let second_location_id = second_location["portable_id"]
        .as_str()
        .expect("portable location ID");
    let response = target.delete_if_match(
        &format!("{location_path}/{second_location_id}"),
        &fixture.access_token,
        &second_tag,
    );
    assert_eq!(response.status().as_u16(), 204);

    let feed = changes(&target, &fixture, &fixture.access_token, cursor);
    assert!(feed["cursor"]
        .as_str()
        .is_some_and(|value| !value.is_empty()));
    let events = feed["changes"].as_array().expect("changes array");
    let own_events: Vec<_> = events
        .iter()
        .filter(|event| event["record_portable_id"] == health_id)
        .collect();
    assert_eq!(own_events.len(), 2);
    assert_eq!(own_events[0]["action"], "create");
    assert_eq!(own_events[1]["action"], "update");
    for event in &own_events {
        assert_eq!(event["record_type"], "HealthEvent");
        assert_eq!(event["record_id"], created["id"]);
        assert_eq!(event["metadata"]["portable_id"], health_id);
        assert_eq!(
            event["metadata"]["person_portable_id"],
            fixture.managed_person_portable_id
        );
        assert!(event["id"].as_i64().is_some());
        assert!(event["occurred_at"]
            .as_str()
            .is_some_and(|value| value.ends_with('Z')));
    }
    assert!(events.windows(2).all(|pair| {
        let previous = pair[0]["occurred_at"].as_str().unwrap();
        let next = pair[1]["occurred_at"].as_str().unwrap();
        previous < next || (previous == next && pair[0]["id"].as_i64() < pair[1]["id"].as_i64())
    }));
    let inclusive = changes(
        &target,
        &fixture,
        &fixture.access_token,
        own_events[0]["occurred_at"].as_str().unwrap(),
    );
    assert!(inclusive["changes"]
        .as_array()
        .unwrap()
        .iter()
        .any(|row| row["id"] == own_events[0]["id"]));
    let location_change =
        row_for_portable_id(&feed["changes"], location_id).expect("location create change");
    assert_eq!(location_change["record_type"], "Location");
    assert_eq!(location_change["action"], "create");
    assert_eq!(location_change["metadata"]["portable_id"], location_id);

    let tombstone =
        row_for_portable_id(&feed["tombstones"], location_id).expect("location tombstone");
    let second_tombstone = row_for_portable_id(&feed["tombstones"], second_location_id)
        .expect("second location tombstone");
    assert_eq!(tombstone["record_type"], "Location");
    assert_eq!(tombstone["action"], "delete");
    assert_eq!(tombstone["metadata"]["portable_id"], location_id);
    assert!(tombstone["id"].as_i64().is_some());
    assert!(tombstone["deleted_at"]
        .as_str()
        .is_some_and(|value| value.ends_with('Z')));
    assert!(second_tombstone["id"].as_i64() > tombstone["id"].as_i64());
    let tombstones = feed["tombstones"].as_array().unwrap();
    assert!(tombstones.windows(2).all(|pair| {
        let previous = pair[0]["deleted_at"].as_str().unwrap();
        let next = pair[1]["deleted_at"].as_str().unwrap();
        previous < next || (previous == next && pair[0]["id"].as_i64() < pair[1]["id"].as_i64())
    }));
}

#[test]
fn change_feed_includes_events_and_tombstones_at_the_exact_cursor() {
    let target = Target::from_env();
    let fixture = fixture();
    let boundary = "2026-01-01T00:00:00Z";
    let feed = changes(&target, &fixture, &fixture.access_token, boundary);
    let portable_id = &fixture.cursor_boundary_location_portable_id;
    let event = row_for_portable_id(&feed["changes"], portable_id).expect("boundary event");
    let tombstone =
        row_for_portable_id(&feed["tombstones"], portable_id).expect("boundary tombstone");
    assert_eq!(event["occurred_at"], boundary);
    assert_eq!(event["action"], "create");
    assert_eq!(tombstone["deleted_at"], boundary);
    assert_eq!(tombstone["action"], "delete");

    let after = changes(
        &target,
        &fixture,
        &fixture.access_token,
        "2026-01-01T00:00:01Z",
    );
    assert!(row_for_portable_id(&after["changes"], portable_id).is_none());
    assert!(row_for_portable_id(&after["tombstones"], portable_id).is_none());
}

#[test]
fn change_feed_validates_cursor_and_current_household_access() {
    let target = Target::from_env();
    let fixture = fixture();
    let path = snapshot_path(&fixture, "sync/changes");
    assert_eq!(
        target
            .get(&path, Some(&fixture.access_token))
            .status()
            .as_u16(),
        400
    );
    let response = target.get(
        &format!("{path}?cursor=not-a-date"),
        Some(&fixture.access_token),
    );
    assert_eq!(response.status().as_u16(), 422);
    assert_eq!(body(response)["error"]["code"], "unprocessable_content");
    assert_eq!(
        target
            .get(&format!("{path}?cursor=1970-01-01T00:00:00Z"), None)
            .status()
            .as_u16(),
        401
    );
    let foreign = format!(
        "/api/v1/households/{}/sync/changes?cursor=1970-01-01T00:00:00Z",
        fixture.foreign_household_id
    );
    assert_eq!(
        target
            .get(&foreign, Some(&fixture.access_token))
            .status()
            .as_u16(),
        403
    );
}

#[test]
fn change_feed_projects_the_current_saved_dose_outcome() {
    let target = Target::from_env();
    let fixture = fixture();
    let snapshot = body(target.get(
        &snapshot_path(&fixture, "sync/snapshot"),
        Some(&fixture.access_token),
    ));
    let cursor = snapshot["data"]["cursor"]
        .as_str()
        .expect("snapshot cursor");
    let date = OffsetDateTime::now_utc().date().to_string();
    let source_path = format!(
        "{}/{}{}",
        snapshot_path(&fixture, "schedules"),
        fixture.managed_schedule_portable_id,
        "/dose_occurrences"
    );
    let projected = body(target.get(
        &format!("{source_path}?start_date={date}&end_date={date}"),
        Some(&fixture.access_token),
    ));
    let occurrence = projected["data"]
        .as_array()
        .expect("occurrences")
        .first()
        .expect("managed occurrence");
    let key = occurrence["key"].as_str().expect("occurrence key");
    let not_taken = target.post_json_authorized(
        &format!("{source_path}/not_taken"),
        &fixture.access_token,
        &json!({"dose_occurrence": {"key": key, "reason": "unwell", "note": "Feed control"}}),
    );
    assert_eq!(not_taken.status().as_u16(), 200);
    let tag = not_taken.headers()["etag"]
        .to_str()
        .expect("outcome ETag")
        .to_owned();
    assert_eq!(body(not_taken)["data"]["outcome"], "not_taken");
    let reopened = target.patch_json_if_match(
        &format!("{source_path}/reopen"),
        &fixture.access_token,
        &json!({"dose_occurrence": {"key": key}}),
        &tag,
    );
    assert_eq!(reopened.status().as_u16(), 200);
    let current = body(reopened)["data"].clone();
    assert_eq!(current["outcome"], "open");

    let feed = changes(&target, &fixture, &fixture.view_access_token, cursor);
    let outcome_events: Vec<_> = feed["changes"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| {
            row["record_type"] == "MedicationDoseOccurrence"
                && row["record"]["source_portable_id"] == fixture.managed_schedule_portable_id
        })
        .collect();
    assert!(!outcome_events.is_empty(), "saved outcome change missing");
    for event in outcome_events {
        assert_eq!(event["record_type"], "MedicationDoseOccurrence");
        assert_eq!(
            event["metadata"]["person_portable_id"],
            fixture.managed_person_portable_id
        );
        assert_eq!(event["record"]["portable_id"], event["record_portable_id"]);
        assert_eq!(event["record"]["outcome"], "open");
        assert_eq!(event["record"]["etag"], current["etag"]);
    }
}

#[test]
fn change_feed_hides_ungranted_person_events_and_tombstones() {
    let target = Target::from_env();
    let fixture = fixture();
    let managed_resume = format!(
        "{}/{}/resume",
        snapshot_path(&fixture, "schedules"),
        fixture.managed_schedule_portable_id
    );
    let hidden_resume = format!(
        "{}/{}/resume",
        snapshot_path(&fixture, "schedules"),
        fixture.hidden_schedule_portable_id
    );
    assert_eq!(
        target
            .patch_json(&managed_resume, &fixture.access_token, &json!({}))
            .status()
            .as_u16(),
        200
    );
    assert_eq!(
        target
            .patch_json(&hidden_resume, &fixture.feed_access_token, &json!({}))
            .status()
            .as_u16(),
        200
    );
    let snapshot = body(target.get(
        &snapshot_path(&fixture, "sync/snapshot"),
        Some(&fixture.view_access_token),
    ));
    let cursor = snapshot["data"]["cursor"]
        .as_str()
        .expect("snapshot cursor");
    let health_path = snapshot_path(&fixture, "health_events");
    let visible = target.post_json_authorized(
        &health_path,
        &fixture.access_token,
        &json!({"health_event": {
            "person_id": fixture.managed_person_portable_id,
            "event_kind": "illness", "title": "Visible feed control", "started_on": "2026-02-25"
        }}),
    );
    assert_eq!(visible.status().as_u16(), 201);
    let visible_id = body(visible)["data"]["portable_id"]
        .as_str()
        .unwrap()
        .to_owned();
    let hidden = target.post_json_authorized(
        &health_path,
        &fixture.feed_access_token,
        &json!({"health_event": {
            "person_id": fixture.hidden_person_portable_id,
            "event_kind": "illness", "title": "Hidden feed event", "started_on": "2026-02-25"
        }}),
    );
    assert_eq!(hidden.status().as_u16(), 201);
    let hidden_id = body(hidden)["data"]["portable_id"]
        .as_str()
        .unwrap()
        .to_owned();
    let visible_pause = format!(
        "{}/{}{}",
        snapshot_path(&fixture, "schedules"),
        fixture.managed_schedule_portable_id,
        "/pause"
    );
    let visible_pause_response =
        target.patch_json(&visible_pause, &fixture.access_token, &json!({}));
    assert_eq!(visible_pause_response.status().as_u16(), 200);
    let visible_pause_id = body(visible_pause_response)["data"]["current_pause_period"]
        ["portable_id"]
        .as_str()
        .expect("visible pause ID")
        .to_owned();
    let hidden_pause = format!(
        "{}/{}{}",
        snapshot_path(&fixture, "schedules"),
        fixture.hidden_schedule_portable_id,
        "/pause"
    );
    let hidden_pause_response =
        target.patch_json(&hidden_pause, &fixture.feed_access_token, &json!({}));
    assert_eq!(hidden_pause_response.status().as_u16(), 200);
    let hidden_pause_id = body(hidden_pause_response)["data"]["current_pause_period"]
        ["portable_id"]
        .as_str()
        .expect("hidden pause ID")
        .to_owned();

    let hidden_assignment_path = format!(
        "{}/{}",
        snapshot_path(&fixture, "person_medications"),
        fixture.hidden_assignment_portable_id
    );
    let hidden_assignment = target.get(&hidden_assignment_path, Some(&fixture.feed_access_token));
    assert_eq!(hidden_assignment.status().as_u16(), 200);
    let tag = hidden_assignment.headers()["etag"]
        .to_str()
        .expect("assignment ETag")
        .to_owned();
    let deletion = target.post_json_authorized(
        &snapshot_path(&fixture, "sync/batches"),
        &fixture.feed_access_token,
        &json!({"batch": {"operations": [{
            "action": "delete", "resource_type": "person_medication",
            "id": fixture.hidden_assignment_portable_id, "if_match": tag,
            "attributes": {}
        }]}}),
    );
    assert_eq!(deletion.status().as_u16(), 201);
    assert!(
        body(deletion)["data"]["results"][0]["record_portable_id"]
            == fixture.hidden_assignment_portable_id,
        "hidden deletion did not return its portable ID"
    );

    let feed = changes(&target, &fixture, &fixture.view_access_token, cursor);
    assert!(
        row_for_portable_id(&feed["changes"], &visible_id).is_some(),
        "managed control missing"
    );
    assert!(
        row_for_portable_id(&feed["changes"], &visible_pause_id).is_some(),
        "managed pause control missing"
    );
    assert!(
        row_for_portable_id(&feed["changes"], &hidden_pause_id).is_none(),
        "hidden pause disclosed"
    );
    let hidden_event_disclosed = contains_identifier(&feed["changes"], &hidden_id);
    let hidden_tombstone_disclosed =
        contains_identifier(&feed["tombstones"], &fixture.hidden_assignment_portable_id);
    let hidden_identifier_disclosed = [
        &fixture.hidden_person_portable_id,
        &fixture.hidden_health_event_portable_id,
        &hidden_id,
        &fixture.hidden_assignment_portable_id,
        &fixture.hidden_schedule_portable_id,
        &fixture.hidden_medication_portable_id,
        &fixture.hidden_dosage_portable_id,
        &fixture.hidden_take_portable_id,
        &fixture.hidden_preference_portable_id,
        &fixture.hidden_pause_period_id,
        &hidden_pause_id,
    ]
    .iter()
    .any(|id| contains_identifier(&feed, id));
    assert!(
        !hidden_event_disclosed && !hidden_tombstone_disclosed && !hidden_identifier_disclosed,
        "hidden event disclosed: {hidden_event_disclosed}; hidden tombstone disclosed: {hidden_tombstone_disclosed}; hidden identifier disclosed: {hidden_identifier_disclosed}"
    );
}
