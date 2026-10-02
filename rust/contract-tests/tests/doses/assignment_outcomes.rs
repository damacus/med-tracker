#[test]
fn direct_assignment_occurrences_require_routine_and_person_access() {
    let target = Target::from_env();
    let fixture = fixture();
    let (date, _) = clock();
    let as_needed = assignment_path(&fixture, fixture.managed_assignment_id);
    assert!(rows(&target, &as_needed, &fixture.access_token, &date).is_empty());
    let (id, medication_id) = create_routine_assignment(&target, &fixture);
    let path = assignment_path(&fixture, id);
    let first = rows(&target, &path, &fixture.view_access_token, &date);
    assert_eq!(first.len(), 1);
    assert_eq!(first[0]["source_type"], "person_medication");
    assert_eq!(first[0]["source_id"], id);
    assert_eq!(first[0]["outcome"], "open");
    assert_eq!(first[0]["window_starts_on"], date);
    assert_eq!(first[0]["window_ends_on"], date);
    assert_eq!(
        rows(&target, &path, &fixture.view_access_token, &date),
        first
    );
    let response = target.post_json_authorized(
        &format!("{path}/not_taken"),
        &fixture.view_access_token,
        &json!({"dose_occurrence": {"key": first[0]["key"], "reason": "unwell"}}),
    );
    assert_eq!(response.status().as_u16(), 403);
    let taken_at = (OffsetDateTime::now_utc() + time::Duration::seconds(1))
        .format(&Rfc3339)
        .unwrap();
    let take_payload = json!({"dose_occurrence": {
        "key": first[0]["key"],
        "taken_at": taken_at,
        "taken_from_medication_id": medication_id
    }});
    let response = target.post_json_authorized(
        &format!("{path}/take"),
        &fixture.view_access_token,
        &take_payload,
    );
    assert_eq!(response.status().as_u16(), 403);
    let response = target.patch_json_if_match(
        &format!("{path}/reopen"),
        &fixture.view_access_token,
        &json!({"dose_occurrence": {"key": first[0]["key"]}}),
        "stale",
    );
    assert_eq!(response.status().as_u16(), 403);
    assert_eq!(rows(&target, &path, &fixture.access_token, &date), first);
    assert_eq!(stock(&target, &fixture, medication_id), "20.0");
    for id in [fixture.hidden_assignment_id, fixture.foreign_assignment_id] {
        let hidden_path = assignment_path(&fixture, id);
        let response = target.get(
            &format!("{hidden_path}?start_date={date}&end_date={date}"),
            Some(&fixture.access_token),
        );
        assert_eq!(response.status().as_u16(), 404);
        let response = target.post_json_authorized(
            &format!("{hidden_path}/not_taken"),
            &fixture.access_token,
            &json!({"dose_occurrence": {"key": first[0]["key"], "reason": "unwell"}}),
        );
        assert_eq!(response.status().as_u16(), 404);
        let response = target.post_json_authorized(
            &format!("{hidden_path}/take"),
            &fixture.access_token,
            &take_payload,
        );
        assert_eq!(response.status().as_u16(), 404);
        let response = target.patch_json_if_match(
            &format!("{hidden_path}/reopen"),
            &fixture.access_token,
            &json!({"dose_occurrence": {"key": first[0]["key"]}}),
            "stale",
        );
        assert_eq!(response.status().as_u16(), 404);
    }
    assert_eq!(rows(&target, &path, &fixture.access_token, &date), first);
    assert_eq!(stock(&target, &fixture, medication_id), "20.0");
}

#[test]
fn direct_assignment_outcome_reopens_then_records_one_take() {
    let target = Target::from_env();
    let fixture = fixture();
    let (date, _) = clock();
    let (id, medication_id) = create_routine_assignment(&target, &fixture);
    let taken_at = (OffsetDateTime::now_utc() + time::Duration::seconds(1))
        .format(&Rfc3339)
        .unwrap();
    let path = assignment_path(&fixture, id);
    let before_stock: f64 = stock(&target, &fixture, medication_id).parse().unwrap();
    let key = rows(&target, &path, &fixture.access_token, &date)[0]["key"].clone();
    let response = target.post_json_authorized(
        &format!("{path}/not_taken"),
        &fixture.access_token,
        &json!({"dose_occurrence": {"key": key, "reason": "unwell", "note": "Resting after treatment"}}),
    );
    assert_eq!(response.status().as_u16(), 200);
    let tag = etag(&response);
    let not_taken_request_id = request_id(&response);
    let not_taken = body(response)["data"].clone();
    assert_eq!(not_taken["outcome"], "not_taken");
    assert_eq!(not_taken["reason"], "unwell");
    assert_eq!(not_taken["note"], "Resting after treatment");
    audit(
        &target,
        &fixture,
        &not_taken_request_id,
        "POST",
        "not_taken",
        200,
        "api/v1/dose_occurrences",
    );
    assert_eq!(
        rows(&target, &path, &fixture.view_access_token, &date)[0],
        not_taken
    );
    let response = target.post_json_authorized(
        &format!("{path}/not_taken"),
        &fixture.access_token,
        &json!({"dose_occurrence": {"key": key, "reason": "unwell", "note": "Resting after treatment"}}),
    );
    assert_eq!(response.status().as_u16(), 200);
    assert_eq!(etag(&response), tag);
    assert_eq!(body(response)["data"], not_taken);
    let response = target.patch_json(
        &format!("{path}/reopen"),
        &fixture.access_token,
        &json!({"dose_occurrence": {"key": key}}),
    );
    assert_eq!(response.status().as_u16(), 428);
    let response = target.patch_json_if_match(
        &format!("{path}/reopen"),
        &fixture.access_token,
        &json!({"dose_occurrence": {"key": key}}),
        "stale",
    );
    assert_eq!(response.status().as_u16(), 409);
    assert_eq!(
        rows(&target, &path, &fixture.access_token, &date)[0],
        not_taken
    );
    let replacement = json!({"dose_occurrence": {
        "key": key, "taken_at": taken_at,
        "client_uuid": client_uuid(&fixture, 2),
        "taken_from_medication_id": medication_id
    }});
    let response =
        target.post_json_authorized(&format!("{path}/take"), &fixture.access_token, &replacement);
    assert_eq!(response.status().as_u16(), 428);
    let response = target.post_json_if_match(
        &format!("{path}/take"),
        &fixture.access_token,
        &replacement,
        "stale",
    );
    assert_eq!(response.status().as_u16(), 409);
    assert_eq!(
        rows(&target, &path, &fixture.access_token, &date)[0],
        not_taken
    );
    assert!(takes_for_medication(&target, &fixture, medication_id).is_empty());
    assert_eq!(
        stock(&target, &fixture, medication_id)
            .parse::<f64>()
            .unwrap(),
        before_stock
    );
    let response = target.patch_json_if_match(
        &format!("{path}/reopen"),
        &fixture.access_token,
        &json!({"dose_occurrence": {"key": key}}),
        &tag,
    );
    assert_eq!(response.status().as_u16(), 200);
    let reopened_tag = etag(&response);
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
    assert_eq!(reopened["etag"], reopened_tag);
    assert_ne!(reopened_tag, tag);
    assert_eq!(
        rows(&target, &path, &fixture.access_token, &date)[0],
        reopened
    );
    assert_eq!(
        stock(&target, &fixture, medication_id)
            .parse::<f64>()
            .unwrap(),
        before_stock
    );
    assert!(takes_for_medication(&target, &fixture, medication_id).is_empty());
    let payload = json!({"dose_occurrence": {
        "key": key,
        "taken_at": taken_at,
        "client_uuid": client_uuid(&fixture, 2),
        "taken_from_medication_id": medication_id
    }});
    let invalid_time = json!({"dose_occurrence": {
        "key": key, "taken_at": "invalid",
        "client_uuid": client_uuid(&fixture, 2),
        "taken_from_medication_id": medication_id
    }});
    let response = target.post_json_authorized(
        &format!("{path}/take"),
        &fixture.access_token,
        &invalid_time,
    );
    assert_eq!(response.status().as_u16(), 422);
    assert_eq!(
        rows(&target, &path, &fixture.access_token, &date)[0],
        reopened
    );
    assert!(takes_for_medication(&target, &fixture, medication_id).is_empty());
    assert_eq!(
        stock(&target, &fixture, medication_id)
            .parse::<f64>()
            .unwrap(),
        before_stock
    );
    let response =
        target.post_json_authorized(&format!("{path}/take"), &fixture.access_token, &payload);
    if response.status().as_u16() != 200 {
        panic!(
            "assignment take: {} {}",
            response.status(),
            response.text().unwrap()
        );
    }
    let take_request_id = request_id(&response);
    let taken = body(response)["data"].clone();
    assert_eq!(taken["outcome"], "taken");
    audit(
        &target,
        &fixture,
        &take_request_id,
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
    let response =
        target.post_json_authorized(&format!("{path}/take"), &fixture.access_token, &payload);
    assert_eq!(response.status().as_u16(), 200);
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
    let response = target.get(&takes_path(&fixture), Some(&fixture.access_token));
    assert_eq!(response.status().as_u16(), 200);
    let take = body(response)["data"]
        .as_array()
        .unwrap()
        .iter()
        .find(|take| take["id"] == taken["medication_take_id"])
        .unwrap()
        .clone();
    assert_eq!(take["person_medication_id"], id);
    assert_eq!(take["medication_id"], medication_id);
    assert_eq!(take["dose_amount"], "1.25");
    assert_eq!(
        takes_for_medication(&target, &fixture, medication_id),
        vec![take]
    );

    let attempted = json!({"dose_occurrence": {"key": key, "reason": "unwell", "note": "private-clinical-text"}});
    let response = target.post_json_authorized(
        &format!("{path}/not_taken"),
        &fixture.view_access_token,
        &attempted,
    );
    assert_eq!(response.status().as_u16(), 403);
    assert!(!body(response).to_string().contains("private-clinical-text"));
    let response = target.patch_json_if_match(
        &format!("{path}/reopen"),
        &fixture.access_token,
        &json!({"dose_occurrence": {"key": key}}),
        taken["etag"].as_str().unwrap(),
    );
    assert_eq!(response.status().as_u16(), 422);
    assert!(!body(response).to_string().contains("private-clinical-text"));
    for hidden_id in [fixture.hidden_assignment_id, fixture.foreign_assignment_id] {
        let hidden_path = assignment_path(&fixture, hidden_id);
        let response = target.get(
            &format!("{hidden_path}?start_date={date}&end_date={date}"),
            Some(&fixture.view_access_token),
        );
        assert_eq!(response.status().as_u16(), 404);
        let response = target.post_json_authorized(
            &format!("{hidden_path}/not_taken"),
            &fixture.view_access_token,
            &attempted,
        );
        assert_eq!(response.status().as_u16(), 404);
        assert!(!body(response).to_string().contains("private-clinical-text"));
    }
    let response = target.post_json_authorized(
        &format!("{path}/not_taken"),
        &fixture.foreign_access_token,
        &attempted,
    );
    assert_eq!(response.status().as_u16(), 403);
    assert!(!body(response).to_string().contains("private-clinical-text"));
    assert_eq!(
        rows(&target, &path, &fixture.view_access_token, &date)[0],
        taken
    );
    assert_eq!(
        stock(&target, &fixture, medication_id)
            .parse::<f64>()
            .unwrap(),
        before_stock - 1.25
    );
    assert_eq!(
        takes_for_medication(&target, &fixture, medication_id).len(),
        1
    );
}
