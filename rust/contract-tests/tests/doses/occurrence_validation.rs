#[test]
fn occurrence_ranges_and_view_token_non_disclosure_are_consistent() {
    let target = Target::from_env();
    let fixture = fixture();
    let (date, _) = clock();
    for path in [
        schedule_path(&fixture, fixture.managed_schedule_id),
        assignment_path(&fixture, fixture.managed_assignment_id),
    ] {
        let response = target.get(
            &format!("{path}?start_date={date}&end_date={date}"),
            Some(&fixture.view_access_token),
        );
        assert_eq!(response.status().as_u16(), 200);
        for invalid in [
            path.clone(),
            format!("{path}?start_date=private-clinical-text&end_date={date}"),
            format!("{path}?start_date=2099-12-31&end_date=2000-01-01"),
            format!("{path}?start_date=2000-01-01&end_date=2000-02-01"),
        ] {
            let response = target.get(&invalid, Some(&fixture.view_access_token));
            assert_eq!(response.status().as_u16(), 422);
            assert!(!body(response).to_string().contains("private-clinical-text"));
        }
    }
    for id in [fixture.hidden_schedule_id, fixture.foreign_schedule_id] {
        let response = target.get(
            &format!(
                "{}?start_date={date}&end_date={date}",
                schedule_path(&fixture, id)
            ),
            Some(&fixture.view_access_token),
        );
        assert_eq!(response.status().as_u16(), 404);
    }
    for id in [fixture.hidden_assignment_id, fixture.foreign_assignment_id] {
        let response = target.get(
            &format!(
                "{}?start_date={date}&end_date={date}",
                assignment_path(&fixture, id)
            ),
            Some(&fixture.view_access_token),
        );
        assert_eq!(response.status().as_u16(), 404);
    }
}

#[test]
fn occurrence_writes_reject_invalid_identity_context_time_and_amount_without_mutation() {
    let target = Target::from_env();
    let fixture = fixture();
    let (date, taken_at) = clock();
    let (schedule_id, medication_id) = create_schedule(&target, &fixture);
    let path = schedule_path(&fixture, schedule_id);
    let source_id = body(target.get(
        &format!(
            "/api/v1/households/{}/schedules/{schedule_id}",
            fixture.household_id
        ),
        Some(&fixture.access_token),
    ))["data"]["portable_id"]
        .as_str()
        .unwrap()
        .to_owned();
    let open = rows(&target, &path, &fixture.access_token, &date)[0].clone();
    let key = open["key"].clone();
    let before_stock = stock(&target, &fixture, medication_id);
    let before_takes = body(target.get(&takes_path(&fixture), Some(&fixture.access_token)))["meta"]
        ["total_count"]
        .clone();
    for (invalid_key, status) in [(json!("private-clinical-text"), 422), (json!(null), 422)] {
        let response = target.post_json_authorized(
            &format!("{path}/not_taken"),
            &fixture.access_token,
            &json!({"dose_occurrence": {"key": invalid_key, "reason": "unwell"}}),
        );
        assert_eq!(response.status().as_u16(), status);
        assert!(!body(response).to_string().contains("private-clinical-text"));
    }
    let response = target.post_json_authorized(
        &format!("{path}/not_taken"),
        &fixture.access_token,
        &json!({"dose_occurrence": {"key": key, "reason": "private-clinical-text"}}),
    );
    assert_eq!(response.status().as_u16(), 422);
    assert!(!body(response).to_string().contains("private-clinical-text"));
    for (time, amount) in [
        (json!("invalid"), json!("1.25")),
        (json!(taken_at), json!(1.25)),
        (json!(taken_at), json!("0")),
        (json!(taken_at), json!("-1")),
    ] {
        let response = target.post_json_authorized(
            &format!("{path}/take"),
            &fixture.access_token,
            &json!({"dose_occurrence": {"key": key, "taken_at": time, "dose_amount": amount,
                "taken_from_medication_id": medication_id}}),
        );
        assert_eq!(response.status().as_u16(), 422);
    }
    let response = target.post_json_authorized(
        &takes_path(&fixture),
        &fixture.access_token,
        &json!({"medication_take": {"source_type": "schedule", "source_id": source_id,
            "taken_at": taken_at, "dose_amount": 1.25}}),
    );
    assert_eq!(response.status().as_u16(), 422);
    let numeric_error = body(response);
    assert_eq!(numeric_error["error"]["code"], "validation_failed");
    assert_eq!(numeric_error["error"]["message"], "Validation failed");
    assert_eq!(numeric_error["error"]["errors"], json!({"dose_amount": ["must be a string"]}));
    for amount in ["0", "-1"] {
        let response = target.post_json_authorized(
            &takes_path(&fixture),
            &fixture.access_token,
            &json!({"medication_take": {"source_type": "schedule",
                "source_id": source_id,
                "taken_at": taken_at, "dose_amount": amount}}),
        );
        assert_eq!(response.status().as_u16(), 422);
    }
    assert_eq!(rows(&target, &path, &fixture.access_token, &date)[0], open);
    assert_eq!(stock(&target, &fixture, medication_id), before_stock);
    assert_eq!(
        body(target.get(&takes_path(&fixture), Some(&fixture.access_token)))["meta"]["total_count"],
        before_takes
    );
}

#[test]
fn malformed_and_future_keys_leave_schedule_and_assignment_writes_unchanged() {
    let target = Target::from_env();
    let fixture = fixture();
    let (date, taken_at) = clock();
    let tomorrow = (OffsetDateTime::now_utc().date() + time::Duration::days(1)).to_string();
    let (schedule_id, schedule_medication_id) = create_schedule(&target, &fixture);
    let (assignment_id, assignment_medication_id) = create_routine_assignment(&target, &fixture);
    for (path, medication_id) in [
        (schedule_path(&fixture, schedule_id), schedule_medication_id),
        (
            assignment_path(&fixture, assignment_id),
            assignment_medication_id,
        ),
    ] {
        let today = rows(&target, &path, &fixture.access_token, &date);
        let future = rows(&target, &path, &fixture.access_token, &tomorrow);
        assert!(!today.is_empty());
        assert!(!future.is_empty());
        assert!(future.iter().all(|row| row["due"] == false));
        let stock_before = stock(&target, &fixture, medication_id);
        let takes_before = take_snapshot(&target, &fixture);
        for key in [json!("private-clinical-text"), future[0]["key"].clone()] {
            let response = target.post_json_authorized(
                &format!("{path}/not_taken"),
                &fixture.access_token,
                &json!({"dose_occurrence": {"key": key, "reason": "unwell"}}),
            );
            assert_eq!(response.status().as_u16(), 422);
            assert_eq!(body(response)["error"]["code"], "invalid_occurrence");
            let response = target.post_json_authorized(
                &format!("{path}/take"),
                &fixture.access_token,
                &json!({"dose_occurrence": {"key": key, "taken_at": taken_at,
                    "taken_from_medication_id": medication_id}}),
            );
            assert_eq!(response.status().as_u16(), 422);
            assert_eq!(body(response)["error"]["code"], "invalid_occurrence");
        }
        assert_eq!(rows(&target, &path, &fixture.access_token, &date), today);
        assert_eq!(
            rows(&target, &path, &fixture.access_token, &tomorrow),
            future
        );
        assert_eq!(stock(&target, &fixture, medication_id), stock_before);
        assert_eq!(take_snapshot(&target, &fixture), takes_before);
    }
}
