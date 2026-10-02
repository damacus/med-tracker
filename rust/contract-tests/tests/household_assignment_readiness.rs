use medtracker_contract_tests::{Fixture, Target, fixture};
use postgres::{Client, NoTls};
use scraper::{Html, Selector};
use serde_json::{Value, json};
use std::env;

fn field(html: &str, name: &str) -> String {
    let document = Html::parse_document(html);
    let node = document.select(&Selector::parse(&format!("[name='{name}']")).unwrap()).next().expect("native assignment field");
    if node.value().name() == "textarea" { node.text().collect() }
    else if node.value().name() == "select" {
        node.select(&Selector::parse("option[selected]").unwrap()).next().and_then(|option| option.value().attr("value")).expect("selected value").into()
    } else { node.value().attr("value").expect("field value").into() }
}
fn form(target: &Target, path: &str) -> String {
    let response = target.get_html(path);
    let status = response.status().as_u16();
    let html = response.text().unwrap();
    assert_eq!(status, 200, "authorised assignment editor {path}");
    html
}
fn read(target: &Target, path: &str) -> Value {
    let response = target.get(path, None);
    let status = response.status().as_u16();
    let body = response.json::<Value>().unwrap();
    assert_eq!(status, 200, "source read: {body}");
    body["data"].clone()
}
fn created(response: reqwest::blocking::Response) -> Value {
    let status = response.status().as_u16();
    let body = response.json::<Value>().unwrap();
    assert_eq!(status, 201, "canonical setup: {body}");
    body["data"].clone()
}
fn setup(client: &str, name: &str) -> (Fixture, Target, String, Value) {
    let fixture = fixture();
    let target = Target::from_env();
    let html = form(&target, "/login");
    assert_eq!(target.post_html_form_from_client("/login", client, &[
        ("authenticity_token".into(), field(&html, "authenticity_token")),
        ("email".into(), fixture.primary_email.clone()), ("password".into(), "password".into()),
    ]).status().as_u16(), 302);
    let html = form(&target, &format!("/households/{}/medications", fixture.household_slug));
    let document = Html::parse_document(&html);
    let csrf = document.select(&Selector::parse("meta[name='csrf-token']").unwrap()).next().unwrap().value().attr("content").unwrap().to_owned();
    let medication = created(target.post_browser_json(&format!("/api/v1/households/{}/medications", fixture.household_id), &csrf,
        &json!({"medication": {"name": name, "location_id": fixture.primary_location_id, "dose_amount": "1.25", "dose_unit": "ml", "current_supply": "200", "reorder_threshold": "3"}})));
    (fixture, target, csrf, medication)
}
fn option(target: &Target, fixture: &Fixture, csrf: &str, medication: &Value, amount: &str) -> Value {
    created(target.post_browser_json(&format!("/api/v1/households/{}/dosage_options", fixture.household_id), csrf,
        &json!({"dosage_option": {"medication_id": medication["id"].as_i64().unwrap().to_string(), "amount": amount, "unit": "ml",
            "frequency": "daily", "default_dose_cycle": "daily", "default_max_daily_doses": 4, "default_min_hours_between_doses": "0", "current_supply": null, "reorder_threshold": null}})))
}
fn fields(html: &str, medication: &Value, amount: &str, notes: &str, selected: &str) -> Vec<(String, String)> {
    vec![
        ("authenticity_token".into(), field(html, "authenticity_token")),
        ("etag".into(), field(html, "etag")), ("submission_id".into(), field(html, "submission_id")),
        ("medication_id".into(), medication["id"].as_i64().unwrap().to_string()),
        ("source_dosage_option_id".into(), selected.into()),
        ("dose_amount".into(), amount.into()), ("dose_unit".into(), "ml".into()),
        ("administration_kind".into(), "as_needed".into()), ("notes".into(), notes.into()),
    ]
}
fn link(db: &mut Client, fixture: &Fixture, source: &Value) -> Option<i64> {
    db.query_one("SELECT source_dosage_option_id FROM person_medications WHERE household_id=$1 AND id=$2", &[&fixture.household_id, &source["id"].as_i64().unwrap()]).unwrap().get(0)
}
fn counts(db: &mut Client, fixture: &Fixture, source: &Value) -> (i64, i64, i64) {
    let row = db.query_one("SELECT (SELECT count(*) FROM versions WHERE household_id=$1 AND item_type='PersonMedication' AND item_id=$2), (SELECT count(*) FROM api_change_events WHERE household_id=$1 AND record_type='PersonMedication' AND record_id=$2), (SELECT count(*) FROM medication_takes WHERE person_medication_id=$2)", &[&fixture.household_id, &source["id"].as_i64().unwrap()]).unwrap();
    (row.get(0), row.get(1), row.get(2))
}
fn assignment(fixture: &Fixture, target: &Target, csrf: &str, medication: &Value, option: Option<&Value>) -> Value {
    let mut attributes = json!({"person_id": fixture.journey_browser_person_id.to_string(), "medication_id": medication["id"].as_i64().unwrap().to_string(), "dose_amount": "1.25", "dose_unit": "ml", "administration_kind": "as_needed"});
    if let Some(option) = option { attributes["source_dosage_option_id"] = option["id"].as_i64().unwrap().to_string().into(); }
    created(target.post_browser_json(&format!("/api/v1/households/{}/person_medications", fixture.household_id), csrf, &json!({"person_medication": attributes})))
}

#[test]
fn linked_assignment_blank_keeps_link_and_explicit_replacement_updates_snapshot() {
    let (fixture, target, csrf, medication) = setup("198.18.53.1", "Readiness linked assignment");
    let first = option(&target, &fixture, &csrf, &medication, "1.25");
    let second = option(&target, &fixture, &csrf, &medication, "2.5");
    let source = assignment(&fixture, &target, &csrf, &medication, Some(&first));
    assert!(source.get("source_dosage_option_id").is_none(), "public response must not invent link identity");
    let api = format!("/api/v1/households/{}/person_medications/{}", fixture.household_id, source["id"].as_i64().unwrap());
    let member = format!("/households/{}/people/{}/assignments/{}", fixture.household_slug, fixture.journey_browser_person_id, source["id"].as_i64().unwrap());
    let mut db = Client::connect(&env::var("CONTRACT_AUDIT_DATABASE_URL").unwrap(), NoTls).unwrap();
    let html = form(&target, &format!("{member}/edit"));
    assert_eq!(field(&html, "source_dosage_option_id"), "");
    let kept = fields(&html, &medication, "1.25", "Keep original linked dose", "");
    assert_eq!(target.post_browser_form(&member, &kept).status().as_u16(), 303);
    assert_eq!(link(&mut db, &fixture, &source), first["id"].as_i64());
    let before = read(&target, &api);
    assert_eq!(before["dose_amount"], "1.25");
    let unchanged_medication = read(&target, &format!("/api/v1/households/{}/medications/{}", fixture.household_id, medication["id"].as_i64().unwrap()));
    let html = form(&target, &format!("{member}/edit"));
    let invalid = fields(&html, &medication, "2.5", "Replacement <dose> & note", "");
    let evidence = counts(&mut db, &fixture, &source);
    let response = target.post_browser_form(&member, &invalid);
    assert_eq!(response.status().as_u16(), 422, "changing a linked dose needs an explicit matching replacement");
    let rejected = response.text().unwrap();
    for name in ["etag", "dose_amount", "dose_unit", "notes", "source_dosage_option_id"] {
        assert_eq!(field(&rejected, name), invalid.iter().find(|(key, _)| key == name).unwrap().1);
    }
    assert_ne!(field(&rejected, "submission_id"), field(&html, "submission_id"), "cached API validation gets a fresh correction key");
    assert_eq!(read(&target, &api), before);
    assert_eq!(link(&mut db, &fixture, &source), first["id"].as_i64());
    assert_eq!(counts(&mut db, &fixture, &source), evidence);
    let replace = fields(&rejected, &medication, "2.5", "Replacement <dose> & note", &second["id"].as_i64().unwrap().to_string());
    assert_eq!(target.post_browser_form(&member, &replace).status().as_u16(), 303);
    assert_eq!(link(&mut db, &fixture, &source), second["id"].as_i64());
    let replaced = read(&target, &api);
    assert_eq!(replaced["dose_amount"], "2.5");
    assert_eq!(replaced["dose_unit"], "ml");
    assert_eq!(replaced["notes"], "Replacement <dose> & note");
    assert_eq!(read(&target, &format!("/api/v1/households/{}/medications/{}", fixture.household_id, medication["id"].as_i64().unwrap())), unchanged_medication);
    assert_eq!(counts(&mut db, &fixture, &source).2, evidence.2);
}

#[test]
fn unlinked_assignment_blank_keeps_manual_dose_without_inventing_a_link() {
    let (fixture, target, csrf, medication) = setup("198.18.53.2", "Readiness unlinked assignment");
    let source = assignment(&fixture, &target, &csrf, &medication, None);
    let api = format!("/api/v1/households/{}/person_medications/{}", fixture.household_id, source["id"].as_i64().unwrap());
    let member = format!("/households/{}/people/{}/assignments/{}", fixture.household_slug, fixture.journey_browser_person_id, source["id"].as_i64().unwrap());
    let mut db = Client::connect(&env::var("CONTRACT_AUDIT_DATABASE_URL").unwrap(), NoTls).unwrap();
    let before_medication = read(&target, &format!("/api/v1/households/{}/medications/{}", fixture.household_id, medication["id"].as_i64().unwrap()));
    for (amount, note) in [("1.25", "Keep manual dose"), ("2.5", "Edit manual dose")] {
        let html = form(&target, &format!("{member}/edit"));
        assert_eq!(field(&html, "source_dosage_option_id"), "");
        assert_eq!(target.post_browser_form(&member, &fields(&html, &medication, amount, note, "")).status().as_u16(), 303);
        assert_eq!(link(&mut db, &fixture, &source), None);
        let saved = read(&target, &api);
        assert_eq!(saved["dose_amount"], amount);
        assert_eq!(saved["notes"], note);
    }
    assert_eq!(read(&target, &format!("/api/v1/households/{}/medications/{}", fixture.household_id, medication["id"].as_i64().unwrap())), before_medication);
    assert_eq!(counts(&mut db, &fixture, &source).2, 0);
}
