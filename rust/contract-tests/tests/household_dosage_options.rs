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
        .expect("native dosage input")
        .to_owned()
}

fn login(target: &Target, fixture: &Fixture, client: &str) {
    let response = target.get_html("/login");
    assert_eq!(response.status().as_u16(), 200);
    let token = input(&response.text().unwrap(), "authenticity_token");
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

fn setup(name: &str) -> (Fixture, authentication::CurrentBearer, Target, i64) {
    let suffix = name
        .bytes()
        .fold(0_u16, |sum, byte| sum.wrapping_add(u16::from(byte)));
    let client = format!("198.18.{}.{}", suffix / 250, suffix % 250 + 1);
    let (fixture, guard) = authentication::fixture_with_current_bearer(&client);
    let target = Target::from_env();
    login(&target, &fixture, &client);
    let response = target.post_json_authorized(
        &format!("/api/v1/households/{}/medications", fixture.household_id),
        &fixture.access_token,
        &json!({"medication": {"name": name, "location_id": fixture.primary_location_id,
            "dose_amount": "2.5", "dose_unit": "ml", "current_supply": "20", "reorder_threshold": "3"}}),
    );
    assert_eq!(response.status().as_u16(), 201);
    let id = response.json::<Value>().unwrap()["data"]["id"]
        .as_i64()
        .unwrap();
    (fixture, guard, target, id)
}

fn web(fixture: &Fixture, id: i64) -> String {
    format!(
        "/households/{}/medications/{id}/dosage_options",
        fixture.household_slug
    )
}

fn fields(html: &str) -> Vec<(String, String)> {
    vec![
        (
            "authenticity_token".into(),
            input(html, "authenticity_token"),
        ),
        ("etag".into(), input(html, "etag")),
        ("amount".into(), "1.25".into()),
        ("unit".into(), "custom unit".into()),
        ("frequency".into(), "Once daily".into()),
        ("description".into(), "Draft notes".into()),
        ("default_max_daily_doses".into(), "4".into()),
        ("default_min_hours_between_doses".into(), "0.5".into()),
        ("default_dose_cycle".into(), "weekly".into()),
        ("current_supply".into(), "0".into()),
        ("reorder_threshold".into(), "".into()),
        ("default_for_adults".into(), "true".into()),
        ("confirm_option_mode".into(), "true".into()),
    ]
}

fn read(target: &Target, fixture: &Fixture, path: &str) -> Value {
    let response = target.get(path, Some(&fixture.access_token));
    assert_eq!(response.status().as_u16(), 200);
    response.json::<Value>().unwrap()["data"].clone()
}

fn write_evidence(parent: i64, option: i64) -> (i64, i64) {
    let mut db = Client::connect(
        &env::var("CONTRACT_AUDIT_DATABASE_URL").expect("disposable database"),
        NoTls,
    )
    .unwrap();
    let versions: i64 = db.query_one("SELECT count(*) FROM versions WHERE (item_type = 'Medication' AND item_id = $1) OR (item_type = 'MedicationDosageOption' AND item_id = $2)", &[&parent, &option]).unwrap().get(0);
    let events: i64 = db.query_one("SELECT count(*) FROM api_change_events WHERE (record_type = 'Medication' AND record_id = $1) OR (record_type = 'MedicationDosageOption' AND record_id = $2)", &[&parent, &option]).unwrap().get(0);
    (versions, events)
}

fn assert_create_evidence(parent: i64, option: i64) {
    let mut db = Client::connect(
        &env::var("CONTRACT_AUDIT_DATABASE_URL").expect("disposable database"),
        NoTls,
    )
    .unwrap();
    let request: String = db.query_one("SELECT request_id FROM versions WHERE item_type = 'MedicationDosageOption' AND item_id = $1 AND event = 'api_create'", &[&option]).unwrap().get(0);
    let parent_versions: i64 = db.query_one("SELECT count(*) FROM versions WHERE item_type = 'Medication' AND item_id = $1 AND request_id = $2", &[&parent, &request]).unwrap().get(0);
    let events: i64 = db.query_one("SELECT count(*) FROM api_change_events WHERE request_id = $1 AND ((record_type = 'Medication' AND record_id = $2) OR (record_type = 'MedicationDosageOption' AND record_id = $3))", &[&request, &parent, &option]).unwrap().get(0);
    assert_eq!(parent_versions, 1);
    assert_eq!(events, 2);
}

fn assert_draft(html: &str, draft: &[(String, String)]) {
    let document = Html::parse_document(html);
    for (name, value) in draft {
        if matches!(name.as_str(), "authenticity_token" | "confirm_option_mode") {
            continue;
        }
        let field = document
            .select(&Selector::parse(&format!("[name='{name}']")).unwrap())
            .next()
            .expect("retained native field");
        match name.as_str() {
            "description" => assert_eq!(field.text().collect::<String>(), *value),
            "default_dose_cycle" => {
                let selected = field
                    .select(&Selector::parse("option[selected]").unwrap())
                    .next()
                    .unwrap();
                assert_eq!(selected.value().attr("value"), Some(value.as_str()));
            }
            "default_for_adults" | "default_for_children" => {
                assert!(field.value().attr("checked").is_some())
            }
            _ => assert_eq!(
                field.value().attr("value"),
                Some(value.as_str()),
                "retained dosage field {name}"
            ),
        }
    }
}

fn create_option(target: &Target, fixture: &Fixture, id: i64) -> (String, Value) {
    let path = web(fixture, id);
    let response = target.get_html(&format!("{path}/new"));
    assert_eq!(response.status().as_u16(), 200, "dosage add form exists");
    let response = target.post_browser_form(&path, &fields(&response.text().unwrap()));
    assert_eq!(response.status().as_u16(), 303);
    let options = read(
        target,
        fixture,
        &format!("/api/v1/households/{}/dosage_options", fixture.household_id),
    );
    let option = options
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["medication_id"] == id)
        .unwrap()
        .clone();
    (format!("{path}/{}", option["id"].as_i64().unwrap()), option)
}

#[test]
fn browser_dosage_create_edit_preserves_exact_custom_units_and_nullable_stock() {
    let (fixture, _guard, target, id) = setup("Browser dosage precision");
    let (path, created) = create_option(&target, &fixture, id);
    assert_create_evidence(id, created["id"].as_i64().unwrap());
    assert_eq!(created["amount"], "1.25");
    assert_eq!(created["unit"], "custom unit");
    assert_eq!(created["current_supply"], "0.0");
    assert!(created["reorder_threshold"].is_null());
    assert_eq!(created["default_dose_cycle"], "weekly");
    let parent = read(
        &target,
        &fixture,
        &format!(
            "/api/v1/households/{}/medications/{id}",
            fixture.household_id
        ),
    );
    assert!(parent["dose_amount"].is_null());
    assert_eq!(parent["current_supply"], "0.0");
    let response = target.get_html(&format!("{path}/edit"));
    assert_eq!(response.status().as_u16(), 200);
    let mut draft = fields(&response.text().unwrap());
    for (name, value) in &mut draft {
        if name == "amount" {
            *value = "2.50".into();
        }
        if name == "current_supply" {
            *value = "".into();
        }
        if name == "reorder_threshold" {
            *value = "0".into();
        }
    }
    assert_eq!(
        target.post_browser_form(&path, &draft).status().as_u16(),
        303
    );
    let saved = read(
        &target,
        &fixture,
        &format!(
            "/api/v1/households/{}/dosage_options/{}",
            fixture.household_id, created["id"]
        ),
    );
    assert_eq!(saved["amount"], "2.5");
    assert!(saved["current_supply"].is_null());
    assert_eq!(saved["reorder_threshold"], "0.0");
    let evidence = write_evidence(id, created["id"].as_i64().unwrap());
    assert_eq!(
        target.post_browser_form(&path, &draft).status().as_u16(),
        409
    );
    assert_eq!(
        write_evidence(id, created["id"].as_i64().unwrap()),
        evidence
    );
    assert_eq!(
        read(
            &target,
            &fixture,
            &format!(
                "/api/v1/households/{}/dosage_options/{}",
                fixture.household_id, created["id"]
            )
        ),
        saved
    );
    assert!(
        target
            .get_html(&web(&fixture, id))
            .text()
            .unwrap()
            .contains("custom unit")
    );
}

#[test]
fn browser_dosage_missing_and_stale_tokens_preserve_draft_without_writing() {
    let (fixture, _guard, target, id) = setup("Browser dosage stale");
    let (path, created) = create_option(&target, &fixture, id);
    let api = format!(
        "/api/v1/households/{}/dosage_options/{}",
        fixture.household_id, created["id"]
    );
    let html = target.get_html(&format!("{path}/edit")).text().unwrap();
    let original = fields(&html);
    let parent_path = format!(
        "/api/v1/households/{}/medications/{id}",
        fixture.household_id
    );
    let parent = read(&target, &fixture, &parent_path);
    let evidence = write_evidence(id, created["id"].as_i64().unwrap());
    for token in [None, Some(""), Some("   "), Some("stale-original-version")] {
        let mut draft = original.clone();
        draft.retain(|(name, _)| name != "etag");
        if let Some(token) = token {
            draft.push(("etag".into(), token.into()));
        }
        let response = target.post_browser_form(&path, &draft);
        assert_eq!(
            response.status().as_u16(),
            if token.is_some_and(|value| !value.trim().is_empty()) {
                409
            } else {
                428
            }
        );
        let retained = response.text().unwrap();
        assert_draft(&retained, &draft);
        assert_eq!(input(&retained, "amount"), "1.25");
        assert_eq!(input(&retained, "unit"), "custom unit");
        assert_eq!(input(&retained, "etag"), token.unwrap_or(""));
        assert_eq!(read(&target, &fixture, &api), created);
        assert_eq!(read(&target, &fixture, &parent_path), parent);
        assert_eq!(
            write_evidence(id, created["id"].as_i64().unwrap()),
            evidence
        );
    }
    let changed = target.patch_json(
        &api,
        &fixture.access_token,
        &json!({"dosage_option": {"amount": "2.75"}}),
    );
    assert_eq!(changed.status().as_u16(), 200);
    let changed = changed.json::<Value>().unwrap()["data"].clone();
    let parent = read(&target, &fixture, &parent_path);
    let evidence = write_evidence(id, created["id"].as_i64().unwrap());
    let response = target.post_browser_form(&path, &original);
    assert_eq!(response.status().as_u16(), 409);
    let retained = response.text().unwrap();
    assert_draft(&retained, &original);
    assert_eq!(input(&retained, "etag"), input(&html, "etag"));
    assert_eq!(input(&retained, "amount"), "1.25");
    assert_eq!(read(&target, &fixture, &api), changed);
    assert_eq!(read(&target, &fixture, &parent_path), parent);
    assert_eq!(
        write_evidence(id, created["id"].as_i64().unwrap()),
        evidence
    );
}

#[test]
fn browser_dosage_rejects_invalid_precision_wrong_parent_and_csrf() {
    let (fixture, _guard, target, id) = setup("Browser dosage rejects");
    let (path, created) = create_option(&target, &fixture, id);
    let api = format!(
        "/api/v1/households/{}/dosage_options/{}",
        fixture.household_id, created["id"]
    );
    let parent_path = format!(
        "/api/v1/households/{}/medications/{id}",
        fixture.household_id
    );
    let parent = read(&target, &fixture, &parent_path);
    let evidence = write_evidence(id, created["id"].as_i64().unwrap());
    let html = target.get_html(&format!("{path}/edit")).text().unwrap();
    let mut draft = fields(&html);
    draft
        .iter_mut()
        .find(|(name, _)| name == "amount")
        .unwrap()
        .1 = "1.234".into();
    draft
        .iter_mut()
        .find(|(name, _)| name == "default_min_hours_between_doses")
        .unwrap()
        .1 = "0.55".into();
    let response = target.post_browser_form(&path, &draft);
    assert_eq!(response.status().as_u16(), 422);
    assert_draft(&response.text().unwrap(), &draft);
    assert_eq!(read(&target, &fixture, &api), created);
    draft
        .iter_mut()
        .find(|(name, _)| name == "authenticity_token")
        .unwrap()
        .1 = "invalid".into();
    assert_eq!(
        target.post_browser_form(&path, &draft).status().as_u16(),
        403
    );
    let wrong = format!(
        "{}/{}",
        web(&fixture, fixture.managed_medication_id),
        created["id"]
    );
    assert_eq!(
        target.get_html(&format!("{wrong}/edit")).status().as_u16(),
        404
    );
    let draft = fields(&html);
    assert_eq!(
        target.post_browser_form(&wrong, &draft).status().as_u16(),
        404
    );
    let foreign = format!(
        "{}/{}",
        web(&fixture, fixture.foreign_medication_id),
        created["id"]
    );
    assert_eq!(
        target.post_browser_form(&foreign, &draft).status().as_u16(),
        404
    );
    assert_eq!(read(&target, &fixture, &api), created);
    assert_eq!(read(&target, &fixture, &parent_path), parent);
    assert_eq!(
        write_evidence(id, created["id"].as_i64().unwrap()),
        evidence
    );
}

#[test]
fn first_option_requires_transition_confirmation_and_duplicate_defaults_retain_draft() {
    let (fixture, _guard, target, id) = setup("Browser dosage defaults");
    let path = web(&fixture, id);
    let parent = format!(
        "/api/v1/households/{}/medications/{id}",
        fixture.household_id
    );
    let before = read(&target, &fixture, &parent);
    let form = target.get_html(&format!("{path}/new"));
    assert_eq!(form.status().as_u16(), 200);
    let mut draft = fields(&form.text().unwrap());
    draft.retain(|(name, _)| name != "confirm_option_mode");
    let rejected = target.post_browser_form(&path, &draft);
    assert_eq!(rejected.status().as_u16(), 422);
    assert_draft(&rejected.text().unwrap(), &draft);
    assert_eq!(read(&target, &fixture, &parent), before);
    let (_, created) = create_option(&target, &fixture, id);
    let before = read(&target, &fixture, &parent);
    let form = target.get_html(&format!("{path}/new"));
    let draft = fields(&form.text().unwrap());
    let rejected = target.post_browser_form(&path, &draft);
    assert_eq!(rejected.status().as_u16(), 422);
    assert_draft(&rejected.text().unwrap(), &draft);
    assert_eq!(read(&target, &fixture, &parent), before);
    assert_eq!(
        read(
            &target,
            &fixture,
            &format!(
                "/api/v1/households/{}/dosage_options/{}",
                fixture.household_id, created["id"]
            )
        ),
        created
    );
    let mut children = draft;
    children.retain(|(name, _)| name != "default_for_adults");
    children.push(("default_for_children".into(), "true".into()));
    assert_eq!(
        target.post_browser_form(&path, &children).status().as_u16(),
        303
    );
    let before = read(&target, &fixture, &parent);
    let rejected = target.post_browser_form(&path, &children);
    assert_eq!(rejected.status().as_u16(), 422);
    assert_draft(&rejected.text().unwrap(), &children);
    assert_eq!(read(&target, &fixture, &parent), before);
}
