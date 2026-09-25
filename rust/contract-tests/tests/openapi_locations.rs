use medtracker_contract_tests::{fixture, Target};
use postgres::{Client, NoTls};
use serde_json::Value;
use std::env;
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
    if let Some(errors) = body["error"].get("errors") {
        for messages in errors.as_object().expect("validation errors").values() {
            assert!(messages
                .as_array()
                .expect("error messages")
                .iter()
                .all(Value::is_string));
        }
    }
}

fn assert_invalid_pagination(query: &str) {
    let fixture = fixture();
    let target = Target::from_env();
    let path = format!(
        "/api/v1/households/{}/locations?{query}",
        fixture.household_id
    );
    let response = target.get(&path, Some(&fixture.access_token));
    assert_eq!(response.status().as_u16(), 422, "{query}");
    assert_error(&response.json().expect("pagination error JSON"));
}

#[test]
fn list_locations_rejects_noninteger_page() {
    assert_invalid_pagination("page=abc");
}

#[test]
fn list_locations_rejects_noninteger_per_page() {
    assert_invalid_pagination("per_page=abc");
}

#[test]
fn list_locations_rejects_overflowing_page() {
    assert_invalid_pagination("page=9223372036854775808");
}

#[test]
fn list_locations_rejects_overflowing_per_page() {
    assert_invalid_pagination("per_page=9223372036854775808");
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

    let unauthenticated = target.get(&path, None);
    assert_eq!(unauthenticated.status().as_u16(), 401);
    assert_error(&unauthenticated.json().expect("unauthorized JSON"));

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

    let wrong_household = target.get(
        &format!(
            "/api/v1/households/{}/locations/{}",
            fixture.foreign_household_id, fixture.foreign_location_portable_id
        ),
        Some(&fixture.access_token),
    );
    assert_eq!(wrong_household.status().as_u16(), 403);
    assert_error(&wrong_household.json().expect("foreign household JSON"));
}

#[test]
fn list_locations_applies_timestamp_filter_and_pagination_bounds() {
    let fixture = fixture();
    let target = Target::from_env();
    let base = format!("/api/v1/households/{}/locations", fixture.household_id);
    let mut db = Client::connect(
        &env::var("CONTRACT_AUDIT_DATABASE_URL").expect("contract database URL"),
        NoTls,
    )
    .expect("contract database");
    let id: i64 = db.query_one(
        "INSERT INTO locations (household_id, name, created_at, updated_at) VALUES ($1, $2, now(), now()) RETURNING id",
        &[&fixture.household_id, &"OpenAPI pagination fixture"],
    ).expect("pagination location").get(0);
    let path = format!("{base}?page=2&per_page=1");
    let page1 = target.get(
        &format!("{base}?page=1&per_page=1"),
        Some(&fixture.access_token),
    );
    assert_eq!(page1.status().as_u16(), 200);
    let page1: Value = page1.json().expect("page 1 JSON");
    let page2 = target.get(&path, Some(&fixture.access_token));
    assert_eq!(page2.status().as_u16(), 200);
    let page2: Value = page2.json().expect("page 2 JSON");
    assert_eq!(keys(&page2), ["data", "meta"]);
    assert_eq!(page2["meta"]["page"], 2);
    assert_eq!(page2["meta"]["per_page"], 1);
    assert!(page2["meta"]["total_count"]
        .as_u64()
        .is_some_and(|n| n >= 2));
    assert_eq!(page2["data"].as_array().expect("page 2 data").len(), 1);
    assert_location(&page2["data"][0]);
    assert_eq!(page1["meta"]["total_count"], page2["meta"]["total_count"]);
    assert_ne!(page1["data"][0]["id"], page2["data"][0]["id"]);

    let max_page = target.get(&format!("{base}?per_page=100"), Some(&fixture.access_token));
    assert_eq!(max_page.status().as_u16(), 200);
    let max_page: Value = max_page.json().expect("maximum page JSON");
    assert_eq!(max_page["meta"]["per_page"], 100);
    assert!(
        max_page["data"]
            .as_array()
            .expect("maximum page data")
            .len()
            <= 100
    );

    let old = target.get(
        &format!("{base}?updated_since=2000-01-01T00%3A00%3A00Z"),
        Some(&fixture.access_token),
    );
    assert_eq!(old.status().as_u16(), 200);
    let old: Value = old.json().expect("old filter JSON");
    assert!(old["meta"]["total_count"].as_u64().is_some_and(|n| n >= 2));
    let future = target.get(
        &format!("{base}?updated_since=2100-01-01T00%3A00%3A00Z"),
        Some(&fixture.access_token),
    );
    assert_eq!(future.status().as_u16(), 200);
    let future: Value = future.json().expect("future filter JSON");
    assert_eq!(future["meta"]["total_count"], 0);
    assert_eq!(future["data"], serde_json::json!([]));
    db.execute("DELETE FROM locations WHERE id = $1", &[&id])
        .expect("cleanup pagination location");
}
