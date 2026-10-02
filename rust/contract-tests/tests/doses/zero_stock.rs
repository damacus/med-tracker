#[test]
fn zero_stock_occurrence_retains_domain_error_and_direct_take_remains_generic_without_writes() {
    let target = Target::from_env();
    let fixture = fixture();
    let (schedule_id, medication_id) = create_schedule(&target, &fixture);
    let api = format!("/api/v1/households/{}", fixture.household_id);
    let source_path = format!("{api}/schedules/{schedule_id}");
    let response = target.get(&source_path, Some(&fixture.access_token));
    assert_eq!(response.status().as_u16(), 200);
    let source_id = body(response)["data"]["portable_id"].as_str().unwrap().to_owned();
    let (date, taken_at) = clock();
    let occurrence_path = schedule_path(&fixture, schedule_id);
    let original_rows = rows(&target, &occurrence_path, &fixture.access_token, &date);
    assert!(!original_rows.is_empty());
    let key = original_rows[0]["key"].clone();
    let response = target.patch_json(
        &format!("{api}/medications/{medication_id}"),
        &fixture.access_token,
        &json!({"medication": {"current_supply": "0.0"}}),
    );
    let status = response.status().as_u16();
    let saved = body(response);
    assert_eq!(status, 200, "zero-stock setup: {saved}");
    let before_stock = stock(&target, &fixture, medication_id);
    assert_eq!(before_stock, "0.0");
    let before_takes = take_snapshot(&target, &fixture);
    let before_rows = rows(&target, &occurrence_path, &fixture.access_token, &date);
    assert_eq!(before_rows, original_rows);
    let before_state = zero_stock_clinical_state(&fixture);
    let response = target.post_json_authorized(
        &takes_path(&fixture),
        &fixture.access_token,
        &json!({"medication_take": {"source_type": "schedule", "source_id": source_id,
            "taken_at": taken_at,
            "client_uuid": "99999999-0000-4000-8000-000000000201"}}),
    );
    let direct_status = response.status().as_u16();
    let direct_error = body(response);
    assert_eq!(stock(&target, &fixture, medication_id), before_stock);
    assert_eq!(take_snapshot(&target, &fixture), before_takes);
    assert_eq!(rows(&target, &occurrence_path, &fixture.access_token, &date), before_rows);
    assert_eq!(zero_stock_clinical_state(&fixture), before_state);
    assert_eq!(direct_status, 422, "direct zero-stock rejection: {direct_error}");
    assert_eq!(direct_error["error"]["code"], "unprocessable_content");
    assert_eq!(direct_error["error"]["message"], "Cannot take medication: out of stock");
    let response = target.post_json_authorized(
        &format!("{occurrence_path}/take"),
        &fixture.access_token,
        &json!({"dose_occurrence": {"key": key, "taken_at": taken_at,
            "client_uuid": "99999999-0000-4000-8000-000000000202"}}),
    );
    let occurrence_status = response.status().as_u16();
    let occurrence_error = body(response);
    assert_eq!(stock(&target, &fixture, medication_id), before_stock);
    assert_eq!(take_snapshot(&target, &fixture), before_takes);
    assert_eq!(rows(&target, &occurrence_path, &fixture.access_token, &date), before_rows);
    assert_eq!(zero_stock_clinical_state(&fixture), before_state);
    assert_eq!(occurrence_status, 422, "occurrence zero-stock rejection: {occurrence_error}");
    assert_eq!(occurrence_error["error"]["code"], "out_of_stock");
    assert_eq!(occurrence_error["error"]["message"], "Dose could not be recorded");
}

#[test]
fn zero_stock_explicit_selection_retains_empty_stock_priority_without_writes() {
    let target = Target::from_env();
    let fixture = fixture();
    let (schedule_id, medication_id) = create_schedule(&target, &fixture);
    let api = format!("/api/v1/households/{}", fixture.household_id);
    let source_path = format!("{api}/schedules/{schedule_id}");
    let response = target.get(&source_path, Some(&fixture.access_token));
    assert_eq!(response.status().as_u16(), 200);
    let source_id = body(response)["data"]["portable_id"].as_str().unwrap().to_owned();
    let (date, taken_at) = clock();
    let occurrence_path = schedule_path(&fixture, schedule_id);
    let original_rows = rows(&target, &occurrence_path, &fixture.access_token, &date);
    assert!(!original_rows.is_empty());
    let key = original_rows[0]["key"].clone();
    let response = target.patch_json(
        &format!("{api}/medications/{medication_id}"),
        &fixture.access_token,
        &json!({"medication": {"current_supply": "0.0"}}),
    );
    let status = response.status().as_u16();
    let saved = body(response);
    assert_eq!(status, 200, "zero-stock setup: {saved}");
    let before_stock = stock(&target, &fixture, medication_id);
    assert_eq!(before_stock, "0.0");
    let before_takes = take_snapshot(&target, &fixture);
    let before_rows = rows(&target, &occurrence_path, &fixture.access_token, &date);
    assert_eq!(before_rows, original_rows);
    let before_state = zero_stock_clinical_state(&fixture);
    let response = target.post_json_authorized(
        &takes_path(&fixture),
        &fixture.access_token,
        &json!({"medication_take": {"source_type": "schedule", "source_id": source_id,
            "taken_at": taken_at, "taken_from_medication_id": medication_id,
            "client_uuid": "99999999-0000-4000-8000-000000000601"}}),
    );
    let direct_status = response.status().as_u16();
    let direct_error = body(response);
    assert_eq!(stock(&target, &fixture, medication_id), before_stock);
    assert_eq!(take_snapshot(&target, &fixture), before_takes);
    assert_eq!(rows(&target, &occurrence_path, &fixture.access_token, &date), before_rows);
    assert_eq!(zero_stock_clinical_state(&fixture), before_state);
    assert_eq!(direct_status, 422, "direct zero-stock rejection: {direct_error}");
    assert_eq!(direct_error["error"]["code"], "unprocessable_content");
    assert_eq!(direct_error["error"]["message"], "Cannot take medication: out of stock");
    let response = target.post_json_authorized(
        &format!("{occurrence_path}/take"),
        &fixture.access_token,
        &json!({"dose_occurrence": {"key": key, "taken_at": taken_at,
            "taken_from_medication_id": medication_id,
            "client_uuid": "99999999-0000-4000-8000-000000000602"}}),
    );
    let occurrence_status = response.status().as_u16();
    let occurrence_error = body(response);
    assert_eq!(stock(&target, &fixture, medication_id), before_stock);
    assert_eq!(take_snapshot(&target, &fixture), before_takes);
    assert_eq!(rows(&target, &occurrence_path, &fixture.access_token, &date), before_rows);
    assert_eq!(zero_stock_clinical_state(&fixture), before_state);
    assert_eq!(occurrence_status, 422, "occurrence zero-stock rejection: {occurrence_error}");
    assert_eq!(occurrence_error["error"]["code"], "out_of_stock");
    assert_eq!(occurrence_error["error"]["message"], "Dose could not be recorded");
}
