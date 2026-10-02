use medtracker_contract_tests::{Fixture, Target, fixture};
use postgres::{Client, NoTls};
use scraper::{Html, Selector};
use serde_json::{Value, json};
use std::env;

fn input(html: &str, name: &str) -> String {
    Html::parse_document(html).select(&Selector::parse(&format!("input[name='{name}']")).unwrap())
        .next().and_then(|field| field.value().attr("value")).expect("native form value").to_owned()
}

fn setup(name: &str, client: &str) -> (Fixture, Target, String, Value) {
    let fixture = fixture();
    let target = Target::from_env();
    let html = target.get_html("/login").text().unwrap();
    assert_eq!(target.post_html_form_from_client("/login", client, &[
        ("email".into(), fixture.primary_email.clone()), ("password".into(), "password".into()),
        ("authenticity_token".into(), input(&html, "authenticity_token")),
    ]).status().as_u16(), 302);
    let html = target.get_html(&format!("/households/{}/medications", fixture.household_slug)).text().unwrap();
    let csrf = Html::parse_document(&html).select(&Selector::parse("meta[name='csrf-token']").unwrap()).next()
        .unwrap().value().attr("content").unwrap().to_owned();
    let response = target.post_browser_json(&format!("/api/v1/households/{}/medications", fixture.household_id), &csrf,
        &json!({"medication": {"name": name, "location_id": fixture.primary_location_id, "dose_amount": "2.5", "dose_unit": "ml", "current_supply": "20", "reorder_threshold": "3"}}));
    assert_eq!(response.status().as_u16(), 201);
    let medication = response.json::<Value>().unwrap()["data"].clone();
    (fixture, target, csrf, medication)
}

fn option(target: &Target, fixture: &Fixture, csrf: &str, parent: &Value, unit: &str, supply: Value) -> Value {
    let response = target.post_browser_json(&format!("/api/v1/households/{}/dosage_options", fixture.household_id), csrf,
        &json!({"dosage_option": {"medication_id": parent["id"].as_i64().unwrap().to_string(), "amount": "1.25", "unit": unit, "frequency": "daily",
            "current_supply": supply, "reorder_threshold": null, "default_max_daily_doses": 4, "default_min_hours_between_doses": "0", "default_dose_cycle": "daily"}}));
    assert_eq!(response.status().as_u16(), 201);
    response.json::<Value>().unwrap()["data"].clone()
}

fn read(target: &Target, path: &str) -> Value {
    let response = target.get(path, None);
    assert_eq!(response.status().as_u16(), 200);
    response.json::<Value>().unwrap()["data"].clone()
}

fn path(fixture: &Fixture, medication: &Value) -> String {
    format!("/households/{}/medications/{}", fixture.household_slug, medication["id"].as_i64().unwrap())
}

fn api(fixture: &Fixture, medication: &Value) -> String {
    format!("/api/v1/households/{}/medications/{}", fixture.household_id, medication["id"].as_i64().unwrap())
}

#[test]
fn option_mode_parent_threshold_is_omitted_and_forged_edit_writes_nothing() {
    let (fixture, target, csrf, medication) = setup("Option threshold boundary", "198.18.32.1");
    option(&target, &fixture, &csrf, &medication, "tablet", json!("12.25"));
    let edit = format!("{}/edit", path(&fixture, &medication));
    let response = target.get_html(&edit);
    assert_eq!(response.status().as_u16(), 200);
    let html = response.text().unwrap();
    assert!(Html::parse_document(&html).select(&Selector::parse("[name='reorder_threshold']").unwrap()).next().is_none(), "option mode must not submit parent threshold");
    let parent_api = api(&fixture, &medication);
    let before = read(&target, &parent_api);
    let id = medication["id"].as_i64().unwrap();
    let mut db = Client::connect(&env::var("CONTRACT_AUDIT_DATABASE_URL").unwrap(), NoTls).unwrap();
    let counts = |db: &mut Client| -> (i64, i64) {
        (db.query_one("SELECT count(*) FROM versions WHERE item_type = 'Medication' AND item_id = $1", &[&id]).unwrap().get(0),
         db.query_one("SELECT count(*) FROM api_change_events WHERE record_type = 'Medication' AND record_id = $1", &[&id]).unwrap().get(0))
    };
    let evidence = counts(&mut db);
    let fields = vec![("authenticity_token".into(), input(&html, "authenticity_token")), ("etag".into(), input(&html, "etag")),
        ("name".into(), "Forged draft".into()), ("location_id".into(), fixture.primary_location_id.to_string()), ("reorder_threshold".into(), "999".into())];
    let response = target.post_browser_form(&path(&fixture, &medication), &fields);
    assert_eq!(response.status().as_u16(), 422);
    assert_eq!(input(&response.text().unwrap(), "name"), "Forged draft");
    assert_eq!(read(&target, &parent_api), before);
    assert_eq!(counts(&mut db), evidence);
}

#[test]
fn mixed_option_inventory_uses_each_option_unit_without_scalar_aggregate_label() {
    let (fixture, target, csrf, medication) = setup("Mixed option stock display", "198.18.32.2");
    let first = option(&target, &fixture, &csrf, &medication, "tablet", json!("12.25"));
    let second = option(&target, &fixture, &csrf, &medication, "capsule", json!("3.5"));
    for route in [format!("/households/{}/medications", fixture.household_slug), path(&fixture, &medication)] {
        let response = target.get_html(&route);
        assert_eq!(response.status().as_u16(), 200);
        let html = response.text().unwrap();
        let document = Html::parse_document(&html);
        for (record, quantity, unit) in [(&first, "12.25", "tablet"), (&second, "3.5", "capsule")] {
            let marker = format!("[data-stock-option-id='{}']", record["id"].as_i64().unwrap());
            let node = document.select(&Selector::parse(&marker).unwrap()).next().expect("individual option inventory");
            let text = node.text().collect::<String>();
            assert!(text.contains(quantity));
            assert!(text.contains(unit));
        }
        let stock = if route.ends_with("/medications") {
            document.select(&Selector::parse("article.med-card").unwrap()).find(|node|
                node.select(&Selector::parse("h2").unwrap()).any(|heading| heading.text().collect::<String>() == medication["name"].as_str().unwrap()))
                .expect("selected medication card")
        } else {
            document.select(&Selector::parse(".med-stock").unwrap()).next().expect("selected medication stock")
        };
        let stock_text = stock.text().collect::<Vec<_>>().join(" ").split_whitespace().collect::<Vec<_>>().join(" ");
        assert!(!stock_text.contains("15.75 ml"), "mixed option sums cannot be labelled with old scalar unit");
    }
}

#[test]
fn all_untracked_options_preserve_and_explain_supported_scalar_fallback() {
    let (fixture, target, csrf, medication) = setup("Untracked option scalar fallback", "198.18.32.3");
    let untracked = option(&target, &fixture, &csrf, &medication, "tablet", Value::Null);
    let parent = read(&target, &api(&fixture, &medication));
    assert_eq!(parent["current_supply"], "20.0");
    assert_eq!(untracked["current_supply"], Value::Null);
    let response = target.get_html(&format!("{}/stock", path(&fixture, &medication)));
    assert_eq!(response.status().as_u16(), 200);
    let document = Html::parse_document(&response.text().unwrap());
    assert!(document.select(&Selector::parse("[data-stock-scalar-fallback]").unwrap()).next().is_some(), "explain parent fallback without changing API stock");
    assert!(document.select(&Selector::parse("a[href$='/stock/adjust']").unwrap()).next().is_none());
}

#[test]
fn tracked_option_removal_replays_exact_payload_and_retains_rejected_draft() {
    let (fixture, target, csrf, medication) = setup("Option stock removal replay", "198.18.32.4");
    let tracked = option(&target, &fixture, &csrf, &medication, "tablet", json!("12.25"));
    let removal = format!("{}/stock/remove", path(&fixture, &medication));
    let response = target.get_html(&removal);
    assert_eq!(response.status().as_u16(), 200);
    let html = response.text().unwrap();
    let mut fields = vec![("authenticity_token".into(), input(&html, "authenticity_token")), ("submission_id".into(), input(&html, "submission_id")),
        ("quantity".into(), "99".into()), ("reason".into(), "dropped".into()), ("note".into(), "Broken after delivery".into()),
        ("dosage_id".into(), tracked["id"].as_i64().unwrap().to_string())];
    let (_, other_target, other_csrf, other_parent) = setup("Other removal stock", "198.18.32.6");
    let other_option = option(&other_target, &fixture, &other_csrf, &other_parent, "tablet", json!("7.5"));
    let other_api = format!("/api/v1/households/{}/dosage_options/{}", fixture.household_id, other_option["id"].as_i64().unwrap());
    let parent_before = read(&target, &api(&fixture, &medication));
    let option_api = format!("/api/v1/households/{}/dosage_options/{}", fixture.household_id, tracked["id"].as_i64().unwrap());
    let option_before = read(&target, &option_api);
    let other_before = read(&target, &other_api);
    let id = medication["id"].as_i64().unwrap();
    let mut db = Client::connect(&env::var("CONTRACT_AUDIT_DATABASE_URL").unwrap(), NoTls).unwrap();
    let counts = |db: &mut Client| -> (i64, i64) {
        (db.query_one("SELECT count(*) FROM versions WHERE item_type = 'Medication' AND item_id = $1", &[&id]).unwrap().get(0),
         db.query_one("SELECT count(*) FROM api_change_events WHERE record_type = 'Medication' AND record_id = $1", &[&id]).unwrap().get(0))
    };
    let evidence = counts(&mut db);
    let original_source = fields[5].1.clone();
    fields[2].1 = "1.25".into();
    for forged in [other_option["id"].as_i64().unwrap(), fixture.foreign_dosage_id] {
        fields[5].1 = forged.to_string();
        let response = target.post_browser_form(&removal, &fields);
        assert_eq!(response.status().as_u16(), 422);
        let rejected = response.text().unwrap();
        assert_eq!(input(&rejected, "quantity"), "1.25");
        assert_eq!(input(&rejected, "submission_id"), fields[1].1);
        assert_eq!(read(&target, &api(&fixture, &medication)), parent_before);
        assert_eq!(read(&target, &option_api), option_before);
        assert_eq!(read(&target, &other_api), other_before);
        assert_eq!(counts(&mut db), evidence);
        assert!(read(&target, &format!("{}/stock_removals?per_page=100", api(&fixture, &medication))).as_array().unwrap().is_empty());
    }
    fields[5].1 = original_source;
    fields[2].1 = "99".into();
    let response = target.post_browser_form(&removal, &fields);
    assert_eq!(response.status().as_u16(), 422);
    let rejected = response.text().unwrap();
    assert_eq!(input(&rejected, "quantity"), "99");
    assert_eq!(input(&rejected, "submission_id"), fields[1].1);
    assert_eq!(read(&target, &api(&fixture, &medication))["current_supply"], "12.25");
    fields[2].1 = "1.25".into();
    assert_eq!(target.post_browser_form(&removal, &fields).status().as_u16(), 303);
    assert_eq!(target.post_browser_form(&removal, &fields).status().as_u16(), 303);
    let history = read(&target, &format!("{}/stock_removals?per_page=100", api(&fixture, &medication)));
    assert_eq!(history.as_array().unwrap().len(), 1);
    assert_eq!(history[0]["submission_id"], fields[1].1);
    assert_eq!(history[0]["quantity"], "1.25");
    assert_eq!(read(&target, &api(&fixture, &medication))["current_supply"], "11.0");
    assert_eq!(read(&target, &option_api)["current_supply"], "11.0");
    fields[2].1 = "2.5".into();
    assert_eq!(target.post_browser_form(&removal, &fields).status().as_u16(), 422);
    assert_eq!(read(&target, &api(&fixture, &medication))["current_supply"], "11.0");
    assert_eq!(read(&target, &option_api)["current_supply"], "11.0");
    assert_eq!(read(&target, &format!("{}/stock_removals?per_page=100", api(&fixture, &medication))).as_array().unwrap().len(), 1);
}

#[test]
fn null_option_uses_real_scalar_fallback_then_empty_parent_rejects_dose_without_writes() {
    let (fixture, target, csrf, medication) = setup("Null option fallback dose", "198.18.32.5");
    let untracked = option(&target, &fixture, &csrf, &medication, "ml", Value::Null);
    let base = format!("/api/v1/households/{}", fixture.household_id);
    let response = target.post_browser_json(&format!("{base}/person_medications"), &csrf,
        &json!({"person_medication": {"person_id": fixture.journey_browser_person_id.to_string(), "medication_id": medication["id"].as_i64().unwrap().to_string(),
            "source_dosage_option_id": untracked["portable_id"], "dose_amount": "1.25", "dose_unit": "ml", "administration_kind": "as_needed"}}));
    assert_eq!(response.status().as_u16(), 201);
    let assignment = response.json::<Value>().unwrap()["data"].clone();
    let detail = path(&fixture, &medication);
    let html = target.get_html(&detail).text().unwrap();
    let mut fields = vec![("authenticity_token".into(), input(&html, "authenticity_token")),
        ("client_uuid".into(), "36300000-0000-4000-8000-000000000001".into()), ("source_type".into(), "person_medication".into()),
        ("source_id".into(), assignment["portable_id"].as_str().unwrap().into()), ("dose_amount".into(), "1.25".into()),
        ("dose_unit".into(), "ml".into()), ("taken_at".into(), input(&html, "taken_at")),
        ("taken_from_medication_id".into(), medication["id"].as_i64().unwrap().to_string())];
    assert_eq!(target.post_browser_form(&format!("{detail}/doses"), &fields).status().as_u16(), 303);
    let parent_api = api(&fixture, &medication);
    assert_eq!(read(&target, &parent_api)["current_supply"], "18.75");
    let option_api = format!("{base}/dosage_options/{}", untracked["id"].as_i64().unwrap());
    assert_eq!(read(&target, &option_api)["current_supply"], Value::Null);
    assert_eq!(target.patch_json(&format!("{parent_api}/adjust_inventory"), &fixture.access_token,
        &json!({"adjustment": {"new_quantity": "0", "reason": "Public scalar fallback empty"}})).status().as_u16(), 200);
    let before = read(&target, &parent_api);
    let before_option = read(&target, &option_api);
    let takes_path = format!("{base}/medication_takes?per_page=100");
    let takes = read(&target, &takes_path);
    let selected = takes.as_array().unwrap().iter().filter(|take| take["person_medication_id"] == assignment["id"]).count();
    assert_eq!(selected, 1);
    let id = medication["id"].as_i64().unwrap();
    let mut db = Client::connect(&env::var("CONTRACT_AUDIT_DATABASE_URL").unwrap(), NoTls).unwrap();
    let counts = |db: &mut Client| -> (i64, i64) {
        (db.query_one("SELECT count(*) FROM versions WHERE item_type = 'Medication' AND item_id = $1", &[&id]).unwrap().get(0),
         db.query_one("SELECT count(*) FROM api_change_events WHERE record_type = 'Medication' AND record_id = $1", &[&id]).unwrap().get(0))
    };
    let evidence = counts(&mut db);
    fields[1].1 = "36300000-0000-4000-8000-000000000002".into();
    assert_eq!(target.post_browser_form(&format!("{detail}/doses"), &fields).status().as_u16(), 422);
    assert_eq!(read(&target, &parent_api), before);
    assert_eq!(read(&target, &option_api), before_option);
    assert_eq!(read(&target, &takes_path), takes);
    assert_eq!(counts(&mut db), evidence);
}
