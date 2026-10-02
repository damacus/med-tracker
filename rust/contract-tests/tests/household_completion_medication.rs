use medtracker_contract_tests::{Fixture, Target, fixture};
use postgres::{Client, NoTls};
use scraper::{Html, Selector};
use serde_json::{Value, json};
use std::{
    env,
    sync::Arc,
    thread,
    time::{Duration, Instant},
};

#[path = "household_completion_medication/authentication.rs"]
mod authentication;

fn input(html: &str, name: &str) -> String {
    let document = Html::parse_document(html);
    document
        .select(&Selector::parse(&format!("input[name='{name}']")).unwrap())
        .next()
        .and_then(|field| field.value().attr("value"))
        .unwrap_or_else(|| {
            panic!(
                "native form value missing for {name}; HTML bytes={}",
                html.len()
            )
        })
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

fn create(target: &Target, fixture: &Fixture, name: &str) -> i64 {
    let response = target.post_json_authorized(&format!("/api/v1/households/{}/medications", fixture.household_id), &fixture.access_token, &json!({"medication": {
        "name": name, "location_id": fixture.primary_location_id, "dose_amount": null, "dose_unit": "ml", "current_supply": "20.75", "reorder_threshold": "3.5", "warnings": "Original warning"
    }}));
    assert_eq!(response.status().as_u16(), 201);
    let body: Value = response.json().unwrap();
    body["data"]["id"].as_i64().unwrap()
}

fn option(target: &Target, fixture: &Fixture, id: i64) -> Value {
    let response = target.post_json_authorized(&format!("/api/v1/households/{}/dosage_options", fixture.household_id), &fixture.access_token, &json!({"dosage_option": {
        "medication_id": id.to_string(), "amount": "1.25", "unit": "ml", "frequency": "daily", "default_max_daily_doses": 4, "default_min_hours_between_doses": "0", "default_dose_cycle": "daily", "current_supply": "4.00"
    }}));
    assert_eq!(response.status().as_u16(), 201);
    response.json::<Value>().unwrap()["data"].clone()
}

fn read(target: &Target, fixture: &Fixture, path: &str) -> Value {
    let response = target.get(path, Some(&fixture.access_token));
    assert_eq!(
        response.status().as_u16(),
        200,
        "persistent read-back: {path}"
    );
    response.json::<Value>().unwrap()["data"].clone()
}

fn draft(fixture: &Fixture, html: &str) -> Vec<(String, String)> {
    vec![
        (
            "authenticity_token".into(),
            input(html, "authenticity_token"),
        ),
        ("etag".into(), input(html, "etag")),
        ("name".into(), "Retained concurrent name".into()),
        ("friendly_name".into(), "Retained concurrent display".into()),
        (
            "description".into(),
            "Retained concurrent description".into(),
        ),
        ("barcode".into(), "retained-barcode".into()),
        (
            "warnings".into(),
            "Retained </textarea><script>alert('warning')</script> & warning".into(),
        ),
        (
            "location_id".into(),
            fixture.primary_location_id.to_string(),
        ),
        ("dose_amount".into(), "2.50".into()),
        ("dose_unit".into(), "ml".into()),
        ("current_supply".into(), "20.75".into()),
        ("reorder_threshold".into(), "3.5".into()),
    ]
}

fn assert_retained(html: &str, fields: &[(String, String)]) {
    for (name, value) in fields {
        if name == "authenticity_token" {
            continue;
        }
        if name == "warnings" || name == "description" {
            let document = Html::parse_document(html);
            let field = document
                .select(&Selector::parse(&format!("textarea[name='{name}']")).unwrap())
                .next()
                .unwrap();
            assert_eq!(field.text().collect::<String>(), *value);
        } else if name == "location_id" || name == "dose_unit" {
            let document = Html::parse_document(html);
            let selector =
                Selector::parse(&format!("select[name='{name}'] option[selected]")).unwrap();
            let selected = document
                .select(&selector)
                .next()
                .expect("retained selected option");
            assert_eq!(selected.value().attr("value"), Some(value.as_str()));
        } else {
            assert_eq!(input(html, name), *value, "rejected scalar draft {name}");
        }
    }
    assert!(!html.contains("<script>alert('warning')</script>"));
}

struct ReadGate {
    db: Client,
    name: String,
    key: i32,
}

impl ReadGate {
    fn new(fixture: &Fixture, medication_id: i64) -> Self {
        let mut db = Client::connect(
            &env::var("CONTRACT_AUDIT_DATABASE_URL").expect("disposable database URL"),
            NoTls,
        )
        .unwrap();
        let session: String = db.query_one("SELECT audit_context->>'session_reference' FROM security_audit_events WHERE actor_account_id = $1 AND household_id = $2 AND metadata->>'controller' = 'api/v1/medications' AND metadata->>'action' = 'show' AND audit_context->>'authentication_method' = 'browser_session' ORDER BY id DESC LIMIT 1", &[&fixture.account_id, &fixture.household_id]).expect("edit GET must establish unique browser audit session").get(0);
        let key = i32::try_from(medication_id).unwrap();
        let name = format!("completion_medication_gate_{key}");
        db.query_one("SELECT pg_advisory_lock(180027, $1)", &[&key])
            .unwrap();
        let mut gate = Self { db, name, key };
        let session = session.replace('\'', "''");
        gate.db.batch_execute(&format!("CREATE FUNCTION {}() RETURNS trigger LANGUAGE plpgsql AS $gate$ BEGIN IF NEW.actor_account_id = {} AND NEW.household_id = {} AND NEW.metadata->>'controller' = 'api/v1/medications' AND NEW.metadata->>'action' = 'show' AND NEW.metadata->>'http_method' = 'GET' AND NEW.metadata->>'status' = '200' AND NEW.audit_context->>'authentication_method' = 'browser_session' AND NEW.audit_context->>'session_reference' = '{}' THEN PERFORM pg_advisory_xact_lock(180027, {}); END IF; RETURN NEW; END $gate$; CREATE TRIGGER {} BEFORE INSERT ON security_audit_events FOR EACH ROW EXECUTE FUNCTION {}();", gate.name, fixture.account_id, fixture.household_id, session, key, gate.name, gate.name)).expect("install disposable browser-read gate");
        gate
    }

    fn wait_for_read(&mut self) -> bool {
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            let blocked: bool = self.db.query_one("SELECT EXISTS(SELECT 1 FROM pg_locks WHERE locktype = 'advisory' AND classid = 180027::oid AND objid = $1::int::oid AND objsubid = 2 AND NOT granted)", &[&self.key]).unwrap().get(0);
            if blocked {
                return true;
            }
            thread::yield_now();
        }
        false
    }

    fn release(&mut self) {
        let released: bool = self
            .db
            .query_one("SELECT pg_advisory_unlock(180027, $1)", &[&self.key])
            .unwrap()
            .get(0);
        assert!(released, "test must own the gate lock");
    }
}

impl Drop for ReadGate {
    fn drop(&mut self) {
        let _ = self
            .db
            .query_one("SELECT pg_advisory_unlock(180027, $1)", &[&self.key]);
        if let Err(error) = self.db.batch_execute(&format!(
            "DROP TRIGGER IF EXISTS {} ON security_audit_events; DROP FUNCTION IF EXISTS {}();",
            self.name, self.name
        )) {
            eprintln!("failed to remove disposable medication read gate: {error}");
        }
    }
}

fn stale_scalar(in_request: bool) {
    let (fixture, mut credential) = authentication::fixture_with_current_bearer(if in_request {
        "198.18.27.12"
    } else {
        "198.18.27.13"
    });
    let browser = Arc::new(Target::from_env());
    login(
        &browser,
        &fixture,
        if in_request {
            "198.18.27.2"
        } else {
            "198.18.27.3"
        },
    );
    let owner = Target::from_env();
    let id = create(
        &owner,
        &fixture,
        if in_request {
            "Contract interleaved first option"
        } else {
            "Contract first option before read"
        },
    );
    let api = format!(
        "/api/v1/households/{}/medications/{id}",
        fixture.household_id
    );
    let web = format!("/households/{}/medications/{id}", fixture.household_slug);
    let edit = browser.get_html(&format!("{web}/edit"));
    assert_eq!(edit.status().as_u16(), 200);
    let captured = draft(&fixture, &edit.text().unwrap());
    let mut gate = in_request.then(|| ReadGate::new(&fixture, id));
    let (worker, inserted) = if let Some(gate) = gate.as_mut() {
        let submitted = captured.clone();
        let path = web.clone();
        let submitting_browser = Arc::clone(&browser);
        let worker = thread::spawn(move || submitting_browser.post_browser_form(&path, &submitted));
        if !gate.wait_for_read() {
            gate.release();
            let response = worker.join().expect("browser request thread");
            panic!(
                "browser medication read never reached deterministic gate; response {}",
                response.status()
            );
        }
        let inserted = option(&owner, &fixture, id);
        (Some(worker), inserted)
    } else {
        (None, option(&owner, &fixture, id))
    };
    let before_response = owner.get(&api, Some(&fixture.access_token));
    assert_eq!(before_response.status().as_u16(), 200);
    let new_etag = before_response
        .headers()
        .get("etag")
        .and_then(|value| value.to_str().ok())
        .expect("parent version after first option");
    assert_ne!(
        new_etag,
        captured.iter().find(|(name, _)| name == "etag").unwrap().1,
        "first option must make the captured form version stale"
    );
    let before = before_response.json::<Value>().unwrap()["data"].clone();
    let stock_path = format!("{api}/stock_removals");
    let stock_before = read(&owner, &fixture, &stock_path);
    let option_path = format!(
        "/api/v1/households/{}/dosage_options/{}",
        fixture.household_id,
        inserted["id"].as_i64().unwrap()
    );
    assert_eq!(read(&owner, &fixture, &option_path), inserted);
    let rejected = if let Some(worker) = worker {
        gate.as_mut().unwrap().release();
        worker.join().expect("browser request thread")
    } else {
        browser.post_browser_form(&web, &captured)
    };
    let status = rejected.status().as_u16();
    let html = rejected.text().unwrap();
    assert_eq!(
        read(&owner, &fixture, &api),
        before,
        "stale edit must not persist medication changes"
    );
    assert_eq!(
        read(&owner, &fixture, &option_path),
        inserted,
        "stale edit must not overwrite option/stock"
    );
    assert_eq!(
        read(&owner, &fixture, &stock_path),
        stock_before,
        "stale edit must not add stock removals"
    );
    assert_eq!(
        status, 409,
        "first option insertion must reject stale scalar form: {html}"
    );
    assert_retained(&html, &captured);
    assert!(credential.revoke(), "remove current test-only bearer");
}

#[test]
fn first_option_inserted_before_medication_read_rejects_stale_scalar_draft_without_writes() {
    stale_scalar(false);
}

#[test]
fn first_option_inserted_between_medication_and_options_reads_rejects_stale_scalar_draft_without_writes()
 {
    stale_scalar(true);
}

#[test]
fn missing_empty_and_whitespace_preconditions_reject_scalar_edits_without_writes() {
    let (fixture, mut credential) = authentication::fixture_with_current_bearer("198.18.27.14");
    let browser = Target::from_env();
    login(&browser, &fixture, "198.18.27.4");
    let owner = Target::from_env();
    let id = create(&owner, &fixture, "Contract missing scalar preconditions");
    let api = format!(
        "/api/v1/households/{}/medications/{id}",
        fixture.household_id
    );
    let web = format!("/households/{}/medications/{id}", fixture.household_slug);
    let before = read(&owner, &fixture, &api);
    for etag in [None, Some(""), Some("   \t ")] {
        let edit = browser.get_html(&format!("{web}/edit"));
        assert_eq!(edit.status().as_u16(), 200);
        let mut fields = draft(&fixture, &edit.text().unwrap());
        fields.retain(|(name, _)| name != "etag");
        if let Some(etag) = etag {
            fields.push(("etag".into(), etag.into()));
        }
        let response = browser.post_browser_form(&web, &fields);
        let status = response.status().as_u16();
        let html = response.text().unwrap();
        assert_eq!(read(&owner, &fixture, &api), before);
        assert_eq!(
            status, 428,
            "missing or blank ETag must require a fresh form: {etag:?}: {html}"
        );
        assert_retained(&html, &fields);
    }
    assert!(credential.revoke(), "remove current test-only bearer");
}

#[test]
fn current_identity_form_cannot_override_option_dose_or_stock_with_scalar_fields() {
    let (fixture, mut credential) = authentication::fixture_with_current_bearer("198.18.27.15");
    let browser = Target::from_env();
    login(&browser, &fixture, "198.18.27.5");
    let owner = Target::from_env();
    let id = create(&owner, &fixture, "Contract malicious scalar override");
    let inserted = option(&owner, &fixture, id);
    let api = format!(
        "/api/v1/households/{}/medications/{id}",
        fixture.household_id
    );
    let web = format!("/households/{}/medications/{id}", fixture.household_slug);
    let edit = browser.get_html(&format!("{web}/edit"));
    assert_eq!(edit.status().as_u16(), 200);
    let captured = draft(&fixture, &edit.text().unwrap());
    let before = read(&owner, &fixture, &api);
    let response = browser.post_browser_form(&web, &captured);
    assert_eq!(response.status().as_u16(), 400);
    assert_eq!(read(&owner, &fixture, &api), before);
    let option_path = format!(
        "/api/v1/households/{}/dosage_options/{}",
        fixture.household_id,
        inserted["id"].as_i64().unwrap()
    );
    assert_eq!(read(&owner, &fixture, &option_path), inserted);
    assert!(credential.revoke(), "remove current test-only bearer");
}
