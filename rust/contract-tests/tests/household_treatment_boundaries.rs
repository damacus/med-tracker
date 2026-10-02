use medtracker_contract_tests::{Fixture, Target, fixture};
use scraper::{Html, Selector};
use serde_json::{Value, json};

fn field(html: &str, name: &str) -> String {
    let document = Html::parse_document(html);
    let node = document
        .select(&Selector::parse(&format!("[name='{name}']")).unwrap())
        .next()
        .expect("native field");
    if node.value().name() == "textarea" {
        node.text().collect()
    } else if node.value().name() == "select" {
        node.select(&Selector::parse("option[selected]").unwrap())
            .next()
            .and_then(|option| option.value().attr("value"))
            .expect("selected value")
            .to_owned()
    } else {
        node.value().attr("value").expect("field value").to_owned()
    }
}

fn read(target: &Target, path: &str) -> Value {
    let response = target.get(path, None);
    assert_eq!(response.status().as_u16(), 200);
    response.json::<Value>().unwrap()["data"].clone()
}

fn setup(name: &str, client: &str) -> (Fixture, Target, String, Value) {
    let fixture = fixture();
    let target = Target::from_env();
    let html = target.get_html("/login").text().unwrap();
    assert_eq!(
        target
            .post_html_form_from_client(
                "/login",
                client,
                &[
                    (
                        "authenticity_token".into(),
                        field(&html, "authenticity_token")
                    ),
                    ("email".into(), fixture.primary_email.clone()),
                    ("password".into(), "password".into()),
                ]
            )
            .status()
            .as_u16(),
        302
    );
    let html = target
        .get_html(&format!(
            "/households/{}/medications",
            fixture.household_slug
        ))
        .text()
        .unwrap();
    let document = Html::parse_document(&html);
    let csrf = document
        .select(&Selector::parse("meta[name='csrf-token']").unwrap())
        .next()
        .unwrap()
        .value()
        .attr("content")
        .unwrap()
        .to_owned();
    let response = target.post_browser_json(&format!("/api/v1/households/{}/medications", fixture.household_id), &csrf,
        &json!({"medication": {"name": name, "location_id": fixture.primary_location_id, "dose_amount": "1.25", "dose_unit": "ml", "current_supply": "200", "reorder_threshold": "3"}}));
    assert_eq!(response.status().as_u16(), 201);
    let medication = response.json::<Value>().unwrap()["data"].clone();
    (fixture, target, csrf, medication)
}

fn prefix(fixture: &Fixture) -> String {
    format!(
        "/households/{}/people/{}",
        fixture.household_slug, fixture.journey_browser_person_id
    )
}

fn form(target: &Target, path: &str) -> String {
    let response = target.get_html(path);
    assert_eq!(
        response.status().as_u16(),
        200,
        "ordinary treatment editor exists"
    );
    let html = response.text().unwrap();
    assert_eq!(
        Html::parse_document(&html)
            .select(&Selector::parse("form.household-form").unwrap())
            .count(),
        1
    );
    html
}

fn common(csrf: &str, medication: &Value) -> Vec<(String, String)> {
    vec![
        ("authenticity_token".into(), csrf.into()),
        (
            "medication_id".into(),
            medication["id"].as_i64().unwrap().to_string(),
        ),
        ("dose_amount".into(), "1.25".into()),
        ("dose_unit".into(), "ml".into()),
        ("max_daily_doses".into(), "4".into()),
        ("min_hours_between_doses".into(), "8".into()),
        ("dose_cycle".into(), "daily".into()),
        ("notes".into(), "Treatment <script>draft</script>".into()),
    ]
}

fn create_source(schedule: bool, client: &str) -> (Fixture, Target, String, Value, Value) {
    let (fixture, target, csrf, medication) = setup(&format!("Treatment boundary {client}"), client);
    let resource = if schedule { "schedules" } else { "person_medications" };
    let key = if schedule { "schedule" } else { "person_medication" };
    let mut attributes = json!({"person_id": fixture.journey_browser_person_id.to_string(), "medication_id": medication["id"].as_i64().unwrap().to_string(), "dose_amount": "1.25", "dose_unit": "ml"});
    if schedule {
        attributes["schedule_type"] = "daily".into();
        attributes["schedule_config"] = json!({"times": ["08:00"]});
        attributes["start_date"] = "2030-03-30".into();
        attributes["end_date"] = "2030-04-10".into();
    } else { attributes["administration_kind"] = "as_needed".into(); }
    let response = target.post_browser_json(&format!("/api/v1/households/{}/{resource}", fixture.household_id), &csrf, &json!({(key): attributes}));
    assert_eq!(response.status().as_u16(), 201);
    let source = response.json::<Value>().unwrap()["data"].clone();
    (fixture, target, csrf, medication, source)
}

fn missing_edit_token(schedule: bool, client: &str) {
    let (fixture, target, csrf, medication, source) = create_source(schedule, client);
    let resource = if schedule { "schedules" } else { "person_medications" };
    let browser = if schedule { "schedules" } else { "assignments" };
    let member = format!("{}/{browser}/{}", prefix(&fixture), source["id"].as_i64().unwrap());
    let html = form(&target, &format!("{member}/edit"));
    let mut fields = common(&csrf, &medication);
    fields.push(("submission_id".into(), field(&html, "submission_id")));
    if schedule {
        for (name, value) in [("schedule_type", "daily"), ("start_date", "2030-03-30"), ("end_date", "2030-04-10"), ("time_0", "08:00")] { fields.push((name.into(), value.into())); }
    } else { fields.push(("administration_kind".into(), "as_needed".into())); }
    for token in [None, Some(""), Some("   ")] {
        let mut attempted = fields.clone();
        if let Some(token) = token { attempted.push(("etag".into(), token.into())); }
        let response = target.post_browser_form(&member, &attempted);
        assert_eq!(response.status().as_u16(), 428);
        let rejected = response.text().unwrap();
        for (name, value) in &attempted { assert_eq!(field(&rejected, name), *value); }
        assert_eq!(field(&rejected, "etag"), token.unwrap_or_default());
        assert_eq!(read(&target, &format!("/api/v1/households/{}/{resource}/{}", fixture.household_id, source["id"].as_i64().unwrap())), source);
    }
}

#[test]
fn assignment_edit_requires_original_token_and_retains_every_entry() { missing_edit_token(false, "198.18.45.1"); }

#[test]
fn schedule_edit_requires_original_token_and_retains_every_entry() { missing_edit_token(true, "198.18.45.2"); }

#[test]
fn assignment_rejects_foreign_person_medication_and_option_without_creating_sources() {
    let (fixture, target, csrf, medication) = setup("Foreign treatment boundaries", "198.18.45.3");
    let base = format!("{}/assignments", prefix(&fixture));
    let html = form(&target, &format!("{base}/new"));
    let before = read(&target, &format!("/api/v1/households/{}/person_medications?per_page=100", fixture.household_id));
    let mut fields = common(&csrf, &medication);
    fields.push(("administration_kind".into(), "as_needed".into()));
    fields.push(("submission_id".into(), field(&html, "submission_id")));
    let foreign = format!("/households/{}/people/{}/assignments", fixture.household_slug, fixture.foreign_person_id);
    assert_eq!(target.post_browser_form(&foreign, &fields).status().as_u16(), 404);
    let mut other = fields.clone();
    other.iter_mut().find(|(name, _)| name == "medication_id").unwrap().1 = fixture.foreign_medication_id.to_string();
    assert_eq!(target.post_browser_form(&base, &other).status().as_u16(), 404);
    let mut other = fields.clone();
    other.push(("source_dosage_option_id".into(), fixture.foreign_dosage_id.to_string()));
    assert_eq!(target.post_browser_form(&base, &other).status().as_u16(), 404);
    let api = format!("/api/v1/households/{}", fixture.household_id);
    let created = target.post_browser_json(&format!("{api}/medications"), &csrf, &json!({"medication": {"name": "Unrelated visible option parent", "location_id": fixture.primary_location_id, "dose_amount": "1.25", "dose_unit": "ml", "current_supply": "20", "reorder_threshold": "3"}}));
    assert_eq!(created.status().as_u16(), 201);
    let parent = created.json::<Value>().unwrap()["data"].clone();
    let created = target.post_browser_json(&format!("{api}/dosage_options"), &csrf, &json!({"dosage_option": {"medication_id": parent["id"].as_i64().unwrap().to_string(), "amount": "1.25", "unit": "ml", "frequency": "Daily", "default_dose_cycle": "daily", "default_max_daily_doses": 4, "default_min_hours_between_doses": "0"}}));
    assert_eq!(created.status().as_u16(), 201);
    let option = created.json::<Value>().unwrap()["data"].clone();
    let mut other = fields.clone();
    other.push(("source_dosage_option_id".into(), option["id"].as_i64().unwrap().to_string()));
    assert_eq!(target.post_browser_form(&base, &other).status().as_u16(), 422);
    assert_eq!(read(&target, &format!("/api/v1/households/{}/person_medications?per_page=100", fixture.household_id)), before);
}
