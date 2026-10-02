use medtracker_contract_tests::{Fixture, Target, fixture};
use postgres::{Client, NoTls};
use scraper::{Html, Selector};
use serde_json::{Value, json};
use std::{collections::BTreeSet, env};

fn input(html: &str, name: &str) -> String {
    Html::parse_document(html)
        .select(&Selector::parse(&format!("input[name='{name}']")).unwrap())
        .next().and_then(|node| node.value().attr("value"))
        .expect("native stock field").to_owned()
}

fn setup(client: &str) -> (Fixture, Target, String) {
    let fixture = fixture();
    let target = Target::from_env();
    let token = input(&target.get_html("/login").text().unwrap(), "authenticity_token");
    assert_eq!(target.post_html_form_from_client("/login", client, &[
        ("email".into(), fixture.primary_email.clone()), ("password".into(), "password".into()),
        ("authenticity_token".into(), token),
    ]).status().as_u16(), 302);
    let response = target.get_html(&format!("/households/{}/medications", fixture.household_slug));
    assert_eq!(response.status().as_u16(), 200);
    let html = response.text().unwrap();
    let csrf = Html::parse_document(&html).select(&Selector::parse("meta[name='csrf-token']").unwrap())
        .next().unwrap().value().attr("content").unwrap().to_owned();
    (fixture, target, csrf)
}

fn medication(target: &Target, fixture: &Fixture, csrf: &str, name: &str, supply: Value) -> Value {
    let response = target.post_browser_json(&format!("/api/v1/households/{}/medications", fixture.household_id), csrf,
        &json!({"medication": {"name": name, "location_id": fixture.primary_location_id,
            "dose_amount": "1.25", "dose_unit": "ml", "current_supply": supply, "reorder_threshold": "3"}}));
    let status = response.status().as_u16();
    let body = response.json::<Value>().expect("medication setup JSON response");
    assert_eq!(status, 201, "Medication setup response: {body}");
    body["data"].clone()
}

fn option(target: &Target, fixture: &Fixture, csrf: &str, parent: &Value, supply: Value) -> Value {
    let response = target.post_browser_json(&format!("/api/v1/households/{}/dosage_options", fixture.household_id), csrf,
        &json!({"dosage_option": {"medication_id": parent["id"].as_i64().unwrap().to_string(),
            "amount": "1.25", "unit": "ml", "frequency": "daily", "default_dose_cycle": "daily",
            "default_max_daily_doses": 4, "default_min_hours_between_doses": "0",
            "current_supply": supply, "reorder_threshold": null}}));
    let status = response.status().as_u16();
    let body = response.json::<Value>().expect("dosage setup JSON response");
    assert_eq!(status, 201, "Dosage setup response: {body}");
    body["data"].clone()
}

fn api(fixture: &Fixture, parent: &Value) -> String {
    format!("/api/v1/households/{}/medications/{}", fixture.household_id, parent["id"].as_i64().unwrap())
}

fn web(fixture: &Fixture, parent: &Value) -> String {
    format!("/households/{}/medications/{}", fixture.household_slug, parent["id"].as_i64().unwrap())
}

fn read(target: &Target, path: &str) -> Value {
    let response = target.get(path, None);
    assert_eq!(response.status().as_u16(), 200);
    response.json::<Value>().unwrap()["data"].clone()
}

fn form(target: &Target, path: &str) -> String {
    let response = target.get_html(path);
    assert_eq!(response.status().as_u16(), 200, "Native page {path}");
    response.text().unwrap()
}

fn removal_fields(html: &str, quantity: &str) -> Vec<(String, String)> {
    vec![("authenticity_token".into(), input(html, "authenticity_token")),
        ("submission_id".into(), input(html, "submission_id")), ("dosage_id".into(), String::new()),
        ("quantity".into(), quantity.into()), ("reason".into(), "dropped".into()),
        ("note".into(), "Dropped <medicine> & bottle".into())]
}

fn counts(db: &mut Client, fixture: &Fixture, parent: &Value) -> (i64, i64, i64, i64) {
    let id = parent["id"].as_i64().unwrap();
    let household = fixture.household_id;
    (db.query_one("SELECT count(*) FROM versions WHERE household_id = $1 AND item_id = $2 AND item_type IN ('Medication', 'MedicationStockRemoval')", &[&household, &id]).unwrap().get(0),
     db.query_one("SELECT count(*) FROM api_change_events WHERE household_id = $1 AND record_id = $2 AND record_type = 'Medication'", &[&household, &id]).unwrap().get(0),
     db.query_one("SELECT count(*) FROM versions v JOIN dosages d ON d.id = v.item_id WHERE v.household_id = $1 AND v.item_type = 'MedicationDosageOption' AND d.medication_id = $2", &[&household, &id]).unwrap().get(0),
     db.query_one("SELECT count(*) FROM api_change_events e JOIN dosages d ON d.id = e.record_id WHERE e.household_id = $1 AND e.record_type = 'MedicationDosageOption' AND d.medication_id = $2", &[&household, &id]).unwrap().get(0))
}

fn retained(html: &str, fields: &[(String, String)]) {
    for name in ["submission_id", "quantity"] {
        assert_eq!(input(html, name), fields.iter().find(|(key, _)| key == name).unwrap().1);
    }
    let document = Html::parse_document(html);
    assert_eq!(document.select(&Selector::parse("textarea[name='note']").unwrap()).next().unwrap().text().collect::<String>(), fields[5].1);
    assert_eq!(document.select(&Selector::parse("select[name='reason'] option[selected]").unwrap()).next().unwrap().value().attr("value"), Some("dropped"));
}

#[test]
fn finite_parent_stock_with_only_null_options_removes_replays_and_rejects_without_changes() {
    let (fixture, target, csrf) = setup("198.18.51.1");
    let parent = medication(&target, &fixture, &csrf, "Completion fallback stock", json!("20"));
    let untracked = option(&target, &fixture, &csrf, &parent, Value::Null);
    let path = format!("{}/stock/remove", web(&fixture, &parent));
    let html = form(&target, &path);
    let text = Html::parse_document(&html).select(&Selector::parse(".med-stock").unwrap()).next().unwrap().text().collect::<Vec<_>>().join(" ");
    assert!(text.split_whitespace().collect::<Vec<_>>().join(" ").contains("20.0 ml"));
    let mut fields = removal_fields(&html, "2");
    let option_api = format!("/api/v1/households/{}/dosage_options/{}", fixture.household_id, untracked["id"].as_i64().unwrap());
    let before_option = read(&target, &option_api);
    assert_eq!(target.post_browser_form(&path, &fields).status().as_u16(), 303, "Finite parent fallback must be removable with a blank dosage_id");
    let saved = read(&target, &api(&fixture, &parent));
    assert_eq!(saved["current_supply"], "18.0");
    assert_eq!(read(&target, &option_api), before_option);
    let history_path = format!("{}/stock_removals?per_page=100", api(&fixture, &parent));
    let history = read(&target, &history_path);
    assert_eq!(history.as_array().unwrap().len(), 1);
    assert_eq!(history[0]["quantity"], "2");
    assert_eq!(history[0]["previous_quantity"], "20");
    assert_eq!(history[0]["remaining_quantity"], "18");
    assert_eq!(history[0]["dosage_id"], Value::Null);
    assert_eq!(history[0]["unit"], "ml");
    assert_eq!(history[0]["reason"], "dropped");
    assert_eq!(history[0]["note"], fields[5].1);
    assert_eq!(history[0]["submission_id"], fields[1].1);
    let mut db = Client::connect(&env::var("CONTRACT_AUDIT_DATABASE_URL").unwrap(), NoTls).unwrap();
    let evidence = counts(&mut db, &fixture, &parent);
    assert_eq!(target.post_browser_form(&path, &fields).status().as_u16(), 303);
    assert_eq!(read(&target, &api(&fixture, &parent)), saved);
    assert_eq!(read(&target, &history_path), history);
    assert_eq!(counts(&mut db, &fixture, &parent), evidence);
    fields[3].1 = "3".into();
    let conflict = target.post_browser_form(&path, &fields);
    assert_eq!(conflict.status().as_u16(), 422);
    retained(&conflict.text().unwrap(), &fields);
    assert_eq!(read(&target, &api(&fixture, &parent)), saved);
    assert_eq!(read(&target, &history_path), history);
    assert_eq!(counts(&mut db, &fixture, &parent), evidence);
    let html = form(&target, &path);
    let fields = removal_fields(&html, "99");
    let insufficient = target.post_browser_form(&path, &fields);
    assert_eq!(insufficient.status().as_u16(), 422);
    retained(&insufficient.text().unwrap(), &fields);
    assert_eq!(read(&target, &api(&fixture, &parent)), saved);
    assert_eq!(read(&target, &option_api), before_option);
    assert_eq!(read(&target, &history_path), history);
    assert_eq!(counts(&mut db, &fixture, &parent), evidence);
}

#[test]
fn null_parent_and_null_options_cannot_remove_stock_or_write_history() {
    let (fixture, target, csrf) = setup("198.18.51.2");
    let parent = medication(&target, &fixture, &csrf, "Completion untracked parent", Value::Null);
    let untracked = option(&target, &fixture, &csrf, &parent, Value::Null);
    let path = format!("{}/stock/remove", web(&fixture, &parent));
    let fields = removal_fields(&form(&target, &path), "2");
    let before = read(&target, &api(&fixture, &parent));
    let option_api = format!("/api/v1/households/{}/dosage_options/{}", fixture.household_id, untracked["id"].as_i64().unwrap());
    let before_option = read(&target, &option_api);
    let mut db = Client::connect(&env::var("CONTRACT_AUDIT_DATABASE_URL").unwrap(), NoTls).unwrap();
    let evidence = counts(&mut db, &fixture, &parent);
    let response = target.post_browser_form(&path, &fields);
    assert_eq!(response.status().as_u16(), 422);
    retained(&response.text().unwrap(), &fields);
    assert_eq!(read(&target, &api(&fixture, &parent)), before);
    assert_eq!(read(&target, &option_api), before_option);
    assert!(read(&target, &format!("{}/stock_removals?per_page=100", api(&fixture, &parent))).as_array().unwrap().is_empty());
    assert_eq!(counts(&mut db, &fixture, &parent), evidence);
}

#[test]
fn tracked_zero_and_mixed_options_require_option_stock_even_with_finite_parent() {
    let (fixture, target, csrf) = setup("198.18.51.3");
    let parent = medication(&target, &fixture, &csrf, "Completion tracked zero", json!("20"));
    let tracked = option(&target, &fixture, &csrf, &parent, json!("0"));
    let parent_api = api(&fixture, &parent);
    let path = format!("{}/stock/remove", web(&fixture, &parent));
    let mut db = Client::connect(&env::var("CONTRACT_AUDIT_DATABASE_URL").unwrap(), NoTls).unwrap();
    for mixed in [false, true] {
        if mixed { option(&target, &fixture, &csrf, &parent, Value::Null); }
        assert_eq!(target.patch_json(&format!("{parent_api}/adjust_inventory"), &fixture.access_token,
            &json!({"adjustment": {"new_quantity": "20", "reason": "Disposable public legacy parent stimulus"}})).status().as_u16(), 200);
        let html = form(&target, &path);
        let document = Html::parse_document(&html);
        let choices = document.select(&Selector::parse("select[name='dosage_id'] option").unwrap())
            .filter_map(|node| node.value().attr("value")).filter(|value| !value.is_empty()).collect::<Vec<_>>();
        assert_eq!(choices, vec![tracked["id"].as_i64().unwrap().to_string()]);
        let before = read(&target, &parent_api);
        assert_eq!(before["current_supply"], "20.0");
        let option_api = format!("/api/v1/households/{}/dosage_options/{}", fixture.household_id, tracked["id"].as_i64().unwrap());
        let before_option = read(&target, &option_api);
        let evidence = counts(&mut db, &fixture, &parent);
        let fields = removal_fields(&html, "2");
        let response = target.post_browser_form(&path, &fields);
        assert_eq!(response.status().as_u16(), 422);
        retained(&response.text().unwrap(), &fields);
        assert_eq!(read(&target, &parent_api), before);
        assert_eq!(read(&target, &option_api), before_option);
        assert_eq!(counts(&mut db, &fixture, &parent), evidence);
        assert!(read(&target, &format!("{parent_api}/stock_removals?per_page=100")).as_array().unwrap().is_empty());
    }
}

#[test]
fn visible_option_collections_above_five_hundred_keep_every_medication_page_complete() {
    let (fixture, target, csrf) = setup("198.18.51.4");
    let first = medication(&target, &fixture, &csrf, "Completion options first", json!("20"));
    let second = medication(&target, &fixture, &csrf, "Completion options last", json!("20"));
    let scalar = medication(&target, &fixture, &csrf, "Completion scalar beside options", json!("20"));
    let first_seed = option(&target, &fixture, &csrf, &first, Value::Null);
    let second_seed = option(&target, &fixture, &csrf, &second, Value::Null);
    let mut db = Client::connect(&env::var("CONTRACT_AUDIT_DATABASE_URL").unwrap(), NoTls).unwrap();
    for (seed, extra) in [(&first_seed, 199_i32), (&second_seed, 300_i32)] {
        let seed_id = seed["id"].as_i64().unwrap();
        let inserted = db.execute("INSERT INTO dosages (household_id, medication_id, amount, unit, frequency, description, current_supply, reorder_threshold, default_dose_cycle, default_max_daily_doses, default_min_hours_between_doses, created_at, updated_at) SELECT d.household_id, d.medication_id, n::numeric + 2, d.unit, d.frequency, 'Disposable option ' || n::text, NULL, NULL, d.default_dose_cycle, d.default_max_daily_doses, d.default_min_hours_between_doses, now(), now() FROM dosages d CROSS JOIN generate_series(1, $2::integer) n WHERE d.id = $1", &[&seed_id, &extra]).unwrap();
        assert_eq!(inserted, extra as u64);
    }
    let mut visible = Vec::new();
    let mut expected_total = None;
    for page in 1..=20 {
        let response = target.get(&format!("/api/v1/households/{}/dosage_options?page={page}&per_page=100", fixture.household_id), None);
        assert_eq!(response.status().as_u16(), 200);
        let body = response.json::<Value>().unwrap();
        let total = body["meta"]["total_count"].as_u64().unwrap();
        if let Some(expected) = expected_total { assert_eq!(total, expected); } else { expected_total = Some(total); }
        let rows = body["data"].as_array().unwrap();
        visible.extend(rows.iter().cloned());
        if visible.len() as u64 == total { break; }
        assert!(!rows.is_empty(), "Authoritative option pages must not end early");
    }
    assert!(expected_total.unwrap() >= 501);
    assert_eq!(visible.len() as u64, expected_total.unwrap());
    let ids = visible.iter().map(|row| row["id"].as_i64().unwrap()).collect::<BTreeSet<_>>();
    assert_eq!(ids.len(), visible.len());
    assert!(!ids.contains(&fixture.foreign_dosage_id));
    for (parent, expected) in [(&first, 200), (&second, 301), (&scalar, 0)] {
        let selected = visible.iter().filter(|row| row["medication_id"] == parent["id"]).collect::<Vec<_>>();
        assert_eq!(selected.len(), expected);
        let detail = web(&fixture, parent);
        for suffix in ["", "/edit", "/stock", "/stock/remove", "/dosage_options/new"] {
            form(&target, &format!("{detail}{suffix}"));
        }
        let html = form(&target, &format!("{detail}/dosage_options"));
        let document = Html::parse_document(&html);
        assert_eq!(document.select(&Selector::parse(".household-list .household-card").unwrap()).count(), expected);
        for row in selected {
            let path = format!("{detail}/dosage_options/{}/edit", row["id"].as_i64().unwrap());
            assert!(document.select(&Selector::parse(&format!("a[href='{path}']")).unwrap()).next().is_some());
        }
    }
    form(&target, &format!("/households/{}/medications", fixture.household_slug));
    form(&target, &format!("{}/stock/adjust", web(&fixture, &scalar)));
    let last = visible.iter().rev().find(|row| row["medication_id"] == second["id"]).unwrap();
    form(&target, &format!("{}/dosage_options/{}/edit", web(&fixture, &second), last["id"].as_i64().unwrap()));
    let assignment = form(&target, &format!("/households/{}/people/{}/assignments/new", fixture.household_slug, fixture.journey_browser_person_id));
    let document = Html::parse_document(&assignment);
    let choices = document.select(&Selector::parse("select[name='source_dosage_option_id'] option").unwrap())
        .filter_map(|node| node.value().attr("value")).filter(|value| !value.is_empty())
        .map(|value| value.parse::<i64>().unwrap()).collect::<BTreeSet<_>>();
    assert_eq!(choices, ids, "Assignment choices must contain every authorised option, including the final page");
}
