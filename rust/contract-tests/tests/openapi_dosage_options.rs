use medtracker_contract_tests::{fixture, Fixture, Target};
use serde_json::{json, Value};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

fn keys(value: &Value) -> Vec<&str> {
    value
        .as_object()
        .expect("object")
        .keys()
        .map(String::as_str)
        .collect()
}

fn base(fixture: &Fixture) -> String {
    format!("/api/v1/households/{}/dosage_options", fixture.household_id)
}

fn valid_request(fixture: &Fixture) -> Value {
    json!({"dosage_option": {
        "medication_id": fixture.managed_medication_id.to_string(),
        "amount": "1.25", "unit": "tablet", "frequency": "daily",
        "description": "Contract option", "default_for_adults": false,
        "default_for_children": false, "default_max_daily_doses": 3,
        "default_min_hours_between_doses": "4.5", "default_dose_cycle": "daily",
        "current_supply": null, "reorder_threshold": "2.50"
    }})
}

fn create(target: &Target, fixture: &Fixture) -> (Value, String) {
    let response = target.post_json_authorized(
        &base(fixture),
        &fixture.access_token,
        &valid_request(fixture),
    );
    assert_eq!(response.status().as_u16(), 201);
    let etag = response
        .headers()
        .get("etag")
        .expect("ETag")
        .to_str()
        .unwrap()
        .to_owned();
    let body: Value = response.json().expect("dosage JSON");
    assert_eq!(keys(&body), ["data"]);
    assert_dosage(&body["data"]);
    (body["data"].clone(), etag)
}

fn decimal(value: &Value) {
    let text = value.as_str().expect("decimal string");
    assert!(!text.is_empty());
    let mut digits = text.strip_prefix('-').unwrap_or(text).split('.');
    let whole = digits.next().unwrap();
    assert!(!whole.is_empty() && whole.bytes().all(|byte| byte.is_ascii_digit()));
    assert!(digits.next().is_none_or(
        |fraction| !fraction.is_empty() && fraction.bytes().all(|byte| byte.is_ascii_digit())
    ));
    assert!(digits.next().is_none());
}

fn assert_dosage(row: &Value) {
    assert_eq!(
        keys(row),
        [
            "amount",
            "current_supply",
            "default_dose_cycle",
            "default_for_adults",
            "default_for_children",
            "default_max_daily_doses",
            "default_min_hours_between_doses",
            "description",
            "frequency",
            "id",
            "medication_id",
            "medication_portable_id",
            "portable_id",
            "reorder_threshold",
            "unit",
            "updated_at"
        ]
    );
    assert!(row["id"].as_u64().is_some_and(|id| id > 0));
    assert!(row["medication_id"].as_u64().is_some_and(|id| id > 0));
    for field in ["portable_id", "medication_portable_id"] {
        let id = row[field].as_str().expect("UUID");
        assert_eq!(id.len(), 36);
        assert!(id
            .bytes()
            .enumerate()
            .all(|(index, byte)| if [8, 13, 18, 23].contains(&index) {
                byte == b'-'
            } else {
                byte.is_ascii_hexdigit()
            }));
    }
    decimal(&row["amount"]);
    decimal(&row["default_min_hours_between_doses"]);
    for field in ["current_supply", "reorder_threshold"] {
        if !row[field].is_null() {
            decimal(&row[field]);
        }
    }
    for field in ["unit", "frequency"] {
        assert!(row[field].as_str().is_some_and(|text| !text.is_empty()));
    }
    assert!(row["description"].is_null() || row["description"].is_string());
    assert!(row["default_for_adults"].is_boolean() && row["default_for_children"].is_boolean());
    assert!(row["default_max_daily_doses"]
        .as_u64()
        .is_some_and(|value| value >= 1));
    assert!(["daily", "weekly", "monthly"].contains(&row["default_dose_cycle"].as_str().unwrap()));
    OffsetDateTime::parse(row["updated_at"].as_str().expect("timestamp"), &Rfc3339)
        .expect("date-time");
}

fn assert_error(response: reqwest::blocking::Response, status: u16) {
    assert_eq!(response.status().as_u16(), status);
    let body: Value = response.json().expect("error JSON");
    assert_eq!(keys(&body), ["error"]);
    assert!(keys(&body["error"])
        .iter()
        .all(|key| ["code", "message", "request_id", "errors"].contains(key)));
    for field in ["code", "message", "request_id"] {
        assert!(body["error"][field]
            .as_str()
            .is_some_and(|text| !text.is_empty()));
    }
}

#[test]
fn create_and_get_dosage_option() {
    let fixture = fixture();
    let target = Target::from_env();
    let (row, etag) = create(&target, &fixture);
    assert_eq!(row["amount"], "1.25");
    assert_eq!(
        row["reorder_threshold"]
            .as_str()
            .unwrap()
            .parse::<f64>()
            .unwrap(),
        2.5
    );
    for id in [
        row["id"].to_string(),
        row["portable_id"].as_str().unwrap().to_owned(),
    ] {
        let response = target.get(
            &format!("{}/{}", base(&fixture), id.trim_matches('"')),
            Some(&fixture.access_token),
        );
        assert_eq!(response.status().as_u16(), 200);
        assert_eq!(
            response.headers().get("etag").unwrap().to_str().unwrap(),
            etag
        );
        let body: Value = response.json().unwrap();
        assert_eq!(keys(&body), ["data"]);
        assert_dosage(&body["data"]);
        assert_eq!(body["data"]["id"], row["id"]);
    }
}

#[test]
fn list_dosage_options_paginates_and_filters() {
    let fixture = fixture();
    let target = Target::from_env();
    let (row, _) = create(&target, &fixture);
    let response = target.get(
        &format!("{}?page=1&per_page=100", base(&fixture)),
        Some(&fixture.access_token),
    );
    assert_eq!(response.status().as_u16(), 200);
    let body: Value = response.json().unwrap();
    assert_eq!(keys(&body), ["data", "meta"]);
    assert_eq!(keys(&body["meta"]), ["page", "per_page", "total_count"]);
    assert_eq!(body["meta"]["page"], 1);
    assert_eq!(body["meta"]["per_page"], 100);
    assert!(body["data"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["id"] == row["id"]));
    for item in body["data"].as_array().unwrap() {
        assert_dosage(item);
    }
    let page1: Value = target
        .get(
            &format!("{}?page=1&per_page=1", base(&fixture)),
            Some(&fixture.access_token),
        )
        .json()
        .unwrap();
    let page2: Value = target
        .get(
            &format!("{}?page=2&per_page=1", base(&fixture)),
            Some(&fixture.access_token),
        )
        .json()
        .unwrap();
    assert_eq!(page1["meta"]["total_count"], page2["meta"]["total_count"]);
    assert_eq!(page1["meta"]["page"], 1);
    assert_eq!(page2["meta"]["page"], 2);
    assert_ne!(page1["data"][0]["id"], page2["data"][0]["id"]);
    let past: Value = target
        .get(
            &format!("{}?updated_since=2020-01-01T00:00:00Z", base(&fixture)),
            Some(&fixture.access_token),
        )
        .json()
        .unwrap();
    assert!(past["data"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["id"] == row["id"]));
    let future: Value = target
        .get(
            &format!("{}?updated_since=2099-01-01T00:00:00Z", base(&fixture)),
            Some(&fixture.access_token),
        )
        .json()
        .unwrap();
    assert_eq!(future["meta"]["total_count"], 0);
    for query in ["page=0", "per_page=101", "page=abc", "updated_since=bad"] {
        assert_error(
            target.get(
                &format!("{}?{query}", base(&fixture)),
                Some(&fixture.access_token),
            ),
            422,
        );
    }
}

#[test]
fn patch_and_put_dosage_option_with_version_checks() {
    let fixture = fixture();
    let target = Target::from_env();
    let (row, etag) = create(&target, &fixture);
    let path = format!("{}/{}", base(&fixture), row["id"]);
    let patch = json!({"dosage_option": {"amount": "2.75"}});
    let response = target.patch_json_if_match(&path, &fixture.access_token, &patch, &etag);
    assert_eq!(response.status().as_u16(), 200);
    let changed_etag = response
        .headers()
        .get("etag")
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();
    let changed: Value = response.json().unwrap();
    assert_dosage(&changed["data"]);
    assert_eq!(changed["data"]["amount"], "2.75");
    assert_eq!(changed["data"]["description"], "Contract option");
    assert_error(
        target.patch_json_if_match(&path, &fixture.access_token, &patch, &etag),
        409,
    );
    let replacement =
        json!({"dosage_option": {"frequency": "weekly", "default_dose_cycle": "weekly"}});
    let response =
        target.put_json_if_match(&path, &fixture.access_token, &replacement, &changed_etag);
    assert_eq!(response.status().as_u16(), 200);
    let after: Value = response.json().unwrap();
    assert_dosage(&after["data"]);
    assert_eq!(after["data"]["frequency"], "weekly");
    assert_eq!(after["data"]["amount"], "2.75");
    assert_error(
        target.put_json_if_match(&path, &fixture.access_token, &replacement, &changed_etag),
        409,
    );
    let response = target.patch_json(
        &path,
        &fixture.access_token,
        &json!({"dosage_option": {"description": "no precondition"}}),
    );
    assert_eq!(response.status().as_u16(), 200);
    let response = target.put_json(
        &path,
        &fixture.access_token,
        &json!({"dosage_option": {"description": "put without precondition"}}),
    );
    assert_eq!(response.status().as_u16(), 200);
}

#[test]
fn simultaneous_updates_with_one_etag_only_one_commits() {
    let fixture = fixture();
    let target = Target::from_env();
    let (row, etag) = create(&target, &fixture);
    let path = format!("{}/{}", base(&fixture), row["id"]);
    let token = fixture.access_token.clone();
    let handles: Vec<_> = ["simultaneous-a", "simultaneous-b"]
        .into_iter()
        .map(|description| {
            let path = path.clone();
            let etag = etag.clone();
            let token = token.clone();
            std::thread::spawn(move || {
                let target = Target::from_env();
                target
                    .patch_json_if_match(
                        &path,
                        &token,
                        &json!({"dosage_option": {"description": description}}),
                        &etag,
                    )
                    .status()
                    .as_u16()
            })
        })
        .collect();
    let mut statuses: Vec<_> = handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .collect();
    statuses.sort();
    assert_eq!(statuses, [200, 409]);
}

#[test]
fn administrator_is_denied_until_dosage_policy_is_specified() {
    let fixture = fixture();
    let target = Target::from_env();
    let base = base(&fixture);
    let path = format!("{base}/{}", fixture.managed_dosage_portable_id);
    assert_error(target.get(&base, Some(&fixture.manager_access_token)), 403);
    assert_error(
        target.post_json_authorized(
            &base,
            &fixture.manager_access_token,
            &valid_request(&fixture),
        ),
        403,
    );
    assert_error(target.get(&path, Some(&fixture.manager_access_token)), 403);
    assert_error(
        target.patch_json(
            &path,
            &fixture.manager_access_token,
            &json!({"dosage_option": {"amount": "2"}}),
        ),
        403,
    );
    assert_error(
        target.put_json(
            &path,
            &fixture.manager_access_token,
            &json!({"dosage_option": {"amount": "2"}}),
        ),
        403,
    );
}

#[test]
fn dosage_option_rejects_invalid_requests_and_missing_auth() {
    let fixture = fixture();
    let target = Target::from_env();
    let base = base(&fixture);
    assert_error(target.get(&base, None), 401);
    assert_error(target.get(&base, Some("invalid-token")), 401);
    assert_error(target.post_json(&base, &valid_request(&fixture)), 401);
    assert_error(
        target.post_json_authorized(&base, "invalid-token", &valid_request(&fixture)),
        401,
    );
    let mut invalid = valid_request(&fixture);
    invalid
        .as_object_mut()
        .unwrap()
        .insert("extra".to_owned(), json!(true));
    assert_error(
        target.post_json_authorized(&base, &fixture.access_token, &invalid),
        422,
    );
    for (field, value) in [
        ("amount", json!(1.25)),
        ("medication_id", json!("+1")),
        ("unit", json!("")),
        ("default_max_daily_doses", json!(0)),
        ("default_dose_cycle", json!("hourly")),
        ("amount", json!("0.00000000000000000000000000001")),
        ("current_supply", json!("1.234")),
        ("current_supply", json!("100000000")),
        ("reorder_threshold", json!("1.234")),
        ("default_min_hours_between_doses", json!("1.23")),
        ("default_min_hours_between_doses", json!("1000")),
    ] {
        let mut invalid = valid_request(&fixture);
        invalid["dosage_option"][field] = value;
        assert_error(
            target.post_json_authorized(&base, &fixture.access_token, &invalid),
            422,
        );
    }
    for field in [
        "medication_id",
        "amount",
        "unit",
        "frequency",
        "default_max_daily_doses",
        "default_min_hours_between_doses",
        "default_dose_cycle",
    ] {
        let mut invalid = valid_request(&fixture);
        invalid["dosage_option"]
            .as_object_mut()
            .unwrap()
            .remove(field);
        assert_error(
            target.post_json_authorized(&base, &fixture.access_token, &invalid),
            422,
        );
    }
    assert_error(
        target.post_raw_json_authorized(&base, &fixture.access_token, "{"),
        400,
    );
    let (row, etag) = create(&target, &fixture);
    let path = format!("{}/{}", base, row["id"]);
    assert_error(target.get(&path, None), 401);
    assert_error(target.get(&path, Some("invalid-token")), 401);
    assert_error(
        target.patch_json_without_auth(&path, &json!({"dosage_option": {"amount": "3"}})),
        401,
    );
    assert_error(
        target.put_json_without_auth(&path, &json!({"dosage_option": {"amount": "3"}})),
        401,
    );
    assert_error(
        target.patch_json(
            &path,
            "invalid-token",
            &json!({"dosage_option": {"amount": "3"}}),
        ),
        401,
    );
    assert_error(
        target.put_json(
            &path,
            "invalid-token",
            &json!({"dosage_option": {"amount": "3"}}),
        ),
        401,
    );
    assert_error(
        target.get(
            &format!("{base}/{}", fixture.foreign_dosage_portable_id),
            Some(&fixture.access_token),
        ),
        404,
    );
    assert_error(
        target.patch_json(
            &format!("{base}/{}", fixture.foreign_dosage_portable_id),
            &fixture.access_token,
            &json!({"dosage_option": {"amount": "3"}}),
        ),
        404,
    );
    assert_error(
        target.put_json(
            &format!("{base}/{}", fixture.foreign_dosage_portable_id),
            &fixture.access_token,
            &json!({"dosage_option": {"amount": "3"}}),
        ),
        404,
    );
    for body in [
        json!({"dosage_option": {}}),
        json!({"dosage_option": {"amount": 2}}),
        json!({"dosage_option": {"description": null}}),
        json!({"dosage_option": {"unknown": true}}),
    ] {
        assert_error(
            target.patch_json_if_match(&path, &fixture.access_token, &body, &etag),
            422,
        );
        assert_error(
            target.put_json_if_match(&path, &fixture.access_token, &body, &etag),
            422,
        );
    }
    for (field, value) in [
        ("current_supply", json!("2.345")),
        ("default_min_hours_between_doses", json!("1.23")),
    ] {
        let mut body = json!({"dosage_option": {}});
        body["dosage_option"][field] = value;
        assert_error(
            target.patch_json_if_match(&path, &fixture.access_token, &body, &etag),
            422,
        );
        assert_error(
            target.put_json_if_match(&path, &fixture.access_token, &body, &etag),
            422,
        );
    }
    let unchanged: Value = target
        .get(&path, Some(&fixture.access_token))
        .json()
        .unwrap();
    assert_eq!(unchanged["data"]["amount"], row["amount"]);
    assert_error(
        target.patch_raw_json_if_match(&path, &fixture.access_token, "{", &etag),
        400,
    );
    assert_error(
        target.put_raw_json_if_match(&path, &fixture.access_token, "{", &etag),
        400,
    );
}
