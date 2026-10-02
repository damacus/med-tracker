use medtracker_contract_tests::{Fixture, Target, fixture};
use postgres::{Client, NoTls};
use scraper::{Html, Selector};
use serde_json::{Value, json};
use std::{collections::BTreeSet, env};

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
fn assignment(fixture: &Fixture, target: &Target, csrf: &str, medication: &Value, option: Option<&Value>) -> Value {
    let mut attributes = json!({"person_id": fixture.journey_browser_person_id.to_string(), "medication_id": medication["id"].as_i64().unwrap().to_string(), "dose_amount": "1.25", "dose_unit": "ml", "administration_kind": "as_needed"});
    if let Some(option) = option { attributes["source_dosage_option_id"] = option["id"].as_i64().unwrap().to_string().into(); }
    created(target.post_browser_json(&format!("/api/v1/households/{}/person_medications", fixture.household_id), csrf, &json!({"person_medication": attributes})))
}



fn collection(target: &Target, base: &str) -> Vec<Value> {
    let mut all = Vec::new();
    let mut expected = None;
    for page in 1..=20 {
        let response = target.get(&format!("{base}?page={page}&per_page=100"), None);
        let status = response.status().as_u16();
        let body = response.json::<Value>().unwrap();
        assert_eq!(status, 200, "authorised collection page: {body}");
        assert_eq!(body["meta"]["page"].as_u64(), Some(page));
        assert_eq!(body["meta"]["per_page"], 100);
        let total = body["meta"]["total_count"].as_u64().unwrap();
        if let Some(previous) = expected { assert_eq!(total, previous); } else { expected = Some(total); }
        let rows = body["data"].as_array().unwrap();
        all.extend(rows.iter().cloned());
        if all.len() as u64 == total { break; }
        assert!(!rows.is_empty(), "fixture pages must not end before the authoritative total");
    }
    assert_eq!(all.len() as u64, expected.unwrap());
    assert_eq!(all.iter().map(|row| row["id"].as_i64().unwrap()).collect::<BTreeSet<_>>().len(), all.len());
    all
}

#[test]
fn person_detail_and_treatment_forms_include_complete_large_medication_assignment_and_schedule_collections() {
    let (fixture, target, csrf, medication) = setup("198.18.54.1", "Large treatment collection seed");
    let assignment = assignment(&fixture, &target, &csrf, &medication, None);
    let api = format!("/api/v1/households/{}", fixture.household_id);
    let schedule = created(target.post_browser_json(&format!("{api}/schedules"), &csrf,
        &json!({"schedule": {"person_id": fixture.journey_browser_person_id.to_string(),
            "medication_id": medication["id"].as_i64().unwrap().to_string(), "dose_amount": "1.25", "dose_unit": "ml",
            "schedule_type": "daily", "schedule_config": {"times": ["08:00"]},
            "start_date": "2030-03-30", "end_date": "2030-04-10"}})));
    let mut db = Client::connect(&env::var("CONTRACT_AUDIT_DATABASE_URL").unwrap(), NoTls).unwrap();
    let added = db.query("INSERT INTO medications (household_id, location_id, name, dose_amount, dose_unit, current_supply, reorder_threshold, default_schedule_type, default_schedule_config, created_by_membership_id, created_at, updated_at) SELECT m.household_id, m.location_id, 'Large treatment collection ' || n::text, m.dose_amount, m.dose_unit, m.current_supply, m.reorder_threshold, m.default_schedule_type, m.default_schedule_config, m.created_by_membership_id, now(), now() FROM medications m CROSS JOIN generate_series(1, 500) n WHERE m.id=$1 RETURNING id", &[&medication["id"].as_i64().unwrap()]).unwrap();
    assert_eq!(added.len(), 500);
    let added_ids = added.iter().map(|row| row.get::<_, i64>(0)).collect::<Vec<_>>();
    assert_eq!(db.execute("INSERT INTO person_medications (household_id, person_id, medication_id, dose_amount, dose_unit, dose_cycle, max_daily_doses, min_hours_between_doses, administration_kind, active, notes, position, created_at, updated_at) SELECT p.household_id, p.person_id, added.medication_id, p.dose_amount, p.dose_unit, p.dose_cycle, p.max_daily_doses, p.min_hours_between_doses, p.administration_kind, p.active, p.notes, p.position + added.n::integer, now(), now() FROM person_medications p CROSS JOIN unnest($2::bigint[]) WITH ORDINALITY AS added(medication_id,n) WHERE p.id=$1", &[&assignment["id"].as_i64().unwrap(), &added_ids]).unwrap(), 500);
    assert_eq!(db.execute("INSERT INTO schedules (household_id, person_id, medication_id, dose_amount, dose_unit, dose_cycle, max_daily_doses, min_hours_between_doses, active, notes, frequency, schedule_type, schedule_config, start_date, end_date, created_at, updated_at) SELECT s.household_id, s.person_id, added.medication_id, s.dose_amount, s.dose_unit, s.dose_cycle, s.max_daily_doses, s.min_hours_between_doses, s.active, s.notes, s.frequency, s.schedule_type, s.schedule_config, s.start_date, s.end_date, now(), now() FROM schedules s CROSS JOIN unnest($2::bigint[]) AS added(medication_id) WHERE s.id=$1", &[&schedule["id"].as_i64().unwrap(), &added_ids]).unwrap(), 500);
    let medications = collection(&target, &format!("{api}/medications"));
    let assignments = collection(&target, &format!("{api}/person_medications"));
    let schedules = collection(&target, &format!("{api}/schedules"));
    for rows in [&medications, &assignments, &schedules] { assert!(rows.len() > 500); }
    let medication_ids = medications.iter().map(|row| row["id"].as_i64().unwrap()).collect::<BTreeSet<_>>();
    assert!(!medication_ids.contains(&fixture.foreign_medication_id));
    let person = format!("/households/{}/people/{}", fixture.household_slug, fixture.journey_browser_person_id);
    let html = form(&target, &person);
    let document = Html::parse_document(&html);
    for (resource, rows) in [("assignments", &assignments), ("schedules", &schedules)] {
        let expected = rows.iter().filter(|row| row["person_id"] == fixture.journey_browser_person_id).map(|row| row["id"].as_i64().unwrap()).collect::<BTreeSet<_>>();
        assert!(expected.len() >= 501);
        let histories = document.select(&Selector::parse(&format!("a[href^='{person}/{resource}/'][href$='/history']")).unwrap())
            .map(|node| node.value().attr("href").unwrap().trim_end_matches("/history").rsplit('/').next().unwrap().parse::<i64>().unwrap()).collect::<BTreeSet<_>>();
        assert_eq!(histories, expected, "person overview must include every authorised {resource} row");
    }
    for suffix in [
        "/assignments/new".to_owned(), "/schedules/new?type=daily".to_owned(),
        format!("/assignments/{}/edit", assignment["id"].as_i64().unwrap()),
        format!("/schedules/{}/edit", schedule["id"].as_i64().unwrap()),
    ] {
        let html = form(&target, &format!("{person}{suffix}"));
        let document = Html::parse_document(&html);
        let actual = document.select(&Selector::parse("select[name='medication_id'] option[value]").unwrap())
            .filter_map(|node| node.value().attr("value").unwrap().parse::<i64>().ok()).collect::<BTreeSet<_>>();
        assert_eq!(actual, medication_ids, "native treatment choices must not drop later medication pages");
        if suffix.ends_with("/edit") { assert!(!field(&html, "etag").is_empty(), "original member version remains available"); }
    }
    assert_eq!(fixture.journey_browser_person_id, fixture.managed_person_id);
    let viewer = Target::from_env();
    let html = form(&viewer, "/login");
    let email: String = db.query_one("SELECT email FROM accounts WHERE id=$1", &[&fixture.view_account_id]).unwrap().get(0);
    assert_eq!(email, fixture.web_view_email);
    assert_eq!(viewer.post_html_form_from_client("/login", "198.18.54.2", &[
        ("authenticity_token".into(), field(&html, "authenticity_token")),
        ("email".into(), email), ("password".into(), "password".into()),
    ]).status().as_u16(), 302);
    let expected_medications = db.query("SELECT m.id FROM medications m WHERE m.household_id=$1 AND (EXISTS (SELECT 1 FROM person_medications p WHERE p.household_id=$1 AND p.person_id=$2 AND p.medication_id=m.id) OR EXISTS (SELECT 1 FROM schedules s WHERE s.household_id=$1 AND s.person_id=$2 AND s.medication_id=m.id) OR (m.created_by_membership_id=$3 AND NOT EXISTS (SELECT 1 FROM person_medications p WHERE p.household_id=$1 AND p.medication_id=m.id) AND NOT EXISTS (SELECT 1 FROM schedules s WHERE s.household_id=$1 AND s.medication_id=m.id)))", &[&fixture.household_id, &fixture.managed_person_id, &fixture.view_membership_id]).unwrap()
        .iter().map(|row| row.get::<_, i64>(0)).collect::<BTreeSet<_>>();
    let expected_assignments = assignments.iter().filter(|row| row["person_id"] == fixture.managed_person_id).map(|row| row["id"].as_i64().unwrap()).collect::<BTreeSet<_>>();
    let expected_schedules = schedules.iter().filter(|row| row["person_id"] == fixture.managed_person_id).map(|row| row["id"].as_i64().unwrap()).collect::<BTreeSet<_>>();
    for (resource, expected) in [("medications", &expected_medications), ("person_medications", &expected_assignments), ("schedules", &expected_schedules)] {
        assert!(expected.len() >= 501);
        let rows = collection(&viewer, &format!("{api}/{resource}"));
        let actual = rows.iter().map(|row| row["id"].as_i64().unwrap()).collect::<BTreeSet<_>>();
        assert_eq!(&actual, expected, "viewer sees complete permitted {resource} pages including pages one and two");
        if resource == "medications" {
            assert!(!actual.contains(&fixture.hidden_medication_id));
            assert!(!actual.contains(&fixture.foreign_medication_id));
        } else {
            assert!(rows.iter().all(|row| row["person_id"] == fixture.managed_person_id));
        }
    }
    let html = form(&viewer, &person);
    let document = Html::parse_document(&html);
    for (resource, expected) in [("assignments", &expected_assignments), ("schedules", &expected_schedules)] {
        let actual = document.select(&Selector::parse(&format!("a[href^='{person}/{resource}/'][href$='/history']")).unwrap())
            .map(|node| node.value().attr("href").unwrap().trim_end_matches("/history").rsplit('/').next().unwrap().parse::<i64>().unwrap()).collect::<BTreeSet<_>>();
        assert_eq!(&actual, expected, "viewer person history includes every permitted {resource}");
    }
    for suffix in ["/new", "/edit", "/pause", "/resume"] {
        assert_eq!(document.select(&Selector::parse(&format!("a[href^='{person}/assignments'][href$='{suffix}'], a[href^='{person}/schedules'][href$='{suffix}']")).unwrap()).count(), 0, "view access does not expose treatment management controls");
    }
}
