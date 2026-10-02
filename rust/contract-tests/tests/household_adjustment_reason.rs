use medtracker_contract_tests::{Fixture, Target, fixture};
use postgres::{Client, NoTls};
use scraper::{Html, Selector};
use serde_json::{Value, json};
use std::env;

#[path = "household_completion_medication/authentication.rs"]
mod authentication;

fn input(html: &str, name: &str) -> String {
    Html::parse_document(html)
        .select(&Selector::parse(&format!("input[name='{name}']")).unwrap())
        .next()
        .and_then(|field| field.value().attr("value"))
        .expect("native stock input")
        .to_owned()
}

fn login(target: &Target, fixture: &Fixture, client: &str) {
    let token = input(
        &target.get_html("/login").text().unwrap(),
        "authenticity_token",
    );
    let response = target.post_html_form_from_client(
        "/login",
        client,
        &[
            ("email".into(), fixture.primary_email.clone()),
            ("password".into(), "password".into()),
            ("authenticity_token".into(), token),
        ],
    );
    assert_eq!(response.status().as_u16(), 302);
}

fn setup(name: &str, client: &str) -> (Fixture, authentication::CurrentBearer, Target, i64) {
    let (fixture, guard) = authentication::fixture_with_current_bearer(client);
    let target = Target::from_env();
    login(&target, &fixture, client);
    let response = target.post_json_authorized(
        &format!("/api/v1/households/{}/medications", fixture.household_id),
        &fixture.access_token,
        &json!({"medication": {"name": name, "location_id": fixture.primary_location_id,
            "dose_amount": "1.25", "dose_unit": "ml", "current_supply": "20", "reorder_threshold": "3"}}),
    );
    let status = response.status().as_u16();
    let body = response.json::<Value>().expect("medication setup response");
    assert_eq!(status, 201, "canonical medication setup: {body}");
    let id = body["data"]["id"]
        .as_i64()
        .unwrap();
    (fixture, guard, target, id)
}

fn api(fixture: &Fixture, id: i64) -> String {
    format!(
        "/api/v1/households/{}/medications/{id}",
        fixture.household_id
    )
}

fn read(target: &Target, fixture: &Fixture, id: i64) -> Value {
    let response = target.get(&api(fixture, id), Some(&fixture.access_token));
    assert_eq!(response.status().as_u16(), 200);
    response.json::<Value>().unwrap()["data"].clone()
}

fn counts(id: i64) -> (i64, i64) {
    let mut db = Client::connect(&env::var("CONTRACT_AUDIT_DATABASE_URL").unwrap(), NoTls).unwrap();
    (
        db.query_one("SELECT count(*) FROM versions WHERE item_type = 'Medication' AND item_id = $1", &[&id]).unwrap().get(0),
        db.query_one("SELECT count(*) FROM api_change_events WHERE record_type = 'Medication' AND record_id = $1", &[&id]).unwrap().get(0),
    )
}


fn exact_reason(value: &Value, reason: &str, event: &str) -> bool {
    match value {
        Value::String(text) => text == reason || text == event,
        Value::Array(values) => values.iter().any(|value| exact_reason(value, reason, event)),
        Value::Object(values) => values.values().any(|value| exact_reason(value, reason, event)),
        _ => false,
    }
}

fn adjustment_reason_round_trip(name: &str, client: &str, reason: String) -> (String, Value) {
    let (fixture, _guard, target, id) = setup(name, client);
    let before = counts(id);
    let mut db = Client::connect(&env::var("CONTRACT_AUDIT_DATABASE_URL").unwrap(), NoTls).unwrap();
    let column = db.query_one(
        "SELECT data_type, character_maximum_length FROM information_schema.columns WHERE table_schema = 'public' AND table_name = 'versions' AND column_name = 'event'",
        &[],
    ).expect("actual audit event column");
    let column_type: String = column.get(0);
    let character_limit: Option<i32> = column.get(1);
    assert_eq!(column_type, "character varying");
    assert_eq!(character_limit, None, "do not assume a 255-character audit column");
    let indexes: Vec<String> = db.query(
        "SELECT indexdef FROM pg_indexes WHERE schemaname = 'public' AND tablename = 'versions' AND indexname = 'index_versions_on_event'",
        &[],
    ).unwrap().into_iter().map(|row| row.get(0)).collect();
    let response = target.patch_json(
        &format!("{}/adjust_inventory", api(&fixture, id)),
        &fixture.access_token,
        &json!({"adjustment": {"new_quantity": "15.12", "reason": reason}}),
    );
    let status = response.status().as_u16();
    let response_request = response.headers().get("x-request-id")
        .and_then(|value| value.to_str().ok()).map(str::to_owned);
    let payload = response.json::<Value>().expect("public adjustment response");
    let saved = read(&target, &fixture, id);
    let after = counts(id);
    assert_eq!(status, 200,
        "reason length={}, public response={}, persisted stock={}, version/sync counts {:?}->{:?}, audit column={column_type:?}/{character_limit:?}, audit indexes={indexes:?}",
        reason.len(), payload, saved["current_supply"], before, after);
    assert_eq!(payload["data"]["current_supply"], "15.12");
    assert_eq!(saved["current_supply"], "15.12");
    assert_eq!(after, (before.0 + 1, before.1 + 1));
    let request = response_request.expect("successful adjustment request ID");
    let row = db.query_one(
        "SELECT event, request_id, object_changes, audit_context::text FROM versions WHERE household_id = $1 AND item_type = 'Medication' AND item_id = $2 ORDER BY id DESC LIMIT 1",
        &[&fixture.household_id, &id],
    ).unwrap();
    let event: String = row.get(0);
    let stored_request: String = row.get(1);
    let changes: Value = serde_json::from_str(&row.get::<_, String>(2)).unwrap();
    let audit: Value = serde_json::from_str(&row.get::<_, String>(3)).unwrap();
    let expected_event = format!("adjust inventory (qty: 15.12, reason: {reason})");
    assert!(event == expected_event || exact_reason(&audit, &reason, &expected_event),
        "full adjustment reason must survive in the event or existing structured audit context");
    assert_eq!(stored_request, request);
    assert_eq!(audit["request_id"], request);
    assert_eq!(audit["household_id"], fixture.household_id);
    assert_eq!(audit["actor_membership_id"], fixture.owner_membership_id);
    assert_eq!(changes["current_supply"], json!(["20.00", "15.12"]));
    let linked: i64 = db.query_one(
        "SELECT count(*) FROM api_change_events WHERE request_id = $1 AND record_type = 'Medication' AND record_id = $2 AND household_id = $3",
        &[&request, &id, &fixture.household_id],
    ).unwrap().get(0);
    assert_eq!(linked, 1);
    (event, audit)
}

fn structured_adjustment(audit: &Value, reason: &str) {
    assert_eq!(audit["inventory_adjustment"]["reason"], reason);
    assert_eq!(audit["inventory_adjustment"]["new_quantity"], "15.12");
}

fn boundary_reason(bytes: usize, multibyte: bool) -> String {
    let overhead = "adjust inventory (qty: 15.12, reason: )".len();
    let size = bytes - overhead;
    if multibyte {
        let mut reason = "💊".repeat(size / 4);
        reason.push_str(&"x".repeat(size % 4));
        reason
    } else {
        "x".repeat(size)
    }
}

fn event_budget_round_trip(name: &str, client: &str, bytes: usize, multibyte: bool) {
    let reason = boundary_reason(bytes, multibyte);
    let full = format!("adjust inventory (qty: 15.12, reason: {reason})");
    assert_eq!(full.len(), bytes, "setup counts complete UTF-8 event bytes");
    if multibyte {
        assert!(full.chars().count() < full.len());
    }
    let (event, audit) = adjustment_reason_round_trip(name, client, reason.clone());
    if bytes <= 1024 {
        assert_eq!(event, full, "event at the display budget retains legacy text");
    } else {
        assert_eq!(event, "adjust inventory (qty: 15.12)", "event over the display budget omits the reason without truncating its structured value");
    }
    structured_adjustment(&audit, &reason);
}

#[test]
fn event_budget_keeps_exact_ascii_event_at_1024_bytes() {
    event_budget_round_trip("Audit ASCII event exact budget", "198.18.59.3", 1024, false);
}

#[test]
fn event_budget_uses_quantity_only_ascii_event_at_1025_bytes() {
    event_budget_round_trip("Audit ASCII event over budget", "198.18.59.4", 1025, false);
}

#[test]
fn event_budget_keeps_exact_multibyte_event_at_1024_bytes() {
    event_budget_round_trip("Audit multibyte event exact budget", "198.18.59.5", 1024, true);
}

#[test]
fn event_budget_uses_quantity_only_multibyte_event_at_1025_bytes() {
    event_budget_round_trip("Audit multibyte event over budget", "198.18.59.6", 1025, true);
}

#[test]
fn three_hundred_character_adjustment_reason_survives_audit_and_stock_write() {
    let reason = "Counted delivery and returned stock. ".repeat(10).chars().take(300).collect::<String>();
    assert_eq!(reason.len(), 300);
    let (event, audit) = adjustment_reason_round_trip("Audit reason 300", "198.18.59.1", reason.clone());
    assert_eq!(event, format!("adjust inventory (qty: 15.12, reason: {reason})"));
    structured_adjustment(&audit, &reason);
}

#[test]
fn varied_large_adjustment_reason_probes_actual_indexed_audit_boundary() {
    let mut state = 0x4d545241_u32;
    let reason = (0..8192).map(|_| {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        char::from(b'!' + (state % 94) as u8)
    }).collect::<String>();
    assert_eq!(reason.len(), 8192);
    assert!(reason.chars().collect::<std::collections::HashSet<_>>().len() > 80);
    let (event, audit) = adjustment_reason_round_trip("Audit reason varied large probe", "198.18.59.2", reason.clone());
    assert_eq!(event, "adjust inventory (qty: 15.12)");
    structured_adjustment(&audit, &reason);
}
