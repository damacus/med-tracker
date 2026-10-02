#[test]
fn timed_schedule_and_routine_cycle_windows_project_only_expected_occurrences() {
    let target = Target::from_env();
    let fixture = fixture();
    let (date, _) = clock();
    let (schedule_id, _) = create_schedule(&target, &fixture);
    let path = schedule_path(&fixture, schedule_id);
    let response = target.patch_json(
        &format!(
            "/api/v1/households/{}/schedules/{schedule_id}",
            fixture.household_id
        ),
        &fixture.access_token,
        &json!({"schedule": {"schedule_config": {"times": ["08:00", "20:00"]}}}),
    );
    assert_eq!(response.status().as_u16(), 200);
    let timed = rows(&target, &path, &fixture.view_access_token, &date);
    assert_eq!(timed.len(), 2);
    assert_ne!(timed[0]["key"], timed[1]["key"]);
    assert_eq!(timed[0]["position"], 1);
    assert_eq!(timed[1]["position"], 2);
    for (row, clock_time) in timed.iter().zip(["08:00:00", "20:00:00"]) {
        let scheduled_at = row["scheduled_at"].as_str().unwrap();
        assert!(
            scheduled_at.starts_with(&format!("{date}T{clock_time}")),
            "unexpected local schedule time: {scheduled_at}"
        );
        assert!(scheduled_at.len() > 19);
    }
    assert_eq!(
        rows(&target, &path, &fixture.view_access_token, &date),
        timed
    );
    let response = target.patch_json(
        &format!(
            "/api/v1/households/{}/schedules/{schedule_id}",
            fixture.household_id
        ),
        &fixture.access_token,
        &json!({"schedule": {"frequency": "As needed"}}),
    );
    assert_eq!(response.status().as_u16(), 200);
    assert!(rows(&target, &path, &fixture.view_access_token, &date).is_empty());

    let (assignment_id, _) = create_routine_assignment(&target, &fixture);
    let assignment = assignment_path(&fixture, assignment_id);
    let source = format!(
        "/api/v1/households/{}/person_medications/{assignment_id}",
        fixture.household_id
    );
    let today = OffsetDateTime::now_utc().date();
    for cycle in ["weekly", "monthly"] {
        let response = target.patch_json(
            &source,
            &fixture.access_token,
            &json!({"person_medication": {"dose_cycle": cycle}}),
        );
        assert_eq!(response.status().as_u16(), 200);
        let projected = rows(&target, &assignment, &fixture.view_access_token, &date);
        assert_eq!(projected.len(), 1);
        let start = if cycle == "weekly" {
            today - time::Duration::days(today.weekday().number_days_from_monday().into())
        } else {
            time::Date::from_calendar_date(today.year(), today.month(), 1).unwrap()
        };
        let end = if cycle == "weekly" {
            start + time::Duration::days(6)
        } else {
            let next = if today.month() == time::Month::December {
                time::Date::from_calendar_date(today.year() + 1, time::Month::January, 1).unwrap()
            } else {
                time::Date::from_calendar_date(today.year(), today.month().next(), 1).unwrap()
            };
            next - time::Duration::days(1)
        };
        assert_eq!(projected[0]["window_starts_on"], start.to_string());
        assert_eq!(projected[0]["window_ends_on"], end.to_string());
        assert_eq!(projected[0]["expected"], true);
        assert_eq!(
            rows(&target, &assignment, &fixture.view_access_token, &date),
            projected
        );
    }
    let response = target.patch_json(
        &source,
        &fixture.access_token,
        &json!({"person_medication": {"administration_kind": "as_needed"}}),
    );
    assert_eq!(response.status().as_u16(), 200);
    assert!(rows(&target, &assignment, &fixture.view_access_token, &date).is_empty());
}

#[test]
fn future_occurrences_and_changed_cycle_keys_cannot_be_resolved() {
    let target = Target::from_env();
    let fixture = fixture();
    let (date, _) = clock();
    let tomorrow = (OffsetDateTime::now_utc().date() + time::Duration::days(1)).to_string();
    let (schedule_id, medication_id) = create_schedule(&target, &fixture);
    let path = schedule_path(&fixture, schedule_id);
    let future = rows(&target, &path, &fixture.access_token, &tomorrow);
    assert!(!future.is_empty());
    assert!(future.iter().all(|row| row["due"] == false));
    let before_stock = stock(&target, &fixture, medication_id);
    let response = target.post_json_authorized(
        &format!("{path}/not_taken"),
        &fixture.access_token,
        &json!({"dose_occurrence": {"key": future[0]["key"], "reason": "unwell"}}),
    );
    assert_eq!(response.status().as_u16(), 422);
    assert_eq!(body(response)["error"]["code"], "invalid_occurrence");
    assert_eq!(stock(&target, &fixture, medication_id), before_stock);

    let (assignment_id, assignment_medication_id) = create_routine_assignment(&target, &fixture);
    let assignment = assignment_path(&fixture, assignment_id);
    let daily = rows(&target, &assignment, &fixture.access_token, &date)[0].clone();
    let response = target.post_json_authorized(
        &format!("{assignment}/not_taken"),
        &fixture.access_token,
        &json!({"dose_occurrence": {"key": daily["key"], "reason": "unwell", "note": "Retained"}}),
    );
    assert_eq!(response.status().as_u16(), 200);
    let saved = body(response)["data"].clone();
    let source = format!(
        "/api/v1/households/{}/person_medications/{assignment_id}",
        fixture.household_id
    );
    let response = target.patch_json(
        &source,
        &fixture.access_token,
        &json!({"person_medication": {"administration_kind": "as_needed"}}),
    );
    assert_eq!(response.status().as_u16(), 200);
    let historical = rows(&target, &assignment, &fixture.view_access_token, &date);
    assert_eq!(historical.len(), 1);
    assert_eq!(historical[0]["key"], saved["key"]);
    assert_eq!(historical[0]["outcome"], "not_taken");
    assert_eq!(historical[0]["reason"], "unwell");
    assert_eq!(historical[0]["note"], "Retained");
    assert_eq!(historical[0]["expected"], false);
    let response = target.post_json_authorized(
        &format!("{assignment}/not_taken"),
        &fixture.access_token,
        &json!({"dose_occurrence": {"key": daily["key"], "reason": "refused"}}),
    );
    assert_eq!(response.status().as_u16(), 409);
    assert_eq!(
        rows(&target, &assignment, &fixture.access_token, &date),
        historical
    );
    assert_eq!(stock(&target, &fixture, assignment_medication_id), "20.0");

    let (other_id, _) = create_routine_assignment(&target, &fixture);
    let other = assignment_path(&fixture, other_id);
    let former_key = rows(&target, &other, &fixture.access_token, &date)[0]["key"].clone();
    let response = target.patch_json(
        &format!(
            "/api/v1/households/{}/person_medications/{other_id}",
            fixture.household_id
        ),
        &fixture.access_token,
        &json!({"person_medication": {"administration_kind": "as_needed"}}),
    );
    assert_eq!(response.status().as_u16(), 200);
    let response = target.post_json_authorized(
        &format!("{other}/not_taken"),
        &fixture.access_token,
        &json!({"dose_occurrence": {"key": former_key, "reason": "unwell"}}),
    );
    assert_eq!(response.status().as_u16(), 422);
    assert!(rows(&target, &other, &fixture.access_token, &date).is_empty());
}
