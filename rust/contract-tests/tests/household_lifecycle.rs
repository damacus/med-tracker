use medtracker_contract_tests::{Fixture, Target, fixture};
use scraper::{Html, Selector};
use serde_json::{Value, json};

fn input_value(html: &str, name: &str) -> String {
    let document = Html::parse_document(html);
    document
        .select(&Selector::parse(&format!("input[name='{name}']")).unwrap())
        .next()
        .and_then(|input| input.value().attr("value"))
        .expect("rendered native form value")
        .to_owned()
}

fn sign_in(target: &Target, fixture: &Fixture, client_ip: &str) {
    let login = target.get_html("/login");
    assert_eq!(login.status().as_u16(), 200);
    let csrf = input_value(&login.text().unwrap(), "authenticity_token");
    let response = target.post_html_form_from_client(
        "/login",
        client_ip,
        &[
            ("email".into(), fixture.primary_email.clone()),
            ("password".into(), "password".into()),
            ("authenticity_token".into(), csrf),
        ],
    );
    assert_eq!(response.status().as_u16(), 302);
}

fn create_medication(target: &Target, fixture: &Fixture, name: &str, dose: Value) -> i64 {
    let response = target.post_json_authorized(
        &format!("/api/v1/households/{}/medications", fixture.household_id),
        &fixture.access_token,
        &json!({"medication": {
            "name": name,
            "location_id": fixture.primary_location_id,
            "dose_amount": dose,
            "dose_unit": "ml",
            "current_supply": "20.75",
            "reorder_threshold": "3.5"
        }}),
    );
    assert_eq!(response.status().as_u16(), 201);
    let body: Value = response.json().unwrap();
    body["data"]["id"].as_i64().expect("created medication ID")
}

fn paths(fixture: &Fixture, id: i64) -> (String, String) {
    (
        format!(
            "/api/v1/households/{}/medications/{id}",
            fixture.household_id
        ),
        format!("/households/{}/medications/{id}", fixture.household_slug),
    )
}

fn all_options(target: &Target, fixture: &Fixture) -> Vec<Value> {
    let mut rows = Vec::new();
    for page in 1..=10 {
        let response = target.get(
            &format!(
                "/api/v1/households/{}/dosage_options?page={page}&per_page=100",
                fixture.household_id
            ),
            Some(&fixture.access_token),
        );
        assert_eq!(response.status().as_u16(), 200);
        let body: Value = response.json().unwrap();
        let data = body["data"].as_array().expect("dosage options page");
        rows.extend(data.iter().cloned());
        let count = body["meta"]["total_count"]
            .as_u64()
            .expect("options total count") as usize;
        if rows.len() >= count {
            return rows;
        }
        assert!(
            !data.is_empty(),
            "options returned an empty page before total count"
        );
    }
    panic!("disposable fixture options exceed the bounded collection limit")
}

fn draft(fixture: &Fixture, html: &str, name: &str, dose: &str) -> Vec<(String, String)> {
    vec![
        (
            "authenticity_token".into(),
            input_value(html, "authenticity_token"),
        ),
        ("etag".into(), input_value(html, "etag")),
        ("name".into(), name.into()),
        ("friendly_name".into(), "Retained lifecycle display".into()),
        ("warnings".into(), "Retained lifecycle warning".into()),
        (
            "location_id".into(),
            fixture.primary_location_id.to_string(),
        ),
        ("dose_amount".into(), dose.into()),
        ("dose_unit".into(), "ml".into()),
        ("current_supply".into(), "20.75".into()),
        ("reorder_threshold".into(), "3.5".into()),
    ]
}

#[test]
fn medication_without_a_dose_or_options_can_add_its_first_scalar_dose_on_edit() {
    let fixture = fixture();
    let target = Target::from_env();
    sign_in(&target, &fixture, "198.18.26.1");
    let name = "Contract blank dose lifecycle";
    let id = create_medication(&target, &fixture, name, Value::Null);
    let (api_path, web_path) = paths(&fixture, id);
    assert!(
        !all_options(&target, &fixture)
            .iter()
            .any(|row| row["medication_id"] == id)
    );
    let edit = target.get_html(&format!("{web_path}/edit"));
    assert_eq!(edit.status().as_u16(), 200);
    let html = edit.text().unwrap();
    let document = Html::parse_document(&html);
    for name in ["dose_amount", "dose_unit", "current_supply"] {
        assert_eq!(
            document
                .select(&Selector::parse(&format!("[name='{name}']")).unwrap())
                .count(),
            1,
            "blank dose without options must retain the scalar {name} control"
        );
    }
    let saved = target.post_browser_form(&web_path, &draft(&fixture, &html, name, "1.25"));
    assert!([302, 303].contains(&saved.status().as_u16()));
    let persisted = target.get(&api_path, Some(&fixture.access_token));
    assert_eq!(persisted.status().as_u16(), 200);
    let persisted: Value = persisted.json().unwrap();
    assert_eq!(persisted["data"]["dose_amount"], "1.25");
    assert_eq!(persisted["data"]["dose_unit"], "ml");
}

#[test]
fn a_stale_scalar_form_rejected_after_dosage_options_are_added_preserves_its_draft() {
    let fixture = fixture();
    let target = Target::from_env();
    sign_in(&target, &fixture, "198.18.26.2");
    let name = "Contract stale scalar mode lifecycle";
    let id = create_medication(&target, &fixture, name, json!("1.25"));
    let (api_path, web_path) = paths(&fixture, id);
    let edit = target.get_html(&format!("{web_path}/edit"));
    assert_eq!(edit.status().as_u16(), 200);
    let html = edit.text().unwrap();
    let captured = draft(&fixture, &html, name, "2.50");
    let etag = input_value(&html, "etag");
    let changed = target.patch_json_if_match(
        &api_path,
        &fixture.access_token,
        &json!({"medication": {"dose_amount": null}}),
        &etag,
    );
    assert_eq!(changed.status().as_u16(), 200);
    let option = target.post_json_authorized(
        &format!("/api/v1/households/{}/dosage_options", fixture.household_id),
        &fixture.access_token,
        &json!({"dosage_option": {
            "medication_id": id.to_string(),
            "amount": "1.25",
            "unit": "ml",
            "frequency": "daily",
            "default_max_daily_doses": 4,
            "default_min_hours_between_doses": "0",
            "default_dose_cycle": "daily",
            "current_supply": "4.00"
        }}),
    );
    assert_eq!(option.status().as_u16(), 201);
    let option: Value = option.json().unwrap();
    let option_id = option["data"]["id"].as_i64().unwrap();
    let before = target.get(&api_path, Some(&fixture.access_token));
    assert_eq!(before.status().as_u16(), 200);
    let before: Value = before.json().unwrap();
    let stale = target.post_browser_form(&web_path, &captured);
    assert_eq!(
        stale.status().as_u16(),
        409,
        "mode changes must preserve the stale precondition outcome"
    );
    let stale_html = stale.text().unwrap();
    assert_eq!(input_value(&stale_html, "dose_amount"), "2.50");
    assert_eq!(
        input_value(&stale_html, "friendly_name"),
        "Retained lifecycle display"
    );
    let document = Html::parse_document(&stale_html);
    let warning = document
        .select(&Selector::parse("textarea[name='warnings']").unwrap())
        .next()
        .expect("retained native warnings field");
    assert_eq!(
        warning.text().collect::<String>(),
        "Retained lifecycle warning"
    );
    let after = target.get(&api_path, Some(&fixture.access_token));
    assert_eq!(after.status().as_u16(), 200);
    let after: Value = after.json().unwrap();
    assert_eq!(after["data"], before["data"]);
    let retained_option = target.get(
        &format!(
            "/api/v1/households/{}/dosage_options/{option_id}",
            fixture.household_id
        ),
        Some(&fixture.access_token),
    );
    assert_eq!(retained_option.status().as_u16(), 200);
    let retained_option: Value = retained_option.json().unwrap();
    assert_eq!(retained_option["data"], option["data"]);
}
