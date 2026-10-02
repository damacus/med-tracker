use medtracker_contract_tests::{fixture, Fixture, Target};
use reqwest::blocking::{Client, Response};
use reqwest::Method;
use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::env;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

const MEDICATION_FIELDS: &[&str] = &[
    "id",
    "portable_id",
    "name",
    "display_name",
    "category",
    "description",
    "dose_amount",
    "dose_unit",
    "current_supply",
    "reorder_threshold",
    "reorder_status",
    "location_id",
    "location_portable_id",
    "updated_at",
    "low_stock",
    "out_of_stock",
    "days_until_low_stock",
    "days_until_out_of_stock",
];

const MEDICATION_TAKE_FIELDS: &[&str] = &[
    "id",
    "portable_id",
    "client_uuid",
    "schedule_id",
    "schedule_portable_id",
    "person_medication_id",
    "person_medication_portable_id",
    "taken_from_medication_id",
    "taken_from_medication_portable_id",
    "taken_from_location_id",
    "taken_from_location_portable_id",
    "dose_amount",
    "dose_unit",
    "taken_at",
    "updated_at",
    "person_id",
    "person_portable_id",
    "medication_id",
    "medication_portable_id",
];

fn assert_fields(value: &Value, expected: &[&str]) {
    let object = value.as_object().expect("JSON object");
    let actual: BTreeSet<&str> = object.keys().map(String::as_str).collect();
    let expected: BTreeSet<&str> = expected.iter().copied().collect();
    assert_eq!(actual, expected);
}

fn assert_positive_id(value: &Value) {
    assert!(value.as_u64().is_some_and(|id| id > 0));
}

fn assert_nullable_id(value: &Value) {
    if !value.is_null() {
        assert_positive_id(value);
    }
}

fn assert_uuid(value: &Value) {
    let value = value.as_str().expect("UUID string");
    assert_eq!(value.len(), 36);
    for (index, byte) in value.bytes().enumerate() {
        if [8, 13, 18, 23].contains(&index) {
            assert_eq!(byte, b'-');
        } else {
            assert!(byte.is_ascii_hexdigit());
        }
    }
}

fn assert_nullable_uuid(value: &Value) {
    if !value.is_null() {
        assert_uuid(value);
    }
}

fn assert_decimal(value: &Value) {
    let value = value.as_str().expect("decimal string");
    let unsigned = value.strip_prefix('-').unwrap_or(value);
    let mut parts = unsigned.split('.');
    let integer = parts.next().unwrap();
    assert!(!integer.is_empty() && integer.bytes().all(|byte| byte.is_ascii_digit()));
    if let Some(fraction) = parts.next() {
        assert!(!fraction.is_empty() && fraction.bytes().all(|byte| byte.is_ascii_digit()));
    }
    assert!(parts.next().is_none());
}

fn assert_nullable_decimal(value: &Value) {
    if !value.is_null() {
        assert_decimal(value);
    }
}

fn assert_nullable_string(value: &Value) {
    if !value.is_null() {
        assert!(value.as_str().is_some());
    }
}

fn assert_timestamp(value: &Value) {
    OffsetDateTime::parse(value.as_str().expect("timestamp string"), &Rfc3339)
        .expect("RFC3339 timestamp");
}

fn assert_nullable_timestamp(value: &Value) {
    if !value.is_null() {
        assert_timestamp(value);
    }
}

fn timestamp_nanos(value: &Value) -> i128 {
    OffsetDateTime::parse(value.as_str().expect("timestamp string"), &Rfc3339)
        .expect("RFC3339 timestamp")
        .unix_timestamp_nanos()
}

fn assert_medication(value: &Value) {
    let object = value.as_object().expect("Medication object");
    for field in MEDICATION_FIELDS {
        assert!(object.contains_key(*field), "required Medication field {field}");
    }
    for field in object.keys() {
        assert!(MEDICATION_FIELDS.contains(&field.as_str()) || ["friendly_name", "barcode", "warnings"].contains(&field.as_str()), "unexpected Medication field {field}");
    }
    for field in ["friendly_name", "barcode", "warnings"] {
        if let Some(value) = object.get(field) {
            assert_nullable_string(value);
        }
    }
    assert_positive_id(&value["id"]);
    assert_uuid(&value["portable_id"]);
    assert!(!value["name"].as_str().unwrap().is_empty());
    assert!(!value["display_name"].as_str().unwrap().is_empty());
    assert_nullable_string(&value["category"]);
    assert_nullable_string(&value["description"]);
    assert_nullable_decimal(&value["dose_amount"]);
    assert_nullable_string(&value["dose_unit"]);
    assert_nullable_decimal(&value["current_supply"]);
    assert_decimal(&value["reorder_threshold"]);
    assert!(
        value["reorder_status"].is_null()
            || ["ordered", "received"].contains(&value["reorder_status"].as_str().unwrap())
    );
    assert_positive_id(&value["location_id"]);
    assert_nullable_uuid(&value["location_portable_id"]);
    assert_timestamp(&value["updated_at"]);
    assert!(value["low_stock"].is_boolean());
    assert!(value["out_of_stock"].is_boolean());
    for field in ["days_until_low_stock", "days_until_out_of_stock"] {
        assert!(value[field].is_null() || value[field].as_i64().is_some());
    }
}

fn assert_medication_take(value: &Value) {
    assert_fields(value, MEDICATION_TAKE_FIELDS);
    assert_positive_id(&value["id"]);
    assert_uuid(&value["portable_id"]);
    assert_nullable_uuid(&value["client_uuid"]);
    for field in [
        "schedule_id",
        "person_medication_id",
        "taken_from_medication_id",
        "taken_from_location_id",
        "person_id",
        "medication_id",
    ] {
        assert_nullable_id(&value[field]);
    }
    for field in [
        "schedule_portable_id",
        "person_medication_portable_id",
        "taken_from_medication_portable_id",
        "taken_from_location_portable_id",
        "person_portable_id",
        "medication_portable_id",
    ] {
        assert_nullable_uuid(&value[field]);
    }
    assert_nullable_decimal(&value["dose_amount"]);
    assert_nullable_string(&value["dose_unit"]);
    assert_nullable_timestamp(&value["taken_at"]);
    assert_timestamp(&value["updated_at"]);
}

fn assert_collection(value: &Value) {
    assert_fields(value, &["data", "meta"]);
    assert_fields(&value["meta"], &["page", "per_page", "total_count"]);
    assert!(value["meta"]["page"].as_u64().unwrap() >= 1);
    assert!((1..=100).contains(&value["meta"]["per_page"].as_u64().unwrap()));
    assert!(value["meta"]["total_count"].as_u64().is_some());
}

fn assert_error_json(response: Response, expected_status: u16) {
    assert_eq!(response.status().as_u16(), expected_status);
    let request_id = response.headers()["x-request-id"]
        .to_str()
        .expect("request ID")
        .to_owned();
    assert!(!request_id.is_empty());
    assert_eq!(response.headers()["content-type"], "application/json");
    let error = body(response);
    assert_fields(&error, &["error"]);
    let error = &error["error"];
    let allowed = if error.get("errors").is_some() {
        &["code", "message", "request_id", "errors"][..]
    } else {
        &["code", "message", "request_id"][..]
    };
    assert_fields(error, allowed);
    assert!(error["code"]
        .as_str()
        .is_some_and(|value| !value.is_empty()));
    assert!(error["message"]
        .as_str()
        .is_some_and(|value| !value.is_empty()));
    assert_eq!(error["request_id"], request_id);
    assert!(error.get("errors").is_none_or(Value::is_object));
}

fn assert_medication_collection(value: &Value) {
    assert_collection(value);
    for item in value["data"].as_array().expect("data array") {
        assert_medication(item);
    }
}

fn assert_medication_take_collection(value: &Value) {
    assert_collection(value);
    for item in value["data"].as_array().expect("data array") {
        assert_medication_take(item);
    }
}

fn body(response: Response) -> Value {
    response.json().expect("JSON response")
}

fn etag(response: &Response) -> String {
    response.headers()["etag"]
        .to_str()
        .expect("ETag header")
        .to_owned()
}

fn medications_path(fixture: &Fixture) -> String {
    format!("/api/v1/households/{}/medications", fixture.household_id)
}

fn medication_takes_path(fixture: &Fixture) -> String {
    format!(
        "/api/v1/households/{}/medication_takes",
        fixture.household_id
    )
}

fn create_medication_payload(fixture: &Fixture, name: &str) -> Value {
    json!({"medication": {
        "name": name,
        "location_id": fixture.primary_location_id,
        "reorder_threshold": "1.25"
    }})
}

fn assert_422(response: Response) {
    assert_eq!(response.status().as_u16(), 422, "{}", body(response));
}

#[test]
fn medication_list_create_get_patch_and_replace_match_openapi_shapes() {
    let fixture = fixture();
    let target = Target::from_env();
    let collection_path = medications_path(&fixture);
    let name = format!(
        "OpenAPI medication {}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );

    let created_response = target.post_json_authorized(
        &collection_path,
        &fixture.access_token,
        &json!({"medication": {
            "name": name,
            "location_id": fixture.primary_location_id,
            "reorder_threshold": "1.25"
        }}),
    );
    assert_eq!(created_response.status().as_u16(), 201);
    let created_etag = etag(&created_response);
    let created_body = body(created_response);
    assert_fields(&created_body, &["data"]);
    assert_medication(&created_body["data"]);
    assert_eq!(created_body["data"]["name"], name);
    let id = created_body["data"]["id"].as_i64().unwrap();

    let list_response = target.get(
        &format!("{collection_path}?page=1&per_page=100"),
        Some(&fixture.access_token),
    );
    assert_eq!(list_response.status().as_u16(), 200);
    let listed = body(list_response);
    assert_medication_collection(&listed);
    assert!(listed["data"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["id"] == id));

    let item_path = format!("{collection_path}/{id}");
    let get_response = target.get(&item_path, Some(&fixture.access_token));
    assert_eq!(get_response.status().as_u16(), 200);
    assert_eq!(etag(&get_response), created_etag);
    let fetched = body(get_response);
    assert_fields(&fetched, &["data"]);
    assert_medication(&fetched["data"]);
    assert_eq!(fetched["data"], created_body["data"]);

    let patch_response = target.patch_json_if_match(
        &item_path,
        &fixture.access_token,
        &json!({"medication": {"friendly_name": "Contract liquid", "current_supply": "7.25"}}),
        &created_etag,
    );
    assert_eq!(patch_response.status().as_u16(), 200);
    let patched_etag = etag(&patch_response);
    assert_ne!(patched_etag, created_etag);
    let patched = body(patch_response);
    assert_fields(&patched, &["data"]);
    assert_medication(&patched["data"]);
    assert_eq!(patched["data"]["display_name"], "Contract liquid");
    assert_eq!(patched["data"]["current_supply"], "7.25");

    let replace_response = target.put_json_if_match(
        &item_path,
        &fixture.access_token,
        &json!({"medication": {"name": "Contract replacement"}}),
        &patched_etag,
    );
    assert_eq!(replace_response.status().as_u16(), 200);
    let replaced_etag = etag(&replace_response);
    assert_ne!(replaced_etag, patched_etag);
    let replaced = body(replace_response);
    assert_fields(&replaced, &["data"]);
    assert_medication(&replaced["data"]);
    assert_eq!(replaced["data"]["name"], "Contract replacement");
    assert_eq!(replaced["data"]["current_supply"], "7.25");

    for method in ["PATCH", "PUT"] {
        let response = if method == "PATCH" {
            target.patch_json_if_match(
                &item_path,
                &fixture.access_token,
                &json!({"medication": {}}),
                &replaced_etag,
            )
        } else {
            target.put_json_if_match(
                &item_path,
                &fixture.access_token,
                &json!({"medication": {}}),
                &replaced_etag,
            )
        };
        assert_eq!(response.status().as_u16(), 422, "{method}");
    }

    let invalid_create = target.post_json_authorized(
        &collection_path,
        &fixture.access_token,
        &json!({"medication": {
            "location_id": fixture.primary_location_id,
            "reorder_threshold": "1.25"
        }}),
    );
    assert_eq!(invalid_create.status().as_u16(), 422);
}

#[test]
fn medication_take_list_and_create_match_openapi_shapes_and_required_fields() {
    let fixture = fixture();
    let target = Target::from_env();
    let collection_path = medication_takes_path(&fixture);

    let list_response = target.get(
        &format!("{collection_path}?page=1&per_page=100"),
        Some(&fixture.access_token),
    );
    assert_eq!(list_response.status().as_u16(), 200);
    let listed = body(list_response);
    assert_medication_take_collection(&listed);

    let taken_at = OffsetDateTime::now_utc()
        .replace_nanosecond(0)
        .unwrap()
        .format(&Rfc3339)
        .unwrap();
    let uuid_bits = (OffsetDateTime::now_utc().unix_timestamp_nanos() as u128) & 0xFFFF_FFFF_FFFF;
    let client_uuid = format!("55555555-5555-4555-8555-{uuid_bits:012x}");
    let created_response = target.post_json_authorized(
        &collection_path,
        &fixture.access_token,
        &json!({"medication_take": {
            "client_uuid": client_uuid,
            "source_type": "person_medication",
            "source_id": fixture.managed_assignment_portable_id,
            "taken_at": taken_at,
            "dose_amount": "1.25",
            "taken_from_medication_id": fixture.managed_medication_id
        }}),
    );
    assert_eq!(created_response.status().as_u16(), 201);
    let created_etag = etag(&created_response);
    let created = body(created_response);
    assert_fields(&created, &["data"]);
    assert_medication_take(&created["data"]);
    assert_eq!(created["data"]["client_uuid"], client_uuid);
    assert_eq!(
        timestamp_nanos(&created["data"]["taken_at"]),
        timestamp_nanos(&json!(taken_at))
    );

    let listed_after_create = target.get(
        &format!("{collection_path}?page=1&per_page=100"),
        Some(&fixture.access_token),
    );
    assert_eq!(listed_after_create.status().as_u16(), 200);
    let listed_after_create = body(listed_after_create);
    assert_medication_take_collection(&listed_after_create);
    let listed_take = listed_after_create["data"]
        .as_array()
        .unwrap()
        .iter()
        .find(|take| take["client_uuid"] == client_uuid)
        .expect("created take is listed");
    assert_eq!(listed_take, &created["data"]);
    assert!(!created_etag.is_empty());

    let invalid_create = target.post_json_authorized(
        &collection_path,
        &fixture.access_token,
        &json!({"medication_take": {
            "source_type": "person_medication",
            "source_id": fixture.managed_assignment_portable_id
        }}),
    );
    assert_eq!(invalid_create.status().as_u16(), 422);
}

#[test]
fn medication_write_bodies_require_json_and_the_documented_wrapper() {
    let fixture = fixture();
    let target = Target::from_env();
    let medications = medications_path(&fixture);
    let takes = medication_takes_path(&fixture);
    let medication = format!("{medications}/{}", fixture.managed_medication_id);
    let before_medication_response = target.get(&medication, Some(&fixture.access_token));
    assert_eq!(before_medication_response.status().as_u16(), 200);
    let before_medication_etag = etag(&before_medication_response);
    let before_medication = body(before_medication_response);
    let before_takes = body(target.get(
        &format!("{takes}?page=1&per_page=100"),
        Some(&fixture.access_token),
    ));

    let mut error_envelopes = 0;
    for path in [&medications, &takes] {
        assert_error_json(
            target.post_raw_json_authorized(path, &fixture.access_token, "{\"invalid\":"),
            400,
        );
        error_envelopes += 1;
        assert_error_json(
            target.post_json_authorized(path, &fixture.access_token, &json!({})),
            400,
        );
        error_envelopes += 1;
    }
    for method in ["PATCH", "PUT"] {
        let response = if method == "PATCH" {
            target.patch_raw_json_if_match(
                &medication,
                &fixture.access_token,
                "{\"invalid\":",
                &before_medication_etag,
            )
        } else {
            target.put_raw_json_if_match(
                &medication,
                &fixture.access_token,
                "{\"invalid\":",
                &before_medication_etag,
            )
        };
        assert_error_json(response, 400);
        error_envelopes += 1;
        let response = if method == "PATCH" {
            target.patch_json_if_match(
                &medication,
                &fixture.access_token,
                &json!({}),
                &before_medication_etag,
            )
        } else {
            target.put_json_if_match(
                &medication,
                &fixture.access_token,
                &json!({}),
                &before_medication_etag,
            )
        };
        assert_error_json(response, 400);
        error_envelopes += 1;
    }

    assert_eq!(error_envelopes, 8);
    let after_medication_response = target.get(&medication, Some(&fixture.access_token));
    assert_eq!(after_medication_response.status().as_u16(), 200);
    assert_eq!(etag(&after_medication_response), before_medication_etag);
    assert_eq!(body(after_medication_response), before_medication);
    let after_takes = body(target.get(
        &format!("{takes}?page=1&per_page=100"),
        Some(&fixture.access_token),
    ));
    assert_medication_take_collection(&after_takes);
    assert_eq!(after_takes, before_takes);
}

#[test]
fn medication_write_requests_validate_openapi_shape_types_and_nonmutation() {
    let fixture = fixture();
    let target = Target::from_env();
    let collection_path = medications_path(&fixture);
    let take_path = medication_takes_path(&fixture);
    let item_path = format!("{collection_path}/{}", fixture.managed_medication_id);

    let before_list = body(target.get(
        &format!("{collection_path}?page=1&per_page=100"),
        Some(&fixture.access_token),
    ));
    assert_medication_collection(&before_list);
    let before_take_list = body(target.get(
        &format!("{take_path}?page=1&per_page=100"),
        Some(&fixture.access_token),
    ));
    assert_medication_take_collection(&before_take_list);

    assert_eq!(
        target
            .post_raw_json_authorized(&collection_path, &fixture.access_token, "{\"medication\":")
            .status()
            .as_u16(),
        400
    );
    assert_eq!(
        target
            .post_json_authorized(
                &collection_path,
                &fixture.access_token,
                &json!({"name": "Missing wrapper", "location_id": fixture.primary_location_id,
                "reorder_threshold": "1.25"}),
            )
            .status()
            .as_u16(),
        400
    );

    let missing_threshold = json!({"medication": {
        "name": "Missing required threshold",
        "location_id": fixture.primary_location_id
    }});
    assert_422(target.post_json_authorized(
        &collection_path,
        &fixture.access_token,
        &missing_threshold,
    ));

    let mut unknown_root = create_medication_payload(&fixture, "Unknown root field");
    unknown_root["additional"] = json!("outside schema");
    assert_422(target.post_json_authorized(&collection_path, &fixture.access_token, &unknown_root));

    let mut unknown_inner = create_medication_payload(&fixture, "Unknown medication field");
    unknown_inner["medication"]["additional"] = json!("outside schema");
    assert_422(target.post_json_authorized(
        &collection_path,
        &fixture.access_token,
        &unknown_inner,
    ));

    let mut wrong_location_type = create_medication_payload(&fixture, "String numeric ID");
    wrong_location_type["medication"]["location_id"] =
        json!(fixture.primary_location_id.to_string());
    assert_422(target.post_json_authorized(
        &collection_path,
        &fixture.access_token,
        &wrong_location_type,
    ));

    let before_item = target.get(&item_path, Some(&fixture.access_token));
    assert_eq!(before_item.status().as_u16(), 200);
    let before_item_etag = etag(&before_item);
    let before_item_body = body(before_item);
    assert_medication(&before_item_body["data"]);

    for method in ["PATCH", "PUT"] {
        let response = if method == "PATCH" {
            target.patch_json_if_match(
                &item_path,
                &fixture.access_token,
                &json!({"medication": {}}),
                &before_item_etag,
            )
        } else {
            target.put_json_if_match(
                &item_path,
                &fixture.access_token,
                &json!({"medication": {}}),
                &before_item_etag,
            )
        };
        assert_422(response);
    }

    let mut unknown_inner_update = json!({"medication": {"friendly_name": "ignored"}});
    unknown_inner_update["medication"]["unexpected"] = json!(true);
    for method in ["PATCH", "PUT"] {
        let response = if method == "PATCH" {
            target.patch_json_if_match(
                &item_path,
                &fixture.access_token,
                &unknown_inner_update,
                &before_item_etag,
            )
        } else {
            target.put_json_if_match(
                &item_path,
                &fixture.access_token,
                &unknown_inner_update,
                &before_item_etag,
            )
        };
        assert_422(response);
    }

    let mut wrong_update_type = json!({"medication": {"friendly_name": "ignored"}});
    wrong_update_type["medication"]["location_id"] = json!(fixture.primary_location_id.to_string());
    assert_422(target.patch_json_if_match(
        &item_path,
        &fixture.access_token,
        &wrong_update_type,
        &before_item_etag,
    ));

    let invalid_take_type = json!({"medication_take": {
        "source_type": "person_medication",
        "source_id": fixture.managed_assignment_portable_id,
        "taken_at": 12345
    }});
    assert_422(target.post_json_authorized(&take_path, &fixture.access_token, &invalid_take_type));

    let after_item = target.get(&item_path, Some(&fixture.access_token));
    assert_eq!(after_item.status().as_u16(), 200);
    assert_eq!(etag(&after_item), before_item_etag);
    assert_eq!(body(after_item), before_item_body);
    let after_list = body(target.get(
        &format!("{collection_path}?page=1&per_page=100"),
        Some(&fixture.access_token),
    ));
    assert_medication_collection(&after_list);
    assert_eq!(after_list, before_list);
    let after_take_list = body(target.get(
        &format!("{take_path}?page=1&per_page=100"),
        Some(&fixture.access_token),
    ));
    assert_medication_take_collection(&after_take_list);
    assert_eq!(after_take_list, before_take_list);

    for path in [&collection_path, &take_path] {
        for query in [
            "page=0",
            "per_page=0",
            "per_page=101",
            "page=not-an-integer",
            "updated_since=not-a-timestamp",
        ] {
            let response = target.get(&format!("{path}?{query}"), Some(&fixture.access_token));
            assert_eq!(response.status().as_u16(), 422, "{path}?{query}");
        }
    }
}

#[test]
fn medication_and_take_list_queries_reject_out_of_contract_pagination_and_filters() {
    let fixture = fixture();
    let target = Target::from_env();
    let mut statuses = Vec::new();
    for path in [medications_path(&fixture), medication_takes_path(&fixture)] {
        for query in [
            "page=0",
            "per_page=0",
            "per_page=101",
            "page=not-an-integer",
            "updated_since=not-a-timestamp",
        ] {
            statuses.push(
                target
                    .get(&format!("{path}?{query}"), Some(&fixture.access_token))
                    .status()
                    .as_u16(),
            );
        }
    }
    assert_eq!(statuses, [422; 10]);
}

#[test]
fn medication_take_rejects_unknown_fields_on_an_untracked_disposable_source() {
    let fixture = fixture();
    let target = Target::from_env();
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos()
        .to_string();
    let medication_response = target.post_json_authorized(
        &medications_path(&fixture),
        &fixture.access_token,
        &json!({"medication": {
            "name": format!("Disposable take contract {unique}"),
            "location_id": fixture.primary_location_id,
            "dose_amount": "1.25",
            "dose_unit": "ml",
            "reorder_threshold": "1"
        }}),
    );
    assert_eq!(medication_response.status().as_u16(), 201);
    let medication = body(medication_response)["data"].clone();
    assert!(medication["current_supply"].is_null());

    let assignment_response = target.post_json_authorized(
        &format!(
            "/api/v1/households/{}/person_medications",
            fixture.household_id
        ),
        &fixture.access_token,
        &json!({"person_medication": {
            "person_id": fixture.managed_person_portable_id,
            "medication_id": medication["portable_id"],
            "dose_amount": "1.25",
            "dose_unit": "ml"
        }}),
    );
    assert_eq!(assignment_response.status().as_u16(), 201);
    let assignment = body(assignment_response)["data"].clone();

    let takes_path = medication_takes_path(&fixture);
    let before_takes = body(target.get(
        &format!("{takes_path}?page=1&per_page=100"),
        Some(&fixture.access_token),
    ));
    let medication_path = format!("{}/{}", medications_path(&fixture), medication["id"]);
    let before_medication_response = target.get(&medication_path, Some(&fixture.access_token));
    assert_eq!(before_medication_response.status().as_u16(), 200);
    let before_medication_etag = etag(&before_medication_response);
    let before_medication = body(before_medication_response);

    let taken_at = OffsetDateTime::now_utc()
        .replace_nanosecond(0)
        .unwrap()
        .format(&Rfc3339)
        .unwrap();
    let valid_take = json!({
        "source_type": "person_medication",
        "source_id": assignment["portable_id"],
        "taken_at": taken_at,
        "dose_amount": "1.25",
        "taken_from_medication_id": medication["id"]
    });
    let mut unknown_root = json!({"medication_take": valid_take});
    unknown_root["unexpected"] = json!(true);
    let mut unknown_inner = json!({"medication_take": valid_take});
    unknown_inner["medication_take"]["unexpected"] = json!(true);
    let wrong_scalar = json!({"medication_take": {
        "source_type": "person_medication",
        "source_id": assignment["portable_id"],
        "taken_at": 12345
    }});

    let statuses = [unknown_root, unknown_inner, wrong_scalar].map(|payload| {
        target
            .post_json_authorized(&takes_path, &fixture.access_token, &payload)
            .status()
            .as_u16()
    });

    let after_takes = body(target.get(
        &format!("{takes_path}?page=1&per_page=100"),
        Some(&fixture.access_token),
    ));
    assert_eq!(
        after_takes["meta"]["total_count"],
        before_takes["meta"]["total_count"]
    );
    let after_medication_response = target.get(&medication_path, Some(&fixture.access_token));
    assert_eq!(after_medication_response.status().as_u16(), 200);
    assert_eq!(etag(&after_medication_response), before_medication_etag);
    assert_eq!(body(after_medication_response), before_medication);
    assert_eq!(statuses, [422, 422, 422]);
}

#[test]
fn medication_updates_reject_null_strings_and_noncontract_decimal_spellings() {
    let fixture = fixture();
    let target = Target::from_env();
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos()
        .to_string();
    let response = target.post_json_authorized(
        &medications_path(&fixture),
        &fixture.access_token,
        &create_medication_payload(&fixture, &format!("Scalar contract {unique}")),
    );
    assert_eq!(response.status().as_u16(), 201);
    let created = body(response)["data"].clone();
    let path = format!("{}/{}", medications_path(&fixture), created["id"]);
    let initial_response = target.get(&path, Some(&fixture.access_token));
    assert_eq!(initial_response.status().as_u16(), 200);
    let initial_etag = etag(&initial_response);
    let initial_body = body(initial_response);

    let mut statuses = Vec::new();
    for field in [
        "friendly_name",
        "barcode",
        "dmd_code",
        "dmd_system",
        "dmd_concept_class",
        "category",
        "description",
        "dose_unit",
        "warnings",
    ] {
        let current = target.get(&path, Some(&fixture.access_token));
        assert_eq!(current.status().as_u16(), 200);
        let current_etag = etag(&current);
        let response = target.patch_json_if_match(
            &path,
            &fixture.access_token,
            &json!({"medication": {field: null}}),
            &current_etag,
        );
        statuses.push(response.status().as_u16());
    }

    for decimal in ["+1", "1_0", "1.251"] {
        let current = target.get(&path, Some(&fixture.access_token));
        assert_eq!(current.status().as_u16(), 200);
        let current_etag = etag(&current);
        let response = target.patch_json_if_match(
            &path,
            &fixture.access_token,
            &json!({"medication": {"reorder_threshold": decimal}}),
            &current_etag,
        );
        statuses.push(response.status().as_u16());
    }

    let after_response = target.get(&path, Some(&fixture.access_token));
    assert_eq!(after_response.status().as_u16(), 200);
    assert_eq!(etag(&after_response), initial_etag);
    assert_eq!(body(after_response), initial_body);
    assert_eq!(statuses, [422; 12]);
}

#[test]
fn medication_take_fields_validate_openapi_types_and_decimal_lexemes() {
    let fixture = fixture();
    let target = Target::from_env();
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos()
        .to_string();
    let medication_response = target.post_json_authorized(
        &medications_path(&fixture),
        &fixture.access_token,
        &json!({"medication": {
            "name": format!("Disposable take scalar contract {unique}"),
            "location_id": fixture.primary_location_id,
            "dose_amount": "1.25",
            "dose_unit": "ml",
            "reorder_threshold": "1"
        }}),
    );
    assert_eq!(medication_response.status().as_u16(), 201);
    let medication = body(medication_response)["data"].clone();
    let assignment_response = target.post_json_authorized(
        &format!(
            "/api/v1/households/{}/person_medications",
            fixture.household_id
        ),
        &fixture.access_token,
        &json!({"person_medication": {
            "person_id": fixture.managed_person_portable_id,
            "medication_id": medication["portable_id"],
            "dose_amount": "1.25",
            "dose_unit": "ml",
            "administration_kind": "as_needed"
        }}),
    );
    assert_eq!(assignment_response.status().as_u16(), 201);
    let assignment = body(assignment_response)["data"].clone();

    let path = medication_takes_path(&fixture);
    let collection = |target: &Target| {
        body(target.get(
            &format!("{path}?page=1&per_page=100"),
            Some(&fixture.access_token),
        ))
    };
    let before_takes = collection(&target);
    let medication_path = format!("{}/{}", medications_path(&fixture), medication["id"]);
    let before_medication_response = target.get(&medication_path, Some(&fixture.access_token));
    assert_eq!(before_medication_response.status().as_u16(), 200);
    let before_medication_etag = etag(&before_medication_response);
    let before_medication = body(before_medication_response);

    let taken_at = OffsetDateTime::now_utc()
        .replace_nanosecond(0)
        .unwrap()
        .format(&Rfc3339)
        .unwrap();
    let valid_payload = || {
        json!({"medication_take": {
            "client_uuid": format!("66666666-6666-4666-8666-{:012x}",
                (OffsetDateTime::now_utc().unix_timestamp_nanos() as u128) & 0xFFFF_FFFF_FFFF),
            "source_type": "person_medication",
            "source_id": assignment["portable_id"],
            "taken_at": taken_at,
            "dose_amount": "1.25",
            "taken_from_medication_id": medication["id"]
        }})
    };
    let invalid = [
        ("source_id", json!(42)),
        ("source_id", json!("+1")),
        ("client_uuid", json!("not-a-uuid")),
        ("client_uuid", Value::Null),
        ("dose_unit", json!("")),
        ("dose_unit", Value::Null),
        ("taken_from_medication_id", json!("1")),
        ("taken_from_medication_id", json!(0)),
        ("dose_amount", json!("+1")),
        ("dose_amount", json!("1_0")),
        ("dose_amount", json!("1.251")),
    ];
    let statuses: Vec<_> = invalid
        .into_iter()
        .map(|(field, value)| {
            let mut payload = valid_payload();
            payload["medication_take"][field] = value;
            target
                .post_json_authorized(&path, &fixture.access_token, &payload)
                .status()
                .as_u16()
        })
        .collect();

    let after_invalid_takes = collection(&target);
    assert_eq!(
        after_invalid_takes["meta"]["total_count"],
        before_takes["meta"]["total_count"]
    );
    let after_invalid_medication = target.get(&medication_path, Some(&fixture.access_token));
    assert_eq!(after_invalid_medication.status().as_u16(), 200);
    assert_eq!(etag(&after_invalid_medication), before_medication_etag);
    assert_eq!(body(after_invalid_medication), before_medication);
    assert_eq!(statuses, [422; 11]);

    let mut valid = valid_payload();
    valid["medication_take"]["dose_amount"] = json!("1.250");
    let created = target.post_json_authorized(&path, &fixture.access_token, &valid);
    assert_eq!(created.status().as_u16(), 201);
    assert_eq!(body(created)["data"]["dose_amount"], "1.25");
    let after_valid_takes = collection(&target);
    assert_eq!(
        after_valid_takes["meta"]["total_count"],
        before_takes["meta"]["total_count"].as_u64().unwrap() + 1
    );
    let after_valid_medication = target.get(&medication_path, Some(&fixture.access_token));
    assert_eq!(after_valid_medication.status().as_u16(), 200);
    assert_eq!(etag(&after_valid_medication), before_medication_etag);
    assert_eq!(body(after_valid_medication), before_medication);
}

fn rate_limited_response(response: Response) {
    assert_eq!(response.status().as_u16(), 429);
    for header in [
        "retry-after",
        "ratelimit-limit",
        "ratelimit-remaining",
        "ratelimit-reset",
    ] {
        assert!(response.headers().get(header).is_some(), "missing {header}");
    }
    assert_eq!(response.headers()["ratelimit-limit"], "300");
    assert_eq!(response.headers()["ratelimit-remaining"], "0");
    assert_eq!(response.headers()["content-type"], "application/json");
    let body: Value = response.json().expect("rate limit JSON");
    assert_fields(&body, &["error"]);
    assert_fields(&body["error"], &["code", "message"]);
    assert_eq!(body["error"]["code"], "rate_limited");
    assert!(!body["error"]["message"].as_str().unwrap().is_empty());
}

fn rate_request(
    client: &Client,
    base_url: &str,
    method: Method,
    path: &str,
    token: &str,
    payload: Option<&Value>,
) -> Response {
    let mut request = client
        .request(
            method,
            format!("{}{}", base_url.trim_end_matches('/'), path),
        )
        .bearer_auth(token);
    if let Some(payload) = payload {
        request = request.json(payload);
    }
    request.send().expect("rate limit request")
}

#[test]
fn all_medication_operations_return_the_documented_shared_rate_limit_response() {
    let fixture = fixture();
    let base_url = env::var("CONTRACT_RATE_BASE_URL").expect("non-loopback API base URL");
    let client = Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(5))
        .build()
        .expect("rate limit client");
    let medication_path = medications_path(&fixture);
    let mut rejected_list = None;
    for _ in 0..601 {
        let response = rate_request(
            &client,
            &base_url,
            Method::GET,
            &medication_path,
            &fixture.access_token,
            None,
        );
        if response.status().as_u16() == 429 {
            rejected_list = Some(response);
            break;
        }
        assert_eq!(response.status().as_u16(), 200);
    }
    rate_limited_response(rejected_list.expect("medication list rate limit"));

    let item_path = format!("{medication_path}/{}", fixture.managed_medication_id);
    let take_path = medication_takes_path(&fixture);
    let empty_medication = json!({"medication": {}});
    let empty_take = json!({"medication_take": {}});
    for (method, path, payload) in [
        (
            Method::POST,
            medication_path.as_str(),
            Some(&empty_medication),
        ),
        (Method::GET, item_path.as_str(), None),
        (Method::PATCH, item_path.as_str(), Some(&empty_medication)),
        (Method::PUT, item_path.as_str(), Some(&empty_medication)),
        (Method::GET, take_path.as_str(), None),
        (Method::POST, take_path.as_str(), Some(&empty_take)),
    ] {
        let response = rate_request(
            &client,
            &base_url,
            method,
            path,
            &fixture.access_token,
            payload,
        );
        rate_limited_response(response);
    }
}
