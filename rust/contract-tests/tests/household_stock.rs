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
    assert_eq!(response.status().as_u16(), 201);
    let id = response.json::<Value>().unwrap()["data"]["id"]
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

fn web(fixture: &Fixture, id: i64) -> String {
    format!(
        "/households/{}/medications/{id}/stock",
        fixture.household_slug
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

fn form(target: &Target, path: &str) -> String {
    let response = target.get_html(path);
    assert_eq!(response.status().as_u16(), 200, "stock action form exists");
    response.text().unwrap()
}

#[test]
fn scalar_adjustment_records_reason_quantity_and_request_linkage() {
    let (fixture, _guard, target, id) = setup("Stock audit reason probe", "198.18.31.1");
    let response = target.patch_json(
        &format!("{}/adjust_inventory", api(&fixture, id)),
        &fixture.access_token,
        &json!({"adjustment": {"new_quantity": "15.12", "reason": "Counted after delivery"}}),
    );
    assert_eq!(response.status().as_u16(), 200);
    let response_request = response.headers()["x-request-id"]
        .to_str()
        .unwrap()
        .to_owned();
    let payload = response.json::<Value>().unwrap();
    assert_eq!(payload["data"]["current_supply"], "15.12");
    let mut db = Client::connect(&env::var("CONTRACT_AUDIT_DATABASE_URL").unwrap(), NoTls).unwrap();
    let row = db.query_one("SELECT event, request_id, object_changes, audit_context::text FROM versions WHERE item_type = 'Medication' AND item_id = $1 ORDER BY id DESC LIMIT 1", &[&id]).unwrap();
    let event: String = row.get(0);
    assert_eq!(
        event,
        "adjust inventory (qty: 15.12, reason: Counted after delivery)"
    );
    let request: String = row.get(1);
    assert_eq!(request, response_request);
    let changes: Value = serde_json::from_str(&row.get::<_, String>(2)).unwrap();
    let audit: Value = serde_json::from_str(&row.get::<_, String>(3)).unwrap();
    assert_eq!(changes["current_supply"], json!(["20.00", "15.12"]));
    assert_eq!(audit["request_id"], request);
    let linked: i64 = db.query_one("SELECT count(*) FROM api_change_events WHERE request_id = $1 AND record_type = 'Medication' AND record_id = $2", &[&request, &id]).unwrap().get(0);
    assert_eq!(linked, 1);
}

#[test]
fn scalar_browser_adjustment_keeps_rejected_draft_and_blocks_forgery() {
    let (fixture, _guard, target, id) = setup("Scalar browser stock probe", "198.18.31.2");
    let path = web(&fixture, id);
    let html = form(&target, &format!("{path}/adjust"));
    let original = read(&target, &fixture, id);
    let evidence = counts(id);
    let fields = vec![
        ("etag".into(), input(&html, "etag")),
        (
            "authenticity_token".into(),
            input(&html, "authenticity_token"),
        ),
        ("new_quantity".into(), "-0.25".into()),
        ("reason".into(), "Counted <delivery> & returns".into()),
    ];
    let response = target.post_browser_form(&format!("{path}/adjust"), &fields);
    assert_eq!(response.status().as_u16(), 422);
    let rejected = response.text().unwrap();
    assert_eq!(input(&rejected, "new_quantity"), "-0.25");
    assert_eq!(input(&rejected, "reason"), "Counted <delivery> & returns");
    assert_eq!(read(&target, &fixture, id), original);
    assert_eq!(counts(id), evidence);
    let response = target.post_browser_form(
        &format!("{path}/adjust"),
        &[
            ("authenticity_token".into(), "forged".into()),
            ("new_quantity".into(), "19.75".into()),
            ("reason".into(), "Counted".into()),
        ],
    );
    assert_eq!(response.status().as_u16(), 403);
    let response = target.post_browser_form(
        &format!("{}/adjust", web(&fixture, fixture.foreign_medication_id)),
        &fields,
    );
    assert_eq!(response.status().as_u16(), 404);
    assert_eq!(read(&target, &fixture, id), original);
    assert_eq!(counts(id), evidence);
    let response = target.post_browser_form(
        &format!("{path}/adjust"),
        &[
            ("etag".into(), input(&html, "etag")),
            (
                "authenticity_token".into(),
                input(&html, "authenticity_token"),
            ),
            ("new_quantity".into(), "19.75".into()),
            ("reason".into(), "Counted".into()),
        ],
    );
    assert_eq!(response.status().as_u16(), 303);
    assert_eq!(read(&target, &fixture, id)["current_supply"], "19.75");
    assert_eq!(counts(id), (evidence.0 + 1, evidence.1 + 1));
}

#[test]
fn browser_order_and_receipt_are_separate_from_stock_adjustment() {
    let (fixture, _guard, target, id) = setup("Browser stock order probe", "198.18.31.3");
    let path = web(&fixture, id);
    let html = form(&target, &format!("{path}/order"));
    let evidence = counts(id);
    let mut fields = vec![
        (
            "authenticity_token".into(),
            input(&html, "authenticity_token"),
        ),
        ("supplier".into(), "Community pharmacy".into()),
        ("quantity".into(), "8.25".into()),
        ("expected_arrival_on".into(), "invalid-date".into()),
    ];
    let response = target.post_browser_form(&format!("{path}/order"), &fields);
    assert_eq!(response.status().as_u16(), 422);
    let rejected = response.text().unwrap();
    for (key, value) in fields.iter().skip(1) {
        assert_eq!(input(&rejected, key), *value);
    }
    assert_eq!(counts(id), evidence);
    fields[3].1 = "2030-05-20".into();
    assert_eq!(
        target
            .post_browser_form(&format!("{path}/order"), &fields)
            .status()
            .as_u16(),
        303
    );
    assert_eq!(read(&target, &fixture, id)["reorder_status"], "ordered");
    let mut db = Client::connect(&env::var("CONTRACT_AUDIT_DATABASE_URL").unwrap(), NoTls).unwrap();
    let row = db.query_one("SELECT order_supplier, order_quantity::text, expected_arrival_on::text, ordered_at IS NOT NULL FROM medications WHERE id = $1", &[&id]).unwrap();
    assert_eq!(row.get::<_, String>(0), "Community pharmacy");
    assert_eq!(row.get::<_, String>(1), "8.25");
    assert_eq!(row.get::<_, String>(2), "2030-05-20");
    assert!(row.get::<_, bool>(3));
    let html = form(&target, &path);
    assert_eq!(
        target
            .post_browser_form(
                &format!("{path}/receive"),
                &[(
                    "authenticity_token".into(),
                    input(&html, "authenticity_token")
                )]
            )
            .status()
            .as_u16(),
        303
    );
    let received = read(&target, &fixture, id);
    assert_eq!(received["reorder_status"], "received");
    assert_eq!(received["current_supply"], "20.0");
    assert_eq!(counts(id), (evidence.0 + 2, evidence.1 + 2));
}

#[test]
fn option_stock_uses_real_option_editor_and_rejects_parent_adjustment() {
    let (fixture, _guard, target, id) = setup("Option stock routing probe", "198.18.31.4");
    let response = target.post_json_authorized(&format!("/api/v1/households/{}/dosage_options", fixture.household_id), &fixture.access_token,
        &json!({"dosage_option": {"medication_id": id.to_string(), "amount": "1.25", "unit": "ml", "frequency": "daily", "current_supply": "12.25", "reorder_threshold": "2.5", "default_max_daily_doses": 4, "default_min_hours_between_doses": "0", "default_dose_cycle": "daily"}}));
    assert_eq!(response.status().as_u16(), 201);
    let option = response.json::<Value>().unwrap()["data"]["id"]
        .as_i64()
        .unwrap();
    let path = web(&fixture, id);
    let html = form(&target, &path);
    let edit = format!(
        "/households/{}/medications/{id}/dosage_options/{option}/edit",
        fixture.household_slug
    );
    assert!(html.contains(&format!("href=\"{edit}\"")));
    assert!(!html.contains(&format!("href=\"{path}/adjust\"")));
    let before = read(&target, &fixture, id);
    let parent_reply = target.get(&api(&fixture, id), Some(&fixture.access_token));
    let parent_etag = parent_reply.headers()["etag"].to_str().unwrap().to_owned();
    let evidence = counts(id);
    assert_eq!(
        target.get_html(&format!("{path}/adjust")).status().as_u16(),
        409
    );
    assert_eq!(
        target
            .post_browser_form(
                &format!("{path}/adjust"),
                &[
                    ("etag".into(), parent_etag),
                    (
                        "authenticity_token".into(),
                        input(&html, "authenticity_token")
                    ),
                    ("new_quantity".into(), "90".into()),
                    ("reason".into(), "forged parent".into())
                ]
            )
            .status()
            .as_u16(),
        409
    );
    assert_eq!(read(&target, &fixture, id), before);
    assert_eq!(counts(id), evidence);
    let html = form(&target, &edit);
    assert!(!input(&html, "etag").is_empty());
    assert_eq!(input(&html, "current_supply"), "12.25");
}
