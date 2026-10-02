#[test]
fn schedule_occurrences_are_stable_person_scoped_and_validate_ranges() {
    let target = Target::from_env();
    let fixture = fixture();
    let (date, taken_at) = clock();
    let path = schedule_path(&fixture, fixture.managed_schedule_id);
    let first = rows(&target, &path, &fixture.view_access_token, &date);
    assert!(!first.is_empty());
    let row = &first[0];
    assert_eq!(row["source_type"], "schedule");
    assert_eq!(row["source_id"], fixture.managed_schedule_id);
    assert_eq!(row["outcome"], "open");
    assert_eq!(row["window_starts_on"], date);
    assert_eq!(row["position"], 1);
    assert_eq!(row["expected"], true);
    assert_eq!(row["due"], true);
    assert!(row["scheduled_at"].is_null());
    assert!(row["etag"].is_null());
    assert_eq!(
        rows(&target, &path, &fixture.view_access_token, &date),
        first
    );

    for invalid in [
        path.clone(),
        format!("{path}?start_date=private-clinical-text&end_date={date}"),
        format!("{path}?start_date=2026-01-01&end_date=2026-02-01"),
        format!("{path}?start_date=2026-02-02&end_date=2026-02-01"),
    ] {
        let response = target.get(&invalid, Some(&fixture.access_token));
        assert_eq!(response.status().as_u16(), 422);
        assert!(!body(response).to_string().contains("private-clinical-text"));
    }
    let response = target.get(&format!("{path}?start_date={date}&end_date={date}"), None);
    assert_eq!(response.status().as_u16(), 401);
    let before_stock = stock(&target, &fixture, fixture.managed_medication_id);
    for id in [fixture.hidden_schedule_id, fixture.foreign_schedule_id] {
        let hidden_path = schedule_path(&fixture, id);
        let response = target.get(
            &format!("{hidden_path}?start_date={date}&end_date={date}"),
            Some(&fixture.access_token),
        );
        assert_eq!(response.status().as_u16(), 404);
        let response = target.post_json_authorized(
            &format!("{hidden_path}/not_taken"),
            &fixture.access_token,
            &json!({"dose_occurrence": {"key": row["key"], "reason": "unwell"}}),
        );
        assert_eq!(response.status().as_u16(), 404);
        let response = target.post_json_authorized(
            &format!("{hidden_path}/take"),
            &fixture.access_token,
            &json!({"dose_occurrence": {
                "key": row["key"],
                "taken_at": taken_at,
                "taken_from_medication_id": fixture.managed_medication_id
            }}),
        );
        assert_eq!(response.status().as_u16(), 404);
        let response = target.patch_json_if_match(
            &format!("{hidden_path}/reopen"),
            &fixture.access_token,
            &json!({"dose_occurrence": {"key": row["key"]}}),
            "stale",
        );
        assert_eq!(response.status().as_u16(), 404);
    }
    assert_eq!(rows(&target, &path, &fixture.access_token, &date), first);
    assert_eq!(
        stock(&target, &fixture, fixture.managed_medication_id),
        before_stock
    );
}

#[test]
fn schedule_not_taken_reopens_with_a_current_version_and_retains_context() {
    let target = Target::from_env();
    let fixture = fixture();
    let (date, _) = clock();
    let (schedule_id, medication_id) = create_schedule(&target, &fixture);
    let path = schedule_path(&fixture, schedule_id);
    let key = rows(&target, &path, &fixture.access_token, &date)[0]["key"]
        .as_str()
        .unwrap()
        .to_owned();
    let before_stock = stock(&target, &fixture, medication_id);
    let payload = json!({"dose_occurrence": {"key": key, "reason": "unwell", "note": "Resting"}});
    let response = target.post_json_authorized(
        &format!("{path}/not_taken"),
        &fixture.access_token,
        &payload,
    );
    assert_eq!(response.status().as_u16(), 200);
    let tag = etag(&response);
    let id = request_id(&response);
    let decision = body(response)["data"].clone();
    assert_eq!(decision["outcome"], "not_taken");
    assert_eq!(decision["reason"], "unwell");
    assert_eq!(decision["note"], "Resting");
    assert_eq!(decision["etag"], tag);
    assert!(decision["resolved_at"].as_str().unwrap().ends_with('Z'));
    assert_eq!(stock(&target, &fixture, medication_id), before_stock);
    audit(
        &target,
        &fixture,
        &id,
        "POST",
        "not_taken",
        200,
        "api/v1/dose_occurrences",
    );
    assert_eq!(
        rows(&target, &path, &fixture.access_token, &date)[0],
        decision
    );

    let response = target.post_json_authorized(
        &format!("{path}/not_taken"),
        &fixture.access_token,
        &payload,
    );
    assert_eq!(response.status().as_u16(), 200);
    let replay_request_id = request_id(&response);
    assert_eq!(etag(&response), tag);
    audit(
        &target,
        &fixture,
        &replay_request_id,
        "POST",
        "not_taken",
        200,
        "api/v1/dose_occurrences",
    );
    let response = target.post_json_authorized(
        &format!("{path}/not_taken"),
        &fixture.access_token,
        &json!({"dose_occurrence": {"key": key, "reason": "refused"}}),
    );
    assert_eq!(response.status().as_u16(), 409);
    assert_eq!(body(response)["error"]["code"], "already_resolved");

    let reopen = format!("{path}/reopen");
    let payload = json!({"dose_occurrence": {"key": key}});
    let response = target.patch_json(&reopen, &fixture.access_token, &payload);
    assert_eq!(response.status().as_u16(), 428);
    assert_eq!(body(response)["error"]["code"], "precondition_required");
    let response = target.patch_json_if_match(&reopen, &fixture.access_token, &payload, "stale");
    assert_eq!(response.status().as_u16(), 409);
    assert_eq!(body(response)["error"]["code"], "sync_conflict");
    let response = target.patch_json_if_match(&reopen, &fixture.view_access_token, &payload, &tag);
    assert_eq!(response.status().as_u16(), 403);
    let response = target.patch_json_if_match(&reopen, &fixture.access_token, &payload, &tag);
    assert_eq!(response.status().as_u16(), 200);
    let reopen_request_id = request_id(&response);
    let reopened = body(response)["data"].clone();
    audit(
        &target,
        &fixture,
        &reopen_request_id,
        "PATCH",
        "reopen",
        200,
        "api/v1/dose_occurrences",
    );
    assert_eq!(reopened["outcome"], "open");
    assert!(reopened["reason"].is_null());
    assert!(reopened["note"].is_null());
    assert_ne!(reopened["etag"], tag);
    assert_eq!(stock(&target, &fixture, medication_id), before_stock);
    assert_eq!(
        rows(&target, &path, &fixture.access_token, &date)[0],
        reopened
    );
}

#[test]
fn schedule_take_replaces_not_taken_once_and_stays_immutable() {
    let target = Target::from_env();
    let fixture = fixture();
    let (date, taken_at) = clock();
    let (schedule_id, medication_id) = create_schedule(&target, &fixture);
    let path = schedule_path(&fixture, schedule_id);
    let key = rows(&target, &path, &fixture.access_token, &date)[0]["key"]
        .as_str()
        .unwrap()
        .to_owned();
    let before_stock: f64 = stock(&target, &fixture, medication_id).parse().unwrap();
    let response = target.post_json_authorized(
        &format!("{path}/not_taken"),
        &fixture.access_token,
        &json!({"dose_occurrence": {"key": key, "reason": "unwell", "note": "Retained decision"}}),
    );
    assert_eq!(response.status().as_u16(), 200);
    let tag = etag(&response);
    let payload = json!({"dose_occurrence": {
        "key": key,
        "taken_at": taken_at,
        "client_uuid": client_uuid(&fixture, 1),
        "dose_amount": "1.25",
        "taken_from_medication_id": medication_id
    }});
    let response =
        target.post_json_authorized(&format!("{path}/take"), &fixture.access_token, &payload);
    assert_eq!(response.status().as_u16(), 428);
    assert_eq!(body(response)["error"]["code"], "precondition_required");
    let response = target.post_json_if_match(
        &format!("{path}/take"),
        &fixture.access_token,
        &payload,
        "stale",
    );
    assert_eq!(response.status().as_u16(), 409);
    let response = target.post_json_if_match(
        &format!("{path}/take"),
        &fixture.view_access_token,
        &payload,
        &tag,
    );
    assert_eq!(response.status().as_u16(), 403);
    assert_eq!(
        stock(&target, &fixture, medication_id)
            .parse::<f64>()
            .unwrap(),
        before_stock
    );

    let response = target.post_json_if_match(
        &format!("{path}/take"),
        &fixture.access_token,
        &payload,
        &tag,
    );
    if response.status().as_u16() != 200 {
        panic!(
            "schedule take: {} {}",
            response.status(),
            response.text().unwrap()
        );
    }
    let id = request_id(&response);
    let taken = body(response)["data"].clone();
    assert_eq!(taken["outcome"], "taken");
    assert!(taken["reason"].is_null());
    assert!(taken["note"].is_null());
    assert!(taken["medication_take_id"].as_i64().is_some());
    assert_ne!(taken["etag"], tag);
    assert_eq!(
        stock(&target, &fixture, medication_id)
            .parse::<f64>()
            .unwrap(),
        before_stock - 1.25
    );
    audit(
        &target,
        &fixture,
        &id,
        "POST",
        "take",
        200,
        "api/v1/dose_occurrences",
    );
    let response =
        target.post_json_authorized(&format!("{path}/take"), &fixture.access_token, &payload);
    if response.status().as_u16() != 200 {
        panic!(
            "schedule take replay: {} {}",
            response.status(),
            response.text().unwrap()
        );
    }
    let replay_request_id = request_id(&response);
    assert_eq!(
        body(response)["data"]["medication_take_id"],
        taken["medication_take_id"]
    );
    audit(
        &target,
        &fixture,
        &replay_request_id,
        "POST",
        "take",
        200,
        "api/v1/dose_occurrences",
    );
    assert_eq!(
        stock(&target, &fixture, medication_id)
            .parse::<f64>()
            .unwrap(),
        before_stock - 1.25
    );
    let response = target.patch_json_if_match(
        &format!("{path}/reopen"),
        &fixture.access_token,
        &json!({"dose_occurrence": {"key": key}}),
        taken["etag"].as_str().unwrap(),
    );
    assert_eq!(response.status().as_u16(), 422);
    assert_eq!(rows(&target, &path, &fixture.access_token, &date)[0], taken);
    let response = target.get(&takes_path(&fixture), Some(&fixture.access_token));
    assert_eq!(response.status().as_u16(), 200);
    let takes = body(response);
    let take = takes["data"]
        .as_array()
        .unwrap()
        .iter()
        .find(|take| take["id"] == taken["medication_take_id"])
        .unwrap();
    assert_eq!(take["dose_amount"], "1.25");
    assert_eq!(take["taken_at"], taken_at);
    assert_eq!(take["schedule_id"], taken["source_id"]);
}
