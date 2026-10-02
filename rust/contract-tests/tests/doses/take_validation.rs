#[test]
fn medication_take_rejects_invalid_time_source_and_future_without_stock_loss() {
    let target = Target::from_env();
    let fixture = fixture();
    let path = takes_path(&fixture);
    let before_stock = stock(&target, &fixture, fixture.managed_medication_id);
    let before_takes = take_snapshot(&target, &fixture);
    let foreign_stock = stock_for(
        &target,
        fixture.foreign_household_id,
        &fixture.foreign_access_token,
        fixture.foreign_medication_id,
    );
    let foreign_takes = take_snapshot_for(
        &target,
        fixture.foreign_household_id,
        &fixture.foreign_access_token,
    );
    let (_, taken_at) = clock();
    let schedule_source_id = schedule_portable_id(&target, &fixture);
    let assignment_source_id = assignment_portable_id(&target, &fixture);
    for (source_type, source_id, time, status, message) in [
        (
            "schedule",
            schedule_source_id.as_str(),
            "invalid",
            422,
            Some("taken_at is invalid"),
        ),
        (
            "unknown",
            schedule_source_id.as_str(),
            taken_at.as_str(),
            404,
            None,
        ),
        (
            "person_medication",
            fixture.foreign_assignment_portable_id.as_str(),
            taken_at.as_str(),
            404,
            None,
        ),
    ] {
        let response = target.post_json_authorized(
            &path,
            &fixture.access_token,
            &json!({"medication_take": {"source_type": source_type, "source_id": source_id, "taken_at": time}}),
        );
        assert_eq!(response.status().as_u16(), status);
        if let Some(message) = message {
            assert_eq!(body(response)["error"]["message"], message);
        }
    }
    let response = target.post_json_authorized(
        &path,
        &fixture.access_token,
        &json!({"medication_take": {
            "source_type": "person_medication", "source_id": assignment_source_id,
            "taken_at": taken_at, "dose_amount": "1",
            "taken_from_medication_id": fixture.foreign_medication_id
        }}),
    );
    assert_eq!(response.status().as_u16(), 422);
    assert!(!body(response)
        .to_string()
        .contains(&fixture.foreign_medication_name));
    assert_eq!(
        stock_for(
            &target,
            fixture.foreign_household_id,
            &fixture.foreign_access_token,
            fixture.foreign_medication_id,
        ),
        foreign_stock
    );
    assert_eq!(
        take_snapshot_for(
            &target,
            fixture.foreign_household_id,
            &fixture.foreign_access_token,
        ),
        foreign_takes
    );
    let future = (OffsetDateTime::now_utc() + time::Duration::hours(2))
        .format(&Rfc3339)
        .unwrap();
    let response = target.post_json_authorized(
        &path,
        &fixture.access_token,
        &json!({"medication_take": {
            "source_type": "person_medication",
            "source_id": assignment_source_id,
            "taken_at": future
        }}),
    );
    assert_eq!(response.status().as_u16(), 422);
    assert_eq!(
        body(response)["error"]["message"],
        "Cannot record a dose more than one hour in the future"
    );
    assert_eq!(
        stock(&target, &fixture, fixture.managed_medication_id),
        before_stock
    );
    assert_eq!(take_snapshot(&target, &fixture), before_takes);
}

#[test]
fn direct_take_failures_leave_stock_and_take_history_unchanged() {
    let target = Target::from_env();
    let fixture = fixture();
    let (_, taken_at) = clock();
    let (schedule_id, medication_id) = create_schedule(&target, &fixture);
    let source_path = format!(
        "/api/v1/households/{}/schedules/{schedule_id}",
        fixture.household_id
    );
    let source = body(target.get(&source_path, Some(&fixture.access_token)))["data"]["portable_id"]
        .as_str()
        .unwrap()
        .to_owned();
    let path = takes_path(&fixture);
    let payload = json!({"medication_take": {"source_type": "schedule", "source_id": source,
        "taken_at": taken_at, "taken_from_medication_id": medication_id}});
    let count = body(target.get(&path, Some(&fixture.access_token)))["meta"]["total_count"].clone();
    let (date, _) = clock();
    let occurrence_path = schedule_path(&fixture, schedule_id);
    let occurrence_key =
        rows(&target, &occurrence_path, &fixture.access_token, &date)[0]["key"].clone();
    let occurrence_payload = json!({"dose_occurrence": {"key": occurrence_key,
        "taken_at": taken_at, "taken_from_medication_id": medication_id}});
    let pause_path = format!(
        "/api/v1/households/{}/schedules/{source}/pause",
        fixture.household_id
    );
    let response = target.patch_json(&pause_path, &fixture.access_token, &json!({}));
    assert_eq!(response.status().as_u16(), 200);
    let response = target.post_json_authorized(
        &format!("{occurrence_path}/take"),
        &fixture.access_token,
        &occurrence_payload,
    );
    assert_eq!(response.status().as_u16(), 422);
    assert_eq!(body(response)["error"]["code"], "paused");
    let response = target.post_json_authorized(&path, &fixture.access_token, &payload);
    assert_eq!(response.status().as_u16(), 422);
    assert_eq!(
        body(response)["error"]["message"],
        "Cannot take medication: paused"
    );
    assert_eq!(stock(&target, &fixture, medication_id), "20.0");
    let resume_path = format!(
        "/api/v1/households/{}/schedules/{source}/resume",
        fixture.household_id
    );
    let response = target.patch_json(&resume_path, &fixture.access_token, &json!({}));
    assert_eq!(response.status().as_u16(), 200);
    let medication_path = format!(
        "/api/v1/households/{}/medications/{medication_id}",
        fixture.household_id
    );
    let response = target.patch_json(
        &medication_path,
        &fixture.access_token,
        &json!({"medication": {"current_supply": "0.0"}}),
    );
    assert_eq!(response.status().as_u16(), 200);
    let response = target.post_json_authorized(
        &format!("{occurrence_path}/take"),
        &fixture.access_token,
        &occurrence_payload,
    );
    assert_eq!(response.status().as_u16(), 422);
    assert_eq!(body(response)["error"]["code"], "out_of_stock");
    let response = target.post_json_authorized(&path, &fixture.access_token, &payload);
    assert_eq!(response.status().as_u16(), 422);
    assert_eq!(
        body(response)["error"]["message"],
        "Cannot take medication: out of stock"
    );
    assert_eq!(stock(&target, &fixture, medication_id), "0.0");
    assert_eq!(
        body(target.get(&path, Some(&fixture.access_token)))["meta"]["total_count"],
        count
    );

    let (cooldown_id, cooldown_medication_id) = create_schedule(&target, &fixture);
    let cooldown_path = format!(
        "/api/v1/households/{}/schedules/{cooldown_id}",
        fixture.household_id
    );
    let response = target.patch_json(
        &cooldown_path,
        &fixture.access_token,
        &json!({"schedule": {"max_daily_doses": 3, "min_hours_between_doses": "24.0"}}),
    );
    assert_eq!(response.status().as_u16(), 200);
    let cooldown_source = body(target.get(&cooldown_path, Some(&fixture.access_token)))["data"]
        ["portable_id"]
        .as_str()
        .unwrap()
        .to_owned();
    let cooldown_payload = json!({"medication_take": {"source_type": "schedule",
        "source_id": cooldown_source, "taken_at": taken_at,
        "taken_from_medication_id": cooldown_medication_id}});
    let response = target.post_json_authorized(&path, &fixture.access_token, &cooldown_payload);
    assert_eq!(response.status().as_u16(), 201);
    let first = body(response)["data"].clone();
    let before = stock(&target, &fixture, cooldown_medication_id);
    let takes_before = take_snapshot(&target, &fixture);
    assert!(takes_before.1.contains(&first["id"].as_i64().unwrap()));
    let response = target.post_json_authorized(&path, &fixture.access_token, &cooldown_payload);
    assert_eq!(response.status().as_u16(), 422);
    assert_eq!(
        body(response)["error"]["message"],
        "Cannot take medication: timing restrictions not met"
    );
    assert_eq!(stock(&target, &fixture, cooldown_medication_id), before);
    assert_eq!(take_snapshot(&target, &fixture), takes_before);
}

#[test]
fn numeric_direct_dose_keeps_earlier_validation_priorities_and_sync_error_without_writes() {
    let target = Target::from_env();
    let fixture = fixture();
    let (schedule_id, medication_id) = create_schedule(&target, &fixture);
    let api = format!("/api/v1/households/{}", fixture.household_id);
    let response = target.get(&format!("{api}/schedules/{schedule_id}"), Some(&fixture.access_token));
    assert_eq!(response.status().as_u16(), 200);
    let source_id = body(response)["data"]["portable_id"].as_str().unwrap().to_owned();
    let (_, taken_at) = clock();
    let before_state = zero_stock_clinical_state(&fixture);
    for sync in [false, true] {
        for index in 0..6 {
            let mut attributes = json!({"source_type": "schedule", "source_id": source_id,
                "taken_at": taken_at, "dose_amount": 1.25,
                "taken_from_medication_id": medication_id,
                "client_uuid": format!("99999999-0000-4000-8000-{:012}", 300 + index + if sync { 100 } else { 0 })});
            let (expected, code, message) = match index {
                0 => (400, "unprocessable_content", "medication_take is required"),
                1 => (422, "unprocessable_content", "unknown request field"),
                2 => {
                    attributes["unexpected"] = json!(true);
                    (422, "unprocessable_content", "unknown medication_take field")
                },
                3 => {
                    attributes["source_id"] = json!("invalid-reference");
                    (422, "unprocessable_content", "invalid medication source")
                },
                4 => {
                    attributes["source_type"] = json!("unsupported_source");
                    (404, "not_found", "Record not found")
                },
                _ if sync => (422, "unprocessable_content", "dose_amount must be a string"),
                _ => (422, "validation_failed", "Validation failed"),
            };
            if sync && index < 2 {
                continue;
            }
            let response = if sync {
                let key = format!("99999999-0000-4000-9000-{index:012}");
                target.post_json_with_key(
                    &format!("{api}/sync/batches"), &fixture.access_token, &key,
                    &json!({"batch": {"operations": [{"resource_type": "medication_take", "action": "create", "attributes": attributes}]}}),
                )
            } else {
                let payload = match index {
                    0 => json!({"medication_take": null}),
                    1 => json!({"medication_take": attributes, "unexpected": true}),
                    _ => json!({"medication_take": attributes}),
                };
                target.post_json_authorized(&takes_path(&fixture), &fixture.access_token, &payload)
            };
            let status = response.status().as_u16();
            let error = body(response);
            assert_eq!(zero_stock_clinical_state(&fixture), before_state, "numeric rejection writes no clinical rows, sync={sync}, case={index}");
            assert_eq!(status, expected, "numeric priority sync={sync}, case={index}: {error}");
            assert_eq!(error["error"]["code"], code);
            assert_eq!(error["error"]["message"], message);
            if !sync && index == 5 {
                assert_eq!(error["error"]["errors"], json!({"dose_amount": ["must be a string"]}));
            } else {
                assert!(error["error"].get("errors").is_none());
            }
        }
    }
}
