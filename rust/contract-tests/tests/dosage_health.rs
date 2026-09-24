use medtracker_contract_tests::{fixture, Fixture, Target};
use reqwest::blocking::Response;
use serde_json::{json, Value};

fn body(response: Response) -> Value {
    response.json().expect("JSON response")
}

fn etag(response: &Response) -> String {
    response.headers()["etag"]
        .to_str()
        .expect("ETag header")
        .to_owned()
}

fn assert_utc_second_timestamp(value: &Value) {
    let timestamp = value.as_str().expect("timestamp string");
    assert_eq!(timestamp.len(), 20);
    assert_eq!(&timestamp[10..11], "T");
    assert!(timestamp.ends_with('Z'));
}

fn dosage_path(fixture: &Fixture) -> String {
    format!("/api/v1/households/{}/dosage_options", fixture.household_id)
}

fn health_path(fixture: &Fixture) -> String {
    format!("/api/v1/households/{}/health_events", fixture.household_id)
}

fn paginated_ids(target: &Target, path: &str, token: &str) -> Vec<i64> {
    let first = target.get(&format!("{path}?page=1&per_page=1"), Some(token));
    assert_eq!(first.status().as_u16(), 200);
    let first_page = body(first);
    let total = first_page["meta"]["total_count"].as_u64().unwrap();
    assert_eq!(first_page["meta"]["page"], 1);
    assert_eq!(first_page["meta"]["per_page"], 1);
    let mut ids = Vec::new();
    for page in 1..=total {
        let collection = if page == 1 {
            first_page.clone()
        } else {
            let response = target.get(&format!("{path}?page={page}&per_page=1"), Some(token));
            assert_eq!(response.status().as_u16(), 200);
            body(response)
        };
        assert_eq!(collection["meta"]["page"], page);
        assert_eq!(collection["meta"]["per_page"], 1);
        assert_eq!(collection["meta"]["total_count"], total);
        assert_eq!(collection["data"].as_array().unwrap().len(), 1);
        ids.push(collection["data"][0]["id"].as_i64().unwrap());
    }
    ids
}

fn assert_audit_action(target: &Target, fixture: &Fixture, controller: &str, action: &str) {
    let path = format!(
        "/api/v1/households/{}/admin/audit_logs",
        fixture.household_id
    );
    let response = target.get(&path, Some(&fixture.access_token));
    assert_eq!(response.status().as_u16(), 200);
    let audit = body(response);
    assert!(audit["data"].as_array().unwrap().iter().any(|event| {
        event["event_type"] == "api.request"
            && event["metadata"]["controller"] == controller
            && event["metadata"]["action"] == action
            && event["metadata"]["status"] == 200
    }));
}

fn create_dosage(
    target: &Target,
    fixture: &Fixture,
    medication_portable_id: &str,
) -> (Value, String) {
    let response = target.post_json_authorized(
        &dosage_path(fixture),
        &fixture.access_token,
        &json!({"dosage_option": {
            "medication_id": medication_portable_id,
            "amount": "2.125", "unit": "ml", "frequency": "Twice daily",
            "description": "Contract liquid dose", "default_for_adults": true,
            "default_max_daily_doses": 2, "default_min_hours_between_doses": "8.5",
            "default_dose_cycle": "daily", "current_supply": "10.25",
            "reorder_threshold": "2.5"
        }}),
    );
    assert_eq!(response.status().as_u16(), 201);
    let tag = etag(&response);
    let created = body(response)["data"].clone();
    assert_eq!(created["medication_portable_id"], medication_portable_id);
    assert_eq!(created["amount"], "2.125");
    assert_eq!(created["unit"], "ml");
    assert_eq!(created["frequency"], "Twice daily");
    assert_eq!(created["description"], "Contract liquid dose");
    assert_eq!(created["default_for_adults"], true);
    assert_eq!(created["default_for_children"], false);
    assert_eq!(created["default_max_daily_doses"], 2);
    assert_eq!(created["default_min_hours_between_doses"], "8.5");
    assert_eq!(created["default_dose_cycle"], "daily");
    (created, tag)
}

#[test]
fn dosage_options_cover_collection_identity_etags_and_retained_updates() {
    let target = Target::from_env();
    let fixture = fixture();
    let base = dosage_path(&fixture);
    let medication_path = format!("/api/v1/households/{}/medications", fixture.household_id);
    let response = target.post_json_authorized(
        &medication_path,
        &fixture.access_token,
        &json!({"medication": {"name": "Contract dosage catalog medicine",
            "location_id": fixture.primary_location_id, "dose_amount": "1", "dose_unit": "ml"}}),
    );
    assert_eq!(response.status().as_u16(), 201);
    let medication = body(response)["data"].clone();
    let medication_id = medication["id"].as_i64().unwrap();
    let medication_portable_id = medication["portable_id"].as_str().unwrap();
    let (created, initial_tag) = create_dosage(&target, &fixture, medication_portable_id);
    let path = format!("{base}/{}", created["portable_id"].as_str().unwrap());
    assert_eq!(created["medication_id"], medication_id);
    assert_eq!(created["medication_portable_id"], medication_portable_id);
    assert_eq!(created["amount"], "2.125");
    assert_eq!(created["default_min_hours_between_doses"], "8.5");
    assert_eq!(created["current_supply"], "10.25");
    assert_eq!(created["reorder_threshold"], "2.5");
    assert_utc_second_timestamp(&created["updated_at"]);

    let response = target.get(&path, Some(&fixture.access_token));
    assert_eq!(response.status().as_u16(), 200);
    assert_eq!(etag(&response), initial_tag);
    assert_eq!(body(response)["data"], created);
    let ids = paginated_ids(&target, &base, &fixture.access_token);
    assert!(ids.contains(&created["id"].as_i64().unwrap()));
    assert!(ids.contains(&fixture.hidden_dosage_id));
    assert!(!ids.contains(&fixture.foreign_dosage_id));
    assert!(ids.len() >= 2);

    let response = target.patch_json_if_match(
        &path,
        &fixture.access_token,
        &json!({"dosage_option": {"amount": "3.125", "frequency": "Every 8 hours"}}),
        "\"stale-etag\"",
    );
    assert_eq!(response.status().as_u16(), 409);
    let response = target.patch_json_if_match(
        &path,
        &fixture.access_token,
        &json!({"dosage_option": {"amount": "3.125", "frequency": "Every 8 hours"}}),
        &initial_tag,
    );
    assert_eq!(response.status().as_u16(), 200);
    let next_tag = etag(&response);
    let patched = body(response)["data"].clone();
    assert_eq!(patched["amount"], "3.125");
    assert_eq!(patched["medication_id"], created["medication_id"]);
    let response = target.put_json_if_match(
        &path,
        &fixture.access_token,
        &json!({"dosage_option": {"description": "Replacement", "current_supply": "8.75"}}),
        &next_tag,
    );
    assert_eq!(response.status().as_u16(), 200);
    let replaced = body(response)["data"].clone();
    assert_eq!(replaced["description"], "Replacement");
    assert_eq!(replaced["current_supply"], "8.75");
    assert_eq!(replaced["amount"], "3.125");
    let response = target.get(&path, Some(&fixture.access_token));
    assert_eq!(body(response)["data"], replaced);
    assert_audit_action(&target, &fixture, "api/v1/dosage_options", "update");
}

#[test]
fn dosage_options_reject_invalid_and_foreign_records_and_track_selected_stock() {
    let target = Target::from_env();
    let fixture = fixture();
    let base = dosage_path(&fixture);
    let response = target.post_json_authorized(
        &base,
        &fixture.access_token,
        &json!({"dosage_option": {"medication_id": fixture.managed_medication_id.to_string(),
            "amount": null, "unit": null, "frequency": null}}),
    );
    assert_eq!(response.status().as_u16(), 422);
    let response = target.post_json_authorized(
        &base,
        &fixture.access_token,
        &json!({"dosage_option": {"medication_id": fixture.foreign_medication_id.to_string(),
            "amount": "1", "unit": "ml", "frequency": "daily"}}),
    );
    assert_eq!(response.status().as_u16(), 404);
    let foreign = format!("{base}/{}", fixture.foreign_dosage_id);
    let response = target.get(&foreign, Some(&fixture.access_token));
    assert_eq!(response.status().as_u16(), 404);
    assert!(!body(response)
        .to_string()
        .contains(&fixture.foreign_medication_name));
    let response = target.patch_json(
        &foreign,
        &fixture.access_token,
        &json!({"dosage_option": {"amount": "5"}}),
    );
    assert_eq!(response.status().as_u16(), 404);
    let response = target.put_json(
        &foreign,
        &fixture.access_token,
        &json!({"dosage_option": {"amount": "5"}}),
    );
    assert_eq!(response.status().as_u16(), 404);

    let (created, _) = create_dosage(&target, &fixture, &fixture.managed_medication_portable_id);
    let path = format!("{base}/{}", created["id"]);
    assert_eq!(created["medication_id"], fixture.managed_medication_id);
    let viewer_ids = paginated_ids(&target, &base, &fixture.view_access_token);
    assert!(viewer_ids.contains(&created["id"].as_i64().unwrap()));
    assert!(!viewer_ids.contains(&fixture.foreign_dosage_id));
    let hidden_path = format!("{base}/{}", fixture.hidden_dosage_id);
    let response = target.get(&hidden_path, Some(&fixture.view_access_token));
    assert_eq!(response.status().as_u16(), 403);
    let response = target.get(&path, Some(&fixture.view_access_token));
    assert_eq!(response.status().as_u16(), 200);
    let response = target.patch_json(
        &path,
        &fixture.view_access_token,
        &json!({"dosage_option": {"amount": "4"}}),
    );
    assert_eq!(response.status().as_u16(), 403);
    let response = target.patch_json(
        &path,
        &fixture.access_token,
        &json!({"dosage_option": {"amount": null}}),
    );
    assert_eq!(response.status().as_u16(), 422);
    let response = target.get(&path, Some(&fixture.access_token));
    assert_eq!(body(response)["data"]["amount"], "2.125");

    let removal = format!(
        "/api/v1/households/{}/medications/{}/stock_removals",
        fixture.household_id, fixture.managed_medication_id
    );
    let response = target.post_json_authorized(
        &removal,
        &fixture.access_token,
        &json!({"stock_removal": {"quantity": "1.25", "dosage_id": created["id"].to_string(),
            "reason": "dropped", "submission_id": "b2000000-0000-4000-8000-000000000001"}}),
    );
    assert_eq!(response.status().as_u16(), 201);
    let removal = body(response)["data"].clone();
    assert_eq!(removal["dosage_id"], created["id"].to_string());
    assert_eq!(removal["remaining_quantity"], "9");
    let response = target.get(&path, Some(&fixture.access_token));
    assert_eq!(body(response)["data"]["current_supply"], "9.0");
    let medication = format!(
        "/api/v1/households/{}/medications/{}",
        fixture.household_id, fixture.managed_medication_id
    );
    let response = target.get(&medication, Some(&fixture.access_token));
    assert_eq!(body(response)["data"]["current_supply"], "9.0");
}

#[test]
#[ignore = "Rails currently exposes hidden dosage options in viewer lists; privacy contract for Rust"]
fn dosage_option_collection_hides_ungranted_medication() {
    let target = Target::from_env();
    let fixture = fixture();
    let ids = paginated_ids(&target, &dosage_path(&fixture), &fixture.view_access_token);
    assert!(!ids.contains(&fixture.hidden_dosage_id));
}

fn create_health_event(
    target: &Target,
    fixture: &Fixture,
    person_portable_id: &str,
    title: &str,
) -> (Value, String) {
    let response = target.post_json_authorized(
        &health_path(fixture),
        &fixture.access_token,
        &json!({"health_event": {"person_id": person_portable_id, "event_kind": "illness",
            "severity": "mild", "title": title, "notes": "Managed at home",
            "started_on": "2026-02-25", "medication_ids": [fixture.managed_medication_portable_id]}}),
    );
    assert_eq!(response.status().as_u16(), 201);
    let tag = etag(&response);
    let created = body(response)["data"].clone();
    assert_eq!(created["person_portable_id"], person_portable_id);
    assert_eq!(
        created["medication_portable_ids"],
        json!([fixture.managed_medication_portable_id])
    );
    assert_eq!(created["event_kind"], "illness");
    assert_eq!(created["severity"], "mild");
    assert_eq!(created["title"], title);
    assert_eq!(created["notes"], "Managed at home");
    assert_eq!(created["started_on"], "2026-02-25");
    assert_eq!(created["ended_on"], Value::Null);
    (created, tag)
}

#[test]
fn health_events_cover_portable_relations_pagination_etags_and_retained_updates() {
    let target = Target::from_env();
    let fixture = fixture();
    let base = health_path(&fixture);
    let (created, initial_tag) = create_health_event(
        &target,
        &fixture,
        &fixture.managed_person_portable_id,
        "Contract cold",
    );
    let path = format!("{base}/{}", created["portable_id"].as_str().unwrap());
    assert_eq!(
        created["person_portable_id"],
        fixture.managed_person_portable_id
    );
    assert_eq!(created["medication_ids"][0], fixture.managed_medication_id);
    assert_eq!(
        created["medication_portable_ids"],
        json!([fixture.managed_medication_portable_id])
    );
    assert_eq!(created["started_on"], "2026-02-25");
    assert_utc_second_timestamp(&created["updated_at"]);
    let response = target.get(&path, Some(&fixture.access_token));
    assert_eq!(response.status().as_u16(), 200);
    assert_eq!(etag(&response), initial_tag);
    assert_eq!(body(response)["data"], created);
    let ids = paginated_ids(&target, &base, &fixture.access_token);
    assert!(ids.contains(&created["id"].as_i64().unwrap()));
    assert!(ids.contains(&fixture.hidden_health_event_id));
    assert!(!ids.contains(&fixture.foreign_health_event_id));
    let response = target.patch_json_if_match(
        &path,
        &fixture.access_token,
        &json!({"health_event": {"title": "Recovering"}}),
        "\"stale-etag\"",
    );
    assert_eq!(response.status().as_u16(), 409);
    let response = target.patch_json_if_match(
        &path,
        &fixture.access_token,
        &json!({"health_event": {"title": "Recovering"}}),
        &initial_tag,
    );
    assert_eq!(response.status().as_u16(), 200);
    let next_tag = etag(&response);
    assert_eq!(body(response)["data"]["title"], "Recovering");
    let response = target.put_json_if_match(
        &path,
        &fixture.access_token,
        &json!({"health_event": {"ended_on": "2026-02-27", "severity": "moderate"}}),
        &next_tag,
    );
    assert_eq!(response.status().as_u16(), 200);
    let replaced = body(response)["data"].clone();
    assert_eq!(replaced["ended_on"], "2026-02-27");
    assert_eq!(replaced["severity"], "moderate");
    assert_eq!(replaced["title"], "Recovering");
    assert_eq!(replaced["medication_ids"], json!([]));
    assert_eq!(replaced["medication_portable_ids"], json!([]));
    let response = target.get(&path, Some(&fixture.access_token));
    assert_eq!(body(response)["data"], replaced);
    assert_audit_action(&target, &fixture, "api/v1/health_events", "update");
}

#[test]
fn health_events_reject_invalid_cross_household_and_person_scope_changes() {
    let target = Target::from_env();
    let fixture = fixture();
    let base = health_path(&fixture);
    let response = target.post_json_authorized(
        &base,
        &fixture.access_token,
        &json!({"health_event": {"person_id": fixture.managed_person_id.to_string(),
            "event_kind": "illness", "title": "", "started_on": ""}}),
    );
    assert_eq!(response.status().as_u16(), 422);
    let response = target.post_json_authorized(
        &base,
        &fixture.access_token,
        &json!({"health_event": {"person_id": fixture.foreign_person_portable_id,
            "event_kind": "illness", "title": "Foreign", "started_on": "2026-02-25"}}),
    );
    assert_eq!(response.status().as_u16(), 404);
    let (managed, _) = create_health_event(
        &target,
        &fixture,
        &fixture.managed_person_portable_id,
        "Visible cold",
    );
    let managed_path = format!("{base}/{}", managed["id"]);
    let hidden_path = format!("{base}/{}", fixture.hidden_health_event_id);
    let foreign_path = format!("{base}/{}", fixture.foreign_health_event_id);
    let visible_ids = paginated_ids(&target, &base, &fixture.view_access_token);
    assert!(visible_ids.contains(&managed["id"].as_i64().unwrap()));
    assert!(!visible_ids.contains(&fixture.hidden_health_event_id));
    assert!(!visible_ids.contains(&fixture.foreign_health_event_id));
    let response = target.get(&managed_path, Some(&fixture.view_access_token));
    assert_eq!(response.status().as_u16(), 200);
    let response = target.get(&hidden_path, Some(&fixture.view_access_token));
    assert_eq!(response.status().as_u16(), 404);
    assert!(!body(response).to_string().contains("Contract hidden event"));
    let response = target.get(&foreign_path, Some(&fixture.access_token));
    assert_eq!(response.status().as_u16(), 404);
    assert!(!body(response)
        .to_string()
        .contains("Contract foreign event"));
    let response = target.patch_json(
        &foreign_path,
        &fixture.access_token,
        &json!({"health_event": {"title": "Changed"}}),
    );
    assert_eq!(response.status().as_u16(), 404);
    let response = target.put_json(
        &foreign_path,
        &fixture.access_token,
        &json!({"health_event": {"title": "Changed"}}),
    );
    assert_eq!(response.status().as_u16(), 404);
    let response = target.patch_json(
        &managed_path,
        &fixture.view_access_token,
        &json!({"health_event": {"title": "Changed"}}),
    );
    assert_eq!(response.status().as_u16(), 403);
    let response = target.post_json_authorized(
        &base,
        &fixture.view_access_token,
        &json!({"health_event": {"person_id": fixture.managed_person_id.to_string(),
            "event_kind": "illness", "title": "Viewer event", "started_on": "2026-02-25"}}),
    );
    assert_eq!(response.status().as_u16(), 403);
    let response = target.patch_json(
        &managed_path,
        &fixture.access_token,
        &json!({"health_event": {"ended_on": "2026-02-24"}}),
    );
    assert_eq!(response.status().as_u16(), 422);
    let response = target.get(&managed_path, Some(&fixture.access_token));
    assert_eq!(body(response)["data"]["ended_on"], Value::Null);
}

#[test]
fn dosage_filters_and_invalid_replacements_preserve_medication_link() {
    let target = Target::from_env();
    let fixture = fixture();
    let base = dosage_path(&fixture);
    let path = format!("{base}/{}", fixture.hidden_dosage_id);
    let response = target.get(&path, Some(&fixture.access_token));
    assert_eq!(response.status().as_u16(), 200);
    let tag = etag(&response);
    let created = body(response)["data"].clone();

    let response = target.get(
        &format!("{base}?updated_since=1970-01-01T00%3A00%3A00Z&per_page=100"),
        Some(&fixture.access_token),
    );
    assert_eq!(response.status().as_u16(), 200);
    assert!(body(response)["data"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["id"] == created["id"]));
    let response = target.get(
        &format!("{base}?updated_since=2099-01-01T00%3A00%3A00Z"),
        Some(&fixture.access_token),
    );
    assert_eq!(response.status().as_u16(), 200);
    assert_eq!(body(response)["meta"]["total_count"], 0);
    let response = target.get(
        &format!("{base}?updated_since=invalid"),
        Some(&fixture.access_token),
    );
    assert_eq!(response.status().as_u16(), 422);

    let response = target.put_json_if_match(
        &path,
        &fixture.access_token,
        &json!({"dosage_option": {"amount": "3"}}),
        "\"stale-etag\"",
    );
    assert_eq!(response.status().as_u16(), 409);
    let response = target.put_json_if_match(
        &path,
        &fixture.access_token,
        &json!({"dosage_option": {"amount": null}}),
        &tag,
    );
    assert_eq!(response.status().as_u16(), 422);
    let response = target.patch_json(
        &path,
        &fixture.access_token,
        &json!({"dosage_option": {"amount": 3.125}}),
    );
    assert_eq!(response.status().as_u16(), 422);
    assert_eq!(
        body(response)["error"]["errors"]["amount"][0],
        "must be a string"
    );
    let response = target.patch_json(
        &path,
        &fixture.access_token,
        &json!({"dosage_option": {"medication_id": fixture.foreign_medication_portable_id}}),
    );
    assert_eq!(response.status().as_u16(), 404);
    let response = target.get(&path, Some(&fixture.access_token));
    assert_eq!(response.status().as_u16(), 200);
    assert_eq!(etag(&response), tag);
    assert_eq!(body(response)["data"], created);
}

#[test]
fn dosage_and_health_reject_json_numbers_and_view_only_writes() {
    let target = Target::from_env();
    let fixture = fixture();
    let dosage_base = dosage_path(&fixture);
    let response = target.post_json_authorized(
        &dosage_base,
        &fixture.access_token,
        &json!({"dosage_option": {"medication_id": fixture.managed_medication_portable_id,
            "amount": 2.125, "unit": "ml", "frequency": "daily"}}),
    );
    assert_eq!(response.status().as_u16(), 422);
    assert_eq!(
        body(response)["error"]["errors"]["amount"][0],
        "must be a string"
    );
    let response = target.post_json_authorized(
        &dosage_base,
        &fixture.view_access_token,
        &json!({"dosage_option": {"medication_id": fixture.managed_medication_portable_id,
            "amount": "1", "unit": "ml", "frequency": "daily"}}),
    );
    assert_eq!(response.status().as_u16(), 403);

    let health_base = health_path(&fixture);
    let response = target.post_json_authorized(
        &health_base,
        &fixture.access_token,
        &json!({"health_event": {"person_id": fixture.managed_person_id,
            "event_kind": "illness", "title": "Numeric person", "started_on": "2026-02-25"}}),
    );
    assert_eq!(response.status().as_u16(), 422);
    assert_eq!(
        body(response)["error"]["errors"]["person_id"][0],
        "must be a string"
    );
    let response = target.post_json_authorized(
        &health_base,
        &fixture.access_token,
        &json!({"health_event": {"person_id": fixture.managed_person_portable_id,
            "event_kind": "illness", "title": "Numeric link", "started_on": "2026-02-25",
            "medication_ids": [fixture.managed_medication_id]}}),
    );
    assert_eq!(response.status().as_u16(), 422);
    assert_eq!(
        body(response)["error"]["errors"]["medication_ids"][0],
        "must be a string"
    );
}

#[test]
fn health_filters_replacements_and_medication_links_preserve_person() {
    let target = Target::from_env();
    let fixture = fixture();
    let base = health_path(&fixture);
    let (created, initial_tag) = create_health_event(
        &target,
        &fixture,
        &fixture.managed_person_portable_id,
        "Contract linked illness",
    );
    let path = format!("{base}/{}", created["portable_id"].as_str().unwrap());
    let response = target.get(
        &format!("{base}?updated_since=1970-01-01T00%3A00%3A00Z&per_page=100"),
        Some(&fixture.view_access_token),
    );
    assert_eq!(response.status().as_u16(), 200);
    assert!(body(response)["data"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["id"] == created["id"]));
    let response = target.get(
        &format!("{base}?updated_since=2099-01-01T00%3A00%3A00Z"),
        Some(&fixture.access_token),
    );
    assert_eq!(response.status().as_u16(), 200);
    assert_eq!(body(response)["meta"]["total_count"], 0);
    let response = target.get(
        &format!("{base}?updated_since=invalid"),
        Some(&fixture.access_token),
    );
    assert_eq!(response.status().as_u16(), 422);

    let response = target.put_json_if_match(
        &path,
        &fixture.access_token,
        &json!({"health_event": {"title": "Stale"}}),
        "\"stale-etag\"",
    );
    assert_eq!(response.status().as_u16(), 409);
    let response = target.put_json_if_match(
        &path,
        &fixture.access_token,
        &json!({"health_event": {"ended_on": "2026-02-24"}}),
        &initial_tag,
    );
    assert_eq!(response.status().as_u16(), 422);
    let response = target.patch_json_if_match(
        &path,
        &fixture.access_token,
        &json!({"health_event": {"medication_ids": [fixture.foreign_medication_portable_id]}}),
        &initial_tag,
    );
    assert_eq!(response.status().as_u16(), 404);
    let medication_base = format!("/api/v1/households/{}/medications", fixture.household_id);
    let response = target.post_json_authorized(
        &medication_base,
        &fixture.access_token,
        &json!({"medication": {"name": "Contract linked replacement medicine",
            "location_id": fixture.primary_location_id}}),
    );
    assert_eq!(response.status().as_u16(), 201);
    let replacement_medication = body(response)["data"].clone();
    let response = target.put_json_if_match(
        &path,
        &fixture.access_token,
        &json!({"health_event": {"person_id": fixture.user_person_id.to_string(),
            "medication_ids": [replacement_medication["portable_id"]]}}),
        &initial_tag,
    );
    assert_eq!(response.status().as_u16(), 200);
    let changed = body(response)["data"].clone();
    assert_eq!(changed["person_id"], created["person_id"]);
    assert_eq!(
        changed["medication_portable_ids"],
        json!([replacement_medication["portable_id"]])
    );
    assert_eq!(changed["title"], created["title"]);
}

#[test]
fn dosage_and_health_grants_distinguish_view_and_manage_access() {
    let target = Target::from_env();
    let fixture = fixture();
    let dosage_base = dosage_path(&fixture);
    let response = target.post_json_authorized(
        &dosage_base,
        &fixture.access_token,
        &json!({"dosage_option": {"medication_id": fixture.managed_medication_portable_id,
            "amount": "7.75", "unit": "ml", "frequency": "daily",
            "default_max_daily_doses": 2, "default_min_hours_between_doses": "8",
            "default_dose_cycle": "daily"}}),
    );
    assert_eq!(response.status().as_u16(), 201);
    let dosage = body(response)["data"].clone();
    let dosage_path = format!("{dosage_base}/{}", dosage["portable_id"].as_str().unwrap());
    for token in [&fixture.view_access_token, &fixture.delegated_access_token] {
        let response = target.get(&dosage_path, Some(token));
        assert_eq!(response.status().as_u16(), 200);
        assert_eq!(body(response)["data"]["id"], dosage["id"]);
        let response = target.patch_json(
            &dosage_path,
            token,
            &json!({"dosage_option": {"amount": "2"}}),
        );
        assert_eq!(response.status().as_u16(), 403);
    }

    let (event, _) = create_health_event(
        &target,
        &fixture,
        &fixture.managed_person_portable_id,
        "Contract grant event",
    );
    let event_path = format!(
        "{}/{}",
        health_path(&fixture),
        event["portable_id"].as_str().unwrap()
    );
    let response = target.get(&event_path, Some(&fixture.view_access_token));
    assert_eq!(response.status().as_u16(), 200);
    let response = target.patch_json(
        &event_path,
        &fixture.view_access_token,
        &json!({"health_event": {"title": "Forbidden view edit"}}),
    );
    assert_eq!(response.status().as_u16(), 403);
    let response = target.patch_json(
        &event_path,
        &fixture.delegated_access_token,
        &json!({"health_event": {"title": "Delegated care edit"}}),
    );
    assert_eq!(response.status().as_u16(), 200);
    assert_eq!(body(response)["data"]["title"], "Delegated care edit");
}

#[test]
#[ignore = "Rails currently raises 500 for a duplicate adult default; validation should return 422"]
fn dosage_option_duplicate_adult_default_returns_validation_error() {
    let target = Target::from_env();
    let fixture = fixture();
    let base = dosage_path(&fixture);
    let (first, _) = create_dosage(&target, &fixture, &fixture.managed_medication_portable_id);
    assert_eq!(first["default_for_adults"], true);
    let response = target.post_json_authorized(
        &base,
        &fixture.access_token,
        &json!({"dosage_option": {"medication_id": fixture.managed_medication_portable_id,
            "amount": "1", "unit": "ml", "frequency": "daily", "default_for_adults": true,
            "default_max_daily_doses": 2, "default_min_hours_between_doses": "8",
            "default_dose_cycle": "daily"}}),
    );
    assert_eq!(response.status().as_u16(), 422);
}

#[test]
#[ignore = "Rails currently raises 500 for an invalid health-event kind; validation should return 422"]
fn health_event_invalid_kind_returns_validation_error() {
    let target = Target::from_env();
    let fixture = fixture();
    let response = target.post_json_authorized(
        &health_path(&fixture),
        &fixture.access_token,
        &json!({"health_event": {"person_id": fixture.managed_person_portable_id,
            "event_kind": "unsupported", "title": "Invalid kind", "started_on": "2026-02-25"}}),
    );
    assert_eq!(response.status().as_u16(), 422);
}
