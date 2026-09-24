use medtracker_contract_tests::{fixture, Target};
use reqwest::blocking::Response;
use serde_json::{json, Value};

fn json_body(response: Response) -> Value {
    response.json().expect("JSON response")
}

fn etag(response: &Response) -> String {
    response.headers()["etag"]
        .to_str()
        .expect("ETag header")
        .to_owned()
}

#[test]
fn current_profile_reads_with_bearer_session() {
    let target = Target::from_env();
    let fixture = fixture();
    let path = format!("/api/v1/households/{}/me", fixture.household_id);
    let response = target.get(&path, Some(&fixture.access_token));
    assert_eq!(response.status().as_u16(), 200);
    let body: Value = response.json().expect("JSON profile");
    assert_eq!(body["data"]["id"], fixture.user_id);
    assert_eq!(body["data"]["membership_role"], "owner");
}

#[test]
fn bearer_session_cannot_read_another_household() {
    let target = Target::from_env();
    let fixture = fixture();
    let path = format!("/api/v1/households/{}/me", fixture.foreign_household_id);
    let response = target.get(&path, Some(&fixture.access_token));
    assert_eq!(response.status().as_u16(), 403);
    let body: Value = response.json().expect("JSON denial");
    assert_eq!(body["error"]["code"], "forbidden");
    assert!(!body.to_string().contains(&fixture.foreign_email));
}

#[test]
fn people_reads_obey_grants_pagination_and_household_boundaries() {
    let target = Target::from_env();
    let fixture = fixture();
    let base = format!("/api/v1/households/{}/people", fixture.household_id);

    let response = target.get(&base, Some(&fixture.access_token));
    assert_eq!(response.status().as_u16(), 200);
    let body = json_body(response);
    let people = body["data"].as_array().expect("people collection");
    assert!(people
        .iter()
        .any(|person| person["id"] == fixture.user_person_id));
    assert!(people
        .iter()
        .any(|person| person["id"] == fixture.managed_person_id));
    assert!(!people
        .iter()
        .any(|person| person["id"] == fixture.hidden_person_id));
    assert!(!people
        .iter()
        .any(|person| person["id"] == fixture.foreign_person_id));
    assert_eq!(body["meta"]["page"], 1);
    assert_eq!(body["meta"]["per_page"], 20);

    let response = target.get(
        &format!("{base}?page=2&per_page=1"),
        Some(&fixture.access_token),
    );
    assert_eq!(response.status().as_u16(), 200);
    let body = json_body(response);
    assert_eq!(body["meta"]["page"], 2);
    assert_eq!(body["meta"]["per_page"], 1);
    assert_eq!(body["data"].as_array().unwrap().len(), 1);
    assert!(body["meta"]["total_count"].as_u64().unwrap() >= 2);

    let response = target.get(
        &format!("{base}/{}", fixture.managed_person_id),
        Some(&fixture.access_token),
    );
    assert_eq!(response.status().as_u16(), 200);
    assert!(!etag(&response).is_empty());
    assert_eq!(json_body(response)["data"]["id"], fixture.managed_person_id);

    let response = target.get(
        &format!("{base}/{}", fixture.foreign_person_id),
        Some(&fixture.access_token),
    );
    assert_eq!(response.status().as_u16(), 404);
    let body = json_body(response);
    assert_eq!(body["error"]["code"], "not_found");
    assert!(!body.to_string().contains(&fixture.foreign_person_name));

    let response = target.get(
        &format!("/api/v1/households/{}/people", fixture.foreign_household_id),
        Some(&fixture.access_token),
    );
    assert_eq!(response.status().as_u16(), 403);
    assert_eq!(json_body(response)["error"]["code"], "forbidden");

    let response = target.get(&base, Some(&fixture.view_access_token));
    assert_eq!(response.status().as_u16(), 200);
    let body = json_body(response);
    let people = body["data"].as_array().unwrap();
    assert!(people
        .iter()
        .any(|person| person["id"] == fixture.managed_person_id));
    assert!(!people
        .iter()
        .any(|person| person["id"] == fixture.hidden_person_id));
    assert!(!people
        .iter()
        .any(|person| person["id"] == fixture.user_person_id));
}

#[test]
fn people_writes_validate_retain_state_and_return_current_etags() {
    let target = Target::from_env();
    let fixture = fixture();
    let base = format!("/api/v1/households/{}/people", fixture.household_id);
    let payload = json!({"person": {
        "name": "Contract edited adult", "date_of_birth": "1980-02-03",
        "person_type": "adult", "has_capacity": true
    }});
    let response = target.post_json_authorized(&base, &fixture.care_access_token, &payload);
    assert_eq!(response.status().as_u16(), 201);
    let created_etag = etag(&response);
    let created = json_body(response)["data"].clone();
    let path = format!("{base}/{}", created["id"].as_i64().unwrap());
    assert_eq!(created["name"], "Contract edited adult");
    assert_eq!(created["date_of_birth"], "1980-02-03");
    assert_eq!(created["has_capacity"], true);

    let response = target.get(&path, Some(&fixture.care_access_token));
    assert_eq!(response.status().as_u16(), 200);
    assert_eq!(etag(&response), created_etag);
    assert_eq!(json_body(response)["data"]["id"], created["id"]);

    let response = target.patch_json(
        &path,
        &fixture.care_access_token,
        &json!({"person": {"name": "Contract patched adult"}}),
    );
    assert_eq!(response.status().as_u16(), 200);
    let patched_etag = etag(&response);
    assert_eq!(
        json_body(response)["data"]["name"],
        "Contract patched adult"
    );
    assert_ne!(patched_etag, created_etag);

    let response = target.get(&path, Some(&fixture.care_access_token));
    assert_eq!(response.status().as_u16(), 200);
    assert_eq!(etag(&response), patched_etag);
    assert_eq!(
        json_body(response)["data"]["name"],
        "Contract patched adult"
    );

    let response = target.put_json(
        &path,
        &fixture.care_access_token,
        &json!({"person": {"name": "Contract put adult"}}),
    );
    assert_eq!(response.status().as_u16(), 200);
    let put_etag = etag(&response);
    assert_ne!(put_etag, patched_etag);
    assert_eq!(json_body(response)["data"]["name"], "Contract put adult");
    let response = target.get(&path, Some(&fixture.care_access_token));
    assert_eq!(response.status().as_u16(), 200);
    assert_eq!(etag(&response), put_etag);
    assert_eq!(json_body(response)["data"]["name"], "Contract put adult");

    let response = target.post_json_authorized(
        &base,
        &fixture.care_access_token,
        &json!({"person": {"name": "", "date_of_birth": "1980-02-03", "person_type": "adult"}}),
    );
    assert_eq!(response.status().as_u16(), 422);
    let body = json_body(response);
    assert_eq!(body["error"]["code"], "validation_failed");
    assert!(body["error"]["errors"]["name"].is_array());

    let response = target.patch_json(
        &format!("{base}/{}", fixture.managed_person_id),
        &fixture.view_access_token,
        &json!({"person": {"name": "Forbidden edit"}}),
    );
    assert_eq!(response.status().as_u16(), 403);
    assert_eq!(json_body(response)["error"]["code"], "forbidden");
}

#[test]
fn person_access_grants_create_list_and_revoke_without_exposing_other_households() {
    let target = Target::from_env();
    let fixture = fixture();
    let base = format!(
        "/api/v1/households/{}/admin/person_access_grants",
        fixture.household_id
    );
    let response = target.get(&base, Some(&fixture.access_token));
    assert_eq!(response.status().as_u16(), 200);
    let body = json_body(response);
    let grants = body["data"].as_array().expect("grant collection");
    assert!(grants
        .iter()
        .any(|grant| grant["person_id"] == fixture.managed_person_id));
    assert!(!grants
        .iter()
        .any(|grant| grant["person_id"] == fixture.foreign_person_id));

    let payload = json!({"person_access_grant": {
        "household_membership_id": fixture.grant_target_membership_id,
        "person_id": fixture.managed_person_id,
        "access_level": "manage", "relationship_type": "carer"
    }});
    let response = target.post_json_authorized(&base, &fixture.access_token, &payload);
    assert_eq!(response.status().as_u16(), 201);
    let created = json_body(response)["data"].clone();
    assert_eq!(created["access_level"], "manage");
    assert_eq!(created["person_id"], fixture.managed_person_id);
    let grant_id = created["id"].as_i64().expect("grant id");

    let response = target.get(&base, Some(&fixture.access_token));
    assert_eq!(response.status().as_u16(), 200);
    assert!(json_body(response)["data"]
        .as_array()
        .unwrap()
        .iter()
        .any(|grant| grant["id"] == grant_id));

    let response = target.delete(&format!("{base}/{grant_id}"), Some(&fixture.access_token));
    assert_eq!(response.status().as_u16(), 204);
    let response = target.get(&base, Some(&fixture.access_token));
    assert_eq!(response.status().as_u16(), 200);
    let body = json_body(response);
    let revoked = body["data"]
        .as_array()
        .unwrap()
        .iter()
        .find(|grant| grant["id"] == grant_id)
        .unwrap();
    assert!(revoked["revoked_at"].is_string());

    let audit_path = format!(
        "/api/v1/households/{}/admin/audit_logs",
        fixture.household_id
    );
    let response = target.get(&audit_path, Some(&fixture.access_token));
    assert_eq!(response.status().as_u16(), 200);
    let audit = json_body(response);
    let changes: Vec<&Value> = audit["data"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|event| {
            event["event_type"] == "household_access.person_grant_changed"
                && event["metadata"]["target_grant_id"] == grant_id
        })
        .collect();
    assert!(changes.len() >= 2);
    assert!(changes
        .iter()
        .all(|event| event["metadata"]["outcome"] == "success"));
    assert!(changes
        .iter()
        .any(|event| event["metadata"]["new_state"]["revoked_at"].is_string()));

    let response = target.post_json_authorized(
        &base,
        &fixture.access_token,
        &json!({"person_access_grant": {
            "household_membership_id": fixture.grant_target_membership_id,
            "person_id": fixture.managed_person_id,
            "access_level": "invalid", "relationship_type": "carer"
        }}),
    );
    assert_eq!(response.status().as_u16(), 422);
    let body = json_body(response);
    assert_eq!(body["error"]["code"], "validation_failed");
    assert!(body["error"]["errors"]["access_level"].is_array());

    let response = target.post_json_authorized(
        &base,
        &fixture.access_token,
        &json!({"person_access_grant": {
            "household_membership_id": fixture.grant_target_membership_id,
            "person_id": fixture.foreign_person_id,
            "access_level": "view", "relationship_type": "carer"
        }}),
    );
    assert_eq!(response.status().as_u16(), 422);
    let body = json_body(response);
    assert!(!body.to_string().contains(&fixture.foreign_person_name));

    let response = target.get(&base, Some(&fixture.view_access_token));
    assert_eq!(response.status().as_u16(), 403);
    assert_eq!(json_body(response)["error"]["code"], "forbidden");
    let response = target.post_json_authorized(&base, &fixture.view_access_token, &payload);
    assert_eq!(response.status().as_u16(), 403);
    let response = target.get(
        &format!(
            "/api/v1/households/{}/admin/person_access_grants",
            fixture.foreign_household_id
        ),
        Some(&fixture.access_token),
    );
    assert_eq!(response.status().as_u16(), 403);
}

#[test]
fn locations_support_pagination_conditional_updates_and_deletion() {
    let target = Target::from_env();
    let fixture = fixture();
    let base = format!("/api/v1/households/{}/locations", fixture.household_id);
    let response = target.post_json_authorized(
        &base,
        &fixture.access_token,
        &json!({"location": {"name": "Contract temporary shelf", "description": "Initial"}}),
    );
    assert_eq!(response.status().as_u16(), 201);
    let created_etag = etag(&response);
    let created = json_body(response)["data"].clone();
    let path = format!("{base}/{}", created["portable_id"].as_str().unwrap());

    let response = target.get(
        &format!("{base}?page=1&per_page=1"),
        Some(&fixture.access_token),
    );
    assert_eq!(response.status().as_u16(), 200);
    let first_page = json_body(response);
    assert_eq!(first_page["meta"]["page"], 1);
    assert_eq!(first_page["meta"]["per_page"], 1);
    assert_eq!(first_page["data"].as_array().unwrap().len(), 1);
    assert!(first_page["meta"]["total_count"].as_u64().unwrap() >= 2);
    let first_id = &first_page["data"][0]["id"];

    let response = target.get(
        &format!("{base}?page=2&per_page=1"),
        Some(&fixture.access_token),
    );
    assert_eq!(response.status().as_u16(), 200);
    let second_page = json_body(response);
    assert_eq!(second_page["meta"]["page"], 2);
    assert_eq!(second_page["meta"]["per_page"], 1);
    assert_eq!(second_page["data"].as_array().unwrap().len(), 1);
    assert_eq!(
        second_page["meta"]["total_count"],
        first_page["meta"]["total_count"]
    );
    assert_ne!(&second_page["data"][0]["id"], first_id);

    let response = target.get(&base, Some(&fixture.access_token));
    assert_eq!(response.status().as_u16(), 200);
    let body = json_body(response);
    assert!(!body["data"]
        .as_array()
        .unwrap()
        .iter()
        .any(|location| location["id"] == fixture.foreign_location_id));

    let response = target.get(&path, Some(&fixture.access_token));
    assert_eq!(response.status().as_u16(), 200);
    assert_eq!(etag(&response), created_etag);
    assert_eq!(json_body(response)["data"]["description"], "Initial");

    let response = target.patch_json(
        &path,
        &fixture.access_token,
        &json!({"location": {"name": "No ETag"}}),
    );
    assert_eq!(response.status().as_u16(), 428);
    assert_eq!(
        json_body(response)["error"]["code"],
        "precondition_required"
    );

    let response = target.patch_json_if_match(
        &path,
        &fixture.access_token,
        &json!({"location": {"description": "Patched"}}),
        &created_etag,
    );
    assert_eq!(response.status().as_u16(), 200);
    let patched_etag = etag(&response);
    assert_eq!(json_body(response)["data"]["description"], "Patched");
    assert_ne!(patched_etag, created_etag);

    let response = target.get(&path, Some(&fixture.access_token));
    assert_eq!(response.status().as_u16(), 200);
    assert_eq!(etag(&response), patched_etag);
    assert_eq!(json_body(response)["data"]["description"], "Patched");

    let response = target.put_json_if_match(
        &path,
        &fixture.access_token,
        &json!({"location": {"name": "Contract final shelf"}}),
        &created_etag,
    );
    assert_eq!(response.status().as_u16(), 409);
    assert_eq!(json_body(response)["error"]["code"], "conflict");

    let response = target.put_json_if_match(
        &path,
        &fixture.access_token,
        &json!({"location": {"name": "Contract final shelf"}}),
        &patched_etag,
    );
    assert_eq!(response.status().as_u16(), 200);
    let final_etag = etag(&response);
    assert_eq!(json_body(response)["data"]["name"], "Contract final shelf");

    let response = target.delete_if_match(&path, &fixture.access_token, &final_etag);
    assert_eq!(response.status().as_u16(), 204);
    let response = target.get(&path, Some(&fixture.access_token));
    assert_eq!(response.status().as_u16(), 404);
    assert_eq!(json_body(response)["error"]["code"], "not_found");

    let response = target.post_json_authorized(
        &base,
        &fixture.access_token,
        &json!({"location": {"name": ""}}),
    );
    assert_eq!(response.status().as_u16(), 422);
    assert!(json_body(response)["error"]["errors"]["name"].is_array());

    let response = target.get(
        &format!("{base}/{}", fixture.foreign_location_id),
        Some(&fixture.access_token),
    );
    assert_eq!(response.status().as_u16(), 404);
    let body = json_body(response);
    assert!(!body.to_string().contains(&fixture.foreign_location_name));
    let response = target.get(&base, Some(&fixture.view_access_token));
    assert_eq!(response.status().as_u16(), 200);
    let response = target.get(
        &format!(
            "/api/v1/households/{}/locations",
            fixture.foreign_household_id
        ),
        Some(&fixture.access_token),
    );
    assert_eq!(response.status().as_u16(), 403);
    let response = target.post_json_authorized(
        &base,
        &fixture.view_access_token,
        &json!({"location": {"name": "Forbidden"}}),
    );
    assert_eq!(response.status().as_u16(), 403);
}

#[test]
fn location_membership_writes_require_manage_access_and_change_person_locations() {
    let target = Target::from_env();
    let fixture = fixture();
    let base = format!(
        "/api/v1/households/{}/locations/{}/location_memberships",
        fixture.household_id, fixture.primary_location_portable_id
    );
    let payload = json!({"location_membership": {"person_id": fixture.managed_person_portable_id}});
    let response = target.post_json_authorized(&base, &fixture.access_token, &payload);
    assert_eq!(response.status().as_u16(), 201);
    let created = json_body(response)["data"].clone();
    assert_eq!(
        created["person_portable_id"],
        fixture.managed_person_portable_id
    );
    assert_eq!(
        created["location_portable_id"],
        fixture.primary_location_portable_id
    );
    let membership_id = created["id"].as_str().expect("membership id");

    let response = target.post_json_authorized(&base, &fixture.access_token, &payload);
    assert_eq!(response.status().as_u16(), 201);
    assert_eq!(json_body(response)["data"]["id"], membership_id);

    let person_path = format!(
        "/api/v1/households/{}/people/{}",
        fixture.household_id, fixture.managed_person_id
    );
    let response = target.get(&person_path, Some(&fixture.access_token));
    assert_eq!(response.status().as_u16(), 200);
    assert!(json_body(response)["data"]["location_ids"]
        .as_array()
        .unwrap()
        .iter()
        .any(|id| id == fixture.primary_location_id));

    let response = target.post_json_authorized(
        &base,
        &fixture.access_token,
        &json!({"location_membership": {"person_id": fixture.foreign_person_portable_id}}),
    );
    assert_eq!(response.status().as_u16(), 404);
    assert!(!json_body(response)
        .to_string()
        .contains(&fixture.foreign_person_name));

    let response = target.post_json_authorized(&base, &fixture.view_access_token, &payload);
    assert_eq!(response.status().as_u16(), 403);
    assert_eq!(json_body(response)["error"]["code"], "forbidden");

    let response = target.delete(
        &format!("{base}/{membership_id}"),
        Some(&fixture.access_token),
    );
    assert_eq!(response.status().as_u16(), 204);
    let response = target.get(&person_path, Some(&fixture.access_token));
    assert_eq!(response.status().as_u16(), 200);
    assert!(!json_body(response)["data"]["location_ids"]
        .as_array()
        .unwrap()
        .iter()
        .any(|id| id == fixture.primary_location_id));
}
