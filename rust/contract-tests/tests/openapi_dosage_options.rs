use medtracker_contract_tests::{fixture, Fixture, Target};
use serde_json::{json, Value};
use std::env;
use std::time::{Duration, Instant};
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

fn database() -> postgres::Client {
    postgres::Client::connect(
        &env::var("CONTRACT_AUDIT_DATABASE_URL").expect("database URL"),
        postgres::NoTls,
    )
    .expect("contract database")
}

struct DisposableApps {
    db: postgres::Client,
    ids: Vec<i64>,
}

impl DisposableApps {
    fn new() -> Self {
        Self {
            db: database(),
            ids: Vec::new(),
        }
    }

    fn issue(&mut self, account_id: i64, membership_id: i64) -> (i64, String) {
        let raw = format!(
            "mt_app_contract_{}",
            OffsetDateTime::now_utc().unix_timestamp_nanos()
        );
        let id: i64 = self.db.query_one(
            "INSERT INTO api_app_tokens (account_id, household_membership_id, token_digest, permissions_version, name, expires_at, last_used_at, created_at, updated_at) VALUES ($1, $2, encode(digest($3, 'sha256'), 'hex'), (SELECT permissions_version FROM household_memberships WHERE id = $2), 'Dosage contract', now() + interval '1 day', now() - interval '10 minutes', now(), now()) RETURNING id",
            &[&account_id, &membership_id, &raw],
        ).expect("app token insert").get(0);
        self.ids.push(id);
        (id, raw)
    }

    fn for_membership(&mut self, membership_id: i64) -> (i64, String) {
        let account_id: i64 = self
            .db
            .query_one(
                "SELECT account_id FROM household_memberships WHERE id = $1",
                &[&membership_id],
            )
            .expect("member account")
            .get(0);
        self.issue(account_id, membership_id)
    }

    fn member_id_for_session(&mut self, raw: &str) -> i64 {
        self.db.query_one(
            "SELECT household_membership_id FROM api_sessions WHERE access_token_digest = encode(digest($1, 'sha256'), 'hex')",
            &[&raw],
        ).expect("session membership").get(0)
    }
}

impl Drop for DisposableApps {
    fn drop(&mut self) {
        for id in &self.ids {
            self.db
                .execute("DELETE FROM api_app_tokens WHERE id = $1", &[id])
                .expect("app token cleanup");
        }
    }
}

struct HouseholdStateGuard {
    db: postgres::Client,
    household_id: i64,
    status: String,
    lifecycle_state: String,
}

impl HouseholdStateGuard {
    fn deactivate(household_id: i64) -> Self {
        let mut db = database();
        let household = db
            .query_one(
                "SELECT status, lifecycle_state FROM households WHERE id = $1",
                &[&household_id],
            )
            .expect("issuing household state");
        let status: String = household.get(0);
        let lifecycle_state: String = household.get(1);
        assert_eq!(status, "active");
        assert_eq!(lifecycle_state, "active");
        db.execute(
            "UPDATE households SET status = 'archived' WHERE id = $1",
            &[&household_id],
        )
        .expect("deactivate issuing household");
        Self {
            db,
            household_id,
            status,
            lifecycle_state,
        }
    }
}

impl Drop for HouseholdStateGuard {
    fn drop(&mut self) {
        self.db
            .execute(
                "UPDATE households SET status = $1, lifecycle_state = $2 WHERE id = $3",
                &[&self.status, &self.lifecycle_state, &self.household_id],
            )
            .expect("restore issuing household state");
    }
}

fn parent_state(id: i64) -> (Option<f64>, Option<String>, String, Option<String>, String) {
    let row = database()
        .query_one(
            "SELECT dose_amount, current_supply::text, reorder_threshold::text, supply_at_last_restock::text, updated_at::text FROM medications WHERE id = $1",
            &[&id],
        )
        .expect("parent medication");
    (row.get(0), row.get(1), row.get(2), row.get(3), row.get(4))
}

fn event_count(record_type: &str, id: i64) -> i64 {
    database()
        .query_one(
            "SELECT count(*) FROM api_change_events WHERE record_type = $1 AND record_id = $2",
            &[&record_type, &id],
        )
        .unwrap()
        .get(0)
}

fn version_count(item_type: &str, id: i64) -> i64 {
    database()
        .query_one(
            "SELECT count(*) FROM versions WHERE item_type = $1 AND item_id = $2",
            &[&item_type, &id],
        )
        .unwrap()
        .get(0)
}

fn new_parent(target: &Target, fixture: &Fixture) -> i64 {
    let response = target.post_json_authorized(
        &format!("/api/v1/households/{}/medications", fixture.household_id),
        &fixture.access_token,
        &json!({"medication": {"name": format!("Dosage parent {}", OffsetDateTime::now_utc().unix_timestamp_nanos()),
            "location_id": fixture.primary_location_id, "dose_amount": "2",
            "dose_unit": "ml", "current_supply": "50", "reorder_threshold": "5"}}),
    );
    assert_eq!(response.status().as_u16(), 201);
    let body: Value = response.json().unwrap();
    body["data"]["id"].as_i64().unwrap()
}

fn request_for_parent(fixture: &Fixture, id: i64) -> Value {
    let mut request = valid_request(fixture);
    request["dosage_option"]["medication_id"] = json!(id.to_string());
    request
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
fn administrator_can_read_and_manage_dosage_options() {
    let fixture = fixture();
    let target = Target::from_env();
    let base = base(&fixture);
    let path = format!("{base}/{}", fixture.managed_dosage_portable_id);
    assert_eq!(
        target
            .get(&base, Some(&fixture.manager_access_token))
            .status()
            .as_u16(),
        200
    );
    assert_eq!(
        target
            .get(&path, Some(&fixture.manager_access_token))
            .status()
            .as_u16(),
        200
    );
    let parent = new_parent(&target, &fixture);
    let created = target.post_json_authorized(
        &base,
        &fixture.manager_access_token,
        &request_for_parent(&fixture, parent),
    );
    assert_eq!(created.status().as_u16(), 201);
    let row: Value = created.json().unwrap();
    let path = format!("{base}/{}", row["data"]["id"]);
    assert_eq!(
        target
            .patch_json(
                &path,
                &fixture.manager_access_token,
                &json!({"dosage_option": {"amount": "2"}})
            )
            .status()
            .as_u16(),
        200
    );
    assert_eq!(
        target
            .put_json(
                &path,
                &fixture.manager_access_token,
                &json!({"dosage_option": {"amount": "3"}})
            )
            .status()
            .as_u16(),
        200
    );
}

#[test]
fn member_reads_only_options_for_visible_medications_and_cannot_write() {
    let fixture = fixture();
    let target = Target::from_env();
    let list = target.get(
        &format!("{}?per_page=100", base(&fixture)),
        Some(&fixture.view_access_token),
    );
    assert_eq!(list.status().as_u16(), 200);
    let body: Value = list.json().unwrap();
    let ids: Vec<i64> = body["data"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["medication_id"].as_i64().unwrap())
        .collect();
    assert!(ids.contains(&fixture.managed_medication_id));
    assert!(!ids.contains(&fixture.hidden_medication_id));
    assert_eq!(
        target
            .get(
                &format!("{}/{}", base(&fixture), fixture.managed_dosage_portable_id),
                Some(&fixture.view_access_token)
            )
            .status()
            .as_u16(),
        200
    );
    assert_error(
        target.get(
            &format!("{}/{}", base(&fixture), fixture.hidden_dosage_portable_id),
            Some(&fixture.view_access_token),
        ),
        404,
    );
    assert_error(
        target.post_json_authorized(
            &base(&fixture),
            &fixture.view_access_token,
            &valid_request(&fixture),
        ),
        403,
    );
    let path = format!("{}/{}", base(&fixture), fixture.managed_dosage_portable_id);
    assert_error(
        target.patch_json(
            &path,
            &fixture.view_access_token,
            &json!({"dosage_option": {"amount": "2"}}),
        ),
        403,
    );
    assert_error(
        target.put_json(
            &path,
            &fixture.view_access_token,
            &json!({"dosage_option": {"amount": "2"}}),
        ),
        403,
    );
}

#[test]
fn create_and_inventory_updates_are_atomic_with_parent_and_sync_events() {
    let fixture = fixture();
    let target = Target::from_env();
    let parent = new_parent(&target, &fixture);
    let before = parent_state(parent);
    let parent_events = event_count("Medication", parent);
    let mut request = request_for_parent(&fixture, parent);
    request["dosage_option"]["current_supply"] = json!("12.25");
    let response = target.post_json_authorized(&base(&fixture), &fixture.access_token, &request);
    assert_eq!(response.status().as_u16(), 201);
    let request_id = response.headers()["x-request-id"]
        .to_str()
        .unwrap()
        .to_owned();
    let body: Value = response.json().unwrap();
    let id = body["data"]["id"].as_i64().unwrap();
    let after = parent_state(parent);
    assert_eq!(after.0, None);
    assert_eq!(after.1.as_deref(), Some("12.25"));
    assert_eq!(after.2, "2.50");
    assert_eq!(after.3.as_deref(), Some("12.25"));
    assert_ne!(before.4, after.4);
    assert_eq!(event_count("Medication", parent), parent_events + 1);
    assert_eq!(event_count("MedicationDosageOption", id), 1);
    let rows = database()
        .query(
            "SELECT record_type FROM api_change_events WHERE request_id = $1",
            &[&request_id],
        )
        .unwrap();
    assert!(rows
        .iter()
        .any(|row| row.get::<_, String>(0) == "Medication"));
    assert!(rows
        .iter()
        .any(|row| row.get::<_, String>(0) == "MedicationDosageOption"));
    let versions: i64 = database().query_one(
        "SELECT count(*) FROM versions WHERE item_type = 'MedicationDosageOption' AND item_id = $1 AND event = 'api_create' AND request_id = $2",
        &[&id, &request_id],
    ).unwrap().get(0);
    assert_eq!(versions, 1);
    let parent_versions: i64 = database().query_one(
        "SELECT count(*) FROM versions WHERE item_type = 'Medication' AND item_id = $1 AND request_id = $2",
        &[&parent, &request_id],
    ).unwrap().get(0);
    assert_eq!(parent_versions, 1);
    let mut second_request = request_for_parent(&fixture, parent);
    second_request["dosage_option"]["current_supply"] = json!("5.00");
    second_request["dosage_option"]["reorder_threshold"] = json!("3.00");
    let second_response =
        target.post_json_authorized(&base(&fixture), &fixture.access_token, &second_request);
    assert_eq!(second_response.status().as_u16(), 201);
    let second: Value = second_response.json().unwrap();
    let second_path = format!("{}/{}", base(&fixture), second["data"]["id"]);
    let mut untracked = request_for_parent(&fixture, parent);
    untracked["dosage_option"]["reorder_threshold"] = json!("99.00");
    assert_eq!(
        target
            .post_json_authorized(&base(&fixture), &fixture.access_token, &untracked)
            .status()
            .as_u16(),
        201
    );
    let aggregated = parent_state(parent);
    assert_eq!(aggregated.1.as_deref(), Some("17.25"));
    assert_eq!(aggregated.2, "5.50");
    assert_eq!(aggregated.3.as_deref(), Some("17.25"));
    let path = format!("{}/{}", base(&fixture), id);
    let changed_response = target.patch_json(
        &path,
        &fixture.access_token,
        &json!({"dosage_option": {"current_supply": "7.00", "reorder_threshold": "1.00"}}),
    );
    assert_eq!(changed_response.status().as_u16(), 200);
    let changed_request_id = changed_response.headers()["x-request-id"].to_str().unwrap();
    let parent_versions: i64 = database().query_one(
        "SELECT count(*) FROM versions WHERE item_type = 'Medication' AND item_id = $1 AND request_id = $2",
        &[&parent, &changed_request_id],
    ).unwrap().get(0);
    assert_eq!(parent_versions, 1);
    let changed = parent_state(parent);
    assert_eq!(changed.1.as_deref(), Some("12.00"));
    assert_eq!(changed.2, "4.00");
    assert_eq!(changed.3.as_deref(), Some("17.25"));
    assert_eq!(
        target
            .put_json(
                &path,
                &fixture.access_token,
                &json!({"dosage_option": {"current_supply": null}})
            )
            .status()
            .as_u16(),
        200
    );
    let still_tracked = parent_state(parent);
    assert_eq!(still_tracked.1.as_deref(), Some("5.00"));
    assert_eq!(still_tracked.2, "3.00");
    assert_eq!(still_tracked.3.as_deref(), Some("17.25"));
    assert_eq!(
        target
            .put_json(
                &second_path,
                &fixture.access_token,
                &json!({"dosage_option": {"current_supply": null}})
            )
            .status()
            .as_u16(),
        200
    );
    let reset = parent_state(parent);
    assert_eq!(reset.1, None);
    assert_eq!(reset.2, "0.00");
    assert_eq!(reset.3, None);
}

#[test]
fn rejects_invalid_quantities_and_duplicate_defaults_without_side_effects() {
    let fixture = fixture();
    let target = Target::from_env();
    let parent = new_parent(&target, &fixture);
    for (field, value) in [
        ("amount", "0"),
        ("amount", "-1"),
        ("default_min_hours_between_doses", "-1"),
        ("current_supply", "-1"),
        ("reorder_threshold", "-1"),
        ("unit", "  "),
        ("frequency", "  "),
    ] {
        let mut request = request_for_parent(&fixture, parent);
        request["dosage_option"][field] = json!(value);
        assert_error(
            target.post_json_authorized(&base(&fixture), &fixture.access_token, &request),
            422,
        );
    }
    let mut request = request_for_parent(&fixture, parent);
    request["dosage_option"]["default_for_adults"] = json!(true);
    let first = target.post_json_authorized(&base(&fixture), &fixture.access_token, &request);
    assert_eq!(first.status().as_u16(), 201);
    let first: Value = first.json().unwrap();
    let before = parent_state(parent);
    let parent_events = event_count("Medication", parent);
    let parent_versions = version_count("Medication", parent);
    assert_error(
        target.post_json_authorized(&base(&fixture), &fixture.access_token, &request),
        422,
    );
    assert_eq!(parent_state(parent), before);
    assert_eq!(event_count("Medication", parent), parent_events);
    assert_eq!(version_count("Medication", parent), parent_versions);
    let mut second = request_for_parent(&fixture, parent);
    second["dosage_option"]["default_for_children"] = json!(true);
    let created = target.post_json_authorized(&base(&fixture), &fixture.access_token, &second);
    assert_eq!(created.status().as_u16(), 201);
    let second: Value = created.json().unwrap();
    let path = format!("{}/{}", base(&fixture), second["data"]["id"]);
    assert_error(
        target.patch_json(
            &path,
            &fixture.access_token,
            &json!({"dosage_option": {"default_for_adults": true}}),
        ),
        422,
    );
    let before = parent_state(parent);
    let parent_events = event_count("Medication", parent);
    let parent_versions = version_count("Medication", parent);
    assert_error(
        target.put_json(
            &format!("{}/{}", base(&fixture), first["data"]["id"]),
            &fixture.access_token,
            &json!({"dosage_option": {"default_for_children": true}}),
        ),
        422,
    );
    assert_eq!(parent_state(parent), before);
    assert_eq!(event_count("Medication", parent), parent_events);
    assert_eq!(version_count("Medication", parent), parent_versions);
    let reread: Value = target
        .get(&path, Some(&fixture.access_token))
        .json()
        .unwrap();
    assert_eq!(reread["data"]["default_for_adults"], false);
    assert_eq!(first["data"]["default_for_adults"], true);
    let overflow_parent = new_parent(&target, &fixture);
    let mut big = request_for_parent(&fixture, overflow_parent);
    big["dosage_option"]["current_supply"] = json!("60000000.00");
    assert_eq!(
        target
            .post_json_authorized(&base(&fixture), &fixture.access_token, &big)
            .status()
            .as_u16(),
        201
    );
    let before = parent_state(overflow_parent);
    let parent_events = event_count("Medication", overflow_parent);
    let parent_versions = version_count("Medication", overflow_parent);
    assert_error(
        target.post_json_authorized(&base(&fixture), &fixture.access_token, &big),
        422,
    );
    assert_eq!(parent_state(overflow_parent), before);
    assert_eq!(event_count("Medication", overflow_parent), parent_events);
    assert_eq!(
        version_count("Medication", overflow_parent),
        parent_versions
    );
}

#[test]
fn database_rejects_null_required_fields_without_changing_valid_dosage() {
    let fixture = fixture();
    let target = Target::from_env();
    let parent = new_parent(&target, &fixture);
    let response = target.post_json_authorized(
        &base(&fixture),
        &fixture.access_token,
        &request_for_parent(&fixture, parent),
    );
    assert_eq!(response.status().as_u16(), 201);
    let created: Value = response.json().unwrap();
    let id = created["data"]["id"].as_i64().unwrap();
    let path = format!("{}/{}", base(&fixture), id);
    let before: Value = target
        .get(&path, Some(&fixture.access_token))
        .json()
        .unwrap();
    let mut db = database();
    for field in [
        "amount",
        "unit",
        "frequency",
        "default_max_daily_doses",
        "default_min_hours_between_doses",
        "default_dose_cycle",
    ] {
        let error = db
            .execute(
                &format!("UPDATE dosages SET {field} = NULL WHERE id = $1"),
                &[&id],
            )
            .unwrap_err();
        assert_eq!(
            error.code().map(|code| code.code()),
            Some("23502"),
            "{field}"
        );
    }
    let after: Value = target
        .get(&path, Some(&fixture.access_token))
        .json()
        .unwrap();
    assert_eq!(after, before);
}

#[test]
fn household_app_tokens_obey_owner_admin_member_scope_and_audit_identity() {
    let fixture = fixture();
    let target = Target::from_env();
    let mut apps = DisposableApps::new();
    let (owner_id, owner) = apps.for_membership(fixture.owner_membership_id);
    let before_touch: String = apps
        .db
        .query_one(
            "SELECT last_used_at::text FROM api_app_tokens WHERE id = $1",
            &[&owner_id],
        )
        .unwrap()
        .get(0);
    let member_id = apps.member_id_for_session(&fixture.view_access_token);
    let (_, member) = apps.for_membership(member_id);
    let collection = base(&fixture);
    let visible = format!("{collection}/{}", fixture.managed_dosage_portable_id);
    let hidden = format!("{collection}/{}", fixture.hidden_dosage_portable_id);
    for token in [&owner, &fixture.manager_app_token, &member] {
        assert_eq!(target.get(&collection, Some(token)).status().as_u16(), 200);
        assert_eq!(target.get(&visible, Some(token)).status().as_u16(), 200);
    }
    assert_error(target.get(&hidden, Some(&member)), 404);
    assert_error(
        target.post_json_authorized(&collection, &member, &valid_request(&fixture)),
        403,
    );
    assert_error(
        target.patch_json(
            &visible,
            &member,
            &json!({"dosage_option": {"amount": "2"}}),
        ),
        403,
    );
    assert_error(
        target.put_json(
            &visible,
            &member,
            &json!({"dosage_option": {"amount": "2"}}),
        ),
        403,
    );
    let parent = new_parent(&target, &fixture);
    let response =
        target.post_json_authorized(&collection, &owner, &request_for_parent(&fixture, parent));
    assert_eq!(response.status().as_u16(), 201);
    let request_id = response.headers()["x-request-id"]
        .to_str()
        .unwrap()
        .to_owned();
    let created: Value = response.json().unwrap();
    let path = format!("{collection}/{}", created["data"]["id"]);
    let audit = apps.db.query_one(
        "SELECT audit_context->>'authentication_method', audit_context->>'session_reference' FROM security_audit_events WHERE request_id = $1 AND event_type = 'api.request'",
        &[&request_id],
    ).expect("app token request audit");
    assert_eq!(audit.get::<_, String>(0), "api_app_token");
    assert_eq!(
        audit.get::<_, String>(1),
        format!("api_app_token:{owner_id}")
    );
    let touched: String = apps
        .db
        .query_one(
            "SELECT last_used_at::text FROM api_app_tokens WHERE id = $1",
            &[&owner_id],
        )
        .unwrap()
        .get(0);
    assert_ne!(before_touch, touched);
    assert_eq!(target.get(&path, Some(&owner)).status().as_u16(), 200);
    let untouched: String = apps
        .db
        .query_one(
            "SELECT last_used_at::text FROM api_app_tokens WHERE id = $1",
            &[&owner_id],
        )
        .unwrap()
        .get(0);
    assert_eq!(touched, untouched);
    assert_eq!(
        target
            .patch_json(
                &path,
                &fixture.manager_app_token,
                &json!({"dosage_option": {"amount": "2"}})
            )
            .status()
            .as_u16(),
        200
    );
    assert_eq!(
        target
            .put_json(&path, &owner, &json!({"dosage_option": {"amount": "3"}}))
            .status()
            .as_u16(),
        200
    );
}

#[test]
fn household_app_tokens_reject_invalid_state_and_foreign_households() {
    let fixture = fixture();
    let target = Target::from_env();
    let mut apps = DisposableApps::new();
    let collection = base(&fixture);
    let (expired_id, expired) = apps.for_membership(fixture.owner_membership_id);
    apps.db.execute("UPDATE api_app_tokens SET created_at = now() - interval '1 day', expires_at = now() - interval '1 second' WHERE id = $1", &[&expired_id]).unwrap();
    let (old_id, old) = apps.for_membership(fixture.owner_membership_id);
    apps.db.execute("UPDATE api_app_tokens SET created_at = now() - interval '13 months', expires_at = now() + interval '1 day' WHERE id = $1", &[&old_id]).unwrap();
    let (revoked_id, revoked) = apps.for_membership(fixture.owner_membership_id);
    apps.db
        .execute(
            "UPDATE api_app_tokens SET revoked_at = now() WHERE id = $1",
            &[&revoked_id],
        )
        .unwrap();
    let (stale_id, stale) = apps.for_membership(fixture.owner_membership_id);
    apps.db
        .execute(
            "UPDATE api_app_tokens SET permissions_version = permissions_version + 1 WHERE id = $1",
            &[&stale_id],
        )
        .unwrap();
    let (_, mismatched) = apps.issue(fixture.account_id, fixture.foreign_membership_id);
    let suspended_member: i64 = apps.db.query_one(
        "SELECT id FROM household_memberships WHERE status = 'suspended' AND household_id = $1 LIMIT 1",
        &[&fixture.auth_suspended_household_id],
    ).unwrap().get(0);
    let (_, suspended) = apps.for_membership(suspended_member);
    let revoked_member_id = apps.member_id_for_session(&fixture.portable_revoked_access_token);
    let (_, revoked_member) = apps.for_membership(revoked_member_id);
    let locked_member: i64 = apps.db.query_one(
        "SELECT hm.id FROM household_memberships hm JOIN account_lockouts l ON l.account_id = hm.account_id WHERE l.deadline > now() LIMIT 1", &[],
    ).unwrap().get(0);
    let (_, locked) = apps.for_membership(locked_member);
    let inactive_member: i64 = apps
        .db
        .query_one(
            "SELECT id FROM household_memberships WHERE household_id = $1 LIMIT 1",
            &[&fixture.auth_inactive_household_id],
        )
        .unwrap()
        .get(0);
    let (_, inactive) = apps.for_membership(inactive_member);
    for token in [
        expired.as_str(),
        old.as_str(),
        revoked.as_str(),
        stale.as_str(),
        mismatched.as_str(),
        suspended.as_str(),
        revoked_member.as_str(),
        locked.as_str(),
        inactive.as_str(),
        fixture.auth_deactivated_app_token.as_str(),
        fixture.fhir_patient_scope_token.as_str(),
    ] {
        assert_error(target.get(&collection, Some(token)), 401);
    }
    let (_, owner) = apps.for_membership(fixture.owner_membership_id);
    assert_error(
        target.get(
            &format!(
                "/api/v1/households/{}/dosage_options",
                fixture.foreign_household_id
            ),
            Some(&owner),
        ),
        403,
    );
    assert_error(
        target.get(
            &format!("/api/v1/households/{}/dosage_options", i64::MAX),
            Some(&owner),
        ),
        404,
    );
    let foreign_collection = format!(
        "/api/v1/households/{}/dosage_options",
        fixture.foreign_household_id
    );
    let foreign_detail = format!(
        "{foreign_collection}/{}",
        fixture.foreign_dosage_portable_id
    );
    assert_error(
        target.post_json_authorized(&foreign_collection, &owner, &valid_request(&fixture)),
        403,
    );
    assert_error(target.get(&foreign_detail, Some(&owner)), 403);
    let update = json!({"dosage_option": {"amount": "3"}});
    assert_error(target.patch_json(&foreign_detail, &owner, &update), 403);
    assert_error(target.put_json(&foreign_detail, &owner, &update), 403);
    assert_error(
        target.get(&collection, Some(&fixture.foreign_app_token)),
        403,
    );
    assert_eq!(
        target
            .get(
                &format!(
                    "/api/v1/households/{}/dosage_options",
                    fixture.foreign_household_id
                ),
                Some(&fixture.foreign_app_token)
            )
            .status()
            .as_u16(),
        200
    );
    let mut foreign = valid_request(&fixture);
    foreign["dosage_option"]["medication_id"] = json!(fixture.foreign_medication_id.to_string());
    assert_error(
        target.post_json_authorized(&collection, &owner, &foreign),
        404,
    );
}

#[test]
fn household_app_tokens_are_rejected_when_issuing_household_is_archived() {
    let fixture = fixture();
    let target = Target::from_env();
    let mut apps = DisposableApps::new();
    let (_, app_token) = apps.for_membership(fixture.owner_membership_id);
    let household_path = base(&fixture);
    assert_eq!(
        target
            .get(&household_path, Some(&app_token))
            .status()
            .as_u16(),
        200
    );
    let state = HouseholdStateGuard::deactivate(fixture.household_id);
    let response = target.get(&household_path, Some(&app_token));
    assert_eq!(response.status().as_u16(), 401);
    let body: Value = response.json().expect("error JSON");
    assert_eq!(body["error"]["code"], "unauthorized");
    drop(state);
    assert_eq!(
        target
            .get(&household_path, Some(&app_token))
            .status()
            .as_u16(),
        200
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

#[test]
fn z_rate_limit_wires_all_five_dosage_operations_without_mutation() {
    let fixture = fixture();
    let rate_origin = env::var("CONTRACT_RATE_BASE_URL").expect("nonloopback API URL");
    let client = reqwest::blocking::Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap();
    let started = Instant::now();
    let mut limited = false;
    for _ in 0..601 {
        let response = client
            .get(format!("{rate_origin}/api/v1/capabilities"))
            .send()
            .unwrap();
        if response.status().as_u16() == 429 {
            assert_eq!(response.headers()["ratelimit-limit"], "300");
            assert_eq!(response.headers()["ratelimit-remaining"], "0");
            let body: Value = response.json().unwrap();
            assert_eq!(keys(&body), ["error"]);
            assert_eq!(keys(&body["error"]), ["code", "message"]);
            assert_eq!(body["error"]["code"], "rate_limited");
            limited = true;
            break;
        }
        assert_eq!(response.status().as_u16(), 200);
    }
    assert!(limited && started.elapsed() < Duration::from_secs(60));
    let collection = format!("{rate_origin}{}", base(&fixture));
    let detail = format!("{collection}/{}", fixture.managed_dosage_portable_id);
    for request in [
        client.get(&collection),
        client.post(&collection).json(&valid_request(&fixture)),
        client.get(&detail),
        client
            .patch(&detail)
            .json(&json!({"dosage_option": {"amount": "99"}})),
        client
            .put(&detail)
            .json(&json!({"dosage_option": {"amount": "99"}})),
    ] {
        assert_eq!(
            request
                .bearer_auth(&fixture.access_token)
                .send()
                .unwrap()
                .status()
                .as_u16(),
            429
        );
    }
}
