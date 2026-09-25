use medtracker_contract_tests::{fixture, Target};
use serde_json::Value;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

fn keys(value: &Value) -> Vec<&str> {
    value
        .as_object()
        .expect("object")
        .keys()
        .map(String::as_str)
        .collect()
}

fn assert_location(location: &Value) {
    assert_eq!(
        keys(location),
        ["description", "id", "name", "portable_id", "updated_at"]
    );
    assert!(location["id"].as_u64().is_some_and(|id| id > 0));
    let portable_id = location["portable_id"].as_str().expect("portable ID");
    assert_eq!(portable_id.len(), 36);
    assert!(portable_id.chars().enumerate().all(|(index, character)| {
        if [8, 13, 18, 23].contains(&index) {
            character == '-'
        } else {
            character.is_ascii_hexdigit()
        }
    }));
    assert!(location["name"]
        .as_str()
        .is_some_and(|name| !name.is_empty()));
    assert!(location["description"].is_string() || location["description"].is_null());
    OffsetDateTime::parse(
        location["updated_at"].as_str().expect("timestamp"),
        &Rfc3339,
    )
    .expect("RFC3339 timestamp");
}

fn assert_error(body: &Value) {
    assert_eq!(keys(body), ["error"]);
    assert!(keys(&body["error"])
        .iter()
        .all(|key| ["code", "message", "request_id", "errors"].contains(key)));
    assert!(body["error"]["code"]
        .as_str()
        .is_some_and(|value| !value.is_empty()));
    assert!(body["error"]["message"]
        .as_str()
        .is_some_and(|value| !value.is_empty()));
    assert!(body["error"]["request_id"]
        .as_str()
        .is_some_and(|value| !value.is_empty()));
}

#[test]
fn list_locations_rejects_documented_invalid_filters() {
    let fixture = fixture();
    let target = Target::from_env();
    let base = format!("/api/v1/households/{}/locations", fixture.household_id);
    for query in [
        "page=0",
        "page=-1",
        "per_page=0",
        "per_page=101",
        "updated_since=not-a-date",
        "updated_since=",
    ] {
        let response = target.get(&format!("{base}?{query}"), Some(&fixture.access_token));
        assert_eq!(response.status().as_u16(), 422, "{query}");
        let body: Value = response.json().expect("validation error JSON");
        assert_error(&body);
    }
}

#[test]
fn list_locations_matches_openapi_collection_and_scope() {
    let fixture = fixture();
    let target = Target::from_env();
    let path = format!("/api/v1/households/{}/locations", fixture.household_id);

    let unauthenticated = target.get(&path, None);
    assert_eq!(unauthenticated.status().as_u16(), 401);
    let error: Value = unauthenticated.json().expect("error JSON");
    assert_error(&error);

    let response = target.get(
        &format!("{path}?page=1&per_page=1"),
        Some(&fixture.access_token),
    );
    assert_eq!(response.status().as_u16(), 200);
    let body: Value = response.json().expect("location collection JSON");
    assert_eq!(keys(&body), ["data", "meta"]);
    assert_eq!(keys(&body["meta"]), ["page", "per_page", "total_count"]);
    assert_eq!(body["meta"]["page"], 1);
    assert_eq!(body["meta"]["per_page"], 1);
    assert!(body["meta"]["total_count"]
        .as_u64()
        .is_some_and(|count| count >= 1));
    assert_eq!(body["data"].as_array().expect("locations").len(), 1);

    let response = target.get(&path, Some(&fixture.access_token));
    assert_eq!(response.status().as_u16(), 200);
    let body: Value = response.json().expect("location collection JSON");
    assert_eq!(keys(&body), ["data", "meta"]);
    let locations = body["data"].as_array().expect("locations");
    assert!(locations
        .iter()
        .any(|item| item["portable_id"] == fixture.primary_location_portable_id));
    assert!(!locations
        .iter()
        .any(|item| item["portable_id"] == fixture.foreign_location_portable_id));
    for item in locations {
        assert_location(item);
    }
}

#[test]
fn get_location_matches_openapi_resource_and_errors() {
    let fixture = fixture();
    let target = Target::from_env();
    let base = format!("/api/v1/households/{}/locations", fixture.household_id);
    let path = format!("{base}/{}", fixture.primary_location_portable_id);

    let response = target.get(&path, Some(&fixture.access_token));
    assert_eq!(response.status().as_u16(), 200);
    assert!(response.headers().get("etag").is_some());
    let body: Value = response.json().expect("location JSON");
    assert_eq!(keys(&body), ["data"]);
    assert_location(&body["data"]);
    assert_eq!(body["data"]["id"], fixture.primary_location_id);
    assert_eq!(
        body["data"]["portable_id"],
        fixture.primary_location_portable_id
    );

    let numeric_path = format!("{base}/{}", fixture.primary_location_id);
    let numeric = target.get(&numeric_path, Some(&fixture.access_token));
    assert_eq!(numeric.status().as_u16(), 200);
    let numeric_body: Value = numeric.json().expect("location JSON by numeric ID");
    assert_eq!(
        numeric_body["data"]["portable_id"],
        fixture.primary_location_portable_id
    );

    let missing = target.get(
        &format!("{base}/{}", fixture.foreign_location_portable_id),
        Some(&fixture.access_token),
    );
    assert_eq!(missing.status().as_u16(), 404);
    let error: Value = missing.json().expect("error JSON");
    assert_error(&error);
}
