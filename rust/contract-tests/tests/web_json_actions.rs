use medtracker_contract_tests::{fixture, Target};
use scraper::{Html, Selector};
use serde_json::{json, Value};
use std::sync::atomic::{AtomicUsize, Ordering};

static LOGIN_CLIENT: AtomicUsize = AtomicUsize::new(1);

fn sign_in(target: &Target, email: &str) {
    let response = target.get_html("/login");
    assert_eq!(response.status().as_u16(), 200);
    let document = Html::parse_document(&response.text().expect("login HTML"));
    let selector = Selector::parse("form[action='/login'] input[name='authenticity_token']")
        .expect("login selector");
    let token = document
        .select(&selector)
        .next()
        .and_then(|input| input.value().attr("value"))
        .expect("login CSRF token");
    let fields = vec![
        ("email".to_string(), email.to_string()),
        ("password".to_string(), "password".to_string()),
        ("authenticity_token".to_string(), token.to_string()),
    ];
    let client_ip = format!("198.18.3.{}", LOGIN_CLIENT.fetch_add(1, Ordering::Relaxed));
    let response = target.post_html_form_from_client("/login", &client_ip, &fields);
    assert_eq!(response.status().as_u16(), 302);
}

fn ai_path(slug: &str) -> String {
    format!("/households/{slug}/ai-medication-suggestions")
}

fn person_path(slug: &str, id: i64) -> String {
    format!("/households/{slug}/people/{id}")
}

fn snapshot(target: &Target, slug: &str) -> Value {
    let response = target.get(&format!("/households/{slug}/offline/snapshot"), None);
    assert_eq!(response.status().as_u16(), 200);
    response.json().expect("household snapshot")
}

fn person_ids(snapshot: &Value) -> Vec<i64> {
    snapshot["data"]["people"]
        .as_array()
        .expect("people")
        .iter()
        .map(|person| person["id"].as_i64().expect("person id"))
        .collect()
}

#[test]
fn ai_suggestion_requires_login_and_paid_entitlement() {
    let fixture = fixture();
    let unauthenticated = Target::from_env();
    let response = unauthenticated.post_json(
        &ai_path(&fixture.web_ai_paid_slug),
        &json!({"medication": {"name": "Calpol Six Plus"}}),
    );
    assert_eq!(response.status().as_u16(), 302);
    assert!(response.headers()["location"]
        .to_str()
        .expect("redirect")
        .starts_with("/login"));

    let free = Target::from_env();
    sign_in(&free, &fixture.web_ai_free_email);
    let response = free.post_json(
        &ai_path(&fixture.web_ai_free_slug),
        &json!({"medication": {"name": "Calpol Six Plus"}}),
    );
    assert_eq!(response.status().as_u16(), 404);
}

#[test]
fn paid_ai_suggestion_is_sourced_and_foreign_household_is_hidden() {
    let fixture = fixture();
    let target = Target::from_env();
    sign_in(&target, &fixture.web_ai_paid_email);
    let response = target.post_json(
        &ai_path(&fixture.web_ai_paid_slug),
        &json!({"medication": {"name": "Calpol Six Plus"}}),
    );
    assert_eq!(response.status().as_u16(), 200);
    let body: Value = response.json().expect("suggestion JSON");
    assert_eq!(
        body["medication"]["description"],
        "Paracetamol pain and fever relief"
    );
    assert_eq!(body["medication"]["warnings"], "Contains paracetamol");
    assert_eq!(body["doses"][0]["amount"], 5);
    assert_eq!(body["doses"][0]["unit"], "ml");
    assert_eq!(body["doses"][0]["default_max_daily_doses"], 4);
    assert_eq!(body["doses"][0]["default_min_hours_between_doses"], 4);
    assert_eq!(body["doses"][0]["default_dose_cycle"], "daily");
    assert_eq!(
        body["doses"][0]["evidence"]["url"],
        body["sources"][0]["url"]
    );
    assert_eq!(
        body["doses"][0]["evidence"]["title"],
        body["sources"][0]["title"]
    );
    assert_eq!(body["sources"][0]["title"], "CALPOL SixPlus");
    assert_eq!(
        body["sources"][0]["url"],
        "https://www.calpol.co.uk/our-products/calpol-sixplus-oral-suspension-paracetamol"
    );
    assert_eq!(
        body["doses"][0]["evidence"]["text"],
        "Children 6-8 years 5ml Up to 4 times in 24 hours"
    );
    assert_eq!(body["errors"], json!([]));

    let foreign = target.post_json(
        &ai_path(&fixture.web_ai_free_slug),
        &json!({"medication": {"name": "Calpol Six Plus"}}),
    );
    assert_eq!(foreign.status().as_u16(), 302);
    assert!(!foreign
        .text()
        .expect("foreign body")
        .contains("Paracetamol pain and fever relief"));
}

#[test]
fn ai_suggestions_throttle_by_ip() {
    let fixture = fixture();
    let target = Target::from_env();
    sign_in(&target, &fixture.web_ai_ip_email);
    let path = ai_path(&fixture.web_ai_ip_slug);
    let body = json!({"medication": {"name": "Calpol Six Plus"}});
    for _ in 0..10 {
        let response = target.post_json_from_web_client(&path, "198.18.0.41", &body);
        assert_eq!(response.status().as_u16(), 200);
    }
    let response = target.post_json_from_web_client(&path, "198.18.0.41", &body);
    assert_eq!(response.status().as_u16(), 429);
    assert!(
        response.headers()["retry-after"]
            .to_str()
            .expect("retry-after")
            .parse::<u64>()
            .expect("seconds")
            > 0
    );
}

#[test]
fn ai_suggestions_throttle_by_account_across_ips() {
    let fixture = fixture();
    let target = Target::from_env();
    sign_in(&target, &fixture.web_ai_user_email);
    let path = ai_path(&fixture.web_ai_user_slug);
    let body = json!({"medication": {"name": "Calpol Six Plus"}});
    for index in 1..=20 {
        let response = target.post_json_from_web_client(&path, &format!("198.18.1.{index}"), &body);
        assert_eq!(response.status().as_u16(), 200, "request {index}");
    }
    let response = target.post_json_from_web_client(&path, "198.18.1.21", &body);
    assert_eq!(response.status().as_u16(), 429);
    assert!(
        response.headers()["retry-after"]
            .to_str()
            .expect("retry-after")
            .parse::<u64>()
            .expect("seconds")
            > 0
    );
}

#[test]
fn person_delete_requires_login_and_preserves_unauthorized_state() {
    let fixture = fixture();
    let path = person_path(&fixture.web_people_slug, fixture.web_people_delete_id);
    let unauthenticated = Target::from_env();
    let response = unauthenticated.delete_json(&path);
    assert_eq!(response.status().as_u16(), 302);
    assert!(response.headers()["location"]
        .to_str()
        .expect("redirect")
        .starts_with("/login"));

    let member = Target::from_env();
    sign_in(&member, &fixture.web_people_member_email);
    let before = snapshot(&member, &fixture.web_people_slug);
    assert!(!person_ids(&before).contains(&fixture.web_people_delete_id));
    let response = member.delete_json(&path);
    assert_eq!(response.status().as_u16(), 404);
    let after = snapshot(&member, &fixture.web_people_slug);
    assert_eq!(person_ids(&before), person_ids(&after));

    let owner = Target::from_env();
    sign_in(&owner, &fixture.web_people_email);
    let before = snapshot(&owner, &fixture.web_people_slug);
    assert!(person_ids(&before).contains(&fixture.web_people_delete_id));
    let foreign = owner.delete_json(&person_path(
        &fixture.web_people_slug,
        fixture.web_people_foreign_id,
    ));
    assert_eq!(foreign.status().as_u16(), 404);
    let after = snapshot(&owner, &fixture.web_people_slug);
    assert_eq!(person_ids(&before), person_ids(&after));

    let foreign_owner = Target::from_env();
    sign_in(&foreign_owner, &fixture.web_people_foreign_email);
    let foreign_before = snapshot(&foreign_owner, &fixture.web_people_foreign_slug);
    assert!(person_ids(&foreign_before).contains(&fixture.web_people_foreign_id));
    let foreign_path = person_path(
        &fixture.web_people_foreign_slug,
        fixture.web_people_foreign_id,
    );
    let denied = owner.delete_json(&foreign_path);
    assert_eq!(denied.status().as_u16(), 302);
    let foreign_after = snapshot(&foreign_owner, &fixture.web_people_foreign_slug);
    assert_eq!(person_ids(&foreign_before), person_ids(&foreign_after));
}

#[test]
fn person_delete_returns_no_content_and_removes_visible_person() {
    let fixture = fixture();
    let target = Target::from_env();
    sign_in(&target, &fixture.web_people_email);
    let before = snapshot(&target, &fixture.web_people_slug);
    assert!(person_ids(&before).contains(&fixture.web_people_delete_id));
    let response = target.delete_json(&person_path(
        &fixture.web_people_slug,
        fixture.web_people_delete_id,
    ));
    assert_eq!(response.status().as_u16(), 204);
    assert!(response.text().expect("empty response").is_empty());
    let after = snapshot(&target, &fixture.web_people_slug);
    assert!(!person_ids(&after).contains(&fixture.web_people_delete_id));
}

#[test]
fn person_with_administration_history_returns_json_validation_and_retains_history() {
    let fixture = fixture();
    let target = Target::from_env();
    sign_in(&target, &fixture.web_people_email);
    let before = snapshot(&target, &fixture.web_people_slug);
    assert!(person_ids(&before).contains(&fixture.web_people_history_id));
    let takes = before["data"]["medication_takes"]
        .as_array()
        .expect("takes");
    assert!(takes
        .iter()
        .any(|take| take["schedule_id"] == fixture.web_people_history_schedule_id));
    let response = target.delete_json(&person_path(
        &fixture.web_people_slug,
        fixture.web_people_history_id,
    ));
    assert_eq!(response.status().as_u16(), 422);
    let error: Value = response.json().expect("validation JSON");
    assert!(error
        .to_string()
        .contains("Person cannot be deleted while administration history exists"));
    let after = snapshot(&target, &fixture.web_people_slug);
    assert_eq!(person_ids(&before), person_ids(&after));
    assert_eq!(before["data"]["schedules"], after["data"]["schedules"]);
    assert_eq!(
        before["data"]["medication_takes"],
        after["data"]["medication_takes"]
    );
}

#[test]
#[ignore]
fn feature_disabled_hides_paid_ai_suggestion() {
    let fixture = fixture();
    let target = Target::from_env();
    sign_in(&target, &fixture.web_ai_paid_email);
    let response = target.post_json(
        &ai_path(&fixture.web_ai_paid_slug),
        &json!({"medication": {"name": "Calpol Six Plus"}}),
    );
    assert_eq!(response.status().as_u16(), 404);
}
