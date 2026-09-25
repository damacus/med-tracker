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

fn retained_stock(snapshot: &Value, medication_id: i64) -> &str {
    snapshot["data"]["medications"]
        .as_array()
        .expect("medications")
        .iter()
        .find(|medication| medication["id"] == medication_id)
        .expect("retained medication")["current_supply"]
        .as_str()
        .expect("current supply")
}

fn household_takes(snapshot: &Value) -> &[Value] {
    snapshot["data"]["medication_takes"]
        .as_array()
        .expect("medication takes")
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
    assert_eq!(response.headers()["cache-control"], "public, max-age=3600");
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
    assert!(managed["person_id"].as_i64().expect("person id") > 0);
    assert_eq!(managed["medication_id"], fixture.retained_medication_id);
    assert_eq!(managed["dose_amount"], "1.0");
    assert_eq!(managed["dose_unit"], "ml");
    assert_eq!(managed["frequency"], "Daily");
    assert_eq!(managed["start_date"], "2026-01-01");
    assert_eq!(managed["end_date"], "2099-12-31");
    assert_eq!(managed["active"], true);
    let eligibility = &managed["offline_eligibility"];
    assert_eq!(eligibility["allowed"], true);
    assert!(eligibility["reason"].is_null());
    assert_eq!(eligibility["dose_amount"], "1.0");
    assert_eq!(eligibility["dose_unit"], "ml");
    let generated_at = snapshot["meta"]["generated_at"]
        .as_str()
        .expect("generated at");
    let valid_until = format!("{}T23:59:59{}", &generated_at[..10], &generated_at[19..]);
    assert_eq!(eligibility["valid_until"], valid_until);
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
fn offline_snapshot_keeps_ineligible_schedules_visible_with_reasons() {
    let fixture = fixture();
    let target = Target::from_env();
    sign_in(&target, &fixture.offline_eligibility_email);
    let path = format!(
        "/households/{}/offline/snapshot",
        fixture.offline_eligibility_household_slug
    );
    let response = target.get(&path, None);
    assert_eq!(response.status().as_u16(), 200);
    let snapshot: Value = response.json().expect("offline eligibility snapshot");
    let schedules = snapshot["data"]["schedules"]
        .as_array()
        .expect("offline schedules");
    let find_schedule = |id| {
        schedules
            .iter()
            .find(|schedule| schedule["id"] == id)
            .expect("ineligible schedule remains visible")
    };
    let inactive = find_schedule(fixture.offline_inactive_schedule_id);
    assert_eq!(inactive["active"], false);
    let cooldown = find_schedule(fixture.offline_cooldown_schedule_id);
    assert!(snapshot["data"]["medication_takes"]
        .as_array()
        .expect("recent takes")
        .iter()
        .any(|take| take["schedule_id"] == fixture.offline_cooldown_schedule_id));
    let expired = find_schedule(fixture.offline_expired_schedule_id);
    assert!(
        expired["end_date"].as_str().expect("expired end date")
            < &snapshot["meta"]["generated_at"]
                .as_str()
                .expect("generated at")[..10]
    );
    for source in [inactive, cooldown, expired] {
        assert_eq!(source["offline_eligibility"]["allowed"], false);
        assert!(!source["offline_eligibility"]["reason"]
            .as_str()
            .expect("disallowed reason")
            .is_empty());
    }
}

#[test]
fn offline_future_queued_take_returns_validation_without_mutation() {
    let fixture = fixture();
    let target = Target::from_env();
    sign_in(&target, &fixture.offline_future_email);
    let snapshot_path = format!(
        "/households/{}/offline/snapshot",
        fixture.offline_future_household_slug
    );
    let before: Value = target
        .get(&snapshot_path, None)
        .json()
        .expect("snapshot before future take");
    assert_eq!(
        retained_stock(&before, fixture.offline_future_medication_id),
        "50.0"
    );
    assert!(household_takes(&before).is_empty());

    let body = json!({
        "client_uuid": format!("00000000-0000-4002-8000-{:012x}", fixture.offline_future_household_id),
        "source_type": "schedule",
        "source_id": fixture.offline_future_schedule_id,
        "taken_at": "2099-01-01T00:00:00Z",
        "dose_amount": "1",
        "taken_from_medication_id": fixture.offline_future_medication_id
    });
    let takes_path = format!(
        "/households/{}/offline/medication_takes",
        fixture.offline_future_household_slug
    );
    let response = target.post_json(&takes_path, &body);
    assert_eq!(response.status().as_u16(), 422);
    let rejected: Value = response.json().expect("future dose error");
    assert_eq!(rejected["error"]["code"], "unprocessable_content");
    assert!(rejected["error"]["message"]
        .as_str()
        .expect("future dose message")
        .contains("future"));

    let after: Value = target
        .get(&snapshot_path, None)
        .json()
        .expect("snapshot after future take");
    assert_eq!(
        retained_stock(&after, fixture.offline_future_medication_id),
        "50.0"
    );
    assert!(household_takes(&after).is_empty());
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
    assert_eq!(
        retained_stock(&snapshot, fixture.retained_medication_id),
        "50.0"
    );
    assert!(household_takes(&snapshot).is_empty());

    let unauthenticated = Target::from_env().post_json(&path, &body);
    assert_eq!(unauthenticated.status().as_u16(), 302);
    assert!(unauthenticated.headers()["location"]
        .to_str()
        .expect("login redirect")
        .starts_with("/login"));

    let foreign_path = format!(
        "/households/{}/offline/medication_takes",
        fixture.retained_foreign_household_slug
    );
    let foreign = target.post_json(&foreign_path, &body);
    assert_eq!(foreign.status().as_u16(), 302);
    let before: Value = target
        .get(&offline_path(&fixture, "/snapshot"), None)
        .json()
        .expect("unchanged offline snapshot");
    assert_eq!(
        retained_stock(&before, fixture.retained_medication_id),
        "50.0"
    );
    assert!(household_takes(&before).is_empty());

    let response = target.post_json(&path, &body);
    assert_eq!(response.status().as_u16(), 201);
    let created: Value = response.json().expect("created take");
    assert_eq!(created["data"]["client_uuid"], client_uuid);
    let take_id = created["data"]["id"].clone();
    let after_create: Value = target
        .get(&offline_path(&fixture, "/snapshot"), None)
        .json()
        .expect("snapshot after create");
    assert_eq!(
        retained_stock(&after_create, fixture.retained_medication_id),
        "49.0"
    );
    let created_takes = household_takes(&after_create);
    assert_eq!(created_takes.len(), 1);
    assert_eq!(created_takes[0]["id"], take_id);
    assert_eq!(created_takes[0]["client_uuid"], client_uuid);
    assert_eq!(
        created_takes[0]["schedule_id"],
        fixture.retained_schedule_id
    );
    assert_eq!(
        created_takes[0]["medication_id"],
        fixture.retained_medication_id
    );
    assert_eq!(
        created_takes[0]["taken_from_medication_id"],
        fixture.retained_medication_id
    );

    let response = target.post_json(&path, &body);
    assert_eq!(response.status().as_u16(), 200);
    let replayed: Value = response.json().expect("replayed take");
    assert_eq!(replayed["data"]["id"], take_id);
    let after_replay: Value = target
        .get(&offline_path(&fixture, "/snapshot"), None)
        .json()
        .expect("snapshot after replay");
    assert_eq!(
        retained_stock(&after_replay, fixture.retained_medication_id),
        "49.0"
    );
    let replay_takes = household_takes(&after_replay);
    assert_eq!(replay_takes.len(), 1);
    assert_eq!(replay_takes[0]["id"], take_id);
    assert_eq!(replay_takes[0], created_takes[0]);

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
