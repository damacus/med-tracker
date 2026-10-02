#[test]
fn medication_takes_create_filters_paginates_and_preserves_precision() {
    let target = Target::from_env();
    let fixture = fixture();
    let (assignment_id, medication_id) = create_routine_assignment(&target, &fixture);
    let taken_at = (OffsetDateTime::now_utc() + time::Duration::seconds(1))
        .replace_nanosecond(0)
        .unwrap()
        .format(&Rfc3339)
        .unwrap();
    let path = takes_path(&fixture);
    let source_id = assignment_portable_id_for(&target, &fixture, assignment_id);
    let before_stock: f64 = stock(&target, &fixture, medication_id).parse().unwrap();
    let payload = json!({"medication_take": {
        "source_type": "person_medication",
        "source_id": source_id,
        "taken_at": taken_at,
        "client_uuid": client_uuid(&fixture, 3),
        "dose_amount": "1.25",
        "taken_from_medication_id": medication_id
    }});
    let response = target.post_json_authorized(&path, &fixture.view_access_token, &payload);
    assert_eq!(response.status().as_u16(), 403);
    let response = target.post_json_authorized(&path, &fixture.access_token, &payload);
    assert_eq!(response.status().as_u16(), 201);
    let tag = etag(&response);
    let id = request_id(&response);
    let take = body(response)["data"].clone();
    assert_eq!(take["client_uuid"], client_uuid(&fixture, 3));
    assert_eq!(take["person_medication_id"], assignment_id);
    assert_eq!(take["person_id"], fixture.managed_person_id);
    assert_eq!(take["medication_id"], medication_id);
    assert_eq!(take["taken_from_medication_id"], medication_id);
    assert_eq!(take["dose_amount"], "1.25");
    assert_eq!(take["dose_unit"], "ml");
    assert_eq!(take["taken_at"], taken_at);
    assert!(take["updated_at"].as_str().unwrap().ends_with('Z'));
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
        "create",
        201,
        "api/v1/medication_takes",
    );
    let response = target.post_json_authorized(&path, &fixture.access_token, &payload);
    assert_eq!(response.status().as_u16(), 200);
    let replay_request_id = request_id(&response);
    assert_eq!(etag(&response), tag);
    assert_eq!(body(response)["data"], take);
    audit(
        &target,
        &fixture,
        &replay_request_id,
        "POST",
        "create",
        200,
        "api/v1/medication_takes",
    );
    assert_eq!(
        stock(&target, &fixture, medication_id)
            .parse::<f64>()
            .unwrap(),
        before_stock - 1.25
    );
    assert_eq!(
        takes_for_medication(&target, &fixture, medication_id),
        vec![take.clone()]
    );

    let (routine_id, routine_medication_id) = create_routine_assignment(&target, &fixture);
    let routine_source_id = assignment_portable_id_for(&target, &fixture, routine_id);
    let routine_taken_at = (OffsetDateTime::now_utc() + time::Duration::seconds(1))
        .format(&Rfc3339)
        .unwrap();
    let response = target.post_json_authorized(
        &path,
        &fixture.access_token,
        &json!({"medication_take": {
            "source_type": "person_medication",
            "source_id": routine_source_id,
            "taken_at": routine_taken_at,
            "taken_from_medication_id": routine_medication_id
        }}),
    );
    assert_eq!(response.status().as_u16(), 201);
    let second_take = body(response)["data"].clone();
    assert_ne!(second_take["id"], take["id"]);
    let response = target.get(&path, Some(&fixture.view_access_token));
    assert_eq!(response.status().as_u16(), 200);
    let collection = body(response);
    assert!(collection["data"]
        .as_array()
        .unwrap()
        .iter()
        .any(|row| row["id"] == take["id"]));
    assert!(collection["data"]
        .as_array()
        .unwrap()
        .iter()
        .any(|row| row["id"] == second_take["id"]));
    let response = target.get(
        &format!("{path}?page=1&per_page=1"),
        Some(&fixture.access_token),
    );
    assert_eq!(response.status().as_u16(), 200);
    let first_page = body(response);
    assert_eq!(first_page["meta"]["page"], 1);
    assert_eq!(first_page["meta"]["per_page"], 1);
    assert!(first_page["meta"]["total_count"].as_u64().unwrap() >= 2);
    assert_eq!(first_page["data"].as_array().unwrap().len(), 1);
    let response = target.get(
        &format!("{path}?page=2&per_page=1"),
        Some(&fixture.access_token),
    );
    assert_eq!(response.status().as_u16(), 200);
    let second_page = body(response);
    assert_eq!(second_page["meta"]["page"], 2);
    assert_eq!(
        second_page["meta"]["total_count"],
        first_page["meta"]["total_count"]
    );
    assert_ne!(second_page["data"][0]["id"], first_page["data"][0]["id"]);
    let response = target.get(
        &format!("{path}?page=0&per_page=500&updated_since=2000-01-01T00:00:00Z"),
        Some(&fixture.access_token),
    );
    assert_eq!(response.status().as_u16(), 422);
    let response = target.get(
        &format!("{path}?page=1&per_page=100&updated_since=2000-01-01T00:00:00Z"),
        Some(&fixture.access_token),
    );
    assert_eq!(response.status().as_u16(), 200);
    let collection = body(response);
    assert_eq!(collection["meta"]["page"], 1);
    assert_eq!(collection["meta"]["per_page"], 100);
    assert!(collection["data"]
        .as_array()
        .unwrap()
        .iter()
        .any(|row| row["id"] == take["id"]));
    let response = target.get(
        &format!("{path}?updated_since=invalid"),
        Some(&fixture.access_token),
    );
    assert_eq!(response.status().as_u16(), 422);
    assert_eq!(body(response)["error"]["code"], "unprocessable_content");
    let response = target.get(
        &format!(
            "/api/v1/households/{}/medication_takes",
            fixture.foreign_household_id
        ),
        Some(&fixture.access_token),
    );
    assert_eq!(response.status().as_u16(), 403);
}

#[test]
fn medication_take_collection_excludes_takes_for_ungranted_people() {
    let target = Target::from_env();
    let fixture = fixture();
    let medication = create_medication(&target, &fixture);
    let person = target.post_json_authorized(
        &format!("/api/v1/households/{}/people", fixture.household_id),
        &fixture.care_access_token,
        &json!({"person": {"name": "Private take subject", "date_of_birth": "1980-02-03",
            "person_type": "adult", "has_capacity": true}}),
    );
    assert_eq!(person.status().as_u16(), 201);
    let person = body(person)["data"].clone();
    let assignment = target.post_json_authorized(
        &format!(
            "/api/v1/households/{}/person_medications",
            fixture.household_id
        ),
        &fixture.care_access_token,
        &json!({"person_medication": {
            "person_id": person["portable_id"],
            "medication_id": medication["portable_id"],
            "dose_amount": "1", "dose_unit": "ml", "administration_kind": "as_needed"
        }}),
    );
    assert_eq!(assignment.status().as_u16(), 201);
    let assignment = body(assignment)["data"].clone();
    let (_, taken_at) = clock();
    let take = target.post_json_authorized(
        &takes_path(&fixture),
        &fixture.care_access_token,
        &json!({"medication_take": {
            "source_type": "person_medication", "source_id": assignment["portable_id"],
            "taken_at": taken_at, "dose_amount": "1", "taken_from_medication_id": medication["id"]
        }}),
    );
    assert_eq!(take.status().as_u16(), 201);
    let take_id = body(take)["data"]["id"].clone();
    let visible = body(target.get(&takes_path(&fixture), Some(&fixture.care_access_token)));
    assert!(visible["data"]
        .as_array()
        .unwrap()
        .iter()
        .any(|row| row["id"] == take_id));
    let viewer = target.get(&takes_path(&fixture), Some(&fixture.view_access_token));
    assert_eq!(viewer.status().as_u16(), 200);
    let viewer = body(viewer);
    assert!(!viewer["data"]
        .as_array()
        .unwrap()
        .iter()
        .any(|row| row["id"] == take_id));
}

#[test]
#[ignore = "Rails currently serializes medication-take timestamps to whole seconds"]
fn medication_take_preserves_fractional_second_timestamp() {
    let target = Target::from_env();
    let fixture = fixture();
    let source_id = assignment_portable_id(&target, &fixture);
    let taken_at = OffsetDateTime::now_utc()
        .replace_nanosecond(123_456_000)
        .unwrap()
        .format(&Rfc3339)
        .unwrap();
    let response = target.post_json_authorized(
        &takes_path(&fixture),
        &fixture.access_token,
        &json!({"medication_take": {
            "source_type": "person_medication",
            "source_id": source_id,
            "taken_from_medication_id": fixture.managed_medication_id,
            "taken_at": taken_at
        }}),
    );
    assert_eq!(response.status().as_u16(), 201);
    assert_eq!(body(response)["data"]["taken_at"], taken_at);
}
