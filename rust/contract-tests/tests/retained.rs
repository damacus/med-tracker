use medtracker_contract_tests::{fixture, Fixture, Target};
use scraper::{Html, Selector};
use serde_json::{json, Value};

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
    let response = target.post_html_form("/login", &fields);
    assert_eq!(response.status().as_u16(), 302);
}

fn offline_path(fixture: &Fixture, suffix: &str) -> String {
    format!(
        "/households/{}/offline{suffix}",
        fixture.retained_household_slug
    )
}

#[test]
fn public_pwa_assets_publish_manifest_and_worker_version() {
    let target = Target::from_env();
    let response = target.get("/manifest.webmanifest", None);
    assert_eq!(response.status().as_u16(), 200);
    assert!(response.headers()["content-type"]
        .to_str()
        .expect("manifest content type")
        .starts_with("application/manifest+json"));
    let csp = response
        .headers()
        .get("content-security-policy")
        .or_else(|| {
            response
                .headers()
                .get("content-security-policy-report-only")
        })
        .expect("content security policy")
        .to_str()
        .expect("content security policy value");
    assert!(csp.contains("worker-src"));
    let manifest: Value = response.json().expect("manifest JSON");
    assert_eq!(manifest["name"], "MedTracker");
    assert_eq!(manifest["short_name"], "MedTracker");
    assert_eq!(manifest["background_color"], "#102447");
    assert_eq!(manifest["theme_color"], "#102447");
    assert!(manifest["icons"]
        .as_array()
        .expect("manifest icons")
        .iter()
        .any(|icon| icon["src"] == "/icons/icon-192.png" && icon["sizes"] == "192x192"));
    assert!(manifest["icons"]
        .as_array()
        .expect("manifest icons")
        .iter()
        .any(|icon| icon["src"] == "/icons/icon-512.png" && icon["sizes"] == "512x512"));

    let response = target.get("/service-worker.js", None);
    assert_eq!(response.status().as_u16(), 200);
    assert!(response.headers()["content-type"]
        .to_str()
        .expect("worker content type")
        .starts_with("text/javascript"));
    let worker = response.text().expect("service worker");
    assert!(worker.contains("const CACHE_VERSION = 'v3'"));
    assert!(worker.contains("self.addEventListener('fetch'"));
}

#[test]
fn offline_snapshot_requires_web_session_and_hides_foreign_household() {
    let fixture = fixture();
    let target = Target::from_env();
    let path = offline_path(&fixture, "/snapshot");
    let unauthenticated = target.get(&path, None);
    assert_eq!(unauthenticated.status().as_u16(), 302);
    assert!(unauthenticated.headers()["location"]
        .to_str()
        .expect("login redirect")
        .starts_with("/login"));

    sign_in(&target, &fixture.retained_email);
    let response = target.get(&path, None);
    assert_eq!(response.status().as_u16(), 200);
    let snapshot: Value = response.json().expect("offline snapshot");
    assert!(snapshot["meta"]["generated_at"].is_string());
    for collection in [
        "people",
        "locations",
        "medications",
        "schedules",
        "person_medications",
        "medication_takes",
    ] {
        assert!(snapshot["data"][collection].is_array(), "{collection}");
    }
    let schedules = snapshot["data"]["schedules"].as_array().expect("schedules");
    let managed = schedules
        .iter()
        .find(|schedule| schedule["id"] == fixture.retained_schedule_id)
        .expect("retained schedule");
    assert!(managed["offline_eligibility"]["allowed"].is_boolean());
    assert!(!snapshot.to_string().contains(&fixture.foreign_person_name));

    let foreign_path = format!(
        "/households/{}/offline/snapshot",
        fixture.retained_foreign_household_slug
    );
    let foreign = target.get(&foreign_path, None);
    assert_eq!(foreign.status().as_u16(), 302);
    assert!(!foreign
        .text()
        .expect("foreign response")
        .contains(&fixture.foreign_email));
}

#[test]
fn offline_queued_take_replays_by_client_uuid_and_rejects_missing_source() {
    let fixture = fixture();
    let target = Target::from_env();
    sign_in(&target, &fixture.retained_email);
    let snapshot: Value = target
        .get(&offline_path(&fixture, "/snapshot"), None)
        .json()
        .expect("offline snapshot");
    let taken_at = snapshot["meta"]["generated_at"]
        .as_str()
        .expect("generated at");
    let client_uuid = format!(
        "00000000-0000-4000-8000-{:012x}",
        fixture.retained_household_id
    );
    let path = offline_path(&fixture, "/medication_takes");
    let body = json!({
        "client_uuid": client_uuid,
        "source_type": "schedule",
        "source_id": fixture.retained_schedule_id,
        "taken_at": taken_at,
        "dose_amount": "1",
        "taken_from_medication_id": fixture.retained_medication_id
    });
    let response = target.post_json(&path, &body);
    assert_eq!(response.status().as_u16(), 201);
    let created: Value = response.json().expect("created take");
    assert_eq!(created["data"]["client_uuid"], client_uuid);
    let take_id = created["data"]["id"].clone();

    let response = target.post_json(&path, &body);
    assert_eq!(response.status().as_u16(), 200);
    let replayed: Value = response.json().expect("replayed take");
    assert_eq!(replayed["data"]["id"], take_id);

    let invalid_uuid = format!(
        "00000000-0000-4001-8000-{:012x}",
        fixture.retained_household_id
    );
    let invalid = json!({
        "client_uuid": invalid_uuid,
        "source_type": "schedule",
        "source_id": 999_999_999,
        "taken_at": taken_at,
        "dose_amount": "1"
    });
    let response = target.post_json(&path, &invalid);
    assert_eq!(response.status().as_u16(), 422);
    let error: Value = response.json().expect("source error");
    assert_eq!(error["error"]["code"], "unprocessable_content");
    assert!(error["error"]["message"]
        .as_str()
        .expect("error message")
        .contains("no longer available"));
}
